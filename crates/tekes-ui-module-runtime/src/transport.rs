//! Local child-process capability transport. IOSurface send rights travel in
//! Mach descriptors over parent-child inherited rights, never via bootstrap_register
//! or numeric port names/IOSurface IDs.
use anyhow::{Result, ensure};
use mach2::{mach_port::*, message::*, port::*, traps::mach_task_self};
use std::{
    collections::HashMap,
    ffi::CString,
    sync::{Mutex, OnceLock},
};

pub(crate) const WORKER_MACH_ENDPOINT_ENV: &str = "TEKESUI_WORKER_MACH_ENDPOINT";
pub(crate) const EXC_MASK_RPC_ALERT: u32 = 0x200;
pub(crate) const EXCEPTION_DEFAULT: i32 = 1;
pub(crate) const THREAD_STATE_NONE: i32 = 5;
const EXC_TYPES_COUNT: usize = 14;
const RCV_TRAILER_AUDIT: i32 =
    (((MACH_MSG_TRAILER_FORMAT_0 & 0xf) << 28) | ((MACH_RCV_TRAILER_AUDIT & 0xf) << 24)) as i32;

#[link(name = "bsm")]
unsafe extern "C" {
    fn audit_token_to_pid(atoken: audit_token_t) -> i32;
}

unsafe extern "C" {
    fn task_get_exception_ports(
        task: u32,
        exception_mask: u32,
        masks: *mut u32,
        masks_cnt: *mut u32,
        old_handlers: *mut u32,
        old_behaviors: *mut i32,
        old_flavors: *mut i32,
    ) -> i32;
    fn task_set_exception_ports(
        task: u32,
        exception_mask: u32,
        new_port: u32,
        behavior: i32,
        new_flavor: i32,
    ) -> i32;
}

static LOCAL_ENDPOINTS: OnceLock<Mutex<HashMap<String, u32>>> = OnceLock::new();

fn local_endpoints() -> &'static Mutex<HashMap<String, u32>> {
    LOCAL_ENDPOINTS.get_or_init(|| Mutex::new(HashMap::new()))
}

fn inherited_endpoint() -> Option<&'static (String, OwnedSendRight)> {
    static INHERITED: OnceLock<Option<(String, OwnedSendRight)>> = OnceLock::new();
    INHERITED
        .get_or_init(|| {
            let name = std::env::var(WORKER_MACH_ENDPOINT_ENV)
                .ok()
                .filter(|value| !value.is_empty());
            let mut masks = [0u32; EXC_TYPES_COUNT];
            let mut ports = [0u32; EXC_TYPES_COUNT];
            let mut behaviors = [0i32; EXC_TYPES_COUNT];
            let mut flavors = [0i32; EXC_TYPES_COUNT];
            let mut count = EXC_TYPES_COUNT as u32;
            // SAFETY: fixed-capacity buffers match `count` and `mach_task_self()`.
            let kr = unsafe {
                task_get_exception_ports(
                    mach_task_self(),
                    EXC_MASK_RPC_ALERT,
                    masks.as_mut_ptr(),
                    &mut count,
                    ports.as_mut_ptr(),
                    behaviors.as_mut_ptr(),
                    flavors.as_mut_ptr(),
                )
            };
            // Clear the inherited exception port so grandchildren spawned by this
            // worker never receive the parent's Mach endpoint.
            // SAFETY: clears this task's `EXC_MASK_RPC_ALERT` slot to `MACH_PORT_NULL`.
            unsafe {
                task_set_exception_ports(
                    mach_task_self(),
                    EXC_MASK_RPC_ALERT,
                    MACH_PORT_NULL,
                    EXCEPTION_DEFAULT,
                    THREAD_STATE_NONE,
                );
            }
            if kr != 0 {
                return None;
            }
            let n = (count as usize).min(EXC_TYPES_COUNT);
            let mut chosen = None;
            for &port in &ports[..n] {
                if port == MACH_PORT_NULL || port == MACH_PORT_DEAD {
                    continue;
                }
                // SAFETY: `task_get_exception_ports` granted one send-right reference in this task.
                let owned = unsafe { OwnedSendRight::from_raw(port) };
                if name.is_some() && chosen.is_none() {
                    chosen = Some(owned);
                }
            }
            name.zip(chosen)
        })
        .as_ref()
}
#[repr(C)]
struct PortMessage {
    header: mach_msg_header_t,
    body: mach_msg_body_t,
    port: mach_msg_port_descriptor_t,
}
/// Own exactly one local send-right reference, including early-error paths.
#[derive(Debug)]
pub(crate) struct OwnedSendRight(u32);
impl OwnedSendRight {
    /// Caller transfers one live send-right reference in this task.
    pub(crate) unsafe fn from_raw(port: u32) -> Self {
        Self(port)
    }
    pub(crate) fn name(&self) -> u32 {
        self.0
    }
}
impl Drop for OwnedSendRight {
    fn drop(&mut self) {
        // SAFETY: `OwnedSendRight` owns exactly one live send-right reference in this task.
        unsafe {
            release(self.0);
        }
    }
}
struct ReceivedMessage(*mut mach_msg_header_t);
impl Drop for ReceivedMessage {
    fn drop(&mut self) {
        unsafe {
            mach_msg_destroy(self.0);
        }
    }
}
#[derive(Debug)]
pub struct Receiver {
    port: u32,
    pub name: String,
}
impl Receiver {
    pub fn new() -> Result<Self> {
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let name = format!(
            "org.tekes.apphost.local.{}.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let mut port = 0;
        // SAFETY: output pointer valid, current task owns allocated receive right.
        unsafe {
            ensure!(
                mach_port_allocate(mach_task_self(), MACH_PORT_RIGHT_RECEIVE, &mut port) == 0,
                "allocate Mach receive"
            );
            let inserted =
                mach_port_insert_right(mach_task_self(), port, port, MACH_MSG_TYPE_MAKE_SEND);
            if inserted != 0 {
                mach_port_destroy(mach_task_self(), port);
                anyhow::bail!("insert send right: {inserted}");
            }
        }
        local_endpoints().lock().unwrap().insert(name.clone(), port);
        Ok(Self { port, name })
    }
    pub(crate) fn raw_port(&self) -> u32 {
        self.port
    }
    #[cfg(test)]
    pub(crate) fn receive_owned(&self) -> Result<OwnedSendRight> {
        self.receive_owned_cancellable_from(Some(std::process::id()), || Ok(false))
    }
    #[cfg(test)]
    pub(crate) fn receive_owned_cancellable(
        &self,
        cancelled: impl FnMut() -> Result<bool>,
    ) -> Result<OwnedSendRight> {
        self.receive_owned_cancellable_from(Some(std::process::id()), cancelled)
    }
    /// Background process transport only. A peer can die after its frame header
    /// but before sending the surface. Check its owned Child on the existing
    /// receive timeout polls, without discarding an already queued final right.
    pub(crate) fn receive_owned_for_process(
        &self,
        control: &crate::ProcessControl,
    ) -> Result<OwnedSendRight> {
        let expected_pid = control
            .pid()
            .ok_or_else(|| anyhow::anyhow!("worker pid unavailable"))?;
        self.receive_owned_for_process_with_pid(control, expected_pid)
    }
    pub(crate) fn receive_owned_for_process_with_pid(
        &self,
        control: &crate::ProcessControl,
        expected_pid: u32,
    ) -> Result<OwnedSendRight> {
        let mut waited = false;
        self.receive_owned_cancellable_from(Some(expected_pid), || {
            let cancelled = control.is_cancelled();
            if waited && !cancelled {
                ensure!(!control.exited()?, "worker exited before surface transfer");
            }
            waited = true;
            Ok(cancelled)
        })
    }
    pub(crate) fn receive_owned_cancellable_from(
        &self,
        expected_pid: Option<u32>,
        mut cancelled: impl FnMut() -> Result<bool>,
    ) -> Result<OwnedSendRight> {
        // Extra aligned space includes the kernel audit trailer. The descriptor is
        // read only after header, descriptor, and sender PID validation.
        let mut words = [0u64; 64];
        let ptr = words.as_mut_ptr().cast::<mach_msg_header_t>();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            ensure!(!cancelled()?, "surface receive cancelled");
            unsafe {
                let result = mach_msg(
                    ptr,
                    MACH_RCV_MSG | MACH_RCV_TIMEOUT | RCV_TRAILER_AUDIT,
                    0,
                    512,
                    self.port,
                    50,
                    MACH_PORT_NULL,
                );
                if result == MACH_RCV_TIMED_OUT {
                    ensure!(
                        std::time::Instant::now() < deadline,
                        "surface receive timed out"
                    );
                    continue;
                }
                ensure!(result == 0, "receive surface right: {result}");
                // The received message owns its descriptors even if validation fails.
                let _message = ReceivedMessage(ptr);
                let msg_size = std::mem::size_of::<PortMessage>();
                let trailer_offset = (msg_size + 3) & !3;
                ensure!(
                    trailer_offset + std::mem::size_of::<mach_msg_audit_trailer_t>() <= 512,
                    "audit trailer exceeds receive buffer"
                );
                let m = &mut *ptr.cast::<PortMessage>();
                ensure!(
                    m.header.msgh_size as usize == msg_size,
                    "invalid surface message size"
                );
                let trailer = ptr
                    .cast::<u8>()
                    .add(trailer_offset)
                    .cast::<mach_msg_audit_trailer_t>()
                    .read_unaligned();
                ensure!(
                    trailer.msgh_trailer_type == MACH_MSG_TRAILER_FORMAT_0
                        && (trailer.msgh_trailer_size as usize)
                            >= std::mem::size_of::<mach_msg_audit_trailer_t>(),
                    "missing Mach audit trailer"
                );
                if let Some(expected) = expected_pid {
                    let sender_pid = audit_token_to_pid(trailer.msgh_audit);
                    ensure!(
                        sender_pid > 0 && sender_pid as u32 == expected,
                        "unexpected surface sender pid"
                    );
                }
                ensure!(
                    m.header.msgh_bits & MACH_MSGH_BITS_COMPLEX != 0
                        && m.body.msgh_descriptor_count == 1
                        && m.port.type_ == MACH_MSG_PORT_DESCRIPTOR as u8
                        && m.port.disposition == MACH_MSG_TYPE_MOVE_SEND as u8
                        && m.header.msgh_id == 1
                        && m.port.name != MACH_PORT_NULL
                        && m.port.name != MACH_PORT_DEAD,
                    "invalid surface right message"
                );
                let right = OwnedSendRight::from_raw(m.port.name);
                // Transfer the one descriptor; the message guard owns any remainder.
                m.port.name = MACH_PORT_NULL;
                return Ok(right);
            }
        }
    }
}
impl Drop for Receiver {
    fn drop(&mut self) {
        if let Some(endpoints) = LOCAL_ENDPOINTS.get() {
            endpoints.lock().unwrap().remove(&self.name);
        }
        unsafe {
            mach_port_destroy(mach_task_self(), self.port);
        }
    }
}
/// Transfer ownership of `surface_port` and send a copy of its send right to `name`.
///
/// # Safety
/// `surface_port` must be a valid Mach send right in `mach_task_self()` whose
/// ownership (one send-right reference) is transferred to this call.
pub unsafe fn send(name: &str, surface_port: u32) -> Result<()> {
    // Adopt before any fallible lookup, so a bad endpoint cannot leak the source.
    // SAFETY: caller transfers ownership of one valid Mach send right in this task.
    let surface = unsafe { OwnedSendRight::from_raw(surface_port) };
    send_owned(name, &surface)
}

pub(crate) fn send_owned(name: &str, surface: &OwnedSendRight) -> Result<()> {
    let _ = CString::new(name)?;
    ensure!(!name.is_empty(), "empty Mach endpoint");
    // Clone one temporary local right under the map lock so Receiver::drop
    // cannot destroy `port` concurrently before `mach_msg`.
    let local_right = {
        let endpoints = local_endpoints().lock().unwrap();
        if let Some(&port) = endpoints.get(name) {
            // SAFETY: `port` is live while `endpoints` holds it and `Receiver::drop` removes first.
            let kr = unsafe { mach_port_mod_refs(mach_task_self(), port, MACH_PORT_RIGHT_SEND, 1) };
            ensure!(kr == 0, "retain local Mach endpoint: {kr}");
            // SAFETY: `mach_port_mod_refs` just incremented send refcount by 1.
            Some(unsafe { OwnedSendRight::from_raw(port) })
        } else {
            None
        }
    };
    let remote_name = if let Some(right) = &local_right {
        right.name()
    } else if let Some((_, right)) = inherited_endpoint().filter(|(expected, _)| expected == name) {
        right.name()
    } else {
        anyhow::bail!("find parent Mach endpoint");
    };
    // SAFETY: all descriptors and ports belong to this task. COPY_SEND retains
    // a right for the receiver; local_right (if taken) is released at scope end.
    unsafe {
        let mut m: PortMessage = std::mem::zeroed();
        m.header.msgh_bits = MACH_MSGH_BITS_COMPLEX | MACH_MSG_TYPE_COPY_SEND;
        m.header.msgh_size = std::mem::size_of::<PortMessage>() as u32;
        m.header.msgh_remote_port = remote_name;
        m.header.msgh_id = 1;
        m.body.msgh_descriptor_count = 1;
        m.port.name = surface.name();
        m.port.disposition = MACH_MSG_TYPE_COPY_SEND as u8;
        m.port.type_ = MACH_MSG_PORT_DESCRIPTOR as u8;
        let result = mach_msg(
            &mut m.header,
            MACH_SEND_MSG | MACH_SEND_TIMEOUT,
            std::mem::size_of::<PortMessage>() as u32,
            0,
            0,
            10_000,
            0,
        );
        ensure!(result == 0, "send surface right: {result}");
    }
    Ok(())
}

/// Deallocate one owned Mach port reference in `mach_task_self()`.
///
/// # Safety
/// `port` must be a valid Mach port name whose reference count may be decremented
/// by this task without dangling other owners.
pub unsafe fn release(port: u32) {
    // SAFETY: caller guarantees ownership of one valid Mach port reference.
    unsafe {
        mach_port_deallocate(mach_task_self(), port);
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    unsafe extern "C" {
        fn mach_port_get_refs(task: u32, name: u32, right: u32, refs: *mut u32) -> i32;
    }
    pub(crate) fn refs(port: u32) -> u32 {
        let mut refs = 0;
        assert_eq!(
            unsafe { mach_port_get_refs(mach_task_self(), port, MACH_PORT_RIGHT_SEND, &mut refs) },
            0
        );
        refs
    }
    impl Receiver {
        pub(crate) fn test_right(&self) -> OwnedSendRight {
            assert_eq!(
                unsafe { mach_port_mod_refs(mach_task_self(), self.port, MACH_PORT_RIGHT_SEND, 1) },
                0
            );
            unsafe { OwnedSendRight::from_raw(self.port) }
        }
    }
    #[test]
    fn right_guard_drops_exactly_one_reference_and_transfer_does_not_release_early() {
        let endpoint = Receiver::new().unwrap();
        let base = refs(endpoint.port);
        let right = endpoint.test_right();
        assert_eq!(refs(endpoint.port), base + 1);
        drop(right);
        assert_eq!(refs(endpoint.port), base);
        let right = endpoint.test_right();
        let raw = right.name();
        std::mem::forget(right);
        unsafe { send(&endpoint.name, raw) }.unwrap();
        assert_eq!(refs(endpoint.port), base);
        let received = endpoint.receive_owned().unwrap();
        assert_eq!(received.name(), endpoint.port);
        assert_eq!(refs(endpoint.port), base + 1);
        drop(received);
        assert_eq!(refs(endpoint.port), base);
    }
    #[test]
    fn invalid_endpoint_releases_transferred_source_before_returning_error() {
        let endpoint = Receiver::new().unwrap();
        let base = refs(endpoint.port);
        for name in ["invalid\0name", "org.tekes.no-such-process-endpoint"] {
            let source = endpoint.test_right();
            let raw = source.name();
            std::mem::forget(source);
            assert!(unsafe { send(name, raw) }.is_err());
            assert_eq!(refs(endpoint.port), base);
        }
    }
    #[test]
    fn cancellation_interrupts_an_already_waiting_mach_receive() {
        use std::sync::{
            Arc,
            atomic::{AtomicBool, Ordering},
            mpsc,
        };
        let endpoint = Receiver::new().unwrap();
        let flag = Arc::new(AtomicBool::new(false));
        let worker_flag = flag.clone();
        let (entered, started) = mpsc::channel();
        let (done, finished) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            let mut attempts = 0;
            let result = endpoint.receive_owned_cancellable(|| {
                attempts += 1;
                // A second poll proves the first real kernel receive timed out.
                if attempts == 2 {
                    entered.send(()).unwrap();
                }
                Ok(worker_flag.load(Ordering::Acquire))
            });
            done.send(result.map(|_| ())).unwrap();
        });
        started
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        flag.store(true, Ordering::Release);
        assert!(
            finished
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap()
                .is_err()
        );
        thread.join().unwrap();
    }
    #[test]
    fn exited_owned_child_interrupts_surface_receive_without_caller_cancel() {
        use std::sync::mpsc;
        let endpoint = Receiver::new().unwrap();
        let control = crate::ProcessControl::new();
        let child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        let pid = child.id();
        control.install(child).unwrap();
        let worker_control = control.clone();
        let (entered, started) = mpsc::channel();
        let (done, finished) = mpsc::channel();
        let thread = std::thread::spawn(move || {
            entered.send(()).unwrap();
            let result = endpoint.receive_owned_for_process(&worker_control);
            worker_control.reap();
            done.send(result.map(|_| ()).map_err(|error| error.to_string()))
                .unwrap();
        });
        started
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert!(!control.is_cancelled());
        assert!(
            std::process::Command::new("/bin/kill")
                .args(["-KILL", &pid.to_string()])
                .status()
                .unwrap()
                .success()
        );
        let completed = finished.recv_timeout(std::time::Duration::from_secs(2));
        if completed.is_err() {
            control.cancel();
        }
        thread.join().unwrap();
        let failure = completed
            .expect("owned peer exit waited for the ten-second surface deadline")
            .expect_err("dead peer without a surface was accepted");
        assert!(
            failure.contains("worker exited before surface transfer"),
            "{failure}"
        );
    }
    #[test]
    fn exited_peer_keeps_an_already_queued_final_surface_right() {
        let endpoint = Receiver::new().unwrap();
        let right = endpoint.test_right();
        let raw = right.name();
        std::mem::forget(right);
        unsafe { send(&endpoint.name, raw) }.unwrap();
        let control = crate::ProcessControl::new();
        let mut child = std::process::Command::new("/usr/bin/true").spawn().unwrap();
        child.wait().unwrap();
        control.install(child).unwrap();
        assert!(control.exited().unwrap());
        let received = endpoint
            .receive_owned_for_process_with_pid(&control, std::process::id())
            .unwrap();
        assert_eq!(received.name(), endpoint.port);
        drop(received);
        control.reap();
    }
    #[test]
    fn foreign_sender_pid_is_rejected_and_destroys_descriptor() {
        let source = Receiver::new().unwrap();
        let endpoint = Receiver::new().unwrap();
        let base = refs(source.port);
        let right = source.test_right();
        let raw = right.name();
        std::mem::forget(right);
        unsafe { send(&endpoint.name, raw) }.unwrap();
        let control = crate::ProcessControl::new();
        let child = std::process::Command::new("/bin/sleep")
            .arg("30")
            .spawn()
            .unwrap();
        control.install(child).unwrap();
        let err = endpoint
            .receive_owned_for_process(&control)
            .expect_err("foreign sender PID must be rejected by Mach audit trailer");
        assert!(
            err.to_string().contains("unexpected surface sender pid"),
            "{err}"
        );
        assert_eq!(
            refs(source.port),
            base,
            "rejected foreign descriptor leaked its local reference"
        );
        control.reap();
    }
    #[test]
    fn rejected_received_descriptor_is_destroyed_with_its_message() {
        let source = Receiver::new().unwrap();
        let endpoint = Receiver::new().unwrap();
        let base = refs(source.port);
        unsafe {
            let mut message: PortMessage = std::mem::zeroed();
            message.header.msgh_bits = MACH_MSGH_BITS_COMPLEX | MACH_MSG_TYPE_COPY_SEND;
            message.header.msgh_size = std::mem::size_of::<PortMessage>() as u32;
            message.header.msgh_remote_port = endpoint.port;
            message.header.msgh_id = 999; // Valid kernel descriptor, wrong application contract.
            message.body.msgh_descriptor_count = 1;
            message.port.name = source.port;
            message.port.disposition = MACH_MSG_TYPE_COPY_SEND as u8;
            message.port.type_ = MACH_MSG_PORT_DESCRIPTOR as u8;
            assert_eq!(
                mach_msg(
                    &mut message.header,
                    MACH_SEND_MSG | MACH_SEND_TIMEOUT,
                    std::mem::size_of::<PortMessage>() as u32,
                    0,
                    0,
                    1000,
                    0
                ),
                0
            );
        }
        assert!(endpoint.receive_owned().is_err());
        assert_eq!(
            refs(source.port),
            base,
            "rejected descriptor leaked its local reference"
        );
    }
}
