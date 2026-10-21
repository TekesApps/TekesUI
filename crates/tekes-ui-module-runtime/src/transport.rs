//! Local child-process capability transport. IOSurface send rights travel in
//! Mach descriptors, never as numeric port names or global IOSurface IDs.
use anyhow::{Result, ensure};
use mach2::{mach_port::*, message::*, port::*, traps::mach_task_self};
use std::ffi::CString;
unsafe extern "C" {
    static bootstrap_port: u32;
    fn bootstrap_register(bp: u32, name: *const i8, port: u32) -> i32;
    fn bootstrap_look_up(bp: u32, name: *const i8, port: *mut u32) -> i32;
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
        release(self.0);
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
            "org.tekes.apphost.{}.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)?
                .as_nanos()
        );
        let c = CString::new(name.clone())?;
        let mut port = 0;
        // SAFETY: output pointers valid, current task owns allocated receive right.
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
            let status = bootstrap_register(bootstrap_port, c.as_ptr(), port);
            if status != 0 {
                mach_port_destroy(mach_task_self(), port);
                anyhow::bail!("register local Mach endpoint {name}: {status}");
            }
        }
        Ok(Self { port, name })
    }
    #[cfg(test)]
    pub(crate) fn receive_owned(&self) -> Result<OwnedSendRight> {
        self.receive_owned_cancellable(|| false)
    }
    pub(crate) fn receive_owned_cancellable(
        &self,
        mut cancelled: impl FnMut() -> bool,
    ) -> Result<OwnedSendRight> {
        // Extra aligned space includes the kernel trailer. The descriptor is
        // read only after a successful receive and header/type validation.
        let mut words = [0u64; 64];
        let ptr = words.as_mut_ptr().cast::<mach_msg_header_t>();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        loop {
            ensure!(!cancelled(), "surface receive cancelled");
            unsafe {
                let result = mach_msg(
                    ptr,
                    MACH_RCV_MSG | MACH_RCV_TIMEOUT,
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
                let m = &mut *ptr.cast::<PortMessage>();
                ensure!(
                    m.header.msgh_size as usize == std::mem::size_of::<PortMessage>(),
                    "invalid surface message size"
                );
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
        unsafe {
            mach_port_destroy(mach_task_self(), self.port);
        }
    }
}
pub fn send(name: &str, surface_port: u32) -> Result<()> {
    // Adopt before any fallible lookup, so a bad endpoint cannot leak the source.
    let surface = unsafe { OwnedSendRight::from_raw(surface_port) };
    let c = CString::new(name)?;
    let mut remote = 0;
    // SAFETY: all descriptors and ports belong to this task. COPY_SEND retains
    // a right for the receiver; both local temporary send rights are released.
    unsafe {
        ensure!(
            bootstrap_look_up(bootstrap_port, c.as_ptr(), &mut remote) == 0,
            "find parent Mach endpoint"
        );
        let remote_right = OwnedSendRight::from_raw(remote);
        let mut m: PortMessage = std::mem::zeroed();
        m.header.msgh_bits = MACH_MSGH_BITS_COMPLEX | MACH_MSG_TYPE_COPY_SEND;
        m.header.msgh_size = std::mem::size_of::<PortMessage>() as u32;
        m.header.msgh_remote_port = remote_right.name();
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
pub fn release(port: u32) {
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
        send(&endpoint.name, raw).unwrap();
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
            assert!(send(name, raw).is_err());
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
                worker_flag.load(Ordering::Acquire)
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
