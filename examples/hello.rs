// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

// crate-type is cdylib

#![feature(generators, proc_macro_hygiene)]

extern crate futures_await as futures;
extern crate gain;

use futures::prelude::{await, *};
use gain::origin;

#[no_mangle]
pub fn main() {
    gain::run(async_block! {
        let _ = await!(origin::write(Vec::from("hello, world\n")));
        Ok(())
    });
}

#[no_mangle]
pub fn twice() {
    let f = async_block! {
        let _ = await!(origin::write(Vec::from("hello, world\n")));
        let _ = await!(origin::write(Vec::from("hello, world\n")));
        Ok(())
    };

    gain::run(f);
}

#[no_mangle]
pub fn hello_debug() {
    gain::debug(gain::DEBUG_SEPARATOR);
    gain::debug("hello");
    gain::debug(gain::DEBUG_SEPARATOR);
    gain::debug("world\n");
}
