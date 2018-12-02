// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

pub fn debug(s: &str) {
    unsafe { gate_debug(s.as_ptr(), s.len()) }
}

pub fn exit(status: i32) -> ! {
    unsafe { gate_exit(status) }
}

pub fn io(recv_target: &mut Vec<u8>, recv_total: usize, send_data: &[u8], flags: u32) -> usize {
    let recv_offset = recv_target.len();

    let mut recv_len: usize = if recv_target.len() >= recv_total {
        0
    } else {
        if recv_target.capacity() < recv_total {
            let n = recv_total - recv_target.capacity();
            recv_target.reserve_exact(n);
        }

        recv_total - recv_offset
    };

    let mut send_len: usize = send_data.len();

    unsafe {
        gate_io(
            recv_target[recv_offset..].as_mut_ptr(),
            &mut recv_len,
            send_data.as_ptr(),
            &mut send_len,
            flags,
        );

        recv_target.set_len(recv_offset + recv_len);
    }

    send_len
}

#[link(wasm_import_module = "gate")]
extern "C" {
    #[link_name = "debug"]
    fn gate_debug(data: *const u8, size: usize);

    #[link_name = "exit"]
    fn gate_exit(status: i32) -> !;

    #[link_name = "io.65536"]
    fn gate_io(
        recv_buf: *mut u8,
        recv_len: *mut usize,
        send_data: *const u8,
        send_len: *mut usize,
        flags: u32,
    );
}
