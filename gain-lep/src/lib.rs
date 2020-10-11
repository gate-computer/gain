// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Access Gain APIs through an interactive interpreter.

#![forbid(unsafe_code)]

pub use lep::{obj, Domain, Fun, FunMut, Name, Obj, Pair, Ref, Res};

pub mod catalog;
pub mod display;
pub mod future;
pub mod identity;
pub mod origin;
pub mod peer;
pub mod peerindex;
mod repl;
pub mod stream;

#[doc(inline)]
pub use display::stringify;

#[doc(inline)]
pub use future::obj_future;

#[doc(inline)]
pub use repl::{repl, repl_default};

/// Register Lep and Gain builtins.
pub fn register(d: &mut Domain) {
    lep::builtin::register(d);
    stream::register(d);
    catalog::register(d);
    identity::register(d);
    origin::register(d);
    peer::register(d);
    peerindex::register(d);
}

fn wrong_number_of_arguments() -> Res {
    Err("wrong number of arguments".to_string())
}
