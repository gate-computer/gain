// Copyright (c) 2019 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

//! Access a local HTTP server.

#[macro_use]
extern crate lazy_static;

use flatbuffers::{get_root, FlatBufferBuilder};
use gain::service::Service;

// The schema file can be found at https://gate.computer/localhost
#[allow(unused_imports)]
#[path = "localhost_generated.rs"]
mod generated;
use self::generated::localhost::flat;

lazy_static! {
    static ref SERVICE: Service = Service::register("gate.computer/localhost");
}

/// Make a GET request.
pub async fn get(uri: &str) -> Response {
    request("GET", uri, None, &[]).await
}

/// Make a POST request.
pub async fn post(uri: &str, content_type: &str, body: &[u8]) -> Response {
    request("POST", uri, Some(content_type), body).await
}

/// Make a PUT request.
pub async fn put(uri: &str, content_type: &str, body: &[u8]) -> Response {
    request("PUT", uri, Some(content_type), body).await
}

/// Make an HTTP request.
pub async fn request(
    method: &str,
    uri: &str,
    content_type: Option<&str>,
    content: &[u8],
) -> Response {
    let mut b = FlatBufferBuilder::new();

    let content = if !content.is_empty() {
        Some(b.create_vector(content))
    } else {
        None
    };

    let content_type = match content_type {
        Some(s) => Some(b.create_string(s)),
        None => None,
    };

    let uri = b.create_string(uri);
    let method = b.create_string(method);

    let function = flat::Request::create(
        &mut b,
        &flat::RequestArgs {
            method: Some(method),
            uri: Some(uri),
            content_type: content_type,
            body: content,
        },
    );

    let call = flat::Call::create(
        &mut b,
        &flat::CallArgs {
            function_type: flat::Function::Request,
            function: Some(function.as_union_value()),
        },
    );

    b.finish_minimal(call);

    SERVICE
        .call(b.finished_data(), |reply: &[u8]| {
            if reply.is_empty() {
                return Response {
                    status_code: 0,
                    content_type: None,
                    content: Vec::new(),
                };
            }

            let r = get_root::<flat::Response>(reply);

            Response {
                status_code: r.status_code(),
                content_type: match r.content_type() {
                    Some(s) => Some(s.to_owned()),
                    None => None,
                },
                content: {
                    let mut v = Vec::new();
                    if let Some(b) = r.body() {
                        v.reserve(b.len());
                        v.extend_from_slice(b);
                    }
                    v
                },
            }
        })
        .await
}

/// HTTP response.
pub struct Response {
    pub status_code: u16,
    pub content_type: Option<String>,
    pub content: Vec<u8>,
}

#[cfg(feature = "lep")]
pub mod lep;
