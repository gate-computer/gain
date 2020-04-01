// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Binding implementation support.

use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;

use lep::{Obj, Res};

pub enum FutState {
    Pending(Pin<Box<dyn Future<Output = Res>>>),
    Done(Res),
    Unknown,
}

/// Asynchronous result.
pub type Fut = Cell<FutState>;

/// Wrap an asynchronous result into a Lep object.
pub fn future_obj<T: Future<Output = Res> + 'static>(future: T) -> Obj {
    Rc::new(Cell::new(FutState::Pending(Box::pin(future))) as Fut)
}

/// Unwrap an asynchronous result if the object contains one.
///
/// Otherwise the original object is returned.
pub async fn obj_future(x: &Obj) -> Res {
    if let Some(cell) = x.downcast_ref::<Fut>() {
        match cell.replace(FutState::Unknown) {
            FutState::Pending(mut b) => {
                let res = b.as_mut().await;
                cell.set(FutState::Done(res.clone()));
                res
            }
            FutState::Done(res) => {
                cell.set(FutState::Done(res.clone()));
                res
            }
            FutState::Unknown => Err("future is unknown".to_string()),
        }
    } else {
        Ok(x.clone())
    }
}
