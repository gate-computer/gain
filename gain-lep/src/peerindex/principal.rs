// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Principal peer index service bindings.

use std::rc::Rc;

use gain::peerindex;
use lep::{obj, Domain, Obj, Res};

use crate::future::future_obj;

/// Register the `peerindex/principal` function.
pub fn register(d: &mut Domain) {
    d.register("peerindex/principal", principal);
}

/// The `peerindex/principal` function.
pub fn principal(args: &Obj) -> Res {
    if args.is::<()>() {
        return Ok(future_obj(async {
            match peerindex::principal::instances().await {
                Ok(mut v) => {
                    let mut list = obj::nil();
                    while let Some(s) = v.pop() {
                        list = obj::pair(Rc::new(s), list);
                    }
                    Ok(list)
                }
                Err(e) => Err(format!("peerindex/principal: {}", e)),
            }
        }));
    }

    crate::wrong_number_of_arguments()
}
