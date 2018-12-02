// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#![feature(generators, proc_macro_hygiene)]

extern crate bincode;
extern crate futures_await as futures;
#[macro_use]
extern crate serde_derive;

mod gate;
pub mod origin;
pub mod packet;

pub use gate::exit;

use bincode::serialize_into;
use futures::prelude::{await, *};
use futures::task::Task;
use futures::{executor, future, task};
use std::collections::{HashMap, HashSet, VecDeque};
use std::io::{Error, ErrorKind};
use std::ops::Deref;
use std::sync::mpsc::{channel, sync_channel, Receiver, Sender, SyncSender, TryRecvError};
use std::sync::Arc;

pub const DEBUG_SEPARATOR: &'static str = "\x1e";

const IO_RECV_WAIT: u32 = 0x1;

#[cfg(debug_assertions)]
pub fn debug(s: &str) {
    gate::debug(s);
}

#[cfg(not(debug_assertions))]
pub fn debug(_: &str) {}

pub fn debugln(s: &str) {
    debug(DEBUG_SEPARATOR);
    debug(s);
    debug("\n");
}

pub fn debug_always(s: &str) {
    gate::debug(s);
}

pub fn debugln_always(s: &str) {
    gate::debug(DEBUG_SEPARATOR);
    gate::debug(s);
    gate::debug("\n");
}

pub fn panic(s: &'static str) -> ! {
    debugln_always(s);
    panic!(s)
}

#[async]
pub fn discover_service(name: String) -> Result<ServiceState, Error> {
    if let Some(service) = state().services.get(&name) {
        if service.done {
            return Ok(service.state.clone());
        }

        return await!(future::poll_fn(move || {
            let service = state().services.get_mut(&name).unwrap();
            if service.done {
                Ok(Async::Ready(service.state.clone()))
            } else {
                service.tasks.push(task::current());
                Ok(Async::NotReady)
            }
        }));
    }

    let index = state().services.len();
    if index > std::i16::MAX as usize {
        return Err(Error::new(ErrorKind::Other, "gain: too many services"));
    }

    state().services.insert(
        name.clone(),
        Box::new(Service {
            state: ServiceState {
                code: index as i16,
                flags: 0,
            },
            done: false,
            tasks: Vec::new(),
        }),
    );

    let req_size = packet::SERVICES_HEADER_SIZE + name.len() + 1;
    let mut req: Vec<u8> = Vec::with_capacity(req_size);

    serialize_into(
        &mut req,
        &packet::new_services_header(
            req_size, // packet size
            1,        // service count
        ),
    )
    .unwrap();

    req.extend_from_slice(name.as_bytes());
    req.push(0);

    let resp = await!(call_service(req))?;

    let offset = packet::SERVICES_HEADER_SIZE + index;
    if resp.len() <= offset {
        return Err(Error::new(ErrorKind::Other, "gain: too many services"));
    }

    let service = state().services.get_mut(&name).unwrap();
    service.state.flags = resp[offset];
    service.done = true;
    for task in &service.tasks {
        task.notify();
    }
    service.tasks.clear();

    Ok(service.state.clone())
}

#[async]
pub fn call_service(data: Vec<u8>) -> Result<Vec<u8>, Error> {
    if data.len() < packet::HEADER_SIZE || data.len() > packet::MAX_SIZE {
        panic("gain::call_service: packet data length out of bounds")
    }

    let header: packet::Header = packet::deserialize_header(&data);
    if header.size as usize != data.len() {
        panic("gain::call_service: packet size header field != packet data length")
    }

    if header.domain != packet::DOMAIN_CALL {
        panic("gain::call_service: wrong domain in packet header")
    }

    let (done_sender, done_receiver) = sync_channel(1);
    let (task_sender, task_receiver) = channel();

    state().send_queue.push_back(OutgoingPacket {
        data: data,
        offset: 0,
        done_sender: None,
        task_receiver: None,
    });

    state().receivers.insert(
        PacketAddress {
            code: header.code,
            domain: packet::DOMAIN_CALL,
        },
        PacketReceiver {
            done_sender: done_sender,
            task_receiver: task_receiver,
        },
    );

    await!(new_channel_future(done_receiver, task_sender))
}

#[async]
pub fn send_stream(data: Vec<u8>) -> Result<(), Error> {
    if data.len() < packet::HEADER_SIZE || data.len() > packet::MAX_SIZE {
        panic("gain::send_stream: packet data length out of bounds")
    }

    let header: packet::Header = packet::deserialize_header(&data);
    if header.size as usize != data.len() {
        panic("gain::send_stream: packet size header field != packet data length")
    }

    if header.domain != packet::DOMAIN_FLOW && header.domain != packet::DOMAIN_DATA {
        panic("gain::send_stream: wrong domain in packet header")
    }

    let (done_sender, done_receiver) = sync_channel(1);
    let (task_sender, task_receiver) = channel();

    state().send_queue.push_back(OutgoingPacket {
        data: data,
        offset: 0,
        done_sender: Some(done_sender),
        task_receiver: Some(task_receiver),
    });

    await!(new_channel_future(done_receiver, task_sender))
}

fn new_channel_future<T>(
    done_receiver: Receiver<T>,
    task_sender: Sender<task::Task>,
) -> impl Future<Item = T, Error = Error> {
    future::poll_fn(move || match done_receiver.try_recv() {
        Ok(x) => Ok(Async::Ready(x)),
        Err(TryRecvError::Empty) => {
            task_sender.send(task::current()).unwrap();
            Ok(Async::NotReady)
        }
        Err(TryRecvError::Disconnected) => {
            Err(Error::new(ErrorKind::Other, "gain: future poll error"))
        }
    })
}

pub fn spawn<T>(future: T)
where
    T: Future<Item = (), Error = ()> + Send + 'static,
{
    state().spawn(future)
}

pub fn run<F>(future: F)
where
    F: Future<Item = (), Error = ()> + Send + 'static,
{
    state().run(future)
}

pub fn run_iteration() -> bool {
    state().run_iteration()
}

fn state() -> &'static mut State {
    static mut STATE: Option<State> = None;

    unsafe {
        match STATE {
            Some(ref mut state) => state,
            None => {
                STATE = Some(State::new());
                match STATE {
                    Some(ref mut state) => state,
                    None => panic!(),
                }
            }
        }
    }
}

struct NotifyProxy {}

impl executor::Notify for NotifyProxy {
    fn notify(&self, id: usize) {
        state().notified_tasks.insert(id);
    }
}

struct OutgoingPacket {
    data: Vec<u8>,
    offset: usize,
    done_sender: Option<SyncSender<()>>,
    task_receiver: Option<Receiver<Task>>,
}

#[derive(Eq, PartialEq, Hash)]
struct PacketAddress {
    code: i16,
    domain: packet::Domain,
}

struct PacketReceiver {
    done_sender: SyncSender<Vec<u8>>,
    task_receiver: Receiver<Task>,
}

#[derive(Clone)]
pub struct ServiceState {
    pub code: i16,
    flags: u8,
}

impl ServiceState {
    pub fn avail(&self) -> bool {
        (self.flags & packet::SERVICE_STATE_AVAIL) != 0
    }
}

struct Service {
    state: ServiceState,
    done: bool,
    tasks: Vec<Task>,
}

struct State {
    notify_handle: executor::NotifyHandle,
    send_queue: VecDeque<OutgoingPacket>,
    receivers: HashMap<PacketAddress, PacketReceiver>,
    tasks: HashMap<usize, Box<executor::Spawn<Future<Item = (), Error = ()> + Send + 'static>>>,
    notified_tasks: HashSet<usize>,

    services: HashMap<String, Box<Service>>,
}

impl State {
    fn new() -> State {
        State {
            notify_handle: executor::NotifyHandle::from(Arc::new(NotifyProxy {})),
            send_queue: VecDeque::new(),
            receivers: HashMap::new(),
            tasks: HashMap::new(),
            notified_tasks: HashSet::new(),

            services: HashMap::new(),
        }
    }

    fn spawn<T>(&mut self, future: T)
    where
        T: Future<Item = (), Error = ()> + Send + 'static,
    {
        let mut task = Box::new(executor::spawn(future));
        let id = task.deref() as *const _ as usize;

        match task.poll_future_notify(&self.notify_handle, id) {
            Ok(Async::Ready(_)) => {}
            Ok(Async::NotReady) => {
                self.tasks.insert(id, task);
            }
            Err(_) => {
                debugln("gain::spawn: future poll error");
            }
        }
    }

    fn run<F>(&mut self, future: F)
    where
        F: Future<Item = (), Error = ()> + Send + 'static,
    {
        spawn(future);

        while self.run_iteration() {}
    }

    fn run_iteration(&mut self) -> bool {
        let mut recv_buf: Vec<u8> = Vec::new();
        let mut recv_size = packet::HEADER_SIZE;

        loop {
            let mut flags: u32 = 0;

            if self.send_queue.is_empty() && self.notified_tasks.is_empty() {
                flags |= IO_RECV_WAIT;
            }

            if !recv_buf.is_empty() {
                flags |= IO_RECV_WAIT;
            }

            let sent = if let Some(send) = self.send_queue.front_mut() {
                send.offset += gate::io(&mut recv_buf, recv_size, &send.data[send.offset..], flags);
                send.offset == send.data.len()
            } else {
                gate::io(&mut recv_buf, recv_size, &[], flags);
                false
            };

            if sent {
                let send = self.send_queue.pop_front().unwrap();

                if let Some(done) = send.done_sender {
                    done.send(()).unwrap();
                }

                if let Some(tasks) = send.task_receiver {
                    for task in tasks.try_iter() {
                        task.notify();
                    }
                }
            }

            if recv_size == packet::HEADER_SIZE && recv_buf.len() == packet::HEADER_SIZE {
                recv_size = packet::deserialize_size(&recv_buf);
            }

            if (recv_buf.len() == 0 || recv_buf.len() == recv_size) && !sent {
                break;
            }
        }

        if recv_buf.len() > 0 {
            let header = packet::deserialize_header(&recv_buf);

            match header.domain {
                packet::DOMAIN_FLOW => {} // TODO
                packet::DOMAIN_DATA => {} // TODO
                _ => {
                    if let Some(receiver) = self.receivers.remove(&PacketAddress {
                        code: header.code,
                        domain: header.domain,
                    }) {
                        receiver.done_sender.send(recv_buf).unwrap();
                        for task in receiver.task_receiver.try_iter() {
                            task.notify();
                        }
                    } else {
                        panic("gain::run: no future for received packet code and domain");
                    }
                }
            }
        }

        loop {
            let id = if let Some(key) = self.notified_tasks.iter().next() {
                let id = *key;
                let mut task = self.tasks.remove(&id).unwrap();

                match task.poll_future_notify(&self.notify_handle, id) {
                    Ok(Async::Ready(_)) => {}
                    Ok(Async::NotReady) => {
                        self.tasks.insert(id, task);
                    }
                    Err(_) => {
                        debugln("gain::run: future poll error");
                    }
                }

                id
            } else {
                break;
            };

            self.notified_tasks.remove(&id);
        }

        !self.tasks.is_empty()
    }
}
