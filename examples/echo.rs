// Copyright (c) 2021 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use gain::origin;
use gain::stream::buf::{Read, ReadWriteStream, DEFAULT_READ_CAPACITY};
use gain::stream::Write;
use gain::task::block_on;
use std::process::exit;

fn main() {
    exit(block_on(async {
        let mut c = ReadWriteStream::new(origin::accept().await.unwrap());
        let mut b = [0; DEFAULT_READ_CAPACITY];

        loop {
            let n = match c.read(&mut b[..]).await {
                Ok(n) => n,
                Err(_) => return 0,
            };

            match c.write_all(&b[..n]).await {
                Ok(_) => {}
                Err(_) => return 1,
            }
        }
    }))
}
