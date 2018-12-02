// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

extern crate bincode;

use bincode::deserialize;

/// Maximum packet size.
pub const MAX_SIZE: usize = 65536;

/// Size of a serialized packet header.
pub const HEADER_SIZE: usize = 8;

/// Size of a serialized service discovery packet header.
pub const SERVICES_HEADER_SIZE: usize = HEADER_SIZE + 8;

/// Size of a serialized stream flow packet header.
pub const FLOW_HEADER_SIZE: usize = HEADER_SIZE;

/// Size of a serialized stream flow item.
pub const FLOW_SIZE: usize = 8;

/// Size of a serialized stream data packet header.
pub const DATA_HEADER_SIZE: usize = HEADER_SIZE + 8;

/// The built-in packet code used for service discovery.
pub const CODE_SERVICES: i16 = -1;

/// Service state flag indicating that the service is currently available.
pub const SERVICE_STATE_AVAIL: u8 = 0x1;

/// Packet domain.
pub type Domain = u8;

/// Call packet domain.
pub const DOMAIN_CALL: Domain = 0;

/// State packet domain.
pub const DOMAIN_STATE: Domain = 1;

/// Stream flow packet domain.
pub const DOMAIN_FLOW: Domain = 2;

/// Stream data packet domain.
pub const DOMAIN_DATA: Domain = 3;

#[derive(Deserialize)]
struct Size(u32);

/// Deserialize the size field of a packet.  Other header fields and packet
/// contents are ignored.
///
/// # Panics
///
/// Panics if the vector is shorter than the size field.
pub fn deserialize_size(data: &Vec<u8>) -> usize {
    let size: Size = deserialize(&data[..4]).unwrap();
    return size.0 as usize;
}

/// Packet header.
#[derive(Deserialize, Serialize)]
pub struct Header {
    pub size: u32,
    pub code: i16,
    pub domain: Domain,
    reserved0: u8,
}

/// New packet header.
pub fn new_header(size: usize, code: i16, domain: Domain) -> Header {
    Header {
        size: size as u32,
        code: code,
        domain: domain,
        reserved0: 0,
    }
}

/// Deserialize the header of a packet.  Packet contents are ignored.
///
/// # Panics
///
/// Panics if the vector is shorter than a packet header.
pub fn deserialize_header(data: &Vec<u8>) -> Header {
    deserialize(&data[..HEADER_SIZE]).unwrap()
}

/// Services packet header.
#[derive(Deserialize, Serialize)]
pub struct ServicesHeader {
    pub packet: Header,
    pub count: u16,
    reserved0: u16,
    reserved1: u32,
}

/// New services packet header.
pub fn new_services_header(size: usize, count: u16) -> ServicesHeader {
    ServicesHeader {
        packet: new_header(size, CODE_SERVICES, DOMAIN_CALL),
        count: count,
        reserved0: 0,
        reserved1: 0,
    }
}

/// New stream flow packet header.
pub fn new_flow_header(size: usize, code: i16) -> Header {
    new_header(size, code, DOMAIN_FLOW)
}

/// Stream flow item.
#[derive(Deserialize, Serialize)]
pub struct Flow {
    pub id: i32,
    pub increment: u32,
}

/// Stream data packet header.
#[derive(Deserialize, Serialize)]
pub struct DataHeader {
    pub packet: Header,
    pub id: i32,
    reserved0: u32,
}

/// New stream data packet header.
pub fn new_data_header(size: usize, code: i16, id: i32) -> DataHeader {
    DataHeader {
        packet: new_header(size, code, DOMAIN_DATA),
        id: id,
        reserved0: 0,
    }
}
