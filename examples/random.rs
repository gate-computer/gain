// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use gain::random::random;
use gain::task::{block_on, spawn_local};

fn main() {
    block_on(async {
        let h1 = spawn_local(f1());
        let h2 = spawn_local(f2());
        h1.await.unwrap();
        h2.await.unwrap();
    })
}

async fn f1() {
    for _ in 0..40 {
        println!("{}", i32::from_le_bytes(random().await));
    }
}

async fn f2() {
    for _ in 0..10 {
        println!("{}", i128::from_le_bytes(random().await));
    }
}
