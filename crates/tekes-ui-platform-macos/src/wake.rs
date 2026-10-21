//! Notification-only background wake, without transferring any UI object.
use std::sync::{
    Arc,
    atomic::{AtomicU8, Ordering},
};

const IDLE: u8 = 0;
const PENDING: u8 = 1;
const CLOSED: u8 = 2;

#[derive(Debug, Default)]
pub(crate) struct WakeState(AtomicU8);

/// A successful request is a notification, not an acknowledgement that the
/// handler has already read the caller's snapshot. Close can cancel queued work.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WakeResult {
    Queued,
    Coalesced,
    Closed,
}

impl WakeState {
    fn request(&self) -> WakeResult {
        // Even a coalesced request must publish with Release. A failed CAS
        // (Acquire only) would not synchronize that publisher with the UI's
        // pending-clear Acquire, leaving a final snapshot change unobserved.
        match self
            .0
            .fetch_update(Ordering::AcqRel, Ordering::Acquire, |state| {
                (state != CLOSED).then_some(PENDING)
            }) {
            Ok(IDLE) => WakeResult::Queued,
            Ok(_) => WakeResult::Coalesced,
            Err(_) => WakeResult::Closed,
        }
    }
    /// Clear BEFORE calling the host. A request during snapshot processing must
    /// schedule another delivery; clearing afterwards would lose that request.
    #[cfg(any(target_os = "macos", test))]
    fn begin_delivery(&self) -> bool {
        self.0
            .compare_exchange(PENDING, IDLE, Ordering::AcqRel, Ordering::Acquire)
            .is_ok()
    }
    #[cfg(any(target_os = "macos", test))]
    pub(crate) fn close(&self) {
        self.0.store(CLOSED, Ordering::Release);
    }
}

/// Cloneable, Send + Sync notification proxy for one window. It owns only an
/// atomic state; clones and queued notifications never keep that window alive.
/// Obtain it on the UI thread with `Window::wake_proxy`. Publish business data
/// through the caller's own synchronized snapshot before calling `wake`.
#[derive(Clone, Debug)]
pub struct WindowWake {
    pub(crate) state: Arc<WakeState>,
}

impl WindowWake {
    /// Queue `Event::Wake` on the owning UI loop. Bursts may coalesce. No frame
    /// or timer is started: the handler reads its snapshot and requests redraw.
    pub fn wake(&self) -> WakeResult {
        let result = self.state.request();
        if result == WakeResult::Queued {
            #[cfg(target_os = "macos")]
            native::enqueue(self.state.clone());
        }
        result
    }
}

#[cfg(target_os = "macos")]
pub(crate) mod native {
    use super::*;
    use crate::{event::Event, shared::Shared};
    use std::{
        cell::RefCell,
        collections::HashMap,
        ffi::c_void,
        rc::{Rc, Weak},
    };

    thread_local! {
        // Only the UI thread accesses Rc/Weak. The queue context contains an Arc
        // of atomics, never a raw NSWindow, Shared pointer, or cross-thread Rc.
        static TARGETS: RefCell<HashMap<usize, Weak<Shared>>> = RefCell::new(HashMap::new());
    }
    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(
            queue: *const c_void,
            context: *mut c_void,
            work: extern "C" fn(*mut c_void),
        );
    }
    fn key(state: &Arc<WakeState>) -> usize {
        Arc::as_ptr(state) as usize
    }
    pub(crate) fn register(shared: &Rc<Shared>) -> WindowWake {
        let state = shared.wake_state.clone();
        if !shared.closed.get() {
            TARGETS.with(|targets| {
                targets
                    .borrow_mut()
                    .insert(key(&state), Rc::downgrade(shared))
            });
        }
        WindowWake { state }
    }
    pub(crate) fn unregister(state: &Arc<WakeState>) {
        state.close();
        let _ = TARGETS.try_with(|targets| targets.borrow_mut().remove(&key(state)));
    }
    pub(super) fn enqueue(state: Arc<WakeState>) {
        let context = Box::into_raw(Box::new(state)).cast();
        // SAFETY: libdispatch invokes deliver once on the main queue; it owns
        // this single Box. The Arc contains atomics only and is Send + Sync.
        unsafe { dispatch_async_f(&raw const _dispatch_main_q, context, deliver) };
    }
    extern "C" fn deliver(context: *mut c_void) {
        // SAFETY: context is precisely the Box allocated by enqueue, consumed
        // once by libdispatch. Keep the Arc alive while resolving its identity.
        let state = unsafe { Box::from_raw(context.cast::<Arc<WakeState>>()) };
        if !state.begin_delivery() {
            return;
        }
        let target =
            TARGETS.with(|targets| targets.borrow().get(&key(&state)).and_then(Weak::upgrade));
        if let Some(shared) = target {
            shared.dispatch(Event::Wake);
        } else {
            state.close();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn proxy_is_send_sync_clone_and_has_no_ui_ownership() {
        fn assert_traits<T: Send + Sync + Clone>() {}
        assert_traits::<WindowWake>();
        let state = Arc::new(WakeState::default());
        let proxy = WindowWake {
            state: state.clone(),
        };
        let other = proxy.clone();
        state.close();
        assert_eq!(proxy.wake(), WakeResult::Closed);
        assert_eq!(other.wake(), WakeResult::Closed);
    }
    #[test]
    fn bursts_coalesce_and_request_during_delivery_is_not_lost() {
        let state = Arc::new(WakeState::default());
        assert_eq!(state.request(), WakeResult::Queued);
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let state = state.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        assert_eq!(state.request(), WakeResult::Coalesced);
                    }
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(state.begin_delivery());
        // Deterministic critical interleaving: UI has cleared pending but has
        // not yet completed its snapshot handler; worker publishes again.
        assert_eq!(state.request(), WakeResult::Queued);
        assert!(state.begin_delivery());
        assert!(!state.begin_delivery());
        assert_eq!(state.0.load(Ordering::Acquire), IDLE);
    }
    #[test]
    fn closing_cancels_pending_without_reopening_or_affecting_another_target() {
        let a = WakeState::default();
        let b = WakeState::default();
        assert_eq!(a.request(), WakeResult::Queued);
        assert_eq!(b.request(), WakeResult::Queued);
        a.close();
        assert!(!a.begin_delivery());
        assert_eq!(a.request(), WakeResult::Closed);
        assert!(b.begin_delivery());
        assert_eq!(b.request(), WakeResult::Queued);
    }

    #[test]
    fn concurrent_publish_and_clear_delivers_the_final_snapshot() {
        use std::sync::atomic::AtomicUsize;
        let state = Arc::new(WakeState::default());
        let snapshot = Arc::new(AtomicUsize::new(0));
        let finished = Arc::new(AtomicUsize::new(0));
        let workers: Vec<_> = (0..4)
            .map(|_| {
                let state = state.clone();
                let snapshot = snapshot.clone();
                let finished = finished.clone();
                std::thread::spawn(move || {
                    for _ in 0..1000 {
                        snapshot.fetch_add(1, Ordering::AcqRel);
                        state.request();
                    }
                    finished.fetch_add(1, Ordering::Release);
                })
            })
            .collect();
        let mut seen = 0;
        loop {
            if state.begin_delivery() {
                seen = snapshot.load(Ordering::Relaxed);
            } else if finished.load(Ordering::Acquire) == 4 {
                // A final request may have arrived between the failed clear
                // and observing all publishers finished. Drain it as a loop
                // would, rather than reading data without a notification.
                if state.begin_delivery() {
                    seen = snapshot.load(Ordering::Relaxed);
                }
                break;
            } else {
                std::thread::yield_now();
            }
        }
        for worker in workers {
            worker.join().unwrap();
        }
        assert_eq!(seen, 4000);
        assert!(!state.begin_delivery());
    }
}
