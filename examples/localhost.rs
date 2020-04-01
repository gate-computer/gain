// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use gain::origin;
use gain::stream::Write;
use gain::task::{block_on, spawn_local};
use gain_localhost as localhost;

fn main() {
    block_on(async {
        let mut tasks = Vec::new();

        tasks.push(spawn_local(handle("/robots.txt")));
        tasks.push(spawn_local(handle("/nonexistent")));
        tasks.push(spawn_local(handle_output("/")));

        for t in tasks {
            t.await;
        }
    });
}

async fn handle(uri: &str) {
    handle_(uri).await;
}

async fn handle_(uri: &str) -> Vec<u8> {
    let res = localhost::get(uri).await;
    println!("{} status code: {}", uri, res.status_code);
    if let Some(s) = &res.content_type {
        println!("{} content type: {}", uri, s);
    }
    res.content
}

async fn handle_output(uri: &str) {
    let content = handle_(uri).await;
    if !content.is_empty() {
        let mut conn = origin::accept().await.unwrap();
        conn.write_all(content.as_slice()).await.unwrap();
    }
}
