// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Value representation.

use lep::Obj;

use crate::future::{Fut, FutState};
use crate::stream::{ReadStreamRef, ReadWriteStreamRef, WriteStreamRef};

/// Stringify Lep and Gain object types.
///
/// None is returned if the type is not supported.
pub fn stringify(x: &Obj) -> Option<String> {
    stringify_ex(x, |_: &Obj| None)
}

/// Stringify custom types.
///
/// All values (except pairs) are passed to the supplied function first; if it
/// returns None, the default implementation (see `stringify`) is used.
pub fn stringify_ex<F: Fn(&Obj) -> Option<String> + Clone>(x: &Obj, f: F) -> Option<String> {
    lep::display::stringify_ex(x, |x: &Obj| -> Option<String> {
        if let Some(s) = f(x) {
            return Some(s);
        }

        if let Some(cell) = x.downcast_ref::<Fut>() {
            return Some(match cell.replace(FutState::Unknown) {
                FutState::Pending(b) => {
                    cell.set(FutState::Pending(b));
                    "FuturePending".to_string()
                }
                FutState::Done(res) => {
                    let s = match res {
                        Ok(ref value) => {
                            let mut s = "Future<".to_string();
                            s.push_str(&stringify_ex(&value, f.clone()).unwrap_or("?".to_string()));
                            s.push('>');
                            s
                        }
                        Err(ref x) => format!("FutureError<{}>", x),
                    };
                    cell.set(FutState::Done(res));
                    s
                }
                FutState::Unknown => "FutureUnknown".to_string(),
            });
        }

        if x.is::<ReadWriteStreamRef>() {
            return Some("ReadWriteStream".to_string());
        }

        if x.is::<ReadStreamRef>() {
            return Some("ReadStream".to_string());
        }

        if x.is::<WriteStreamRef>() {
            return Some("WriteStream".to_string());
        }

        if let Some(data) = x.downcast_ref::<Vec<u8>>() {
            return Some(format!("Vec{:x?}", data.as_slice()));
        }

        None
    })
}
