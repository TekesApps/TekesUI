//! The single process/line-transport primitive used by V1 View and V2 services.
//! Control JSON IPC uses dedicated FD 3 (`TEKESUI_WORKER_IPC_FD`), keeping child
//! `stdin`/`stdout` isolated from protocol framing.
use anyhow::{Context, Result, ensure};
use std::{
    ffi::{CString, c_char, c_int},
    io::{BufRead, BufReader, Read, Write},
    os::unix::{
        ffi::OsStrExt,
        io::{AsRawFd, FromRawFd, OwnedFd},
        net::UnixStream,
    },
    sync::atomic::{AtomicBool, Ordering},
};

pub(crate) const MAX_PACKET: usize = 1_048_576;
pub(crate) const WORKER_IPC_FD_ENV: &str = "TEKESUI_WORKER_IPC_FD";
const CHILD_IPC_FD: c_int = 3;

mod sys {
    use std::ffi::c_int;
    #[cfg(target_os = "macos")]
    use std::ffi::{c_char, c_void};

    pub(crate) const F_GETFD: c_int = 1;
    pub(crate) const F_SETFD: c_int = 2;
    pub(crate) const FD_CLOEXEC: c_int = 1;
    #[cfg(target_os = "macos")]
    pub(crate) const F_DUPFD_CLOEXEC: c_int = 67;
    #[cfg(target_os = "macos")]
    pub(crate) const O_RDONLY: c_int = 0;

    #[cfg(target_os = "macos")]
    pub(crate) type SpawnFileActions = *mut c_void;
    #[cfg(target_os = "macos")]
    pub(crate) type SpawnAttr = *mut c_void;

    unsafe extern "C" {
        pub(crate) fn fcntl(fd: c_int, cmd: c_int, ...) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn pipe(fds: *mut c_int) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn waitpid(pid: c_int, status: *mut c_int, options: c_int) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawn_file_actions_init(file_actions: *mut SpawnFileActions) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawn_file_actions_destroy(
            file_actions: *mut SpawnFileActions,
        ) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawn_file_actions_addopen(
            file_actions: *mut SpawnFileActions,
            fd: c_int,
            path: *const c_char,
            oflag: c_int,
            mode: u16,
        ) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawn_file_actions_adddup2(
            file_actions: *mut SpawnFileActions,
            fd: c_int,
            newfd: c_int,
        ) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawnattr_init(attr: *mut SpawnAttr) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawnattr_destroy(attr: *mut SpawnAttr) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawnattr_setexceptionports_np(
            attr: *mut SpawnAttr,
            mask: u32,
            new_port: u32,
            behavior: c_int,
            flavor: c_int,
        ) -> c_int;
        #[cfg(target_os = "macos")]
        pub(crate) fn posix_spawn(
            pid: *mut c_int,
            path: *const c_char,
            file_actions: *const SpawnFileActions,
            attrp: *const SpawnAttr,
            argv: *const *mut c_char,
            envp: *const *mut c_char,
        ) -> c_int;
    }
}

pub(crate) struct WorkerIpcWriter(UnixStream);

impl Write for WorkerIpcWriter {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.0.write(buf)
    }
    fn flush(&mut self) -> std::io::Result<()> {
        self.0.flush()
    }
}

impl Drop for WorkerIpcWriter {
    fn drop(&mut self) {
        let _ = self.0.shutdown(std::net::Shutdown::Write);
    }
}

pub(crate) struct Pipes {
    pub child: crate::control::SpawnedChild,
    pub pid: u32,
    pub input: WorkerIpcWriter,
    pub output: BufReader<UnixStream>,
}

fn prepare_child_ipc_fd(child_stream: &UnixStream) -> Result<(c_int, Option<OwnedFd>)> {
    let raw = child_stream.as_raw_fd();
    #[cfg(target_os = "macos")]
    if raw == CHILD_IPC_FD {
        // posix_spawn dup2(3, 3) leaves FD_CLOEXEC set; bounce through an FD >= 4.
        // SAFETY: `raw` is a valid open socket descriptor.
        let dup = unsafe { sys::fcntl(raw, sys::F_DUPFD_CLOEXEC, CHILD_IPC_FD + 1) };
        ensure!(dup >= 0, "duplicate child IPC descriptor");
        // SAFETY: `dup` is a freshly allocated owned descriptor.
        let owned = unsafe { OwnedFd::from_raw_fd(dup) };
        return Ok((owned.as_raw_fd(), Some(owned)));
    }
    Ok((raw, None))
}

#[cfg(target_os = "macos")]
struct SpawnActions(sys::SpawnFileActions);

#[cfg(target_os = "macos")]
impl SpawnActions {
    fn new() -> Result<Self> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: valid pointer to uninitialized `posix_spawn_file_actions_t`.
        let rc = unsafe { sys::posix_spawn_file_actions_init(&mut raw) };
        ensure!(
            rc == 0,
            "posix_spawn_file_actions_init failed: {}",
            std::io::Error::from_raw_os_error(rc)
        );
        Ok(Self(raw))
    }
}

#[cfg(target_os = "macos")]
impl Drop for SpawnActions {
    fn drop(&mut self) {
        // SAFETY: initialized by `posix_spawn_file_actions_init`.
        unsafe {
            sys::posix_spawn_file_actions_destroy(&mut self.0);
        }
    }
}

#[cfg(target_os = "macos")]
struct SpawnAttr(sys::SpawnAttr);

#[cfg(target_os = "macos")]
impl SpawnAttr {
    fn new() -> Result<Self> {
        let mut raw = std::ptr::null_mut();
        // SAFETY: valid pointer to uninitialized `posix_spawnattr_t`.
        let rc = unsafe { sys::posix_spawnattr_init(&mut raw) };
        ensure!(
            rc == 0,
            "posix_spawnattr_init failed: {}",
            std::io::Error::from_raw_os_error(rc)
        );
        Ok(Self(raw))
    }
}

#[cfg(target_os = "macos")]
impl Drop for SpawnAttr {
    fn drop(&mut self) {
        // SAFETY: initialized by `posix_spawnattr_init`.
        unsafe {
            sys::posix_spawnattr_destroy(&mut self.0);
        }
    }
}

#[cfg(target_os = "macos")]
pub(crate) fn spawn_with_receiver(
    args: &[&str],
    receiver: Option<&crate::transport::Receiver>,
) -> Result<Pipes> {
    let (pipes, stderr_read) = spawn_posix(args, receiver, false)?;
    drop(stderr_read);
    Ok(pipes)
}

#[cfg(target_os = "macos")]
fn spawn_posix(
    args: &[&str],
    receiver: Option<&crate::transport::Receiver>,
    capture_stderr: bool,
) -> Result<(Pipes, Option<std::fs::File>)> {
    let exe = std::env::current_exe()?;
    let exe_c = CString::new(exe.as_os_str().as_bytes())?;
    let mut argv_storage = Vec::with_capacity(args.len() + 1);
    argv_storage.push(exe_c.clone());
    for arg in args {
        argv_storage.push(CString::new(*arg)?);
    }
    let mut argv_ptrs: Vec<*mut c_char> =
        argv_storage.iter().map(|s| s.as_ptr().cast_mut()).collect();
    argv_ptrs.push(std::ptr::null_mut());

    let mut env_storage = Vec::new();
    for (key, value) in std::env::vars_os() {
        if key == WORKER_IPC_FD_ENV || key == crate::transport::WORKER_MACH_ENDPOINT_ENV {
            continue;
        }
        let mut entry = Vec::with_capacity(key.len() + value.len() + 1);
        entry.extend_from_slice(key.as_bytes());
        entry.push(b'=');
        entry.extend_from_slice(value.as_bytes());
        if let Ok(c) = CString::new(entry) {
            env_storage.push(c);
        }
    }
    env_storage.push(CString::new(format!("{WORKER_IPC_FD_ENV}={CHILD_IPC_FD}"))?);
    if let Some(receiver) = receiver {
        env_storage.push(CString::new(format!(
            "{}={}",
            crate::transport::WORKER_MACH_ENDPOINT_ENV,
            receiver.name
        ))?);
    }
    let mut envp_ptrs: Vec<*mut c_char> =
        env_storage.iter().map(|s| s.as_ptr().cast_mut()).collect();
    envp_ptrs.push(std::ptr::null_mut());

    let (parent_stream, child_stream) = UnixStream::pair()?;
    let (src_ipc_fd, _dup_guard) = prepare_child_ipc_fd(&child_stream)?;

    let (stderr_read, stderr_write) = if capture_stderr {
        let mut fds = [0 as c_int; 2];
        // SAFETY: 2-element stack array is valid for `pipe(2)`.
        ensure!(
            unsafe { sys::pipe(fds.as_mut_ptr()) } == 0,
            "pipe for stderr failed"
        );
        for fd in fds {
            // SAFETY: valid fresh pipe descriptors.
            unsafe {
                let flags = sys::fcntl(fd, sys::F_GETFD, 0);
                if flags >= 0 {
                    sys::fcntl(fd, sys::F_SETFD, flags | sys::FD_CLOEXEC);
                }
            }
        }
        // SAFETY: freshly created owned pipe descriptors.
        unsafe {
            (
                Some(std::fs::File::from_raw_fd(fds[0])),
                Some(std::fs::File::from_raw_fd(fds[1])),
            )
        }
    } else {
        (None, None)
    };

    let mut actions = SpawnActions::new()?;
    let dev_null = c"/dev/null";
    // SAFETY: valid initialized `SpawnActions` and constants/descriptors.
    unsafe {
        let rc = sys::posix_spawn_file_actions_addopen(
            &mut actions.0,
            0,
            dev_null.as_ptr(),
            sys::O_RDONLY,
            0,
        );
        ensure!(rc == 0, "posix_spawn addopen stdin failed: {rc}");
        let rc = sys::posix_spawn_file_actions_adddup2(&mut actions.0, src_ipc_fd, CHILD_IPC_FD);
        ensure!(rc == 0, "posix_spawn adddup2 IPC fd failed: {rc}");
        if let Some(ref err_w) = stderr_write {
            let rc = sys::posix_spawn_file_actions_adddup2(&mut actions.0, err_w.as_raw_fd(), 2);
            ensure!(rc == 0, "posix_spawn adddup2 stderr failed: {rc}");
        }
    }

    let mut attr = SpawnAttr::new()?;
    // SAFETY: valid initialized `SpawnAttr`; transfers send right of `receiver` (or NULL)
    // exclusively to this child via `EXC_MASK_RPC_ALERT`.
    unsafe {
        let rc = sys::posix_spawnattr_setexceptionports_np(
            &mut attr.0,
            crate::transport::EXC_MASK_RPC_ALERT,
            receiver.map_or(mach2::port::MACH_PORT_NULL, |r| r.raw_port()),
            crate::transport::EXCEPTION_DEFAULT,
            crate::transport::THREAD_STATE_NONE,
        );
        ensure!(rc == 0, "posix_spawnattr_setexceptionports_np failed: {rc}");
    }

    let mut pid: c_int = 0;
    // SAFETY: all pointers (`exe_c`, `argv_ptrs`, `envp_ptrs`) stay alive through the call.
    let rc = unsafe {
        sys::posix_spawn(
            &mut pid,
            exe_c.as_ptr(),
            &actions.0,
            &attr.0,
            argv_ptrs.as_ptr(),
            envp_ptrs.as_ptr(),
        )
    };
    drop(stderr_write);
    drop(_dup_guard);
    drop(child_stream);
    ensure!(
        rc == 0 && pid > 0,
        "posix_spawn worker failed: {}",
        std::io::Error::from_raw_os_error(rc)
    );

    let reader = BufReader::new(parent_stream.try_clone()?);
    Ok((
        Pipes {
            child: crate::control::SpawnedChild::Posix { pid, reaped: false },
            pid: pid as u32,
            input: WorkerIpcWriter(parent_stream),
            output: reader,
        },
        stderr_read,
    ))
}

#[cfg(not(target_os = "macos"))]
pub(crate) fn spawn(args: &[&str]) -> Result<Pipes> {
    use std::{
        os::unix::process::CommandExt,
        process::{Command, Stdio},
    };
    let (parent_stream, child_stream) = UnixStream::pair()?;
    let (src_fd, dup_guard) = prepare_child_ipc_fd(&child_stream)?;
    let mut cmd = Command::new(std::env::current_exe()?);
    cmd.args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .env(WORKER_IPC_FD_ENV, CHILD_IPC_FD.to_string());
    // SAFETY: `src_fd` stays open in child before exec and `dup2` clears CLOEXEC on FD 3.
    unsafe {
        cmd.pre_exec(move || {
            let rc = sys::dup2_clears_cloexec(src_fd, CHILD_IPC_FD);
            let _ = rc;
            Ok(())
        });
    }
    let child = cmd.spawn()?;
    drop(dup_guard);
    drop(child_stream);
    let pid = child.id();
    let reader = BufReader::new(parent_stream.try_clone()?);
    Ok(Pipes {
        child: crate::control::SpawnedChild::Std(child),
        pid,
        input: WorkerIpcWriter(parent_stream),
        output: reader,
    })
}

pub(crate) fn worker_ipc() -> Result<(BufReader<UnixStream>, WorkerIpcWriter)> {
    static TAKEN: AtomicBool = AtomicBool::new(false);
    let raw = std::env::var(WORKER_IPC_FD_ENV).context(
        "worker IPC descriptor not provided; direct execution outside parent spawn is unsupported",
    )?;
    let fd: c_int = raw.parse().context("invalid worker IPC descriptor")?;
    ensure!(fd >= CHILD_IPC_FD, "worker IPC descriptor must be >= 3");
    ensure!(
        !TAKEN.swap(true, Ordering::AcqRel),
        "worker IPC descriptor already taken"
    );
    // Mark FD 3 close-on-exec so any subprocess spawned by this worker cannot inherit it.
    // SAFETY: `fd` is verified >= 3 and validated by `fcntl(F_GETFD)`.
    unsafe {
        let flags = sys::fcntl(fd, sys::F_GETFD, 0);
        ensure!(flags >= 0, "worker IPC descriptor unavailable");
        ensure!(
            sys::fcntl(fd, sys::F_SETFD, flags | sys::FD_CLOEXEC) == 0,
            "failed to mark worker IPC descriptor close-on-exec"
        );
    }
    // SAFETY: `fd` was inherited from parent spawn for this worker and taken once above.
    let stream = unsafe { UnixStream::from_raw_fd(fd) };
    let reader = BufReader::new(stream.try_clone()?);
    Ok((reader, WorkerIpcWriter(stream)))
}

#[doc(hidden)]
pub fn take_worker_ipc_for_test() -> Result<(Box<dyn BufRead + Send>, Box<dyn Write + Send>)> {
    let (reader, writer) = worker_ipc()?;
    Ok((Box::new(reader), Box::new(writer)))
}

#[doc(hidden)]
pub struct RawWorkerForTest {
    pub pid: u32,
    pub input: Option<Box<dyn Write + Send>>,
    pub output: Box<dyn BufRead + Send>,
    child: Option<crate::control::SpawnedChild>,
    stderr: Option<std::fs::File>,
}

impl std::fmt::Debug for RawWorkerForTest {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawWorkerForTest")
            .field("pid", &self.pid)
            .finish()
    }
}

impl RawWorkerForTest {
    pub fn wait_with_stderr(mut self) -> Result<(bool, String)> {
        self.input.take();
        let mut stderr = String::new();
        if let Some(mut file) = self.stderr.take() {
            file.read_to_string(&mut stderr)?;
        }
        let success = match self.child.take() {
            #[cfg(target_os = "macos")]
            Some(crate::control::SpawnedChild::Posix { pid, .. }) => {
                let mut status = 0i32;
                loop {
                    let rc = unsafe { sys::waitpid(pid, &mut status, 0) };
                    if rc == pid {
                        break (status & 0x7f) == 0 && ((status >> 8) & 0xff) == 0;
                    }
                    let err = std::io::Error::last_os_error();
                    if err.raw_os_error() == Some(4) {
                        continue;
                    }
                    return Err(err.into());
                }
            }
            Some(crate::control::SpawnedChild::Std(mut child)) => child.wait()?.success(),
            None => false,
        };
        Ok((success, stderr))
    }
}

#[cfg(target_os = "macos")]
#[doc(hidden)]
pub fn spawn_raw_worker_for_test(args: &[&str], capture_stderr: bool) -> Result<RawWorkerForTest> {
    let (pipes, stderr) = spawn_posix(args, None, capture_stderr)?;
    Ok(RawWorkerForTest {
        pid: pipes.pid,
        input: Some(Box::new(pipes.input)),
        output: Box::new(pipes.output),
        child: Some(pipes.child),
        stderr,
    })
}

pub(crate) fn read_packet(reader: &mut impl BufRead) -> Result<Option<Vec<u8>>> {
    let mut bytes = Vec::new();
    let read = reader
        .take((MAX_PACKET + 1) as u64)
        .read_until(b'\n', &mut bytes)
        .context("worker packet read")?;
    if read == 0 {
        return Ok(None);
    }
    ensure!(
        read <= MAX_PACKET && bytes.last() == Some(&b'\n'),
        "oversized or incomplete worker packet"
    );
    bytes.pop();
    Ok(Some(bytes))
}
pub(crate) fn write_packet(writer: &mut impl Write, bytes: &[u8]) -> Result<()> {
    ensure!(
        bytes.len() < MAX_PACKET && !bytes.contains(&b'\n'),
        "invalid worker packet size/framing"
    );
    writer.write_all(bytes)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn packet_limits_reject_partial_or_oversized_data_without_unbounded_allocation() {
        assert!(read_packet(&mut &b"partial"[..]).is_err());
        let huge = vec![b'x'; MAX_PACKET + 10];
        let mut input = &huge[..];
        assert!(read_packet(&mut input).is_err());
        assert_eq!(input.len(), 9);
        assert_eq!(
            read_packet(&mut &b"ok\n"[..]).unwrap(),
            Some(b"ok".to_vec())
        );
        assert!(read_packet(&mut &b""[..]).unwrap().is_none());
    }
    #[test]
    fn worker_ipc_requires_inherited_descriptor() {
        if std::env::var_os(WORKER_IPC_FD_ENV).is_none() {
            assert!(worker_ipc().is_err());
        }
    }
}
