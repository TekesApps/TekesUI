//! Real ProcessComponent navigation acceptance. No UI/GPU objects are constructed.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tekes_ui_app_host::navigation::*;
use tekes_ui_module_runtime::{component::*, *};
static FACTORY: AtomicBool = AtomicBool::new(false);
struct ProbeModule(NavigationModule);
impl ProcessModule for ProbeModule {
    fn handle(&mut self, request: RequestBody, e: &ComponentEmitter) -> Result<Value> {
        if let RequestBody::Message { topic, payload } = &request {
            if topic == "acceptance-fault" {
                self.0.inject_acceptance_fault(match payload.as_str() {
                    Some("before-rename") => Fault::BeforeRename,
                    Some("after-rename") => Fault::AfterRename,
                    Some("before-cleanup") => Fault::BeforeCleanup,
                    _ => Fault::None,
                });
                return Ok(Value::Null);
            }
            if topic == "acceptance-pause" {
                e.emit(ComponentEvent::Message {
                    topic: "paused".into(),
                    payload: Value::Null,
                })?;
                loop {
                    std::thread::park();
                }
            }
        }
        self.0.handle(request, e)
    }
}
fn factory() -> Box<dyn ProcessModule> {
    FACTORY.store(true, Ordering::SeqCst);
    Box::new(ProbeModule(NavigationModule::default()))
}
struct Client {
    p: ProcessComponent,
    responses: HashMap<u64, Result<Value, String>>,
    toolbar: Vec<ToolbarAction>,
    generation: u64,
    pid: u32,
    paused: bool,
}
impl Client {
    fn new(epoch: u64) -> Result<Self> {
        let p = ProcessComponent::spawn(ProcessComponentOptions::new(MODULE_ID, epoch), || {})?;
        let mut c = Self {
            p,
            responses: HashMap::new(),
            toolbar: vec![],
            generation: 0,
            pid: 0,
            paused: false,
        };
        c.wait(|c| {
            matches!(
                c.p.status(),
                ProcessStatus::Ready { .. } | ProcessStatus::Failed { .. }
            )
        })?;
        let ProcessStatus::Ready { pid, capabilities } = c.p.status() else {
            bail!("{:?}", c.p.status());
        };
        ensure!(!capabilities.view && pid != std::process::id());
        c.pid = pid;
        Ok(c)
    }
    fn poll(&mut self) {
        for packet in self.p.drain_events() {
            match packet.output {
                Output::Response { request, result } => {
                    self.responses.insert(request, result);
                }
                Output::Event(ComponentEvent::Toolbar {
                    revision,
                    scope,
                    items,
                }) => {
                    assert_eq!(scope, ToolbarScope::Window { window: 1 });
                    assert!(revision > self.generation);
                    self.generation = revision;
                    self.toolbar = items;
                }
                Output::Event(ComponentEvent::Message { topic, .. }) if topic == "paused" => {
                    self.paused = true
                }
                _ => {}
            }
        }
    }
    fn wait(&mut self, mut predicate: impl FnMut(&mut Self) -> bool) -> Result<()> {
        let until = Instant::now() + Duration::from_secs(15);
        loop {
            self.poll();
            if predicate(self) {
                return Ok(());
            }
            ensure!(Instant::now() < until, "deadline: {:?}", self.p.status());
            ensure!(
                !matches!(self.p.status(), ProcessStatus::Failed { .. }),
                "{:?}",
                self.p.status()
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    fn raw(&mut self, topic: &str, payload: Value) -> Result<Value> {
        let id = self.p.request(RequestBody::Message {
            topic: topic.into(),
            payload,
        })?;
        self.answer(id)
    }
    fn answer(&mut self, id: u64) -> Result<Value> {
        self.wait(|c| c.responses.contains_key(&id))?;
        self.responses
            .remove(&id)
            .unwrap()
            .map_err(anyhow::Error::msg)
    }
    fn call(&mut self, c: Command) -> Result<Reply> {
        Ok(serde_json::from_value(
            self.raw("navigation", serde_json::to_value(c)?)?,
        )?)
    }
    fn fault(&mut self, name: &str) -> Result<()> {
        self.raw("acceptance-fault", json!(name))?;
        Ok(())
    }
    fn kill(&mut self) -> Result<()> {
        self.p.close();
        self.wait(|c| c.p.status() == ProcessStatus::Closed)
    }
    fn open(&mut self, root: &Path) -> Result<Reply> {
        self.call(Command::Open {
            directory: root.into(),
            history_id: "prototype".into(),
            window: 1,
        })
    }
}
fn publish(target: Option<&Entry>) -> Result<String> {
    if let Some(e) = target {
        ensure!(
            ["projects", "thread", "annotation"].contains(&e.route.as_str()),
            "route not installed"
        );
    }
    Ok(format!(
        "published:{}",
        target.map_or("empty", |e| e.id.as_str())
    ))
}
fn ready(c: &mut Client, h: &mut NavigationHost, reply: Reply) -> Result<RecoveryApplied> {
    let Reply::RestoreOffer(offer) = reply else {
        bail!("expected offer, got {reply:?}");
    };
    let applied = h.restore(offer, publish)?;
    ensure!(!h.input_open);
    let Reply::RecoveryReady(confirmed) = c.call(Command::RecoveryApplied(applied.clone()))? else {
        bail!("expected ready");
    };
    h.ready(&confirmed)?;
    Ok(applied)
}
fn entry(id: &str) -> Entry {
    Entry {
        id: id.into(),
        route: "thread".into(),
        params: json!({"thread_id":id}),
    }
}
fn execute(c: &mut Client, h: &mut NavigationHost, id: &str) -> Result<Applied> {
    let Reply::Execute(tx) = c.call(Command::Navigate(Action::Push(entry(id))))? else {
        bail!("expected execute");
    };
    h.execute(tx, publish)
}
fn commit(c: &mut Client, h: &mut NavigationHost, applied: Applied) -> Result<History> {
    let Reply::HistoryCommitted(history) = c.call(Command::Applied(applied.clone()))? else {
        bail!("expected committed");
    };
    h.committed(&applied.transaction, &history)?;
    Ok(history)
}
fn fresh(root: &Path, epoch: u64) -> Result<(Client, NavigationHost)> {
    fs::create_dir_all(root)?;
    let mut c = Client::new(epoch)?;
    let mut h = NavigationHost::new(c.p.session().clone());
    let offer = c.open(root)?;
    ready(&mut c, &mut h, offer)?;
    ensure!(
        c.toolbar.len() == 2
            && c.toolbar.iter().all(|i| i.command.title.is_empty()
                && i.icon.is_some()
                && i.accessible_name.is_some()
                && !i.command.geometry_dependent)
    );
    Ok((c, h))
}
fn restarted(root: &Path, epoch: u64, h: &mut NavigationHost) -> Result<Client> {
    let mut c = Client::new(epoch)?;
    h.rebind(c.p.session().clone());
    let r = c.open(root)?;
    ready(&mut c, h, r)?;
    Ok(c)
}
fn state(c: &mut Client) -> Result<(History, bool)> {
    let Reply::State { history, ready, .. } = c.call(Command::Inspect)? else {
        bail!("state expected");
    };
    Ok((history, ready))
}
fn accept(root: &Path) -> Result<Value> {
    fs::create_dir_all(root)?;
    let mut records = vec![];
    // Normal push/back/forward, branching, duplicate receipt and payload collision.
    let d = root.join("normal");
    let (mut c, mut h) = fresh(&d, 100)?;
    let mut pids = vec![c.pid];
    let a = execute(&mut c, &mut h, "A")?;
    ensure!(
        !d.join("nav.json").exists(),
        "intent changed committed JSON"
    );
    let duplicate = h.execute(a.transaction.clone(), |_| bail!("duplicate execution"))?;
    ensure!(duplicate == a);
    let mut forged = a.transaction.clone();
    forged.target = Some(entry("forged"));
    ensure!(h.execute(forged, publish).is_err());
    let committed = commit(&mut c, &mut h, a.clone())?;
    ensure!(committed.revision == 1);
    ensure!(
        h.execute(a.transaction.clone(), publish).is_err(),
        "retired ID replayed"
    );
    let b = execute(&mut c, &mut h, "B")?;
    commit(&mut c, &mut h, b)?;
    ensure!(
        c.toolbar
            .iter()
            .find(|i| i.command.id == BACK)
            .unwrap()
            .command
            .enabled
    );
    let old_registration = c.generation;
    let request = c.p.invoke(BACK)?;
    let Reply::Execute(tx) = serde_json::from_value(c.answer(request)?)? else {
        bail!("toolbar did not request execution");
    };
    let applied = h.execute(tx, publish)?;
    commit(&mut c, &mut h, applied)?;
    ensure!(
        c.p.request(RequestBody::Invoke {
            command: BACK,
            registration: old_registration,
            window: 1
        })
        .is_err()
    );
    let request = c.p.invoke(FORWARD)?;
    let Reply::Execute(tx) = serde_json::from_value(c.answer(request)?)? else {
        bail!("forward missing");
    };
    let applied = h.execute(tx, publish)?;
    commit(&mut c, &mut h, applied)?;
    let Reply::Execute(tx) = c.call(Command::Navigate(Action::Back))? else {
        bail!("back missing");
    };
    let applied = h.execute(tx, publish)?;
    commit(&mut c, &mut h, applied)?;
    let branch = execute(&mut c, &mut h, "C")?;
    let hist = commit(&mut c, &mut h, branch)?;
    ensure!(
        hist.entries
            .iter()
            .map(|e| e.id.as_str())
            .collect::<Vec<_>>()
            == vec!["A", "C"]
    );
    let mut competing = Client::new(100)?;
    ensure!(competing.open(&d).is_err(), "second writer accepted");
    competing.kill()?;
    records.push(json!({"case":"normal-toolbar-branch-idempotence-single-writer","revision":hist.revision,"history_id":hist.history_id,"session":c.p.session(),"registration":c.generation}));
    c.kill()?;

    // Failed executions are terminal too; they cannot be re-executed by a duplicate request.
    let d = root.join("failed-execution");
    let (mut c, mut h) = fresh(&d, 109)?;
    let bad = Entry {
        id: "bad".into(),
        route: "not-installed".into(),
        params: json!({}),
    };
    let Reply::Execute(tx) = c.call(Command::Navigate(Action::Push(bad)))? else {
        bail!("execute missing");
    };
    ensure!(h.execute(tx.clone(), publish).is_err());
    ensure!(
        h.execute(tx.clone(), |_| panic!("failed execution repeated"))
            .is_err()
    );
    ensure!(h.failed_receipt(&tx).is_ok());
    c.kill()?;
    let mut c = Client::new(109)?;
    h.rebind(c.p.session().clone());
    let Reply::NeedReceipt(restored_tx) = c.open(&d)? else {
        bail!("failure receipt query missing");
    };
    ensure!(restored_tx == tx && h.failed_receipt(&tx).is_ok());
    ensure!(c.call(Command::ReconcileFailed(tx.clone()))? == Reply::Aborted);
    h.aborted(&tx)?;
    let offer = c.call(Command::Recover)?;
    ready(&mut c, &mut h, offer)?;
    ensure!(state(&mut c)?.0.revision == 0 && !d.join("nav.json").exists());
    let a = execute(&mut c, &mut h, "valid-after-failure")?;
    commit(&mut c, &mut h, a.clone())?;
    let Reply::HistoryCommitted(hist) = c.call(Command::Applied(a.clone()))? else {
        bail!("duplicate success not idempotent");
    };
    ensure!(hist.revision == 1);
    records.push(json!({"case":"failed-result-reconcile-and-duplicate-commit","failed_navigation_id":tx.navigation_id,"revision":hist.revision,"failed_execution_repeated":false}));
    c.kill()?;

    // Corrupt/future files are retained byte-for-byte, never silently initialized.
    for (name,bytes) in [("corrupt",b"{broken".as_slice()),("future",br#"{"version":99,"history_id":"prototype","revision":0,"last_commit_id":null,"cursor":null,"entries":[]}"#.as_slice())] {
        let d=root.join(name);fs::create_dir_all(&d)?;fs::write(d.join("nav.json"),bytes)?;
        let mut c=Client::new(110)?;ensure!(c.open(&d).is_err());ensure!(fs::read(d.join("nav.json"))?==bytes);c.kill()?;
        records.push(json!({"case":name,"original_bytes_preserved":true}));
    }

    // Both processes lose volatile execution receipts: keep unknown intent gated, do not replay.
    let d = root.join("missing-receipt");
    let (mut c, mut h) = fresh(&d, 111)?;
    let applied = execute(&mut c, &mut h, "unknown")?;
    c.kill()?;
    drop(h);
    let mut c = Client::new(112)?;
    let mut h = NavigationHost::new(c.p.session().clone());
    let Reply::NeedReceipt(tx) = c.open(&d)? else {
        bail!("unknown receipt must be requested");
    };
    ensure!(h.receipt(&tx).is_err() && !h.input_open);
    ensure!(c.call(Command::Applied(applied)).is_err());
    ensure!(
        c.call(Command::Navigate(Action::Push(entry("must-not-run"))))
            .is_err()
    );
    ensure!(state(&mut c)?.0.revision == 0 && d.join("nav.pending.json").exists());
    h.rebind(c.p.session().clone());
    records.push(json!({"case":"both-restarted-receipt-unavailable","navigation_id":tx.navigation_id,"host_epoch":112,"gate_open":h.input_open,"explicit_unknown":true,"revision":0}));
    c.kill()?;

    // 1: Drop the final confirmation at the Host boundary; retry same Applied.
    let d = root.join("lost-ready");
    let mut c = Client::new(101)?;
    pids.push(c.pid);
    let mut h = NavigationHost::new(c.p.session().clone());
    let Reply::RestoreOffer(offer) = c.open(&d)? else {
        bail!("offer missing");
    };
    let applied = h.restore(offer, publish)?;
    let ignored = c.call(Command::RecoveryApplied(applied.clone()))?;
    drop(ignored);
    ensure!(!h.input_open, "lost Ready opened input");
    let Reply::RecoveryReady(confirmed) = c.call(Command::RecoveryApplied(applied.clone()))? else {
        bail!("retry failed");
    };
    h.ready(&confirmed)?;
    let tx = execute(&mut c, &mut h, "A")?;
    commit(&mut c, &mut h, tx)?;
    ensure!(
        c.call(Command::RecoveryApplied(applied.clone())).is_err(),
        "old baseline revived"
    );
    records.push(json!({"case":"lost-RecoveryReady","session":c.p.session(),"recovery_id":applied.offer.recovery_id,"revision":0,"gate_before_retry":false,"gate_after_retry":true,"stale_baseline_rejected":true}));
    c.kill()?;

    // 2a: nav cleans pending, dies before Host receives HistoryCommitted.
    let d = root.join("cleaned-pending-lost-commit");
    let (mut c, mut h) = fresh(&d, 102)?;
    let applied = execute(&mut c, &mut h, "A")?;
    let Reply::HistoryCommitted(_) = c.call(Command::Applied(applied.clone()))? else {
        bail!("commit missing");
    };
    ensure!(!d.join("nav.pending.json").exists());
    ensure!(h.receipt(&applied.transaction).is_ok());
    c.kill()?;
    let mut c = restarted(&d, 102, &mut h)?;
    pids.push(c.pid);
    let Reply::HistoryCommitted(hist) =
        c.call(Command::CommitStatus(applied.transaction.clone()))?
    else {
        bail!("commit query missing");
    };
    h.committed(&applied.transaction, &hist)?;
    ensure!(
        hist.revision == 1 && hist.entries.len() == 1 && h.receipt(&applied.transaction).is_err()
    );
    records.push(json!({"case":"pending-cleaned-confirmation-lost","session":c.p.session(),"navigation_id":applied.transaction.navigation_id,"revision":hist.revision,"last_commit_id":hist.last_commit_id,"receipt_reconciled":true}));
    c.kill()?;

    // 2b: Host retires receipt; nav dies before pending cleanup.
    let d = root.join("retired-receipt-pending-remains");
    let (mut c, mut h) = fresh(&d, 103)?;
    c.fault("before-cleanup")?;
    let applied = execute(&mut c, &mut h, "A")?;
    let hist = commit(&mut c, &mut h, applied.clone())?;
    ensure!(d.join("nav.pending.json").exists() && h.receipt(&applied.transaction).is_err());
    c.kill()?;
    let mut c = restarted(&d, 103, &mut h)?;
    pids.push(c.pid);
    ensure!(!d.join("nav.pending.json").exists());
    ensure!(state(&mut c)?.0 == hist);
    records.push(json!({"case":"receipt-retired-before-pending-cleanup","session":c.p.session(),"navigation_id":applied.transaction.navigation_id,"revision":hist.revision,"last_commit_id":hist.last_commit_id,"reconciled_without_receipt":true}));
    c.kill()?;

    // Execution happened, nav dies before journaling success; retained receipt is queried.
    let d = root.join("receipt-reconcile");
    let (mut c, mut h) = fresh(&d, 104)?;
    let applied = execute(&mut c, &mut h, "A")?;
    c.kill()?;
    let mut c = Client::new(104)?;
    h.rebind(c.p.session().clone());
    let Reply::NeedReceipt(tx) = c.open(&d)? else {
        bail!("receipt query missing");
    };
    ensure!(
        c.call(Command::Applied(applied.clone())).is_err(),
        "old ordinary ack accepted"
    );
    let receipt = h.receipt(&tx)?;
    let Reply::HistoryCommitted(hist) = c.call(Command::Reconcile(receipt))? else {
        bail!("reconcile failed");
    };
    h.committed(&tx, &hist)?;
    let offer = c.call(Command::Recover)?;
    ready(&mut c, &mut h, offer)?;
    records.push(json!({"case":"nav-restart-before-success-journal","revision":hist.revision,"navigation_id":tx.navigation_id,"old_ack_rejected":true}));
    c.kill()?;

    // Replacement boundary faults must not publish commit or permit the next history mutation.
    for (epoch, fault) in [(105, "before-rename"), (106, "after-rename")] {
        let d = root.join(fault);
        let (mut c, mut h) = fresh(&d, epoch)?;
        let applied = execute(&mut c, &mut h, "A")?;
        c.fault(fault)?;
        ensure!(c.call(Command::Applied(applied.clone())).is_err());
        ensure!(
            d.join("nav.pending.json").exists()
                && c.call(Command::Navigate(Action::Push(entry("B")))).is_err()
        );
        ensure!(d.join("nav.json").exists() == (fault == "after-rename"));
        ensure!(state(&mut c)?.0.revision == 0);
        c.kill()?;
        let mut c = restarted(&d, epoch, &mut h)?;
        let (hist, is_ready) = state(&mut c)?;
        ensure!(hist.revision == 1 && hist.entries.len() == 1 && is_ready);
        records.push(json!({"case":fault,"replacement_visible":fault=="after-rename","published_before_recovery":false,"revision_after_recovery":hist.revision,"session":c.p.session()}));
        c.kill()?;
    }

    // 3: nav crashes both before Ready creation and before its delivery.
    for phase in ["before-ready", "after-ready"] {
        let d = root.join(format!("nav-crash-{phase}"));
        let (mut c, mut h) = fresh(&d, 107)?;
        let Reply::RestoreOffer(offer) = c.call(Command::Recover)? else {
            bail!("offer missing");
        };
        let applied = h.restore(offer, publish)?;
        if phase == "after-ready" {
            let ignored = c.call(Command::RecoveryApplied(applied.clone()))?;
            drop(ignored);
        }
        c.kill()?;
        let mut c = Client::new(107)?;
        h.rebind(c.p.session().clone());
        ensure!(h.ready(&applied).is_err() && !h.input_open);
        ensure!(c.call(Command::RecoveryApplied(applied.clone())).is_err());
        let offer = c.open(&d)?;
        ready(&mut c, &mut h, offer)?;
        records.push(json!({"case":format!("nav-crash-{phase}"),"session":c.p.session(),"old_recovery_id":applied.offer.recovery_id,"stale_ready_rejected":true,"gate_after_recovery":h.input_open}));
        c.kill()?;
    }

    // A hung worker cannot block the Host-side API/event loop; no GUI acceptance is implied.
    let d = root.join("nonblocking");
    let (mut c, _) = fresh(&d, 108)?;
    c.p.request(RequestBody::Message {
        topic: "acceptance-pause".into(),
        payload: Value::Null,
    })?;
    c.wait(|c| c.paused)?;
    let mut ticks = 0;
    for _ in 0..100 {
        let _ = c.p.status();
        let _ = c.p.drain_events();
        ticks += 1;
    }
    c.kill()?;
    records.push(json!({"case":"paused-worker-host-event-pump","ticks":ticks,"closed_and_reaped":true,"physical_gui_verified":false}));

    // Real Host process crashes, not only replacement of an in-memory adapter.
    for phase in ["before-ready", "after-ready"] {
        let d = root.join(format!("host-crash-{phase}"));
        fs::create_dir_all(&d)?;
        let mut child = std::process::Command::new(std::env::current_exe()?)
            .arg("--host-crash-probe")
            .arg(&d)
            .arg(phase)
            .stdout(std::process::Stdio::null())
            .spawn()?;
        let deadline = Instant::now() + Duration::from_secs(15);
        while !d.join("host-paused.json").exists() {
            ensure!(Instant::now() < deadline, "host probe deadline");
            ensure!(child.try_wait()?.is_none(), "host probe exited");
            std::thread::sleep(Duration::from_millis(5));
        }
        let old: Value = serde_json::from_slice(&fs::read(d.join("host-paused.json"))?)?;
        child.kill()?;
        child.wait()?;
        // Runtime's stdin closure stops the orphaned service worker; wait for its namespace lock.
        let deadline = Instant::now() + Duration::from_secs(15);
        loop {
            let lock = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .open(d.join("nav.lock"))?;
            if lock.try_lock().is_ok() {
                break;
            }
            ensure!(
                Instant::now() < deadline,
                "orphaned navigation did not exit"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        let (mut c, mut h) = fresh(&d, 201)?;
        let stale: RecoveryApplied = serde_json::from_value(old["applied"].clone())?;
        ensure!(h.ready(&stale).is_err());
        ensure!(c.call(Command::RecoveryApplied(stale)).is_err());
        let (hist, is_ready) = state(&mut c)?;
        ensure!(hist.revision == 1 && hist.current() == Some(&entry("persisted")) && is_ready);
        records.push(json!({"case":format!("real-Host-kill-{phase}"),"old_host_pid":old["host_pid"],"old_nav_pid":old["nav_pid"],"host_epoch":201,"session":c.p.session(),"revision":hist.revision,"last_commit_id":hist.last_commit_id,"stale_ready_rejected":true}));
        c.kill()?;
    }
    ensure!(!FACTORY.load(Ordering::SeqCst), "factory ran in parent");
    let result = json!({"result":"passed","scope":"real headless subprocess navigation and storage, not native GUI/three-View assembly acceptance","parent_pid":std::process::id(),"navigation_pids":pids,"headless":true,"factory_in_parent":false,"cases":records});
    fs::write(
        root.join("acceptance.json"),
        serde_json::to_vec_pretty(&result)?,
    )?;
    Ok(result)
}
fn host_probe(root: &Path, phase: &str) -> Result<()> {
    let (mut c, mut h) = fresh(root, 200)?;
    let applied = execute(&mut c, &mut h, "persisted")?;
    commit(&mut c, &mut h, applied)?;
    let Reply::RestoreOffer(offer) = c.call(Command::Recover)? else {
        bail!("offer missing");
    };
    let applied = h.restore(offer, publish)?;
    if phase == "after-ready" {
        let ignored = c.call(Command::RecoveryApplied(applied.clone()))?;
        drop(ignored);
    }
    // Atomic marker for the external test supervisor to kill this actual Host process.
    let marker = json!({"host_pid":std::process::id(),"nav_pid":c.pid,"applied":applied});
    fs::write(root.join("host-paused.tmp"), serde_json::to_vec(&marker)?)?;
    fs::rename(root.join("host-paused.tmp"), root.join("host-paused.json"))?;
    loop {
        std::thread::park();
    }
}
fn main() -> Result<()> {
    let args = std::env::args().collect::<Vec<_>>();
    if run_component_worker_from_args(
        &[ComponentDefinition {
            id: MODULE_ID,
            capabilities: Capabilities::HEADLESS,
            create: factory,
        }],
        &args,
    )? {
        return Ok(());
    }
    if args.get(1).map(String::as_str) == Some("--host-crash-probe") {
        return host_probe(
            Path::new(args.get(2).context("missing root")?),
            args.get(3).context("missing phase")?,
        );
    }
    let root = if let Some(p) = args.get(1) {
        PathBuf::from(p)
    } else {
        std::env::temp_dir().join(format!(
            "tekes-navigation-{}-{}",
            std::process::id(),
            SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos()
        ))
    };
    ensure!(
        !root.exists() || fs::read_dir(&root)?.next().is_none(),
        "acceptance requires a fresh fixture directory"
    );
    println!("{}", accept(&root)?);
    Ok(())
}
