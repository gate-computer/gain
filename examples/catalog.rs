// Copyright (c) 2021 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
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
