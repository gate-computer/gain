// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use super::{discover_service, packet, send_stream};
use bincode::serialize;
use futures::prelude::{await, *};
use std::io::Error;

#[async]
pub fn write(data: Vec<u8>) -> Result<(), Error> {
    let mut buf = Vec::with_capacity(packet::DATA_HEADER_SIZE + data.len());
    buf.resize(packet::DATA_HEADER_SIZE, 0);
    buf.extend(data);

    let service_state = await!(discover_service("origin".to_string()))?;

    let packet_size = packet::FLOW_HEADER_SIZE + packet::FLOW_SIZE;
    let mut flow = Vec::with_capacity(packet_size);
    flow.resize(packet_size, 0);

    flow[..packet::FLOW_HEADER_SIZE].clone_from_slice(
        serialize(&packet::new_flow_header(packet_size, service_state.code))
            .unwrap()
            .as_slice(),
    );

    flow[packet::FLOW_HEADER_SIZE..].clone_from_slice(
        serialize(&packet::Flow {
            id: 0,
            increment: 0,
        })
        .unwrap()
        .as_slice(),
    );

    await!(send_stream(flow))?;

    let packet_size = buf.len();

    buf[..packet::DATA_HEADER_SIZE].clone_from_slice(
        serialize(&packet::new_data_header(packet_size, service_state.code, 0))
            .unwrap()
            .as_slice(),
    );

    await!(send_stream(buf))
}
