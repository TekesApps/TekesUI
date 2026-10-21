//! Opt-in, finite correctness scenario for the public window wake contract.
//! No polling/frame pump: workers only notify after a snapshot mutation or a
//! finite audit checkpoint. This is not a latency/performance measurement.
use std::{
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    time::Duration,
};
use tekes_ui_platform_macos::{Event, WakeResult, Window, WindowWake};

#[derive(Debug)]
pub(crate) struct WakeAcceptance {
    out: PathBuf,
    value: Arc<AtomicUsize>,
    done: Arc<AtomicBool>,
    proxy: Option<WindowWake>,
    notifications: usize,
    redraws: usize,
    input_events: usize,
    animation_events: usize,
    trace: Vec<serde_json::Value>,
}
impl WakeAcceptance {
    pub(crate) fn from_env() -> Option<Self> {
        Some(Self {
            out: PathBuf::from(std::env::var_os("TEKESUI_WAKE_AUDIT_OUT")?),
            value: Default::default(),
            done: Default::default(),
            proxy: None,
            notifications: 0,
            redraws: 0,
            input_events: 0,
            animation_events: 0,
            trace: Vec::new(),
        })
    }
    /// Observes the real host path, retaining ordinary Gallery rendering.
    pub(crate) fn observe(&mut self, event: &Event, window: &Window) {
        match event {
            Event::Created => {
                window.set_animating(false);
                let proxy = window.wake_proxy();
                self.proxy = Some(proxy.clone());
                let value = self.value.clone();
                let done = self.done.clone();
                std::thread::spawn(move || {
                    // Finite pauses establish separate idle/changed periods;
                    // there is no repeated timer checking for business changes.
                    std::thread::sleep(Duration::from_secs(2));
                    value.store(1, Ordering::Release);
                    if proxy.wake() == WakeResult::Closed {
                        return;
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    let workers: Vec<_> = (0..8)
                        .map(|_| {
                            let proxy = proxy.clone();
                            let value = value.clone();
                            std::thread::spawn(move || {
                                for _ in 0..8 {
                                    value.fetch_add(1, Ordering::AcqRel);
                                    proxy.wake();
                                }
                            })
                        })
                        .collect();
                    for worker in workers {
                        worker.join().unwrap();
                    }
                    std::thread::sleep(Duration::from_secs(2));
                    done.store(true, Ordering::Release);
                    proxy.wake(); // One explicit audit checkpoint, not a poll.
                });
            }
            Event::Wake => {
                self.notifications += 1;
                let value = self.value.load(Ordering::Acquire);
                self.trace.push(serde_json::json!({"event":"wake", "snapshot":value,"redraws_before":self.redraws,"animating":window.is_animating(),"audit_checkpoint":self.done.load(Ordering::Acquire)}));
                window.set_title(&format!(
                    "TekesUI Gallery · 后台快照 {value} · 通知 {}",
                    self.notifications
                ));
                window.request_redraw();
            }
            Event::Redraw => self.redraws += 1,
            Event::AnimationFrame(_) => self.animation_events += 1,
            Event::Mouse(_)
            | Event::Key(_)
            | Event::TextCommit { .. }
            | Event::TextPreedit { .. }
            | Event::TextCommand(_)
            | Event::Scroll(_) => self.input_events += 1,
            _ => {}
        }
        self.save(false, None);
    }
    fn save(&self, closed: bool, after_close: Option<WakeResult>) {
        std::fs::create_dir_all(&self.out).unwrap();
        let record = serde_json::json!({"snapshot":self.value.load(Ordering::Acquire),"notifications":self.notifications,"redraws":self.redraws,"input_events":self.input_events,"animation_events":self.animation_events,"audit_checkpoint":self.done.load(Ordering::Acquire),"trace":self.trace,"closed":closed,"after_close":after_close.map(|r| format!("{r:?}")),"scope":"Gallery native event-loop correctness; no physical IME/VoiceOver/performance claim"});
        std::fs::write(
            self.out.join("wake-events.json"),
            serde_json::to_vec_pretty(&record).unwrap(),
        )
        .unwrap();
    }
}
impl Drop for WakeAcceptance {
    fn drop(&mut self) {
        // Shared::finish closes the proxy BEFORE dropping the handler.
        self.save(true, self.proxy.as_ref().map(WindowWake::wake));
    }
}
