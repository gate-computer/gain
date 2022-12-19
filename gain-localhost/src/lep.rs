// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Lep bindings.

use std::rc::Rc;

use gain_lep::future::future_obj;
use gain_lep::{Domain, Obj, Pair, Res};

/// Register all localhost functions.
pub fn register(d: &mut Domain) {
    d.register("localhost/get", get);
}

/// The `localhost/get` function.
pub fn get(args: &Obj) -> Res {
    if let Some(pair) = args.downcast_ref::<Pair>() {
        if pair.1.is::<()>() {
            let arg = pair.0.clone();
            if arg.is::<String>() {
                return Ok(future_obj(async move {
                    let uri = arg.downcast_ref::<String>().unwrap();
                    let res = crate::get(uri).await;
                    if res.status_code >= 200 && res.status_code < 400 {
                        Ok(Rc::new(res.content) as Obj)
                    } else {
                        Err(format!("status: {}", res.status_code))
                    }
                }));
            }

            return Err("not a string".to_string());
        }
    }

    Err("wrong number of arguments".to_string())
}
