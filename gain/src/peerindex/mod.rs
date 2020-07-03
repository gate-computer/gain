// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Find other program instances.

use std::fmt;

use crate::error::ErrorCode;

pub mod principal;

#[derive(Debug, Eq, PartialEq)]
pub enum ErrorKind {
    Other,
    NotRegistered,
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
            1 => ErrorKind::NotRegistered,
            _ => ErrorKind::Other,
        }
    }
}

impl ErrorCode for Error {
    fn as_i16(&self) -> i16 {
        self.code
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        match self.kind() {
            ErrorKind::NotRegistered => f.write_str("not registered"),
            _ => self.code.fmt(f),
        }
    }
}
