// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Restrict execution privileges.

use crate::service::Service;

lazy_static! {
    static ref SERVICE: Service = Service::register("scope");
}

const CALL_RESTRICT: u8 = 0;

/// Restrict execution privileges to the specified set.  Privileges cannot be
/// added; each invocation can only remove privileges (extraneous scope is
/// ignored).  Actual privileges depend also on the execution environment, and
/// may vary during program execution.
pub async fn restrict(scope: &[&str]) {
    if scope.len() > 255 {
        panic!("scope is too large");
    }

    let mut bufsize = 1 + 1;
    for s in scope.iter() {
        let n = s.as_bytes().len();
        if n > 255 {
            panic!("scope string is too long");
        }
        bufsize += 1 + n;
    }

    let mut buf = Vec::with_capacity(bufsize);
    buf.push(CALL_RESTRICT);
    buf.push(scope.len() as u8);
    for s in scope.iter() {
        let b = s.as_bytes();
        buf.push(b.len() as u8);
        buf.extend_from_slice(b);
    }

    SERVICE
        .call(buf.as_slice(), |reply: &[u8]| {
            if reply.is_empty() {
                panic!("unknown scope service call");
            }

            let error = i16::from_le_bytes(reply[..2].try_into().unwrap());
            if error != 0 {
                panic!("unexpected scope service call error");
            }
        })
        .await;
}

/// Represents system access.
pub const SCOPE_SYSTEM: &str = "program:system";
