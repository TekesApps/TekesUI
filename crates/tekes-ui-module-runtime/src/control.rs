//! Shared child ownership for service and View transports.
use anyhow::{Result, ensure};
use std::{
    process::Child,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[derive(Debug, Default)]
struct State {
    child: Mutex<Option<Child>>,
    cancelled: AtomicBool,
    installed: AtomicBool,
}
/// A cloneable cancellation handle, created before launching a worker thread.
/// `cancel` signals/terminates only; pipe reads, GPU waits and waitpid are never
/// performed on its caller. The transport owner reaps on its background thread.
#[derive(Clone, Debug, Default)]
pub struct ProcessControl(Arc<State>);
impl ProcessControl {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn cancel(&self) {
        self.0.cancelled.store(true, Ordering::Release);
        if let Some(child) = self.0.child.lock().unwrap().as_mut() {
            let _ = child.kill();
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    pub fn pid(&self) -> Option<u32> {
        self.0.child.lock().unwrap().as_ref().map(Child::id)
    }
    pub(crate) fn exited(&self) -> Result<bool> {
        let mut child = self.0.child.lock().unwrap();
        Ok(match child.as_mut() {
            Some(child) => child.try_wait()?.is_some(),
            None => false,
        })
    }
    pub(crate) fn install(&self, mut child: Child) -> Result<()> {
        if self.0.installed.swap(true, Ordering::AcqRel) {
            // Caller is the launcher thread. Never leak the rejected new child.
            let _ = child.kill();
            let _ = child.wait();
            anyhow::bail!("process control cannot be reused for another instance");
        }
        let mut slot = self.0.child.lock().unwrap();
        ensure!(slot.is_none(), "child already installed");
        *slot = Some(child);
        if self.is_cancelled() {
            let _ = slot.as_mut().unwrap().kill();
        }
        Ok(())
    }
    /// Background transport owner only. Take out of the lock before waitpid.
    pub(crate) fn reap(&self) {
        self.cancel();
        let child = self.0.child.lock().unwrap().take();
        if let Some(mut child) = child {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
