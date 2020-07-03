// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use std::str;

use futures::channel::mpsc;
use futures::StreamExt;

use gain::stream::buf::{Read, ReadWriteStream};
use gain::stream::{Close, Write};
use gain::task::block_on;
use gain::{peer, peerindex};

fn main() {
    block_on(async {
        let (sender, mut receiver) = mpsc::unbounded::<String>();

        peerindex::principal::register(Box::new(move |name: &str| {
            println!("peer {} is connecting", name);
            sender.unbounded_send(name.into()).unwrap();
        }))
        .await;

        let mut peer_name: Option<String> = None;

        for name in peerindex::principal::qualified_instance_names()
            .await
            .unwrap()
        {
            println!("indexed peer: {}", name);
            peer_name = Some(name);
        }

        if peer_name.is_none() {
            println!("waiting for peers");
            peer_name = Some(receiver.next().await.unwrap());
        }

        let name = peer_name.unwrap();
        println!("connecting to {}", name);
        let conn = peer::connect(&name).await.unwrap();
        println!("connected");

        let mut conn = ReadWriteStream::new(conn);

        println!("sending");
        conn.write("hello, peer".as_bytes()).await.unwrap();
        println!("sent");

        let mut buf: [u8; 256] = [0; 256];
        println!("receiving");
        let n = conn.read(&mut buf[..]).await.unwrap();
        println!("received: {}", str::from_utf8(&buf[..n]).unwrap());

        conn.close().await.unwrap();
        println!("closed");
    });
}
