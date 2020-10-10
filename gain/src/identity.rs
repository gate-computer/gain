// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Identity information for this execution context.

use crate::service::Service;

lazy_static! {
    static ref SERVICE: Service = Service::register("identity");
}

const CALL_PRINCIPAL_ID: u8 = 1;
const CALL_INSTANCE_ID: u8 = 2;

/// Get an id of this program's owner, if any.
pub async fn principal_id() -> Option<String> {
    get_id(CALL_PRINCIPAL_ID).await
}

/// Get the instance id of this program invocation, if there is one.  It may
/// change if the program is suspended and resumed.
pub async fn instance_id() -> Option<String> {
    get_id(CALL_INSTANCE_ID).await
}

async fn get_id(call: u8) -> Option<String> {
    SERVICE
        .call(&[call], |reply: &[u8]| {
            if !reply.is_empty() {
                Some(String::from_utf8(reply.to_vec()).unwrap())
            } else {
                None
            }
        })
        .await
}
