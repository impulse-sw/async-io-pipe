//! Asynchronous pipe reader module.

use std::io::{PipeReader, Read};
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::io::{AsyncRead, ReadBuf};

#[cfg(unix)]
use std::os::fd::AsRawFd;
#[cfg(unix)]
use std::task::ready;
#[cfg(unix)]
use tokio::io::unix::AsyncFd;

#[cfg(not(any(unix, windows)))]
compile_error!("async-io-pipe is only supported on Unix and Windows platforms");

/// Asynchronous pipe reader.
///
/// Used to coherently read data from `stdout`/`stderr` pipes for `std::io::Command`/`async_process::Command`.
///
/// Replaces `io-mux` because doesn't require EOF and doesn't cause any deadlocks.
pub struct AsyncPipeReader {
  #[cfg(unix)]
  inner: AsyncFd<PipeReader>,

  #[cfg(windows)]
  receiver: tokio::sync::mpsc::UnboundedReceiver<std::io::Result<Vec<u8>>>,
  #[cfg(windows)]
  buffer: Vec<u8>,
  #[cfg(windows)]
  buffer_pos: usize,
}

impl AsyncPipeReader {
  /// Creates asynchronous reader from `std::io::PipeReader`.
  #[cfg(unix)]
  pub fn new(pipe: PipeReader) -> std::io::Result<Self> {
    let fd = pipe.as_raw_fd();
    unsafe {
      let flags = libc::fcntl(fd, libc::F_GETFL);
      libc::fcntl(fd, libc::F_SETFL, flags | libc::O_NONBLOCK);
    }
    Ok(Self { inner: AsyncFd::new(pipe)? })
  }

  /// Creates asynchronous reader from `std::io::PipeReader`.
  #[cfg(windows)]
  pub fn new(mut pipe: PipeReader) -> std::io::Result<Self> {
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
    std::thread::spawn(move || {
      let mut buf = [0u8; 4096];
      loop {
        match pipe.read(&mut buf) {
          Ok(0) => break,
          Ok(n) => {
            if tx.send(Ok(buf[..n].to_vec())).is_err() {
              break;
            }
          }
          Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => break,
          Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
          Err(e) => {
            let _ = tx.send(Err(e));
            break;
          }
        }
      }
    });
    Ok(Self { receiver: rx, buffer: Vec::new(), buffer_pos: 0 })
  }

  /// Reads available data from the pipe.
  #[cfg(unix)]
  pub async fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
    loop {
      let mut guard = self.inner.readable().await?;
      match guard.try_io(|inner| inner.get_ref().read(out)) {
        Ok(result) => return result,
        Err(_would_block) => continue,
      }
    }
  }

  /// Reads available data from the pipe.
  #[cfg(windows)]
  pub async fn read(&mut self, out: &mut [u8]) -> std::io::Result<usize> {
    if self.buffer_pos < self.buffer.len() {
      let n = out.len().min(self.buffer.len() - self.buffer_pos);
      out[..n].copy_from_slice(&self.buffer[self.buffer_pos..self.buffer_pos + n]);
      self.buffer_pos += n;
      return Ok(n);
    }

    match self.receiver.recv().await {
      Some(Ok(data)) => {
        let n = out.len().min(data.len());
        out[..n].copy_from_slice(&data[..n]);
        if n < data.len() {
          self.buffer = data;
          self.buffer_pos = n;
        }
        Ok(n)
      }
      Some(Err(e)) => Err(e),
      None => Ok(0),
    }
  }
}

#[cfg(unix)]
impl AsyncRead for AsyncPipeReader {
  fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
    let this = self.get_mut();
    loop {
      let mut guard = ready!(this.inner.poll_read_ready(cx))?;
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

#[cfg(windows)]
impl AsyncRead for AsyncPipeReader {
  fn poll_read(self: Pin<&mut Self>, cx: &mut Context<'_>, buf: &mut ReadBuf<'_>) -> Poll<std::io::Result<()>> {
    let this = self.get_mut();

    if this.buffer_pos < this.buffer.len() {
      let n = buf.remaining().min(this.buffer.len() - this.buffer_pos);
      buf.put_slice(&this.buffer[this.buffer_pos..this.buffer_pos + n]);
      this.buffer_pos += n;
      return Poll::Ready(Ok(()));
    }

    match this.receiver.poll_recv(cx) {
      Poll::Ready(Some(Ok(data))) => {
        let n = buf.remaining().min(data.len());
        buf.put_slice(&data[..n]);
        if n < data.len() {
          this.buffer = data;
          this.buffer_pos = n;
        }
        Poll::Ready(Ok(()))
      }
      Poll::Ready(Some(Err(e))) => Poll::Ready(Err(e)),
      Poll::Ready(None) => Poll::Ready(Ok(())),
      Poll::Pending => Poll::Pending,
    }
  }
}
