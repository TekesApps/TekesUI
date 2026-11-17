//! Shared child ownership for service and View transports.
use anyhow::{Result, ensure};
use std::{
    process::Child,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};
#[cfg(target_os = "macos")]
mod sys {
    pub(crate) const SIGKILL: i32 = 9;
    pub(crate) const WNOHANG: i32 = 1;
    pub(crate) const EINTR: i32 = 4;

    unsafe extern "C" {
        pub(crate) fn kill(pid: i32, sig: i32) -> i32;
        pub(crate) fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    }
}

#[derive(Debug)]
pub(crate) enum SpawnedChild {
    #[cfg_attr(target_os = "macos", allow(dead_code))]
    Std(Child),
    #[cfg(target_os = "macos")]
    Posix { pid: i32, reaped: bool },
}

impl SpawnedChild {
    fn id(&self) -> u32 {
        match self {
            Self::Std(child) => child.id(),
            #[cfg(target_os = "macos")]
            Self::Posix { pid, .. } => *pid as u32,
        }
    }

    fn kill(&mut self) {
        match self {
            Self::Std(child) => {
                let _ = child.kill();
            }
            #[cfg(target_os = "macos")]
            Self::Posix { pid, reaped } => {
                if !*reaped {
                    // SAFETY: `pid` belongs to this owned un-reaped child process.
                    unsafe {
                        sys::kill(*pid, sys::SIGKILL);
                    }
                }
            }
        }
    }

    fn try_wait(&mut self) -> std::io::Result<bool> {
        match self {
            Self::Std(child) => Ok(child.try_wait()?.is_some()),
            #[cfg(target_os = "macos")]
            Self::Posix { pid, reaped } => {
                if *reaped {
                    return Ok(true);
                }
                let mut status = 0i32;
                loop {
                    // SAFETY: `status` is valid and `pid` is an un-reaped child owned by `self`.
                    let rc = unsafe { sys::waitpid(*pid, &mut status, sys::WNOHANG) };
                    if rc == 0 {
                        return Ok(false);
                    }
                    if rc == *pid {
                        *reaped = true;
                        return Ok(true);
                    }
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(sys::EINTR) {
                        continue;
                    }
                    return Err(err);
                }
            }
        }
    }

    fn wait(&mut self) {
        match self {
            Self::Std(child) => {
                let _ = child.wait();
            }
            #[cfg(target_os = "macos")]
            Self::Posix { pid, reaped } => {
                if *reaped {
                    return;
                }
                let mut status = 0i32;
                loop {
                    // SAFETY: `status` is valid and `pid` is an un-reaped child owned by `self`.
                    let rc = unsafe { sys::waitpid(*pid, &mut status, 0) };
                    if rc == *pid {
                        *reaped = true;
                        return;
                    }
                    if rc < 0 && std::io::Error::last_os_error().raw_os_error() == Some(sys::EINTR)
                    {
                        continue;
                    }
                    *reaped = true;
                    return;
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct State {
    child: Mutex<Option<SpawnedChild>>,
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
            child.kill();
        }
    }
    pub fn is_cancelled(&self) -> bool {
        self.0.cancelled.load(Ordering::Acquire)
    }
    pub fn pid(&self) -> Option<u32> {
        self.0.child.lock().unwrap().as_ref().map(SpawnedChild::id)
    }
    pub(crate) fn exited(&self) -> Result<bool> {
        let mut child = self.0.child.lock().unwrap();
        Ok(match child.as_mut() {
            Some(child) => child.try_wait()?,
            None => false,
        })
    }
    #[cfg_attr(not(test), allow(dead_code))]
    pub(crate) fn install(&self, child: Child) -> Result<()> {
        self.install_spawned(SpawnedChild::Std(child))
    }
    pub(crate) fn install_spawned(&self, mut child: SpawnedChild) -> Result<()> {
        if self.0.installed.swap(true, Ordering::AcqRel) {
            // Caller is the launcher thread. Never leak the rejected new child.
            child.kill();
            child.wait();
            anyhow::bail!("process control cannot be reused for another instance");
        }
        let mut slot = self.0.child.lock().unwrap();
        ensure!(slot.is_none(), "child already installed");
        *slot = Some(child);
        if self.is_cancelled() {
            slot.as_mut().unwrap().kill();
        }
        Ok(())
    }
    /// Background transport owner only. Take out of the lock before waitpid.
    pub(crate) fn reap(&self) {
        self.cancel();
        let child = self.0.child.lock().unwrap().take();
        if let Some(mut child) = child {
            child.kill();
            child.wait();
        }
    }
}
