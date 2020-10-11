// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Peer service bindings.

use std::cell::RefCell;
use std::rc::Rc;

use gain::peer;
use gain::stream::buf::ReadWriteStream;
use lep::{Domain, Obj, Pair, Res};

use crate::future::future_obj;

/// Register all peer functions.
pub fn register(d: &mut Domain) {
    register_connect(d);
}

/// Register the `peer/connect` function.
pub fn register_connect(d: &mut Domain) {
    d.register("peer/connect", connect);
}

/// The `peer/connect` function.
pub fn connect(args: &Obj) -> Res {
    if let Some(pair0) = args.downcast_ref::<Pair>() {
        if !pair0.0.is::<String>() {
            return Err("not a string".to_string());
        }

        let arg0 = pair0.0.clone();

        if pair0.1.is::<()>() {
            return Ok(future_obj(async move {
                match peer::connect(arg0.downcast_ref::<String>().unwrap()).await {
                    Ok(conn) => Ok(Rc::new(RefCell::new(ReadWriteStream::new(conn))) as Obj),
                    Err(e) => Err(format!("peer/connect: {}", e)),
                }
            }));
        } else {
            let pair1 = pair0.1.downcast_ref::<Pair>().unwrap();
            if !pair1.0.is::<String>() {
                return Err("not a string".to_string());
            }

            if pair1.1.is::<()>() {
                let arg1 = pair1.0.clone();

                return Ok(future_obj(async move {
                    match peer::connect_group(
                        arg0.downcast_ref::<String>().unwrap(),
                        arg1.downcast_ref::<String>().unwrap(),
                    )
                    .await
                    {
                        Ok(conn) => Ok(Rc::new(RefCell::new(ReadWriteStream::new(conn))) as Obj),
                        Err(e) => Err(format!("peer/connect: {}", e)),
                    }
                }));
            }
        }
    }

    crate::wrong_number_of_arguments()
}
