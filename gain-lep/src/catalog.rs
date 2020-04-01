// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Catalog service bindings.

use std::rc::Rc;

use gain::catalog::json;
use lep::{obj, Domain, Obj, Res};

use crate::future::future_obj;

/// Register the `catalog` function.
pub fn register(d: &mut Domain) {
    d.register("catalog", catalog);
}

/// The `catalog` function.
pub fn catalog(args: &Obj) -> Res {
    if args.is::<()>() {
        return Ok(future_obj(async {
            Ok(Rc::new(obj::Name(json().await.trim().to_string())) as Obj)
        }));
    }

    crate::wrong_number_of_arguments()
}
