// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Common error types.

/// Raw error code accessor.
pub trait ErrorCode {
    /// Get the raw error code.
    fn as_i16(&self) -> i16;
}
