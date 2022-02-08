// Copyright (c) 2022 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Generate random values.

use crate::service::Service;

lazy_static! {
    static ref SERVICE: Service = Service::register("random");
}

pub async fn random<T>() -> T
where
    T: AsMut<[u8]> + Default,
{
    let mut buf: T = Default::default();
    let mut offset = 0;

    while offset < buf.as_mut().len() {
        offset += SERVICE
            .call(&[], |src: &[u8]| {
                let dest = &mut buf.as_mut()[offset..];
                let n = dest.len().min(src.len());
                dest[..n].copy_from_slice(&src[..n]);
                n
            })
            .await;
    }

    buf
}
