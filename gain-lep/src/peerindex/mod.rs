// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Peer index service bindings.

use lep::Domain;

pub mod principal;

/// Register all peer index functions.
pub fn register(d: &mut Domain) {
    principal::register(d);
}
