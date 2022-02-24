// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use std::cell::Cell;
use std::io::{stdout, Write};
use std::process::exit;

use gain::scope::{restrict, SCOPE_SYSTEM};
use gain::stream::Recv;
use gain::task::block_on;
use gain_shell::spawn;

fn main() {
    exit(block_on(async {
        restrict(&[SCOPE_SYSTEM]).await;

        let mut output = spawn("echo -n hello,  && echo \\ world").await.unwrap();

        restrict(&[]).await;

        let result: Cell<Option<i32>> = Cell::new(None);

        output
            .recv(8192, |b: &[u8], note: i32| {
                result.set(Some(note));
                if stdout().write(b).unwrap() < b.len() {
                    exit(1);
                }
                b.len()
            })
            .await;

        result.get().unwrap()
    }));
}
