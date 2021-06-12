// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Programmer-readable catalog of available services.

use crate::service::Service;

lazy_static! {
    static ref SERVICE: Service = Service::register("catalog");
}

/// Get a JSON document describing available services.
pub async fn json() -> String {
    SERVICE
        .call("json".as_bytes(), |reply: &[u8]| {
            String::from_utf8_lossy(reply).to_string()
        })
        .await
}
