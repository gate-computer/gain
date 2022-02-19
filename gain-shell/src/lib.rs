// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Access the host system.

#[macro_use]
extern crate lazy_static;

use std::fmt;

use gain::service::Service;
use gain::stream::RecvStream;

lazy_static! {
    static ref SERVICE: Service = Service::register("gate.computer/shell");
}

pub async fn spawn(command: &str) -> Result<RecvStream, Error> {
    SERVICE
        .call(command.as_bytes(), |reply: &[u8]| {
            let error = i16::from_le_bytes(reply[..2].try_into().unwrap());
            let id = i32::from_le_bytes(reply[4..8].try_into().unwrap());

            if id >= 0 {
                let stream = SERVICE.input_stream(id);
                if error == 0 {
                    return Ok(stream);
                }
            }

            Err(Error::new(error))
        })
        .await
}

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Other,
    Quota,
    User,
    WorkDir,
    Executable,
}

#[derive(Debug)]
pub struct Error {
    code: i16,
}

impl Error {
    fn new(code: i16) -> Self {
        Self { code }
    }

    pub fn kind(&self) -> ErrorKind {
        match self.code {
            1 => ErrorKind::Quota,
            2 => ErrorKind::User,
            3 => ErrorKind::WorkDir,
            4 => ErrorKind::Executable,
            _ => ErrorKind::Other,
        }
    }

    pub fn as_i16(&self) -> i16 {
        self.code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.kind() {
            ErrorKind::Quota => f.write_str("not enough quota"),
            ErrorKind::User => f.write_str("user not found"),
            ErrorKind::WorkDir => f.write_str("work directory error"),
            ErrorKind::Executable => f.write_str("executable error"),
            _ => self.code.fmt(f),
        }
    }
}
