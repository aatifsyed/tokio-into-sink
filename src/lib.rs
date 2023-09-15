//! Use an [`AsyncWrite`] as a [`Sink`]`<Item: AsRef<[u8]>`.
//!
//! This adapter produces a sink that will write each value passed to it into the underlying writer.
//! Note that this function consumes the given writer, returning a wrapped version.
//!
//! ```
//! use tokio_into_sink::IntoSinkExt as _;
//! use futures::{stream, StreamExt as _};
//! use std::io;
//!
//! # tokio::runtime::Builder::new_current_thread().build().unwrap().block_on(async {
//! let stream = stream::iter(["hello", "world"]).map(io::Result::Ok);
//! let write = tokio::fs::File::create("/dev/null").await.unwrap();
//! let sink = write.into_sink();
//! stream.forward(sink).await.unwrap();
//! # } ) // block_on
//! ```
//!
//! Ported from [`futures::io::AsyncWriteExt::into_sink`](https://docs.rs/futures/0.3.28/futures/io/trait.AsyncWriteExt.html#method.into_sink).

use std::{
    io,
    pin::Pin,
    task::{ready, Context, Poll},
};

use futures_sink::Sink;
use pin_project_lite::pin_project;
use tokio::io::AsyncWrite;

pub trait IntoSinkExt: AsyncWrite {
    /// See the [module documentation](mod@self).
    fn into_sink<Item>(self) -> IntoSink<Self, Item>
    where
        Self: Sized;
}

impl<W> IntoSinkExt for W
where
    W: AsyncWrite,
{
    fn into_sink<Item>(self) -> IntoSink<Self, Item>
    where
        Self: Sized,
    {
        IntoSink {
            writer: self,
            buffer: None,
        }
    }
}

#[derive(Debug)]
struct Cursor<T> {
    offset: usize,
    inner: T,
}

pin_project! {
    /// See the [module documentation](mod@self).
    #[derive(Debug)]
    pub struct IntoSink<W, Item> {
        #[pin]
        writer: W,
        buffer: Option<Cursor<Item>>,
    }
}

impl<W, Item> IntoSink<W, Item>
where
    W: AsyncWrite,
    Item: AsRef<[u8]>,
{
    /// If we have an outstanding block in `buffer` attempt to push it into the writer, does _not_
    /// flush the writer after it succeeds in pushing the block into it.
    fn poll_flush_buffer(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<io::Result<()>> {
        let mut this = self.project();
        if let Some(cursor) = this.buffer {
            loop {
                let bytes = cursor.inner.as_ref();
                let written = ready!(this.writer.as_mut().poll_write(cx, &bytes[cursor.offset..]))?;
                cursor.offset += written;
                if cursor.offset == bytes.len() {
                    break;
                }
            }
        }
        *this.buffer = None;
        Poll::Ready(Ok(()))
    }
}

impl<W, Item> Sink<Item> for IntoSink<W, Item>
where
    W: AsyncWrite,
    Item: AsRef<[u8]>,
{
    type Error = io::Error;

    fn poll_ready(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.poll_flush_buffer(cx))?;
        Poll::Ready(Ok(()))
    }

    fn start_send(self: Pin<&mut Self>, item: Item) -> Result<(), Self::Error> {
        debug_assert!(self.buffer.is_none());
        *self.project().buffer = Some(Cursor {
            offset: 0,
            inner: item,
        });
        Ok(())
    }

    fn poll_flush(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush_buffer(cx))?;
        ready!(self.project().writer.poll_flush(cx))?;
        Poll::Ready(Ok(()))
    }

    fn poll_close(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        ready!(self.as_mut().poll_flush_buffer(cx))?;
        ready!(self.project().writer.poll_shutdown(cx))?;
        Poll::Ready(Ok(()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use futures::{executor::block_on, stream, StreamExt as _};
    use std::io;

    #[test]
    fn readme() {
        assert!(
            std::process::Command::new("cargo")
                .args(["rdme", "--check"])
                .output()
                .expect("couldn't run `cargo rdme`")
                .status
                .success(),
            "README.md is out of date - bless the new version by running `cargo rdme`"
        )
    }

    #[test]
    fn test() {
        block_on(async {
            let stream = stream::iter(["hello", "world"]).map(io::Result::Ok);
            let mut v = vec![];
            let sink = (&mut v).into_sink();
            stream.forward(sink).await.unwrap();
            assert_eq!(v, b"helloworld");
        })
    }
}
