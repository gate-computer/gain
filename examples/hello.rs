// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

#![cfg_attr(target_os = "unknown", no_main)]

use gain::stream::{Close, Write};
use gain::task::{block_on, spawn};
use gain::{catalog, identity, origin};

async fn do_stuff() {
    println!("Accepting origin connection");
    let mut conn = origin::accept().await.unwrap();
    println!("Origin connection accepted");
    let who = match identity::principal_id().await {
        Some(s) => s,
        None => "world".to_string(),
    };
    conn.write_all(format!("hello, {}\n", who).as_bytes())
        .await
        .unwrap();
    let (_, w) = conn.split();
    let (mut w, mut c) = w.split();
    w.write_all(&catalog::json().await.as_bytes())
        .await
        .unwrap();
    c.close().await;
    println!("Origin connection has been closed");
    if let Ok(n) = w.write("test".as_bytes()).await {
        if n != 0 {
            panic!("write after close");
        }
    }
    drop(w);
    println!("Origin connection dropped");
}

fn hello() -> i32 {
    block_on(async {
        let handle = spawn(async {
            println!("In another task");
            0 // Code for success.
        });
        do_stuff().await;
        handle.await.unwrap()
    })
}

// Invoked by the implicit entry function of a one-shot wasm32-wasi program.
#[cfg(target_os = "wasi")]
fn main() {
    std::process::exit(hello());
}

// An explicit entry function for a reentrant wasm32-unknown-unknown program.
#[cfg(target_os = "unknown")]
#[no_mangle]
pub fn greet() -> i32 {
    hello()
}
