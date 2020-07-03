// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
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
pub async fn register(listener: Box<dyn Fn(&str)>) {
    peer::register_group(GROUP_NAME, listener).await;
    SERVICE.send_info(&[]).await;
}

/// List peers without group name prefixes.
pub async fn instance_names() -> Result<Vec<String>, Error> {
    get_instance_names(false).await
}

/// List peers with group name prefixes.
pub async fn qualified_instance_names() -> Result<Vec<String>, Error> {
    get_instance_names(true).await
}

async fn get_instance_names(qualify: bool) -> Result<Vec<String>, Error> {
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
                let namelen = reply[0] as usize;
                reply = &reply[1..];

                let mut name = String::new();
                if qualify {
                    name.push_str(GROUP_NAME);
                    name.push(':');
                }
                name.push_str(str::from_utf8(&reply[..namelen]).unwrap());
                reply = &reply[namelen..];

                list.push(name);
            }

            Ok(list)
        })
        .await
}
