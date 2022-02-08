// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use gain::random::random;
use gain::task::block_on;

fn main() {
    block_on(async {
        for _ in 0..10 {
            println!("{}", i128::from_le_bytes(random().await));
        }
    })
}
