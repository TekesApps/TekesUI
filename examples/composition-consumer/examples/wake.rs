//! Independent public-API notification consumer. No Gallery, Tokio or renderer.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    if let Some(audit) = macos::WakeAcceptance::from_env() {
        macos::run_audit(audit);
        return;
    }
    macos::run_minimal();
}

#[cfg(target_os = "macos")]
mod macos {
    use std::{
        path::PathBuf,
        sync::{
            Arc, Mutex,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        },
        time::Duration,
    };
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, MenuBar, WakeResult, Window, WindowConfig, WindowHandler, WindowWake,
    };

    pub(super) fn run_minimal() {
        #[derive(Default)]
        struct Host {
            snapshot: Arc<AtomicUsize>,
            proxy: Arc<Mutex<Option<WindowWake>>>,
            notifications: usize,
        }
        impl WindowHandler for Host {
            fn handle(&mut self, event: Event, window: &mut Window) {
                match event {
                    Event::Created => {
                        window.set_animating(false);
                        let proxy = window.wake_proxy();
                        *self.proxy.lock().unwrap() = Some(proxy.clone());
                        let snapshot = self.snapshot.clone();
                        std::thread::spawn(move || {
                            for value in 1..=3 {
                                std::thread::sleep(Duration::from_millis(500));
                                snapshot.store(value, Ordering::Release);
                                if proxy.wake() == WakeResult::Closed {
                                    break;
                                }
                            }
                        });
                    }
                    Event::Wake => {
                        self.notifications += 1;
                        let value = self.snapshot.load(Ordering::Acquire);
                        window
                            .set_title(&format!("后台快照 {value} · 通知 {}", self.notifications));
                        window.request_redraw();
                    }
                    Event::CloseRequested => window.close(),
                    _ => {}
                }
            }
        }
        let host = Host::default();
        let proxy = host.proxy.clone();
        App::new(AppConfig {
            quit_after_last_window_closed: true,
            ..AppConfig::default()
        })
        .run(WindowConfig::default(), MenuBar::default(), host);
        if let Some(proxy) = proxy.lock().unwrap().as_ref() {
            assert_eq!(proxy.wake(), WakeResult::Closed);
            assert_eq!(proxy.clone().wake(), WakeResult::Closed);
        }
    }

    pub(super) fn run_audit(audit: WakeAcceptance) {
        App::new(AppConfig {
            quit_after_last_window_closed: true,
            ..AppConfig::default()
        })
        .run(WindowConfig::default(), MenuBar::default(), audit);
    }

    #[derive(Debug)]
    pub(super) struct WakeAcceptance {
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
        pub(super) fn from_env() -> Option<Self> {
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

        fn save(&self, closed: bool, after_close: Option<WakeResult>) {
            std::fs::create_dir_all(&self.out).unwrap();
            let record = serde_json::json!({
                "snapshot": self.value.load(Ordering::Acquire),
                "notifications": self.notifications,
                "redraws": self.redraws,
                "input_events": self.input_events,
                "animation_events": self.animation_events,
                "audit_checkpoint": self.done.load(Ordering::Acquire),
                "trace": self.trace,
                "closed": closed,
                "after_close": after_close.map(|r| format!("{r:?}")),
                "scope": "native event-loop correctness; no physical IME/VoiceOver/performance claim"
            });
            std::fs::write(
                self.out.join("wake-events.json"),
                serde_json::to_vec_pretty(&record).unwrap(),
            )
            .unwrap();
        }
    }

    impl WindowHandler for WakeAcceptance {
        fn handle(&mut self, event: Event, window: &mut Window) {
            match &event {
                Event::Created => {
                    window.set_animating(false);
                    let proxy = window.wake_proxy();
                    self.proxy = Some(proxy.clone());
                    let value = self.value.clone();
                    let done = self.done.clone();
                    std::thread::spawn(move || {
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
                                        let _ = proxy.wake();
                                    }
                                })
                            })
                            .collect();
                        for worker in workers {
                            worker.join().unwrap();
                        }
                        std::thread::sleep(Duration::from_secs(2));
                        done.store(true, Ordering::Release);
                        let _ = proxy.wake();
                    });
                }
                Event::Wake => {
                    self.notifications += 1;
                    let value = self.value.load(Ordering::Acquire);
                    self.trace.push(serde_json::json!({
                        "event": "wake",
                        "snapshot": value,
                        "redraws_before": self.redraws,
                        "animating": window.is_animating(),
                        "audit_checkpoint": self.done.load(Ordering::Acquire)
                    }));
                    window.set_title(&format!(
                        "TekesUI · 后台快照 {value} · 通知 {}",
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
    }

    impl Drop for WakeAcceptance {
        fn drop(&mut self) {
            self.save(true, self.proxy.as_ref().map(WindowWake::wake));
        }
    }
}
