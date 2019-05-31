// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Communicate with the invoker of the program instance.
//!
//! This can be thought of as standard I/O streams.

use std::convert::TryInto;
use std::fmt;

use crate::service::Service;
use crate::stream::RecvWriteStream;

lazy_static! {
    static ref SERVICE: Service = Service::register("origin");
}

const ACCEPT_REPLY_SIZE: usize = 8;

/// Reason for connection acceptance failure.
///
/// No reasons have been defined yet.
#[derive(Debug)]
pub struct AcceptError(i16);

impl AcceptError {
    /// Get the raw error code.
    pub fn as_i16(&self) -> i16 {
        self.0
    }
}

impl fmt::Display for AcceptError {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        self.0.fmt(f)
    }
}

/// Accept a new incoming connection.
///
/// The call is blocked while no connection is available, or the
/// environment-dependent maximum number of simultaneous connections is
/// reached.
///
/// Typically there is a correspondence between a connection and a program
/// invocation or resumption.
pub async fn accept() -> Result<RecvWriteStream, AcceptError> {
    SERVICE
        .call(&[], |reply: &[u8]| {
            if reply.len() < ACCEPT_REPLY_SIZE {
                return Err(AcceptError(0));
            }

            let error = i16::from_le_bytes(reply[4..6].try_into().unwrap());
            if error != 0 {
                return Err(AcceptError(error));
            }

            Ok(SERVICE.stream(i32::from_le_bytes(reply[..4].try_into().unwrap())))
        })
        .await
}
