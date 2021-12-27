// Copyright (c) 2018 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use std::ptr::{null, null_mut};
use std::time::Duration;

pub const MAX_RECV_SIZE: usize = 65536;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Ciovec {
    pub buf: *const u8,
    pub buf_len: usize,
}

impl Ciovec {
    pub fn new(buf: &[u8]) -> Self {
        if buf.is_empty() {
            Self::default()
        } else {
            Self {
                buf: &buf[0] as *const _,
                buf_len: buf.len(),
            }
        }
    }
}

impl Default for Ciovec {
    fn default() -> Self {
        Self {
            buf: null(),
            buf_len: 0,
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Iovec {
    pub buf: *mut u8,
    pub buf_len: usize,
}

impl Default for Iovec {
    fn default() -> Self {
        Self {
            buf: null_mut(),
            buf_len: 0,
        }
    }
}

pub unsafe fn io(recv: &[Iovec], send: &[Ciovec], timeout: Option<Duration>) -> (usize, usize) {
    let mut received: usize = 0;
    let mut sent: usize = 0;

    io_65536(
        recv.as_ptr(),
        recv.len(),
        &mut received,
        send.as_ptr(),
        send.len(),
        &mut sent,
        match timeout {
            Some(t) => {
                let n = t.as_nanos();
                if n <= 0x7fffffffffffffff {
                    n as i64
                } else {
                    0x7fffffffffffffff
                }
            }
            None => -1,
        },
    );

    (received, sent)
}

#[link(wasm_import_module = "gate")]
extern "C" {
    fn io_65536(
        recv_vec: *const Iovec,
        recv_vec_len: usize,
        received_bytes: *mut usize,
        send_vec: *const Ciovec,
        send_vec_len: usize,
        sent_bytes: *mut usize,
        timeout: i64,
    );
}
