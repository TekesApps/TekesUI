//! Shared parent-side process connection and frame acquisition. Experimental API.
//!
//! This is the existing blocking worker-thread transport, not the completed UI
//! ProcessContainer lifecycle. Callers must keep request/wait off the UI thread.
use crate::surface as gpu;
use anyhow::{Context, Result, ensure};
use std::{
    io::BufReader,
    process::{ChildStdin, ChildStdout},
};
use tekes_ui_app_protocol::{Frame, Request, VERSION};
/// Immutable wire metadata plus one owned, receiver-local IOSurface capability.
/// Dropping a queued, rejected or never-imported frame releases its right.
#[derive(Debug)]
pub struct ReceivedFrame {
    frame: Frame,
    right: Option<crate::transport::OwnedSendRight>,
}
impl std::ops::Deref for ReceivedFrame {
    type Target = Frame;
    fn deref(&self) -> &Frame {
        &self.frame
    }
}
impl ReceivedFrame {
    pub(crate) fn received(frame: Frame, right: crate::transport::OwnedSendRight) -> Self {
        Self {
            frame,
            right: Some(right),
        }
    }
    pub fn has_surface(&self) -> bool {
        self.right.is_some()
    }
    /// Keep data-only metadata. Any unimported surface right is released.
    pub fn into_metadata(self) -> Frame {
        self.frame
    }
}
pub struct ModuleProcess {
    control: crate::ProcessControl,
    pid: u32,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    ports: crate::transport::Receiver,
    failed: bool,
}
impl ModuleProcess {
    pub fn spawn(module: &str) -> Result<Self> {
        Self::spawn_with_control(module, crate::ProcessControl::new())
    }
    /// Run on a background thread; the caller may cancel even before spawn starts.
    pub fn spawn_with_control(module: &str, control: crate::ProcessControl) -> Result<Self> {
        ensure!(!control.is_cancelled(), "worker launch cancelled");
        let ports = crate::transport::Receiver::new()?;
        let connection = crate::pipes::spawn(&["--worker", module, &ports.name])?;
        let pid = connection.child.id();
        control.install(connection.child)?;
        Ok(Self {
            input: connection.input,
            output: connection.output,
            control,
            pid,
            failed: false,
            ports,
        })
    }
    pub fn pid(&self) -> u32 {
        self.pid
    }
    pub fn control(&self) -> crate::ProcessControl {
        self.control.clone()
    }
    pub fn request(&mut self, r: &Request) -> Result<ReceivedFrame> {
        ensure!(!self.control.is_cancelled(), "worker cancelled");
        ensure!(!self.failed, "worker connection previously failed");
        let result = self.request_inner(r);
        if result.is_err() {
            self.failed = true;
            self.control.cancel();
        }
        result
    }
    fn request_inner(&mut self, r: &Request) -> Result<ReceivedFrame> {
        crate::pipes::write_packet(&mut self.input, &serde_json::to_vec(r)?)?;
        let bytes = crate::pipes::read_packet(&mut self.output)?.context("worker exited")?;
        // Receive into RAII before parsing or rejecting the matching metadata.
        let right = self
            .ports
            .receive_owned_cancellable(|| self.control.is_cancelled())?;
        ensure!(!self.control.is_cancelled(), "worker cancelled");
        let frame: Frame = serde_json::from_slice(&bytes).context("bad worker frame")?;
        ensure!(
            frame.version == VERSION && frame.seq == r.seq && frame.viewport == r.viewport,
            "wrong frame identity"
        );
        ensure!(
            frame.pid == self.pid && frame.pid != std::process::id(),
            "not a separate process"
        );
        ensure!(
            frame.width == r.width
                && frame.height == r.height
                && frame.pixel_width == (r.width as f32 * r.scale).round() as u32
                && frame.pixel_height == (r.height as f32 * r.scale).round() as u32,
            "wrong frame geometry"
        );
        Ok(ReceivedFrame {
            frame,
            right: Some(right),
        })
    }
}
impl Drop for ModuleProcess {
    fn drop(&mut self) {
        self.control.reap();
    }
}
/// Import a received frame into this consumer's GPU. Consumes the local Mach
/// right even on import failure; it cannot accidentally be imported twice.
pub fn import_frame(
    context: &tekes_ui_render::gpu::GpuContext,
    frame: &mut ReceivedFrame,
) -> Result<wgpu::Texture> {
    let right = frame
        .right
        .take()
        .context("frame surface right already consumed")?;
    gpu::import_owned(
        context,
        (frame.pixel_width, frame.pixel_height),
        right,
        frame.registry,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transport::{Receiver, tests::refs};
    fn frame(right: crate::transport::OwnedSendRight) -> ReceivedFrame {
        ReceivedFrame {
            frame: Frame {
                version: VERSION,
                seq: 1,
                viewport: 1,
                pid: 1,
                registry: 0,
                width: 1,
                height: 1,
                pixel_width: 1,
                pixel_height: 1,
                port: 0,
                revision: 0,
                toolbar: Vec::new(),
            },
            right: Some(right),
        }
    }
    #[test]
    fn queued_or_never_imported_frame_drop_releases_its_capability() {
        let endpoint = Receiver::new().unwrap();
        let right = endpoint.test_right();
        let name = right.name();
        let count = refs(name);
        let received = frame(right);
        assert!(received.has_surface());
        assert_eq!(
            received.port, 0,
            "wire metadata never exposes the local capability"
        );
        drop(received);
        assert_eq!(refs(name), count - 1);
        let received = frame(endpoint.test_right());
        let metadata = received.into_metadata();
        assert_eq!(metadata.seq, 1);
        assert_eq!(refs(name), count - 1);
    }
    #[test]
    fn failed_gpu_lookup_consumes_once_without_releasing_other_owners() {
        let renderer = tekes_ui_render::Renderer::new_headless().unwrap();
        let endpoint = Receiver::new().unwrap();
        let right = endpoint.test_right();
        let name = right.name();
        let count = refs(name);
        let mut received = frame(right);
        // A real Mach send right, deliberately not an IOSurface capability.
        assert!(import_frame(&renderer.gpu_context(), &mut received).is_err());
        assert!(!received.has_surface());
        assert_eq!(refs(name), count - 1);
        assert!(import_frame(&renderer.gpu_context(), &mut received).is_err());
        drop(received);
        assert_eq!(refs(name), count - 1);
    }
}
