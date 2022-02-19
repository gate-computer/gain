// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Access the host system.

#[macro_use]
extern crate lazy_static;

use gain::service::Service;
use gain::stream::RecvStream;

lazy_static! {
    static ref SERVICE: Service = Service::register("gate.computer/shell");
}

pub async fn spawn() -> Option<RecvStream> {
    SERVICE
        .call("echo hello, world".as_bytes(), |reply: &[u8]| {
            let error = i16::from_le_bytes(reply[..2].try_into().unwrap());
            let id = i32::from_le_bytes(reply[4..8].try_into().unwrap());

            if id >= 0 {
                let stream = SERVICE.input_stream(id);
                if error == 0 {
                    return Some(stream);
                }
            }

            None // TODO: error
        })
        .await
}
