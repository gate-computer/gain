// Copyright (c) 2021 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use gain::stream::Write;
use gain::task::block_on;
use gain::{catalog, origin};

fn main() {
    block_on(async {
        origin::accept()
            .await
            .unwrap()
            .write_all(&catalog::json().await.as_bytes())
            .await
            .unwrap()
    })
}
