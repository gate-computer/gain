// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Buffered I/O streams.

use std::cell::RefCell;
use std::io;
use std::num::NonZeroI32;
use std::rc::Rc;
use std::task::Waker;

use crate::stream::{
    Close, CloseStream, ErrorCode, Recv, RecvOnlyStream, RecvStream, RecvWriteStream, Write,
    WriteOnlyStream,
};

#[derive(PartialEq)]
pub(crate) enum BufResult {
    Pending,
    Eof,
    Err(ErrorCode),
}

/// Read buffer.
pub struct Buf {
    pub(crate) data: Vec<u8>,
    pub(crate) result: BufResult,
    pub(crate) waker: Option<Waker>,
}

impl Buf {
    pub(crate) fn new(result: BufResult) -> Self {
        Self {
            data: Vec::new(),
            result,
            waker: None,
        }
    }

    /// Returns the number of buffered bytes.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Access the buffered bytes.
    pub fn as_slice(&self) -> &[u8] {
        self.data.as_slice()
    }

    /// Remove bytes from the start of the buffer.
    pub fn consume(&mut self, n: usize) {
        self.data = self.data.split_off(n);
    }

    /// Remove all bytes from the buffer.
    pub fn consume_all(&mut self) {
        self.data = Vec::new();
    }
}

impl io::Read for Buf {
    fn read(&mut self, mut dest: &mut [u8]) -> io::Result<usize> {
        let n = io::Write::write(&mut dest, self.data.as_slice())?;
        if n > 0 {
            self.data = self.data.split_off(n);
        }
        Ok(n)
    }
}

pub(crate) type SharedBuf = Rc<RefCell<Buf>>;

/// Buffered data reader.
pub trait Read {
    /// Read some bytes into a slice.  Returns a future.
    fn read<'a>(&'a mut self, dest: &'a mut [u8]) -> future::Read;

    /// Read buffered data.  Returns a future.
    ///
    /// The receptor must be prepared to handle as much data as the buffer can
    /// hold.
    ///
    /// The value returned by the receptor is passed through.  If the stream
    /// has been closed, the default value is returned.
    fn buf_read<'a, R, T>(&'a mut self, min_read: usize, receptor: R) -> future::BufRead<'a, R, T>
    where
        R: FnOnce(&mut Buf) -> T + Unpin,
        T: Default;
}

pub mod future {
    use std::future::Future;
    use std::io;
    use std::pin::Pin;
    use std::task::{Context, Poll};

    use super::{Buf, BufResult, SharedBuf};

    /// Asynchronous read.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct Read<'a> {
        pub(crate) shared: &'a mut SharedBuf,
        pub(crate) dest: &'a mut [u8],
    }

    impl<'a> Future for Read<'a> {
        type Output = io::Result<usize>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let m = self.get_mut();
            let mut buf = m.shared.borrow_mut();

            if !buf.data.is_empty() {
                Poll::Ready(io::Read::read(&mut *buf, &mut m.dest))
            } else {
                match buf.result {
                    BufResult::Pending => {
                        buf.waker = Some(cx.waker().clone());
                        Poll::Pending
                    }
                    BufResult::Eof => Poll::Ready(Ok(0)),
                    BufResult::Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e))),
                }
            }
        }
    }

    /// Asynchronous read.
    #[must_use = "futures do nothing unless you `.await` or poll them"]
    pub struct BufRead<'a, R, T>
    where
        R: FnOnce(&mut Buf) -> T + Unpin,
        T: Default,
    {
        pub(crate) shared: &'a mut SharedBuf,
        pub(crate) min_read: usize,
        pub(crate) receptor: Option<R>,
    }

    impl<'a, R, T> Future for BufRead<'a, R, T>
    where
        R: FnOnce(&mut Buf) -> T + Unpin,
        T: Default,
    {
        type Output = io::Result<T>;

        fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
            let mut min_read = self.min_read;

            let m = self.get_mut();
            let mut buf = m.shared.borrow_mut();

            if buf.result != BufResult::Pending {
                min_read = 1;
            }

            if buf.len() >= min_read {
                Poll::Ready(Ok((m.receptor.take().unwrap())(&mut buf)))
            } else {
                match buf.result {
                    BufResult::Pending => {
                        buf.waker = Some(cx.waker().clone());
                        Poll::Pending
                    }
                    BufResult::Eof => Poll::Ready(Ok(Default::default())),
                    BufResult::Err(e) => Poll::Ready(Err(io::Error::new(io::ErrorKind::Other, e))),
                }
            }
        }
    }
}

async fn receive(shared: SharedBuf, mut stream: RecvOnlyStream, capacity: usize) {
    let note = stream
        .recv(capacity, |src: &[u8], _: i32| {
            let mut buf = shared.borrow_mut();
            buf.data.extend_from_slice(src);
            if let Some(w) = buf.waker.take() {
                w.wake();
            }
            src.len()
        })
        .await
        .unwrap();

    let mut buf = shared.borrow_mut();
    buf.result = match NonZeroI32::new(note) {
        None => BufResult::Eof,
        Some(n) => BufResult::Err(ErrorCode(n)),
    };
    if let Some(w) = buf.waker.take() {
        w.wake();
    }
}

/// Buffer size used by `ReadStream::new` and `ReadWriteStream::new`.
pub const DEFAULT_READ_CAPACITY: usize = 8192;

/// Buffered input stream.
pub struct ReadStream {
    shared: SharedBuf,
    closer: CloseStream,
}

impl ReadStream {
    /// Convert an unbuffered input stream into a buffered input stream.
    pub fn new(stream: RecvStream) -> Self {
        Self::with_capacity(DEFAULT_READ_CAPACITY, stream)
    }

    /// Convert an unbuffered input stream into an input stream with custom
    /// buffer size.
    pub fn with_capacity(capacity: usize, stream: RecvStream) -> Self {
        let (receiver, closer) = stream.split();
        Self::with_custom_closer(capacity, receiver, closer)
    }

    fn with_custom_closer(capacity: usize, receiver: RecvOnlyStream, closer: CloseStream) -> Self {
        let shared: SharedBuf = Rc::new(RefCell::new(Buf::new(BufResult::Pending)));
        crate::task::spawn_local(receive(shared.clone(), receiver, capacity));
        Self { shared, closer }
    }
}

impl Default for ReadStream {
    fn default() -> Self {
        Self {
            shared: Rc::new(RefCell::new(Buf::new(BufResult::Eof))),
            closer: Default::default(),
        }
    }
}

impl From<RecvStream> for ReadStream {
    fn from(stream: RecvStream) -> Self {
        Self::new(stream)
    }
}

impl Read for ReadStream {
    fn read<'a>(&'a mut self, dest: &'a mut [u8]) -> future::Read {
        future::Read {
            shared: &mut self.shared,
            dest,
        }
    }

    fn buf_read<'a, R, T>(&'a mut self, min_read: usize, receptor: R) -> future::BufRead<'a, R, T>
    where
        R: FnOnce(&mut Buf) -> T + Unpin,
        T: Default,
    {
        if min_read == 0 {
            panic!("minimum read length is zero");
        }

        future::BufRead {
            shared: &mut self.shared,
            min_read,
            receptor: Some(receptor),
        }
    }
}

impl Close for ReadStream {
    fn close(&mut self) -> super::future::Close {
        self.closer.close()
    }
}

/// Bidirectional stream with input buffering.
pub struct ReadWriteStream {
    r: ReadStream,
    w: WriteOnlyStream,
}

impl ReadWriteStream {
    /// Convert an unbuffered stream into a stream with input buffering.
    pub fn new(stream: RecvWriteStream) -> Self {
        Self::with_read_capacity(DEFAULT_READ_CAPACITY, stream)
    }

    /// Convert an unbuffered stream into a stream with custom input buffer
    /// size.
    pub fn with_read_capacity(capacity: usize, stream: RecvWriteStream) -> Self {
        let (receiver, writer, closer) = stream.split3();
        Self {
            r: ReadStream::with_custom_closer(capacity, receiver, closer),
            w: writer,
        }
    }
}

impl Default for ReadWriteStream {
    fn default() -> Self {
        Self {
            r: Default::default(),
            w: Default::default(),
        }
    }
}

impl From<RecvWriteStream> for ReadWriteStream {
    fn from(stream: RecvWriteStream) -> Self {
        Self::new(stream)
    }
}

impl Read for ReadWriteStream {
    fn read<'a>(&'a mut self, dest: &'a mut [u8]) -> future::Read {
        self.r.read(dest)
    }

    fn buf_read<'a, R, T>(&'a mut self, min_read: usize, receptor: R) -> future::BufRead<'a, R, T>
    where
        R: FnOnce(&mut Buf) -> T + Unpin,
        T: Default,
    {
        self.r.buf_read(min_read, receptor)
    }
}

impl Write for ReadWriteStream {
    fn write<'a>(&'a mut self, data: &'a [u8]) -> super::future::Write {
        self.w.write(data)
    }

    fn write_note<'a>(&'a mut self, data: &'a [u8], note: i32) -> super::future::Write {
        self.w.write_note(data, note)
    }

    fn write_all<'a>(&'a mut self, data: &'a [u8]) -> super::future::WriteAll {
        self.w.write_all(data)
    }
}

impl Close for ReadWriteStream {
    fn close(&mut self) -> super::future::Close {
        self.r.close()
    }
}
