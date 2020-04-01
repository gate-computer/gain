// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use gain::origin;
use gain::stream::{RecvWriteStream, Write};
use gain::task::block_on;
use gain_lep::repl;
use lep::{Domain, State};

fn main() {
    block_on(async {
        match origin::accept().await {
            Ok(conn) => handle(conn).await,
            Err(e) => panic!("accept error: {}", e),
        }
    });
}

async fn handle(mut conn: RecvWriteStream) {
    match conn.write_all("welcome\n".as_bytes()).await {
        Ok(()) => {}
        Err(e) => {
            println!("write error: {}", e);
            return;
        }
    }

    let mut domain = Domain::new();
    gain_lep::register(&mut domain);

    repl(conn, domain, State::new()).await;
}
