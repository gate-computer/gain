// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Service binding implementation support.

use crate::core;
use crate::packet::Code;
use crate::stream::{RecvStream, RecvWriteStream, WriteStream};

pub mod future {
    pub use crate::core::CallFuture as Call;
    pub use crate::core::InfoRecvFuture as InfoRecv;
    pub use crate::core::InfoSendFuture as InfoSend;
}

/// Reason for service registration failure.
#[derive(Debug)]
pub enum RegistrationError {
    NameAlreadyRegistered,
    TooManyServices,
}

/// Handle to a registered service.
///
/// If a stream is opened as a result of service registration or a call, the
/// appropriate stream constructor must be called immediately.  Consequently, a
/// buffered stream cannot be used to receive data which includes stream ids.
pub struct Service {
    code: Code,
}

impl Service {
    /// Register a service or panic.
    pub fn register(name: &'static str) -> Self {
        match Self::try_register(name) {
            Ok(this) => this,
            Err(e) => panic!("{}: {:?}", name, e),
        }
    }

    /// Register a service.
    pub fn try_register(name: &'static str) -> Result<Self, RegistrationError> {
        Ok(Self {
            code: core::register_service(name)?,
        })
    }

    /// Call the service.  Returns a future.
    ///
    /// The receptor is invoked with the reply content, and its return value is
    /// passed through.
    pub fn call<'a, R, T>(&self, content: &'a [u8], receptor: R) -> future::Call<'a, R, T>
    where
        R: FnOnce(&[u8]) -> T + Unpin,
    {
        future::Call::new(self.code, content, receptor)
    }

    /// Receive info packets from the service repeatedly.  Returns a future.
    pub fn recv_info<R>(&self, receptor: R) -> future::InfoRecv<R>
    where
        R: Fn(&[u8]) + Unpin,
    {
        future::InfoRecv::new(self.code, receptor)
    }

    /// Send an info packet to the service.  Returns a future.
    pub fn send_info<'a>(&self, content: &'a [u8]) -> future::InfoSend<'a> {
        future::InfoSend::new(self.code, content)
    }

    /// Construct a handle to a new bidirectional stream.
    pub fn stream(&self, id: i32) -> RecvWriteStream {
        RecvWriteStream::new(core::init_stream(
            self.code,
            id,
            core::STREAM_SELF_FLOW
                | core::STREAM_SELF_DATA
                | core::STREAM_PEER_FLOW
                | core::STREAM_PEER_DATA,
        ))
    }

    /// Construct a handle to a new unidirectional stream.
    pub fn input_stream(&self, id: i32) -> RecvStream {
        RecvStream::new(core::init_stream(
            self.code,
            id,
            core::STREAM_SELF_FLOW | core::STREAM_PEER_DATA,
        ))
    }

    /// Construct a handle to a new unidirectional stream.
    pub fn output_stream(&self, id: i32) -> WriteStream {
        WriteStream::new(core::init_stream(
            self.code,
            id,
            core::STREAM_SELF_DATA | core::STREAM_PEER_FLOW,
        ))
    }
}
