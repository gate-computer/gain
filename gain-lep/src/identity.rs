// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Identity service bindings.

use std::rc::Rc;

use gain::identity;
use lep::{obj, Domain, Obj, Res};

use crate::future::future_obj;

/// Register all identity service functions.
pub fn register(d: &mut Domain) {
    register_principal(d);
    register_instance(d);
}

/// Register the `identity/principal` function.
pub fn register_principal(d: &mut Domain) {
    d.register("identity/principal", principal);
}

/// Register the `identity/instance` function.
pub fn register_instance(d: &mut Domain) {
    d.register("identity/instance", instance);
}

/// The `identity/principal` function.
pub fn principal(args: &Obj) -> Res {
    if args.is::<()>() {
        return Ok(future_obj(async {
            Ok(match identity::principal_id().await {
                Some(id) => Rc::new(id),
                None => obj::nil(),
            })
        }));
    }

    crate::wrong_number_of_arguments()
}

/// The `identity/instance` function.
pub fn instance(args: &Obj) -> Res {
    if args.is::<()>() {
        return Ok(future_obj(async {
            Ok(match identity::instance_id().await {
                Some(id) => Rc::new(id),
                None => obj::nil(),
            })
        }));
    }

    crate::wrong_number_of_arguments()
}
