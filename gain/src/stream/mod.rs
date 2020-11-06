// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! I/O streams.

use crate::core::{self, Stream, StreamFlags};

pub mod buf;

/// Data subscriber and receiver.
pub trait Recv {
    /// Receive data packets repeatedly.  Returns a future.
    ///
    /// The initial reception capacity determines the maximum data packet size
    /// which the first receptor invocation may receive.  Subsequent reception
    /// capacity is the sum of the unused capacity (subscribed minus received)
    /// and the number returned by the receptor.
    ///
    /// The receptor must be prepared to handle as much data as is subscribed
    /// at any given time.
    ///
    /// The call returns once the stream is closed or the reception capacity
    /// drops to zero.
    fn recv<R>(&mut self, capacity: usize, receptor: R) -> future::Recv<R>
    where
        R: Fn(&[u8]) -> usize + Unpin;
}

/// Data writer.
pub trait Write {
    /// Write part of a byte slice.  Returns a future.
    fn write<'a>(&'a mut self, data: &'a [u8]) -> future::Write;

    /// Write a whole byte slice.  Returns a future.
    fn write_all<'a>(&'a mut self, data: &'a [u8]) -> future::WriteAll;
}

/// Stream closer.
pub trait Close {
    /// Close a stream.  Returns a future.
    fn close(&mut self) -> future::Close;
}

pub mod future {
    pub use crate::core::StreamCloseFuture as Close;
    pub use crate::core::StreamRecvFuture as Recv;
    pub use crate::core::StreamWriteAllFuture as WriteAll;
    pub use crate::core::StreamWriteFuture as Write;
}

/// Bidirectional stream.
pub struct RecvWriteStream {
    s: Option<Stream>,
}

impl RecvWriteStream {
    pub(crate) fn new(s: Option<Stream>) -> Self {
        Self { s }
    }

    /// Split the stream into unidirectional parts.
    pub fn split(mut self) -> (RecvStream, WriteStream) {
        let s = self.s.take();
        (RecvStream::new(s.clone()), WriteStream::new(s))
    }

    /// Split the stream into unidirectional parts which can be closed
    /// asynchronously.  The single CloseStream closes both directions.
    pub fn split3(mut self) -> (RecvOnlyStream, WriteOnlyStream, CloseStream) {
        let s = self.s.take();
        (
            RecvOnlyStream::new(s.clone()),
            WriteOnlyStream::new(s.clone()),
            CloseStream::new(s, core::STREAM_SELF_FLOW | core::STREAM_SELF_DATA),
        )
    }
}

impl Default for RecvWriteStream {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Recv for RecvWriteStream {
    fn recv<R>(&mut self, capacity: usize, receptor: R) -> future::Recv<R>
    where
        R: Fn(&[u8]) -> usize + Unpin,
    {
        future::Recv::new(&mut self.s, capacity, receptor)
    }
}

impl Write for RecvWriteStream {
    fn write<'a>(&'a mut self, data: &'a [u8]) -> future::Write {
        future::Write::new(&mut self.s, data)
    }

    fn write_all<'a>(&'a mut self, data: &'a [u8]) -> future::WriteAll {
        future::WriteAll::new(&mut self.s, data)
    }
}

impl Close for RecvWriteStream {
    fn close(&mut self) -> future::Close {
        future::Close::new(
            self.s.take(),
            core::STREAM_SELF_FLOW | core::STREAM_SELF_DATA,
            core::STREAM_PEER_DATA | core::STREAM_PEER_FLOW,
        )
    }
}

impl Drop for RecvWriteStream {
    fn drop(&mut self) {
        core::drop_stream(
            self.s.take(),
            core::STREAM_SELF_FLOW | core::STREAM_SELF_DATA,
        )
    }
}

/// Input stream.
pub struct RecvStream {
    s: Option<Stream>,
}

impl RecvStream {
    pub(crate) fn new(s: Option<Stream>) -> Self {
        Self { s }
    }

    /// Detach the closing functionality.
    pub fn split(mut self) -> (RecvOnlyStream, CloseStream) {
        let s = self.s.take();
        (
            RecvOnlyStream::new(s.clone()),
            CloseStream::new(s, core::STREAM_SELF_FLOW),
        )
    }
}

impl Default for RecvStream {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl From<RecvWriteStream> for RecvStream {
    fn from(stream: RecvWriteStream) -> Self {
        let (r, _) = stream.split();
        r
    }
}

impl Recv for RecvStream {
    fn recv<R>(&mut self, capacity: usize, receptor: R) -> future::Recv<R>
    where
        R: Fn(&[u8]) -> usize + Unpin,
    {
        future::Recv::new(&mut self.s, capacity, receptor)
    }
}

impl Close for RecvStream {
    fn close(&mut self) -> future::Close {
        future::Close::new(
            self.s.take(),
            core::STREAM_SELF_FLOW,
            core::STREAM_PEER_DATA,
        )
    }
}

impl Drop for RecvStream {
    fn drop(&mut self) {
        core::drop_stream(self.s.take(), core::STREAM_SELF_FLOW)
    }
}

/// Input stream which can be closed asynchronously.
pub struct RecvOnlyStream {
    s: Option<Stream>,
}

impl RecvOnlyStream {
    fn new(s: Option<Stream>) -> Self {
        Self { s }
    }
}

impl Default for RecvOnlyStream {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Recv for RecvOnlyStream {
    fn recv<R>(&mut self, capacity: usize, receptor: R) -> future::Recv<R>
    where
        R: Fn(&[u8]) -> usize + Unpin,
    {
        future::Recv::new(&mut self.s, capacity, receptor)
    }
}

impl Drop for RecvOnlyStream {
    fn drop(&mut self) {
        core::drop_stream(self.s.take(), core::STREAM_SELF_FLOW)
    }
}

/// Output stream.
pub struct WriteStream {
    s: Option<Stream>,
}

impl WriteStream {
    pub(crate) fn new(s: Option<Stream>) -> Self {
        Self { s }
    }

    /// Detach the closing functionality.
    pub fn split(mut self) -> (WriteOnlyStream, CloseStream) {
        let s = self.s.take();
        (
            WriteOnlyStream::new(s.clone()),
            CloseStream::new(s, core::STREAM_SELF_DATA),
        )
    }
}

impl Default for WriteStream {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl From<RecvWriteStream> for WriteStream {
    fn from(stream: RecvWriteStream) -> Self {
        let (_, w) = stream.split();
        w
    }
}

impl Write for WriteStream {
    fn write<'a>(&'a mut self, data: &'a [u8]) -> future::Write {
        future::Write::new(&mut self.s, data)
    }

    fn write_all<'a>(&'a mut self, data: &'a [u8]) -> future::WriteAll {
        future::WriteAll::new(&mut self.s, data)
    }
}

impl Close for WriteStream {
    fn close(&mut self) -> future::Close {
        future::Close::new(
            self.s.take(),
            core::STREAM_SELF_DATA,
            core::STREAM_PEER_FLOW,
        )
    }
}

impl Drop for WriteStream {
    fn drop(&mut self) {
        core::drop_stream(self.s.take(), core::STREAM_SELF_DATA)
    }
}

/// Output stream which can be closed asynchronously.
pub struct WriteOnlyStream {
    s: Option<Stream>,
}

impl WriteOnlyStream {
    fn new(s: Option<Stream>) -> Self {
        Self { s }
    }
}

impl Default for WriteOnlyStream {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Write for WriteOnlyStream {
    fn write<'a>(&'a mut self, data: &'a [u8]) -> future::Write {
        future::Write::new(&mut self.s, data)
    }

    fn write_all<'a>(&'a mut self, data: &'a [u8]) -> future::WriteAll {
        future::WriteAll::new(&mut self.s, data)
    }
}

impl Drop for WriteOnlyStream {
    fn drop(&mut self) {
        core::drop_stream(self.s.take(), core::STREAM_SELF_DATA)
    }
}

/// Used to close associated `RecvOnlyStream` and/or `WriteOnlyStream`.
pub struct CloseStream {
    s: Option<Stream>,
    how: StreamFlags,
}

impl CloseStream {
    fn new(s: Option<Stream>, how: StreamFlags) -> Self {
        Self { s, how }
    }
}

impl Default for CloseStream {
    fn default() -> Self {
        Self::new(Default::default(), 0)
    }
}

impl Close for CloseStream {
    fn close(&mut self) -> future::Close {
        let wait = self.how << 2; // STREAM_SELF -> STREAM_PEER
        future::Close::new(self.s.take(), self.how, wait)
    }
}

impl Drop for CloseStream {
    fn drop(&mut self) {
        core::drop_stream(self.s.take(), self.how)
    }
}
