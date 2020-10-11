// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use std::cell::RefCell;

use gain::task::block_on;
use gain::{origin, peerindex};
use gain_lep::repl_default;
use lep::{Domain, State};

fn main() {
    block_on(async {
        let friends = RefCell::new(Vec::new());
        let befriender = friends.clone();

        peerindex::principal::register(Box::new(move |name: &str| {
            befriender.borrow_mut().push(name.to_string());
        }))
        .await;

        let default = move || -> Vec<u8> {
            let mut msg = "friendly principal peers:\n".to_string();
            for name in friends.borrow().iter() {
                msg.push_str("    ");
                msg.push_str(name);
                msg.push('\n');
            }
            msg.into()
        };

        match origin::accept().await {
            Ok(conn) => {
                async {
                    let mut domain = Domain::new();
                    gain_lep::register(&mut domain);
                    gain_localhost::lep::register(&mut domain);
                    repl_default(conn, domain, State::new(), default).await;
                }
                .await
            }
            Err(e) => panic!("accept error: {}", e),
        }
    });
}
