// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use std::cell::RefCell;
use std::collections::{HashMap, HashSet};
use std::error;
use std::fmt;
use std::future::Future;
use std::io::{self, Error, ErrorKind};
use std::marker::PhantomData;
use std::mem::take;
use std::mem::transmute;
use std::num::NonZeroI32;
use std::pin::Pin;
use std::process::exit;
use std::ptr::NonNull;
use std::rc::Rc;
use std::slice;
use std::task::{Context, Poll, Waker};
use std::time::Duration;

use crate::gate::{self, Ciovec, Iovec, MAX_RECV_SIZE};
use crate::packet::{
    self, Code, StreamId, ALIGNMENT, CODE_SERVICES, DATA_HEADER_SIZE, DOMAIN_CALL, DOMAIN_DATA,
    DOMAIN_FLOW, DOMAIN_INFO, FLOW_SIZE, HEADER_SIZE, SERVICES_HEADER_SIZE, SERVICE_STATE_AVAIL,
};
use crate::service::RegistrationError;
use crate::task::spawn_local;
use crate::threadunsafe::ThreadUnsafeRefCell;

static PADDING: [u8; ALIGNMENT] = [0; ALIGNMENT];

lazy_static! {
    static ref SERVICE_NAMES: ThreadUnsafeRefCell<HashSet<&'static str>> = Default::default();
    static ref SERVICE_STATES: ThreadUnsafeRefCell<Vec<ServiceState>> = Default::default();
    static ref STREAMS: ThreadUnsafeRefCell<HashMap<(Code, StreamId), Stream>> = Default::default();
    static ref SEND_LIST: ThreadUnsafeRefCell<SendList> = Default::default();
    static ref RECV_BUF: ThreadUnsafeRefCell<RecvBuf> = Default::default();
}

struct ServiceState {
    avail_or_blocked: SendList,
    replies: SendList,
    info_recv: Recv,
}

impl ServiceState {
    fn new_unavail() -> Self {
        Self {
            avail_or_blocked: SendList::default(),
            replies: SendList::default(),
            info_recv: Recv::None,
        }
    }

    fn is_avail(&self) -> bool {
        !self.avail_or_blocked.is_valid()
    }

    fn blocked(&mut self) -> Option<&mut SendList> {
        if self.is_avail() {
            None
        } else {
            Some(&mut self.avail_or_blocked)
        }
    }

    fn set_avail_unchecked(&mut self) -> SendList {
        self.avail_or_blocked.invalidate_unchecked()
    }

    fn set_unavail_unchecked(&mut self) -> &mut SendList {
        self.avail_or_blocked = SendList::default();
        &mut self.avail_or_blocked
    }
}

#[derive(Clone, Copy, Default)]
struct RecvSpan {
    off: usize,
    end: usize,
}

impl RecvSpan {
    fn is_empty(&self) -> bool {
        self.off == self.end
    }
}

struct RecvBuf {
    buf: Vec<u8>,
    head: RecvSpan,
    tail: RecvSpan,
}

impl RecvBuf {
    fn head_slice(&'_ self) -> &'_ [u8] {
        &self.buf[self.head.off..self.head.end]
    }

    fn consume(&'_ mut self, off: usize) -> &'_ [u8] {
        if self.head.off != off {
            die("consumed offset not found at head of receive buffer");
        }
        self.consumed();
        let end = off + packet::size(&self.buf[off..]);
        &self.buf[off..end]
    }

    fn consumed(&mut self) {
        let new_off = packet::align(self.head.off + packet::size(&self.buf[self.head.off..]));
        if new_off > self.head.end {
            die("receive buffer head span is too short when consuming packet");
        }
        self.head.off = new_off;
        if self.head.is_empty() {
            self.head = self.tail;
            self.tail = RecvSpan::default();
        }
    }
}

impl Default for RecvBuf {
    fn default() -> Self {
        // Array caused out-of-bounds memory access when RECV_BUF is borrowed.
        Self {
            buf: vec![0; MAX_RECV_SIZE * 2],
            head: RecvSpan::default(),
            tail: RecvSpan::default(),
        }
    }
}

pub type StreamFlags = u8;

pub const STREAM_SELF_FLOW: StreamFlags = 1 << 0;
pub const STREAM_SELF_DATA: StreamFlags = 1 << 1;
pub const STREAM_PEER_DATA: StreamFlags = STREAM_SELF_FLOW << 2;
pub const STREAM_PEER_FLOW: StreamFlags = STREAM_SELF_DATA << 2;

enum Recv {
    None,
    Wake(Waker),
    Some(usize),
}

impl Recv {
    fn take_waker(&mut self) -> Option<Waker> {
        if let Self::Wake(_) = self {
            if let Self::Wake(w) = take(self) {
                return Some(w);
            }
        }

        None
    }
}

impl Default for Recv {
    fn default() -> Self {
        Self::None
    }
}

pub type Stream = Rc<RefCell<StreamState>>;

pub struct StreamState {
    code: Code,
    id: StreamId,
    flags: StreamFlags,

    recv: Recv,
    recv_err: i32,
    writable: usize,
    writer: Option<Waker>,
    write_err: i32,
    closers: Vec<Waker>,

    close_flow_share: Share,
    close_flow_packet: [u8; HEADER_SIZE + FLOW_SIZE],
    close_data_share: Share,
    close_data_packet: [u8; DATA_HEADER_SIZE],
}

impl StreamState {
    fn new(code: Code, id: StreamId, flags: StreamFlags) -> Self {
        StreamState {
            code,
            id,
            flags,

            recv: Recv::None,
            recv_err: 0,
            writable: 0,
            writer: None,
            write_err: 0,
            closers: Vec::new(),

            close_flow_share: Share::default(),
            close_flow_packet: [0; HEADER_SIZE + FLOW_SIZE],
            close_data_share: Share::default(),
            close_data_packet: [0; DATA_HEADER_SIZE],
        }
    }

    fn clear_flags(&mut self, how: StreamFlags) {
        if (self.flags & how) != how {
            panic!("stream state does not contain closing flags");
        }

        self.flags &= !how;
    }

    fn send_close_packets(&mut self, how: StreamFlags) {
        if (self.flags & how) != 0 {
            panic!("stream state still contains closing flags when sending packet");
        }

        let mut send_list = SEND_LIST.borrow_mut();

        if (how & STREAM_SELF_FLOW) != 0 {
            let len = self.close_flow_packet.len();
            packet::header_into(&mut self.close_flow_packet, len, self.code, DOMAIN_FLOW);
            packet::flow_into(&mut self.close_flow_packet, 0, self.id, 0);
            self.close_flow_share.send[0] = Ciovec::new(&self.close_flow_packet);
            send_list.push_back(SendLink::new(&mut self.close_flow_share));
        }

        if (how & STREAM_SELF_DATA) != 0 {
            let len = self.close_data_packet.len();
            packet::data_header_into(&mut self.close_data_packet, len, self.code, self.id, 0);
            self.close_data_share.send[0] = Ciovec::new(&self.close_data_packet);
            send_list.push_back(SendLink::new(&mut self.close_data_share));
        }
    }

    fn detach_closed(&self) {
        if self.flags == 0 {
            STREAMS.borrow_mut().remove(&(self.code, self.id));
        }
    }
}

struct SendList {
    front: SendLink,
    back: SendLink,
}

impl SendList {
    fn invalid() -> Self {
        Self {
            front: SendLink::invalid(),
            back: SendLink::invalid(),
        }
    }

    fn is_valid(&self) -> bool {
        self.front.is_valid()
    }

    fn invalidate_unchecked(&mut self) -> Self {
        let valid = Self {
            front: self.front,
            back: self.back,
        };
        *self = Self::invalid();
        valid
    }

    fn push_back(&mut self, new: SendLink) {
        if let Some(share) = self.back.as_mut() {
            share.next = new;
            self.back = new;
        } else {
            self.front = new;
            self.back = new;
        }
    }

    fn pop_front(&mut self) -> Option<SendLink> {
        let mut old = self.front.take();
        if let Some(share) = old.as_mut() {
            self.front = share.next.take();
            if self.front.is_none() {
                self.back = SendLink::none();
            }
            Some(old)
        } else {
            None
        }
    }

    fn remove(&mut self, index: usize) -> SendLink {
        if index == 0 {
            self.pop_front().unwrap()
        } else {
            unsafe { self.remove_nonzero_index(index) }
        }
    }

    unsafe fn remove_nonzero_index(&mut self, index: usize) -> SendLink {
        let mut prev = self.front;
        for _ in 1..index {
            prev = prev.as_mut().unwrap().next;
        }
        let mut prev_share = prev.as_mut().unwrap();
        let mut old = prev_share.next.take();
        let old_share = old.as_mut().unwrap();
        prev_share.next = old_share.next.take();
        if prev_share.next.is_none() {
            self.back = prev;
        }
        old
    }
}

impl Default for SendList {
    fn default() -> Self {
        Self {
            front: SendLink::none(),
            back: SendLink::none(),
        }
    }
}

#[derive(Copy, Clone)]
struct SendLink {
    addr: usize,
}

impl SendLink {
    fn new(ptr: &mut Share) -> Self {
        Self {
            addr: ptr as *mut _ as usize,
        }
    }

    fn none() -> Self {
        Self { addr: 0 }
    }

    fn invalid() -> Self {
        Self {
            addr: usize::max_value(),
        }
    }

    fn is_valid(&self) -> bool {
        self.addr != usize::max_value()
    }

    fn is_none(&self) -> bool {
        self.addr == 0
    }

    fn is_nop(&mut self) -> bool {
        if let Some(share) = self.as_mut() {
            share.is_nop()
        } else {
            false
        }
    }

    fn take(&mut self) -> Self {
        let link = *self;
        self.addr = 0;
        link
    }

    fn as_mut<'a>(&'a mut self) -> Option<&'a mut Share> {
        if self.addr != 0 {
            Some(unsafe {
                transmute::<&mut Share, &'a mut Share>(
                    NonNull::new_unchecked(self.addr as *mut Share).as_mut(),
                )
            })
        } else {
            None
        }
    }
}

struct Reply {
    x: isize,
}

impl Reply {
    fn expected() -> Self {
        Self { x: -1 }
    }

    fn is_expected(&self) -> bool {
        self.x == -1
    }

    fn set_offset(&mut self, offset: usize) {
        self.x = offset as isize;
    }

    fn offset(&self) -> Option<usize> {
        if self.x >= 0 {
            Some(self.x as usize)
        } else {
            None
        }
    }
}

impl Default for Reply {
    fn default() -> Self {
        Self { x: -2 } // Reply not expected.
    }
}

struct Share {
    send: [Ciovec; 2],
    sent: usize,
    reply: Reply,
    waker: Option<Waker>,
    next: SendLink,
}

impl Share {
    fn header(&self) -> &[u8] {
        unsafe { slice::from_raw_parts(self.send[0].buf, self.send[0].buf_len) }
    }

    fn code(&self) -> Code {
        packet::code(self.header())
    }

    fn unaligned_send_len(&self) -> usize {
        let mut len = 0;
        for span in self.send.iter() {
            len += span.buf_len;
        }
        len
    }

    fn is_sent(&self) -> bool {
        self.sent == packet::align(self.unaligned_send_len())
    }

    fn is_nop(&self) -> bool {
        !self.reply.is_expected() && self.unaligned_send_len() == 0
    }
}

impl Default for Share {
    fn default() -> Self {
        Self {
            send: [Ciovec::default(), Ciovec::default()],
            sent: 0,
            reply: Reply::default(),
            waker: None,
            next: SendLink::none(),
        }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
struct ServiceFuture {
    share: Share,
    packet: Vec<u8>,
    started: bool,
}

impl ServiceFuture {
    fn new(packet: Vec<u8>) -> Self {
        let mut this = Self {
            share: Share::default(),
            packet,
            started: false,
        };
        this.share.send[0] = Ciovec::new(this.packet.as_slice());
        this
    }
}

impl Future for ServiceFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if !self.started {
            SEND_LIST
                .borrow_mut()
                .push_back(SendLink::new(&mut self.share));

            self.started = true;
        } else if self.share.is_sent() {
            return Poll::Ready(());
        }

        self.share.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

/// Asynchronous call.  Must be polled to completion.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct CallFuture<'a, R, T>
where
    R: FnOnce(&[u8]) -> T + Unpin,
{
    share: Share,
    header: [u8; HEADER_SIZE],
    _content: PhantomData<&'a [u8]>,
    receptor: Option<R>,
    polling: bool,
}

impl<'a, R, T> CallFuture<'a, R, T>
where
    R: FnOnce(&[u8]) -> T + Unpin,
{
    pub(crate) fn new(code: Code, content: &'a [u8], receptor: R) -> Self {
        let mut this = Self {
            share: Share::default(),
            header: [0; HEADER_SIZE],
            _content: PhantomData,
            receptor: Some(receptor),
            polling: false,
        };
        let len = this.header.len() + content.len();
        packet::header_into(&mut this.header, len, code, DOMAIN_CALL);
        // Header may move before polling, so its address cannot be taken yet.
        this.share.send[1] = Ciovec::new(content);
        this.share.reply = Reply::expected();
        this
    }
}

impl<R, T> Future for CallFuture<'_, R, T>
where
    R: FnOnce(&[u8]) -> T + Unpin,
{
    type Output = T;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if !self.polling {
            self.share.send[0] = Ciovec::new(&self.header);
            let mut link = Some(SendLink::new(&mut self.share));

            let code = self.share.code();
            if code >= 0 {
                let mut service_states = SERVICE_STATES.borrow_mut();
                if let Some(list) = service_states[code as usize].blocked() {
                    list.push_back(link.take().unwrap());
                }
            }

            if let Some(link) = link.take() {
                SEND_LIST.borrow_mut().push_back(link);
            }
        } else if let Some(offset) = self.share.reply.offset() {
            let mut recv_buf = RECV_BUF.borrow_mut();
            let x = (self.receptor.take().unwrap())(&recv_buf.consume(offset)[HEADER_SIZE..]);
            self.polling = false;
            return Poll::Ready(x);
        }

        self.share.waker = Some(cx.waker().clone());
        self.polling = true;
        Poll::Pending
    }
}

impl<R, T> Drop for CallFuture<'_, R, T>
where
    R: FnOnce(&[u8]) -> T + Unpin,
{
    fn drop(&mut self) {
        let this = unsafe { Pin::new_unchecked(self) }; // See pin module doc.

        if this.polling {
            die("call future dropped before completion");
        }
    }
}

/// Asynchronous info packet reception.  Must be polled to completion once
/// started.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct InfoRecvFuture<R>
where
    R: Fn(&[u8]) + Unpin,
{
    code: Code,
    receptor: R,
}

impl<R> InfoRecvFuture<R>
where
    R: Fn(&[u8]) + Unpin,
{
    pub(crate) fn new(code: Code, receptor: R) -> Self {
        Self { code, receptor }
    }
}

impl<R> Future for InfoRecvFuture<R>
where
    R: Fn(&[u8]) + Unpin,
{
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let mut service_states = SERVICE_STATES.borrow_mut();
        let service = &mut service_states[self.code as usize];

        if let Recv::Some(offset) = take(&mut service.info_recv) {
            let mut recv_buf = RECV_BUF.borrow_mut();
            (self.receptor)(&recv_buf.consume(offset)[HEADER_SIZE..]);
        }

        service.info_recv = Recv::Wake(cx.waker().clone());
        Poll::Pending
    }
}

/// Asynchronous info packet send.  Must be polled to completion.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct InfoSendFuture<'a> {
    share: Share,
    header: [u8; HEADER_SIZE],
    _content: PhantomData<&'a [u8]>,
    polling: bool,
}

impl<'a> InfoSendFuture<'a> {
    pub(crate) fn new(code: Code, content: &'a [u8]) -> Self {
        let mut this = Self {
            share: Share::default(),
            header: [0; HEADER_SIZE],
            _content: PhantomData,
            polling: false,
        };
        let len = this.header.len() + content.len();
        packet::header_into(&mut this.header, len, code, DOMAIN_INFO);
        // Header may move before polling, so its address cannot be taken yet.
        this.share.send[1] = Ciovec::new(content);
        this
    }
}

impl Future for InfoSendFuture<'_> {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if !self.polling {
            self.share.send[0] = Ciovec::new(&self.header);
            let mut link = Some(SendLink::new(&mut self.share));

            let code = self.share.code();
            if code >= 0 {
                let mut service_states = SERVICE_STATES.borrow_mut();
                if let Some(list) = service_states[code as usize].blocked() {
                    list.push_back(link.take().unwrap());
                }
            }

            if let Some(link) = link.take() {
                SEND_LIST.borrow_mut().push_back(link);
            }
        } else if self.share.is_sent() {
            self.polling = false;
            return Poll::Ready(());
        }

        self.share.waker = Some(cx.waker().clone());
        self.polling = true;
        Poll::Pending
    }
}

impl Drop for InfoSendFuture<'_> {
    fn drop(&mut self) {
        let this = unsafe { Pin::new_unchecked(self) }; // See pin module doc.

        if this.polling {
            die("info future dropped before completion");
        }
    }
}

/// Asynchronous reception.  Must be polled to completion once started.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct StreamRecvFuture<'a, R>
where
    R: Fn(&[u8], i32) -> usize + Unpin,
{
    s: &'a Option<Stream>,
    receptor: R,
    unsubscribed: u64, // Requested by caller, but flow packet not sent yet.
    unreceived: i32,   // Flow packet sent, but data not received yet.
    flow_share: Share,
    flow_packet: [u8; HEADER_SIZE + FLOW_SIZE],
}

impl<'a, R> StreamRecvFuture<'a, R>
where
    R: Fn(&[u8], i32) -> usize + Unpin,
{
    pub(crate) fn new(s: &'a Option<Stream>, cap: usize, receptor: R) -> Self {
        Self {
            s,
            receptor,
            unsubscribed: cap as u64,
            unreceived: 0,
            flow_share: Share::default(),
            flow_packet: [0; HEADER_SIZE + FLOW_SIZE],
        }
    }

    fn flow_increment(&self) -> i32 {
        let max_flow = i32::max_value() - self.unreceived;
        std::cmp::min(self.unsubscribed, max_flow as u64) as i32
    }

    fn can_send_flow_packet(&self) -> bool {
        self.flow_increment() > 0 && self.flow_share.is_sent()
    }

    fn send_flow_packet(&mut self, id: StreamId) {
        let increment = self.flow_increment();
        self.unsubscribed -= increment as u64;
        self.unreceived += increment;
        packet::flow_into(&mut self.flow_packet, 0, id, increment);

        self.flow_share.sent = 0;
        SEND_LIST
            .borrow_mut()
            .push_back(SendLink::new(&mut self.flow_share));
    }
}

impl<R> Future for StreamRecvFuture<'_, R>
where
    R: Fn(&[u8], i32) -> usize + Unpin,
{
    type Output = Option<i32>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if let Some(s) = self.s {
            let mut s = s.borrow_mut();

            if self.can_send_flow_packet() {
                let len = self.flow_packet.len();
                packet::header_into(&mut self.flow_packet, len, s.code, DOMAIN_FLOW);
                self.flow_share.send[0] = Ciovec::new(&self.flow_packet);
                self.send_flow_packet(s.id);
            }

            if let Recv::Some(offset) = take(&mut s.recv) {
                let mut recv_buf = RECV_BUF.borrow_mut();
                let p = recv_buf.consume(offset);
                let note = packet::data_note(p);
                let data = &p[DATA_HEADER_SIZE..];

                if data.len() > self.unreceived as usize {
                    panic!("received data exceeds subscription");
                }
                self.unreceived -= data.len() as i32;

                if let Some(n) = self
                    .unsubscribed
                    .checked_add((self.receptor)(data, note) as u64)
                {
                    self.unsubscribed = n;
                } else {
                    panic!("reception capacity out of bounds");
                }

                if self.can_send_flow_packet() {
                    self.send_flow_packet(s.id);
                }
            }

            if (s.flags & STREAM_PEER_DATA) == 0 {
                return Poll::Ready(Some(s.recv_err)); // Closed.
            }

            if self.unsubscribed == 0 && self.unreceived == 0 {
                return Poll::Ready(None); // Kept open.
            }

            s.recv = Recv::Wake(cx.waker().clone());
            return Poll::Pending;
        }

        Poll::Ready(Some(0)) // Closed; default note.
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
pub struct StreamErrorCode(pub NonZeroI32);

impl error::Error for StreamErrorCode {}

impl fmt::Display for StreamErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

/// Asynchronous write.  Must be polled to completion once started.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct StreamWriteFuture<'a> {
    s: &'a Option<Stream>,
    share: Share,
    header: [u8; DATA_HEADER_SIZE],
    _data: PhantomData<&'a [u8]>,
    note: i32,
    writing: bool,
}

impl<'a> StreamWriteFuture<'a> {
    pub(crate) fn new(s: &'a Option<Stream>, data: &'a [u8], note: i32) -> Self {
        let mut this = Self {
            s,
            share: Share::default(),
            header: [0; DATA_HEADER_SIZE],
            _data: PhantomData,
            note,
            writing: false,
        };
        this.share.send[1] = Ciovec::new(data);
        this
    }
}

impl Future for StreamWriteFuture<'_> {
    type Output = io::Result<usize>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Some(s) = self.s {
            if !self.writing {
                let mut s = s.borrow_mut();

                if (s.flags & STREAM_PEER_FLOW) == 0 {
                    return Poll::Ready(match NonZeroI32::new(s.write_err) {
                        None => Ok(0),
                        Some(n) => Err(io::Error::new(io::ErrorKind::Other, StreamErrorCode(n))),
                    });
                }

                if s.writable == 0 {
                    s.writer = Some(cx.waker().clone());
                    return Poll::Pending;
                }

                if s.writable < self.share.send[1].buf_len {
                    self.share.send[1].buf_len = s.writable;
                }
                s.writable -= self.share.send[1].buf_len;

                let len = self.header.len() + self.share.send[1].buf_len;
                let note = self.note;
                packet::data_header_into(&mut self.header, len, s.code, s.id, note);
                self.share.send[0] = Ciovec::new(&self.header);
                SEND_LIST
                    .borrow_mut()
                    .push_back(SendLink::new(&mut self.share));
                self.writing = true;
            } else if self.share.is_sent() {
                self.writing = false;
                return Poll::Ready(Ok(self.share.send[1].buf_len));
            }

            self.share.waker = Some(cx.waker().clone());
            Poll::Pending
        } else {
            Poll::Ready(Ok(0))
        }
    }
}

impl Drop for StreamWriteFuture<'_> {
    fn drop(&mut self) {
        let this = unsafe { Pin::new_unchecked(self) }; // See pin module doc.

        if this.writing {
            die("write future dropped before completion");
        }
    }
}

/// Asynchronous write.  Must be polled to completion once started.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct StreamWriteAllFuture<'a> {
    inner: StreamWriteFuture<'a>,
    pending: &'a [u8],
}

impl<'a> StreamWriteAllFuture<'a> {
    pub(crate) fn new(s: &'a Option<Stream>, data: &'a [u8]) -> Self {
        Self {
            inner: StreamWriteFuture::new(s, data, 0),
            pending: data,
        }
    }
}

impl Future for StreamWriteAllFuture<'_> {
    type Output = io::Result<()>;

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        loop {
            let n = match unsafe { Pin::new_unchecked(&mut self.inner) }.poll(cx) {
                Poll::Ready(Ok(n)) => n,
                Poll::Ready(Err(e)) => break Poll::Ready(Err(e)),
                Poll::Pending => break Poll::Pending,
            };

            if n == 0 {
                break Poll::Ready(Err(Error::new(
                    ErrorKind::WriteZero,
                    "stream closed".to_string(),
                )));
            }

            self.pending = &self.pending[n..];
            if self.pending.is_empty() {
                break Poll::Ready(Ok(()));
            }

            self.inner.share.send[1] = Ciovec::new(self.pending);
            self.inner.share.sent = 0;
        }
    }
}

/// Asynchronous closure.  Must be polled to completion once started.
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct StreamCloseFuture {
    s: Option<Stream>,
    how: StreamFlags,
    wait: StreamFlags,
}

impl StreamCloseFuture {
    pub(crate) fn new(s: Option<Stream>, how: StreamFlags, wait: StreamFlags) -> Self {
        Self { s, how, wait }
    }
}

impl Future for StreamCloseFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        let how = self.how;
        self.how = 0;

        if let Some(ref s) = self.s {
            let mut s = s.borrow_mut();

            if how != 0 {
                s.clear_flags(how);
                s.send_close_packets(how);
            }

            if (s.flags & self.wait) != 0 {
                s.closers.push(cx.waker().clone());
                return Poll::Pending;
            }

            s.detach_closed();
        }

        self.s = None; // For drop implementation.
        Poll::Ready(())
    }
}

impl Drop for StreamCloseFuture {
    fn drop(&mut self) {
        let this = unsafe { Pin::new_unchecked(self) }; // See pin module doc.

        if this.s.is_some() {
            die("close future dropped before completion");
        }
    }
}

#[must_use = "futures do nothing unless you `.await` or poll them"]
pub(crate) struct YieldFuture {
    share: Share,
    started: bool,
}

impl YieldFuture {
    pub(crate) fn new() -> Self {
        Self {
            share: Share::default(),
            started: false,
        }
    }
}

impl Future for YieldFuture {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if !self.started {
            SEND_LIST
                .borrow_mut()
                .push_back(SendLink::new(&mut self.share));

            self.started = true;
        } else if self.share.is_sent() {
            return Poll::Ready(());
        }

        self.share.waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

pub fn register_service(name: &'static str) -> Result<Code, RegistrationError> {
    let namedata = name.as_bytes();
    if namedata.is_empty() || namedata.len() > 127 {
        panic!("service name length out of bounds");
    }

    if !SERVICE_NAMES.borrow_mut().insert(name) {
        return Err(RegistrationError::NameAlreadyRegistered);
    }

    let mut service_states = SERVICE_STATES.borrow_mut();
    if service_states.len() > Code::max_value() as usize {
        return Err(RegistrationError::TooManyServices);
    }

    service_states.reserve(1);

    let size = SERVICES_HEADER_SIZE + 1 + name.len();
    let mut p = Vec::with_capacity(size);
    p.resize(SERVICES_HEADER_SIZE, 0);
    packet::services_header_into(&mut p, size, 1);
    p.push(name.len() as u8);
    p.extend_from_slice(namedata);
    spawn_local(ServiceFuture::new(p));

    let code = service_states.len() as Code;
    service_states.push(ServiceState::new_unavail());
    println!("gain: service #{} name {:?}", code, name);
    Ok(code)
}

pub fn init_stream(code: Code, id: StreamId, flags: StreamFlags) -> Option<Stream> {
    if id < 0 {
        panic!("negative stream id");
    }

    let s = Rc::new(RefCell::new(StreamState::new(code, id, flags)));

    if STREAMS.borrow_mut().insert((code, id), s.clone()).is_some() {
        panic!("stream already exists");
    }

    Some(s)
}

pub fn drop_stream(s: Option<Stream>, how: StreamFlags) {
    if let Some(s) = s {
        let mut s = s.borrow_mut();
        let how = how & s.flags;
        s.clear_flags(how);
        s.send_close_packets(how);
        s.detach_closed();
    }
}

fn peer_closed_stream(s: &mut StreamState, how: StreamFlags) {
    s.clear_flags(how);

    while let Some(w) = s.closers.pop() {
        w.wake();
    }

    if (how & STREAM_PEER_DATA) != 0 {
        if let Some(w) = s.recv.take_waker() {
            w.wake()
        }
    }

    if (how & STREAM_PEER_FLOW) != 0 {
        if let Some(w) = s.writer.take() {
            w.wake();
        }
    }
}

pub fn io() {
    let flags = perform_io();

    if flags & gate::FLAG_STARTED_OR_RESUMED != 0 {
        // TODO
    }

    process_received();
}

fn perform_io() -> u64 {
    let mut send_list = SEND_LIST.borrow_mut();
    let mut recv_buf = RECV_BUF.borrow_mut();

    // Don't perform I/O yet if there are unhandled received packets.
    if !recv_buf.tail.is_empty() {
        return 0;
    } else {
        let head = recv_buf.head_slice();
        if head.len() >= HEADER_SIZE {
            let packet_end = recv_buf.head.off + packet::align(packet::size(head));
            if packet_end <= recv_buf.head.end {
                return 0;
            }
        }
    }

    let mut recv_vec: [Iovec; 2] = [Iovec::default(); 2];
    let recv_vec_len: usize;

    if recv_buf.head.is_empty() {
        // First half of RECV_BUF.
        recv_vec[0].buf = recv_buf.buf.as_mut_ptr();
        recv_vec[0].buf_len = MAX_RECV_SIZE;
        recv_vec_len = 1;
    } else {
        // Append to partially received packet...
        recv_vec[0].buf = unsafe { recv_buf.buf.as_mut_ptr().add(recv_buf.head.end) };

        let packet_head = recv_buf.head_slice();
        if packet_head.len() >= HEADER_SIZE {
            let packet_end = recv_buf.head.off + packet::align(packet::size(packet_head));
            if packet_end < MAX_RECV_SIZE {
                // ...rest of first half of RECV_BUF.
                recv_vec[0].buf_len = MAX_RECV_SIZE - recv_buf.head.end;
                recv_vec_len = 1;
            } else {
                // ...rest of packet.
                recv_vec[0].buf_len = packet_end - recv_buf.head.end;

                // Prefix of first half.
                recv_vec[1].buf = recv_buf.buf.as_mut_ptr();
                recv_vec[1].buf_len = recv_buf.head.off;
                recv_vec_len = 2;
            }
        } else {
            // ...rest of packet header only (should not happen in practice).
            recv_vec[0].buf_len = HEADER_SIZE - packet_head.len();
            recv_vec_len = 1;
        }
    }

    let mut wait = true;

    // Handle yields.
    while send_list.front.is_nop() {
        let mut link = send_list.pop_front().unwrap();
        let share = link.as_mut().unwrap();
        if let Some(w) = share.waker.take() {
            w.wake();
        }

        wait = false;
    }

    let mut send_vec: [Ciovec; 3] = [Ciovec::default(); 3];
    let mut send_vec_len = 0;

    if let Some(share) = send_list.front.as_mut() {
        let mut offset = share.sent as isize;

        for span in share.send.iter() {
            if offset < span.buf_len as isize {
                if offset > 0 {
                    send_vec[send_vec_len].buf = unsafe { span.buf.offset(offset) };
                    send_vec[send_vec_len].buf_len = span.buf_len - offset as usize;
                } else {
                    send_vec[send_vec_len] = *span;
                }
                send_vec_len += 1;
            }

            offset -= span.buf_len as isize;
        }

        let mut n = packet::pad_len(share.send[0].buf_len + share.send[1].buf_len);
        if offset > 0 {
            n -= offset;
        }
        if n > 0 {
            send_vec[send_vec_len].buf = &PADDING[0];
            send_vec[send_vec_len].buf_len = n as usize;
            send_vec_len += 1;
        }
    }

    let timeout = if wait { None } else { Some(Duration::ZERO) };

    let (recv_len, send_len, flags) = unsafe {
        gate::io(
            &recv_vec[..recv_vec_len],
            &send_vec[..send_vec_len],
            timeout,
        )
    };

    if send_len > 0 {
        let share = send_list.front.as_mut().unwrap();
        share.sent += send_len;
        if share.is_sent() {
            let reply = share.reply.is_expected();
            if !reply {
                if let Some(w) = share.waker.take() {
                    w.wake();
                }
            }
            let code = share.code();
            let link = send_list.pop_front().unwrap();
            if reply {
                SERVICE_STATES.borrow_mut()[code as usize]
                    .replies
                    .push_back(link);
            }
        }
    }

    if recv_len > 0 {
        if recv_buf.head.is_empty() {
            // First half of RECV_BUF.
            recv_buf.head.off = 0;
            recv_buf.head.end = recv_len;
        } else {
            // Append to partially received packet...
            let packet_head = recv_buf.head_slice();
            if packet_head.len() >= HEADER_SIZE {
                let packet_size = packet::align(packet::size(packet_head));
                let packet_end = recv_buf.head.off + packet_size;
                if packet_end < MAX_RECV_SIZE {
                    // ...rest of first half of RECV_BUF.
                    recv_buf.head.end += recv_len;
                } else if recv_len <= packet_size {
                    // ...rest of packet.
                    recv_buf.head.end += recv_len;
                } else {
                    // ...rest of packet.
                    let packet_rest = packet_end - recv_buf.head.end;
                    recv_buf.head.end = packet_end;

                    // Prefix of first half.
                    recv_buf.tail.off = 0;
                    recv_buf.tail.end = recv_len - packet_rest;
                }
            } else {
                // ...rest of packet header only.
                recv_buf.head.end += recv_len;
            }
        }
    }

    flags
}

fn process_received() {
    let mut recv_buf = RECV_BUF.borrow_mut();
    let p = recv_buf.head_slice();
    if p.len() < HEADER_SIZE {
        return;
    }

    let size = packet::size(p);
    if recv_buf.head.end < recv_buf.head.off + packet::align(size) {
        return;
    }

    let p = &p[..size];
    let code = packet::code(p);
    let domain = packet::domain(p);
    let index = packet::index(p);
    let mut future_consumer = false;

    if code == CODE_SERVICES {
        match domain {
            DOMAIN_CALL | DOMAIN_INFO => {
                let mut service_states = SERVICE_STATES.borrow_mut();
                let mut send_list = SEND_LIST.borrow_mut();

                for (i, flags) in packet::service_states(p).iter().enumerate() {
                    let service = &mut service_states[i];

                    let avail = (flags & SERVICE_STATE_AVAIL) != 0;
                    if avail == service.is_avail() {
                        continue;
                    }

                    if avail {
                        println!("gain: service #{} available", i);

                        let mut old_blocked = service.set_avail_unchecked();

                        while let Some(x) = old_blocked.pop_front() {
                            send_list.push_back(x);
                        }
                    } else {
                        println!("gain: service #{} unavailable", i);

                        let new_blocked = service.set_unavail_unchecked();

                        let mut prev = SendLink::none();
                        let mut curr = send_list.front;

                        while let Some(curr_share) = curr.as_mut() {
                            let next = curr_share.next;

                            if curr_share.code() == i as Code {
                                if let Some(prev_share) = prev.as_mut() {
                                    prev_share.next = next;
                                } else {
                                    send_list.front = next;
                                }
                                if next.is_none() {
                                    send_list.back = prev;
                                }

                                curr_share.next = SendLink::none();
                                new_blocked.push_back(curr);
                            } else {
                                prev = curr;
                            }

                            curr = next;
                        }
                    }
                }
            }

            _ => {}
        }
    } else {
        match domain {
            DOMAIN_CALL => {
                let mut service_states = SERVICE_STATES.borrow_mut();
                let mut link = service_states[code as usize].replies.remove(index);
                let share = link.as_mut().unwrap();

                future_consumer = true;
                share.reply.set_offset(recv_buf.head.off);
                if let Some(w) = share.waker.take() {
                    w.wake();
                }
            }

            DOMAIN_INFO => {
                let mut service_states = SERVICE_STATES.borrow_mut();
                let service = &mut service_states[code as usize];

                match take(&mut service.info_recv) {
                    Recv::None => {
                        service.info_recv = Recv::Some(recv_buf.head.off);
                    }
                    Recv::Wake(w) => {
                        service.info_recv = Recv::Some(recv_buf.head.off);
                        w.wake();
                    }
                    Recv::Some(_) => {
                        // Don't overwrite unhandled offset.
                    }
                }
                future_consumer = true;
            }

            DOMAIN_FLOW => {
                for i in 0..packet::flow_count(p) {
                    let flow = packet::flow(p, i);
                    let mut streams = STREAMS.borrow_mut();

                    if {
                        let mut s = streams
                            .get_mut(&(code, flow.id))
                            .expect("flow packet received for unknown service or stream")
                            .borrow_mut();

                        if flow.increment > 0 {
                            s.writable += flow.increment as usize;
                            if let Some(w) = s.writer.take() {
                                w.wake();
                            }
                        } else if flow.increment == 0 {
                            peer_closed_stream(&mut s, STREAM_PEER_FLOW);
                        } else {
                            s.write_err = flow.increment;
                        }

                        s.flags == 0
                    } {
                        streams.remove(&(code, flow.id));
                    }
                }
            }

            DOMAIN_DATA => {
                let id = packet::data_id(p);
                let data_size = size - DATA_HEADER_SIZE;
                let mut streams = STREAMS.borrow_mut();

                if {
                    let mut s = streams
                        .get_mut(&(code, id))
                        .expect("data packet received for unknown service or stream")
                        .borrow_mut();

                    if data_size > 0 {
                        match take(&mut s.recv) {
                            Recv::None => {
                                s.recv = Recv::Some(recv_buf.head.off);
                            }
                            Recv::Wake(w) => {
                                s.recv = Recv::Some(recv_buf.head.off);
                                w.wake();
                            }
                            Recv::Some(_) => {
                                // Don't overwrite unhandled offset.
                            }
                        }
                        future_consumer = true;
                    } else {
                        s.recv_err = packet::data_note(p);
                        peer_closed_stream(&mut s, STREAM_PEER_DATA);
                    }

                    s.flags == 0
                } {
                    streams.remove(&(code, id));
                }
            }

            _ => {}
        }
    }

    if !future_consumer {
        recv_buf.consumed();
    }
}

fn die(s: &'static str) -> ! {
    eprintln!("gain: {}", s);
    exit(1)
}
