// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Origin service bindings.

use std::cell::RefCell;
use std::rc::Rc;

use gain::origin;
use gain::stream::buf::ReadWriteStream;
use lep::{Domain, Obj, Res};

use crate::future::future_obj;

/// Register all origin functions.
pub fn register(d: &mut Domain) {
    register_accept(d);
}

/// Register the `origin/accept` function.
pub fn register_accept(d: &mut Domain) {
    d.register("origin/accept", accept);
}

/// The `origin/accept` function.
pub fn accept(args: &Obj) -> Res {
    if args.is::<()>() {
        Ok(future_obj(async {
            match origin::accept().await {
                Ok(conn) => Ok(Rc::new(RefCell::new(ReadWriteStream::new(conn))) as Obj),
                Err(e) => Err(format!("origin/accept: {}", e)),
            }
        }))
    } else {
        crate::wrong_number_of_arguments()
    }
}
