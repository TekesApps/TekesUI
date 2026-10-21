//! Independent public-API notification consumer. No Gallery, Tokio or renderer.
#[cfg(not(target_os = "macos"))]
fn main() {}

#[cfg(target_os = "macos")]
fn main() {
    use std::sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    };
    use tekes_ui_platform_macos::{
        App, AppConfig, Event, MenuBar, WakeResult, Window, WindowConfig, WindowHandler, WindowWake,
    };
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
                            std::thread::sleep(std::time::Duration::from_millis(500));
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
                    window.set_title(&format!("后台快照 {value} · 通知 {}", self.notifications));
                    window.request_redraw();
                }
                Event::CloseRequested => window.close(),
                _ => {}
            }
        }
    }
    let host = Host::default();
    let proxy = host.proxy.clone();
    App::new(AppConfig::default()).run(WindowConfig::default(), MenuBar::default(), host);
    if let Some(proxy) = proxy.lock().unwrap().as_ref() {
        assert_eq!(proxy.wake(), WakeResult::Closed);
        assert_eq!(proxy.clone().wake(), WakeResult::Closed);
    }
}
