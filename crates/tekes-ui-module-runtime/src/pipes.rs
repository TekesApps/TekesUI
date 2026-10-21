//! The single process/line-transport primitive used by V1 View and V2 services.
use anyhow::{Context, Result, ensure};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::{Child, ChildStdin, ChildStdout, Command, Stdio},
};
pub(crate) const MAX_PACKET: usize = 1_048_576;
pub(crate) struct Pipes {
    pub child: Child,
    pub input: ChildStdin,
    pub output: BufReader<ChildStdout>,
}
pub(crate) fn spawn(args: &[&str]) -> Result<Pipes> {
    let mut child = Command::new(std::env::current_exe()?)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()?;
    let Some(input) = child.stdin.take() else {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("worker stdin unavailable");
    };
    let Some(output) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        anyhow::bail!("worker stdout unavailable");
    };
    Ok(Pipes {
        child,
        input,
        output: BufReader::new(output),
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
}
