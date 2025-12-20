//! `async-io-pipe`
//!
//! Pipe your stdout/stderr together with asynchronous streams.
//!
//! ```rust
//! use async_io_pipe::async_pipe;
//! use std::process::Command;
//!
//! #[tokio::main]
//! async fn main() {
//!   let (writer, reader) = async_pipe().unwrap();
//!   let mut child = Command::new("cargo")
//!     .arg("build")
//!     .stdout(writer.try_clone().unwrap())
//!     .stderr(writer)
//!     .spawn()
//!     .unwrap();
//!
//!   let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
//!   let io_handle = tokio::spawn(async move {
//!     let mut chunk = [0u8; 2048];
//!     loop {
//!       match reader.read(&mut chunk).await {
//!         Ok(0) | Err(_) => break,
//!         Ok(n) => { let _ = tx.send(chunk[..n].to_vec()); },
//!       }
//!     }
//!   });
//!
//!   let status = child.wait().unwrap().success();
//!
//!   io_handle.abort();
//!   let mut buffer = vec![];
//!   while let Some(chunk) = rx.recv().await {
//!     buffer.extend_from_slice(&chunk);
//!   }
//!
//!   println!("Buffer data:\n{}", String::from_utf8_lossy(&buffer));
//! }
//! ```
//!
//! # `async-io-pipe` vs [`io-mux`](https://github.com/joshtriplett/io-mux)
//!
//! `README.md` from `io-mux` says this:
//!
//! ```markdown
//! Note that reading provides no "EOF" indication; if no further data arrives, it will block forever. Avoid reading after the source of the data exits.
//! ```
//!
//! So, even if you're using `async` feature of `io-mux`, you'll encounter deadlock and won't be able to handle this even by trying to drop spawned task.
//! `async-io-pipe` internally sets `O_NONBLOCK` flag for pipe socket, so you can read all output even with no `EOF` indication and then drop task.

#![deny(warnings, missing_docs, clippy::todo, clippy::unimplemented)]

mod pipe_reader;

pub use pipe_reader::AsyncPipeReader;

/// Returns a `std::io::PipeWriter` and `AsyncPipeReader`.
pub fn async_pipe() -> std::io::Result<(std::io::PipeWriter, AsyncPipeReader)> {
  let (pipe_reader, pipe_writer) = std::io::pipe()?;
  let async_pipe_reader = AsyncPipeReader::new(pipe_reader)?;
  Ok((pipe_writer, async_pipe_reader))
}
