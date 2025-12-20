//! Asynchronous pipe reader module.

use std::io::{PipeReader, Read};
use std::os::fd::AsRawFd;
use std::pin::Pin;
use std::task::{Context, Poll, ready};

use tokio::io::unix::AsyncFd;
use tokio::io::{AsyncRead, ReadBuf};

/// Asynchronous pipe reader.
///
/// Used to coherently read data from `stdout`/`stderr` pipes for `std::io::Command`/`async_process::Command`.
///
/// Replaces `io-mux` because doesn't require EOF and doesn't cause any deadlocks.
pub struct AsyncPipeReader {
  inner: AsyncFd<PipeReader>,
}

impl AsyncPipeReader {
  /// Creates asynchronous reader from `std::io::PipeReader`.
  pub fn new(pipe: PipeReader) -> std::io::Result<Self> {
    let fd = pipe.as_raw_fd();
    unsafe {
      let flags = nix::libc::fcntl(fd, nix::libc::F_GETFL);
      nix::libc::fcntl(fd, nix::libc::F_SETFL, flags | nix::libc::O_NONBLOCK);
    }
    Ok(Self {
      inner: AsyncFd::new(pipe)?,
    })
  }

  /// Reads available data from the pipe.
  pub async fn read(&self, out: &mut [u8]) -> std::io::Result<usize> {
    loop {
      let mut guard = self.inner.readable().await?;

      match guard.try_io(|inner| inner.get_ref().read(out)) {
        Ok(result) => return result,
        Err(_would_block) => continue,
      }
    }
  }
}

impl AsyncRead for AsyncPipeReader {
  fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
    loop {
      let mut guard = ready!(self.inner.poll_read_ready(cx))?;

      let unfilled = buf.initialize_unfilled();
      match guard.try_io(|inner| inner.get_ref().read(unfilled)) {
        Ok(Ok(len)) => {
          buf.advance(len);
          return Poll::Ready(Ok(()));
        }
        Ok(Err(err)) => return Poll::Ready(Err(err)),
        Err(_would_block) => continue,
      }
    }
  }
}
