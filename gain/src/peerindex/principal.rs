// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Find your program instances.

use std::convert::TryInto;
use std::str;

use crate::peer;
use crate::peerindex::Error;
use crate::service::Service;

/// Peer group name.
pub static GROUP_NAME: &str = "index/principal";

lazy_static! {
    static ref SERVICE: Service = Service::register("peerindex/principal");
}

/// Register this program instance in the index.  The listener is invoked when
/// a peer tries to connect and there isn't an ongoing connection process with
/// that peer.
pub async fn register(listener: Box<dyn Fn(&str, &str)>) {
    peer::register_group(GROUP_NAME, listener).await;
    SERVICE.send_info(&[]).await;
}

/// List peers.
pub async fn instances() -> Result<Vec<String>, Error> {
    SERVICE
        .call(&[], |reply: &[u8]| {
            if reply.len() < 4 {
                return Err(Error::new(0));
            }

            let error = i16::from_le_bytes(reply[..2].try_into().unwrap());
            if error != 0 {
                return Err(Error::new(error));
            }

            let count = u16::from_le_bytes(reply[2..4].try_into().unwrap());
            let mut list = Vec::with_capacity(count as usize);

            let mut reply = &reply[4..];

            for _ in 0..count {
                let size = reply[0] as usize;
                reply = &reply[1..];
                list.push(String::from_utf8(reply[..size].into()).unwrap());
                reply = &reply[size..];
            }

            Ok(list)
        })
        .await
}
