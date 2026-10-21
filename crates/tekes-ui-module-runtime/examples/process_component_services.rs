//! Independent headless consumer: no UI, Renderer, Scene, Atlas or Mach transport.
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::{
    sync::atomic::{AtomicBool, Ordering},
    time::{Duration, Instant},
};
use tekes_ui_module_runtime::{component::*, *};
static CREATED: AtomicBool = AtomicBool::new(false);
struct Counter(u64);
impl ProcessModule for Counter {
    fn started(&mut self, emitter: &ComponentEmitter) -> Result<()> {
        emitter.emit(ComponentEvent::Toolbar {
            revision: 1,
            scope: ToolbarScope::Window { window: 1 },
            items: vec![ToolbarAction {
                command: tekes_ui_app_protocol::ToolbarItem {
                    id: 7,
                    title: String::new(),
                    enabled: true,
                    geometry_dependent: false,
                },
                icon: Some("next".into()),
                accessible_name: Some("Next".into()),
                tooltip: Some("Advance".into()),
            }],
        })
    }
    fn handle(&mut self, request: RequestBody, emitter: &ComponentEmitter) -> Result<Value> {
        match request {
            RequestBody::Message { topic, .. } if topic == "hang" => {
                emitter.emit(ComponentEvent::Message {
                    topic: "entered-hang".into(),
                    payload: Value::Null,
                })?;
                loop {
                    std::thread::park();
                }
            }
            RequestBody::Message { .. } | RequestBody::Invoke { .. } => self.0 += 1,
            RequestBody::Stop | RequestBody::Render { .. } => unreachable!(),
        }
        Ok(json!(self.0))
    }
}
fn create() -> Box<dyn ProcessModule> {
    CREATED.store(true, Ordering::SeqCst);
    Box::new(Counter(0))
}
fn until(mut predicate: impl FnMut() -> bool) -> Result<()> {
    let deadline = Instant::now() + Duration::from_secs(10);
    while !predicate() {
        ensure!(Instant::now() < deadline, "correctness deadline exceeded");
        std::thread::sleep(Duration::from_millis(5));
    }
    Ok(())
}
fn ready(process: &ProcessComponent) -> Result<u32> {
    until(|| {
        matches!(
            process.status(),
            ProcessStatus::Ready { .. } | ProcessStatus::Failed { .. }
        )
    })?;
    let ProcessStatus::Ready { pid, .. } = process.status() else {
        anyhow::bail!("{:?}", process.status());
    };
    until(|| {
        process
            .drain_events()
            .iter()
            .any(|p| matches!(p.output, Output::Event(ComponentEvent::Toolbar { .. })))
    })?;
    Ok(pid)
}
fn response(process: &ProcessComponent, id: u64) -> Result<Value> {
    let mut result = None;
    until(|| {
        for packet in process.drain_events() {
            if let Output::Response {
                request,
                result: value,
            } = packet.output
                && request == id
            {
                result = Some(value);
            }
        }
        result.is_some()
    })?;
    result.unwrap().map_err(anyhow::Error::msg)
}
// Deliberately invalid peers exercise parent rejection without the normal worker driver.
fn invalid_peer(mode: &str) -> Result<()> {
    use std::io::Write;
    let mut line = String::new();
    std::io::stdin().read_line(&mut line)?;
    let ClientMessage::Start {
        session, required, ..
    } = serde_json::from_str(&line)?
    else {
        anyhow::bail!("missing start");
    };
    let mut packet = ServerMessage {
        version: VERSION,
        session,
        serial: 1,
        output: Output::Ready {
            pid: std::process::id(),
            capabilities: required,
        },
    };
    if mode == "fault-session" {
        packet.session.host_epoch += 1;
    }
    println!("{}", serde_json::to_string(&packet)?);
    std::io::stdout().flush()?;
    if mode == "fault-response" {
        packet.serial += 1;
        packet.output = Output::Response {
            request: 999,
            result: Ok(Value::Null),
        };
        println!("{}", serde_json::to_string(&packet)?);
    }
    if mode == "fault-overflow" {
        for serial in 2..100 {
            packet.serial = serial;
            packet.output = Output::Event(ComponentEvent::Message {
                topic: "flood".into(),
                payload: Value::Null,
            });
            println!("{}", serde_json::to_string(&packet)?);
        }
    }
    if mode == "fault-large" {
        std::io::stdout().write_all(&vec![b'x'; 1_048_577])?;
    }
    std::io::stdout().flush()?;
    loop {
        std::thread::park();
    }
}
fn main() -> Result<()> {
    let arguments: Vec<String> = std::env::args().collect();
    if arguments.get(1).map(String::as_str) == Some("--tekesui-component")
        && arguments.get(2).is_some_and(|s| s.starts_with("fault-"))
    {
        return invalid_peer(&arguments[2]);
    }
    if run_component_worker_from_args(
        &[ComponentDefinition {
            id: "counter",
            capabilities: Capabilities::HEADLESS,
            create,
        }],
        &std::env::args().collect::<Vec<_>>(),
    )? {
        return Ok(());
    }
    ensure!(!CREATED.load(Ordering::SeqCst), "factory ran in parent");
    let first = ProcessComponent::spawn(ProcessComponentOptions::new("counter", 42), || {})?;
    let second = ProcessComponent::spawn(ProcessComponentOptions::new("counter", 42), || {})?;
    let a = ready(&first)?;
    let b = ready(&second)?;
    ensure!(a != b && a != std::process::id() && b != std::process::id());
    ensure!(first.session() != second.session());
    ensure!(first.invoke(99).is_err());
    let id = first.invoke(7)?;
    ensure!(response(&first, id)? == json!(1));
    let id = first.invoke(7)?;
    ensure!(response(&first, id)? == json!(2));
    let id = second.invoke(7)?;
    ensure!(response(&second, id)? == json!(1));
    let _hang = first.request(RequestBody::Message {
        topic: "hang".into(),
        payload: Value::Null,
    })?;
    until(|| {
        first.drain_events().iter().any(|p| matches!(&p.output, Output::Event(ComponentEvent::Message { topic, .. }) if topic == "entered-hang"))
    })?;
    // One outstanding hung request plus 63 queued requests reaches the shared bound.
    for _ in 0..63 {
        first.invoke(7)?;
    }
    ensure!(first.invoke(7).is_err(), "unbounded outstanding requests");
    first.close();
    until(|| first.status() == ProcessStatus::Closed)?;
    let id = second.invoke(7)?;
    ensure!(response(&second, id)? == json!(2), "other worker affected");
    let stop = second.request(RequestBody::Stop)?;
    ensure!(response(&second, stop)? == Value::Null);
    until(|| second.status() == ProcessStatus::Closed)?;
    // Drop while a worker is blocked, without retaining a ProcessComponent.
    let dropped = ProcessComponent::spawn(ProcessComponentOptions::new("counter", 43), || {})?;
    let c = ready(&dropped)?;
    dropped.request(RequestBody::Message {
        topic: "hang".into(),
        payload: Value::Null,
    })?;
    until(|| {
        dropped.drain_events().iter().any(|p| matches!(&p.output, Output::Event(ComponentEvent::Message { topic, .. }) if topic == "entered-hang"))
    })?;
    drop(dropped);
    until(|| {
        !std::process::Command::new("/bin/ps")
            .args(["-p", &c.to_string(), "-o", "pid="])
            .output()
            .unwrap()
            .status
            .success()
    })?;
    ensure!(!CREATED.load(Ordering::SeqCst));
    for mode in [
        "fault-session",
        "fault-response",
        "fault-overflow",
        "fault-large",
    ] {
        let faulty = ProcessComponent::spawn(ProcessComponentOptions::new(mode, 44), || {})?;
        until(|| matches!(faulty.status(), ProcessStatus::Failed { .. }))?;
        ensure!(faulty.invoke(7).is_err());
    }
    println!(
        "{}",
        json!({"result":"passed", "pids":[a,b,c],"headless":true,"factory_in_parent":false,"separate_state":true,"toolbar_without_frame":true,"hung_close_reaped":true,"drop_reaped":true,"graceful_stop":true,"other_worker_survived":true,"performance_test":false,"bounded_requests":true,"invalid_session_rejected":true,"unknown_response_rejected":true,"output_overflow_rejected":true,"oversized_output_rejected":true})
    );
    Ok(())
}
