// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Communicate with other program instances.

use std::collections::HashMap;
use std::convert::TryInto;
use std::fmt;
use std::str;

use futures_channel::oneshot::{channel, Sender};

use crate::error::ErrorCode;
use crate::service::Service;
use crate::stream::RecvWriteStream;
use crate::task::spawn_local;
use crate::threadunsafe::ThreadUnsafeRefCell;

lazy_static! {
    static ref SERVICE: Service = Service::register("peer");
    static ref GROUPS: ThreadUnsafeRefCell<HashMap<String, Box<dyn Fn(&str)>>> = Default::default();
    static ref CONNS: ThreadUnsafeRefCell<HashMap<String, Sender<RecvWriteStream>>> =
        Default::default();
}

/// Register a peer group implementation.
pub async fn register_group(group_name: &str, listener: Box<dyn Fn(&str)>) {
    let mut groups = GROUPS.borrow_mut();
    let init = groups.is_empty();

    if let Some(_) = groups.insert(group_name.into(), listener) {
        panic!("peer group {} already registered", group_name);
    }

    if init {
        spawn_local(handle_info_packets());
        SERVICE.send_info(&[]).await;
    }
}

async fn handle_info_packets() {
    SERVICE
        .recv_info(|content: &[u8]| {
            let id = i32::from_le_bytes(content[..4].try_into().unwrap());
            let name = str::from_utf8(&content[4..]).unwrap();
            if id < 0 {
                let i = name.find(':').unwrap();
                let group_name = &name[..i];
                let groups = GROUPS.borrow();
                groups[group_name](name);
            } else {
                let stream = SERVICE.stream(id);
                let mut conns = CONNS.borrow_mut();
                drop(conns.remove(name).unwrap().send(stream));
            }
        })
        .await
        .unwrap();
}

/// Connect to a peer.
pub async fn connect(qualified_name: &str) -> Result<RecvWriteStream, ConnectError> {
    let (sender, receiver) = channel();

    {
        let mut conns = CONNS.borrow_mut();
        if conns.get(qualified_name).is_some() {
            return Err(ConnectError::already_connecting());
        }
        conns.insert(qualified_name.into(), sender);
    }

    match SERVICE
        .call(
            qualified_name.as_bytes(),
            |reply: &[u8]| -> Result<(), ConnectError> {
                if reply.len() < 2 {
                    return Err(ConnectError::new(0));
                }

                let error = i16::from_le_bytes(reply[..2].try_into().unwrap());
                if error != 0 {
                    return Err(ConnectError::new(error));
                }

                Ok(())
            },
        )
        .await
    {
        Ok(()) => Ok(receiver.await.unwrap()),
        Err(e) => Err(e),
    }
}

/// Connect to a peer.  Group name is specified separately.
pub async fn connect_group(
    group_name: &str,
    short_name: &str,
) -> Result<RecvWriteStream, ConnectError> {
    let mut name = group_name.to_string();
    name.push(':');
    name.push_str(short_name);
    connect(&name).await
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
        Self::new(4)
    }

    pub fn kind(&self) -> ConnectErrorKind {
        match self.code {
            1 => ConnectErrorKind::GroupNotFound,
            2 => ConnectErrorKind::PeerNotFound,
            3 => ConnectErrorKind::Singularity,
            4 => ConnectErrorKind::AlreadyConnecting,
            5 => ConnectErrorKind::AlreadyConnected,
            _ => ConnectErrorKind::Other,
        }
    }
}

impl ErrorCode for ConnectError {
    fn as_i16(&self) -> i16 {
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
