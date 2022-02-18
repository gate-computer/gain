// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Communicate with other program instances.

use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::str;

use futures_channel::oneshot::{channel, Sender};

use crate::service::Service;
use crate::stream::RecvWriteStream;
use crate::task::spawn_local;
use crate::threadunsafe::ThreadUnsafeRefCell;

lazy_static! {
    static ref SERVICE: Service = Service::register("peer");
    static ref GROUPS: ThreadUnsafeRefCell<HashMap<Vec<u8>, Box<dyn Fn(&str, &str)>>> =
        Default::default();
    static ref CONNS: ThreadUnsafeRefCell<HashMap<(Vec<u8>, Vec<u8>), Sender<(RecvWriteStream, String)>>> =
        Default::default();
}

/// Register a peer group implementation.
pub async fn register_group(group_name: &str, listener: Box<dyn Fn(&str, &str)>) {
    let mut groups = GROUPS.borrow_mut();
    let init = groups.is_empty();

    if groups.insert(group_name.into(), listener).is_some() {
        panic!("peer group {} already registered", group_name);
    }

    if init {
        spawn_local(handle_info_packets());
        SERVICE.send_info(&[]).await;
    }
}

fn parse_name<'a>(b: &'a [u8]) -> (&'a [u8], &'a [u8]) {
    let size = b[0] as usize;
    let b = &b[1..];
    let name = &b[..size];
    let b = &b[size..];
    (name, b)
}

async fn handle_info_packets() {
    SERVICE
        .recv_info(|b: &[u8]| {
            let id = i32::from_le_bytes(b[..4].try_into().unwrap());
            let b = &b[8..];
            let (group_name, b) = parse_name(b);
            let (peer_name, b) = parse_name(b);
            let (type_name, _) = parse_name(b);

            if id < 0 {
                let groups = GROUPS.borrow();
                groups[group_name](
                    str::from_utf8(peer_name).unwrap(),
                    str::from_utf8(type_name).unwrap(),
                );
            } else {
                let stream = SERVICE.stream(id);
                let mut conns = CONNS.borrow_mut();
                let _ = conns
                    .remove(&(group_name.into(), peer_name.into()))
                    .unwrap()
                    .send((stream, String::from_utf8(type_name.into()).unwrap()));
            }
        })
        .await;
}

/// Connect to a peer within a group.  Specify the incoming content type.  The
/// outgoing content type is returned along with the stream.
pub async fn connect(
    group_name: &str,
    peer_name: &str,
    type_name: &str,
) -> Result<(RecvWriteStream, String), ConnectError> {
    let group_name = group_name.as_bytes();
    if group_name.len() > 255 {
        panic!("group name is too long");
    }

    let peer_name = peer_name.as_bytes();
    if peer_name.len() > 255 {
        panic!("peer name is too long");
    }

    let type_name = type_name.as_bytes();
    if type_name.len() > 255 {
        panic!("type name is too long");
    }

    let (sender, receiver) = channel();

    {
        let mut conns = CONNS.borrow_mut();
        let key = (group_name.into(), peer_name.into());
        if conns.get(&key).is_some() {
            return Err(ConnectError::already_connecting());
        }
        conns.insert(key, sender);
    }

    let mut buf =
        Vec::with_capacity(8 + 1 + group_name.len() + 1 + peer_name.len() + 1 + type_name.len());
    buf.resize(8, 0); // Reserved.
    buf.push(group_name.len() as u8);
    buf.extend_from_slice(group_name);
    buf.push(peer_name.len() as u8);
    buf.extend_from_slice(peer_name);
    buf.push(type_name.len() as u8);
    buf.extend_from_slice(type_name);

    SERVICE
        .call(buf.as_slice(), |buf: &[u8]| -> Result<(), ConnectError> {
            if buf.len() < 2 {
                return Err(ConnectError::new(0));
            }

            let error = i16::from_le_bytes(buf[..2].try_into().unwrap());
            match error {
                0 => Ok(()),
                1 => panic!("ABI violation"),
                _ => Err(ConnectError::new(error)),
            }
        })
        .await?;

    Ok(receiver.await.unwrap())
}

#[derive(Debug, Eq, PartialEq)]
pub enum ConnectErrorKind {
    Other,
    GroupNotFound,
    PeerNotFound,
    Singularity,
    AlreadyConnecting,
    AlreadyConnected,
}

#[derive(Debug)]
pub struct ConnectError {
    code: i16,
}

impl ConnectError {
    fn new(code: i16) -> Self {
        Self { code }
    }

    fn already_connecting() -> Self {
        Self::new(5)
    }

    pub fn kind(&self) -> ConnectErrorKind {
        match self.code {
            2 => ConnectErrorKind::GroupNotFound,
            3 => ConnectErrorKind::PeerNotFound,
            4 => ConnectErrorKind::Singularity,
            5 => ConnectErrorKind::AlreadyConnecting,
            6 => ConnectErrorKind::AlreadyConnected,
            _ => ConnectErrorKind::Other,
        }
    }

    pub fn as_i16(&self) -> i16 {
        self.code
    }
}

impl fmt::Display for ConnectError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.kind() {
            ConnectErrorKind::GroupNotFound => f.write_str("group not found"),
            ConnectErrorKind::PeerNotFound => f.write_str("peer not found"),
            ConnectErrorKind::Singularity => f.write_str("singularity"),
            ConnectErrorKind::AlreadyConnecting => f.write_str("already connecting"),
            ConnectErrorKind::AlreadyConnected => f.write_str("already connected"),
            _ => self.code.fmt(f),
        }
    }
}
