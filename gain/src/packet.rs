// Copyright (c) 2018 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use std::convert::TryInto;
use std::io::Write;

pub const HEADER_SIZE: usize = 8;
pub const FLOW_SIZE: usize = 8;
pub const DATA_HEADER_SIZE: usize = HEADER_SIZE + 8;
pub const SERVICES_HEADER_SIZE: usize = HEADER_SIZE + 2;

pub type Code = i16;
pub const CODE_SERVICES: Code = -1;

pub type Domain = u8;
pub const DOMAIN_CALL: Domain = 0;
pub const DOMAIN_INFO: Domain = 1;
pub const DOMAIN_FLOW: Domain = 2;
pub const DOMAIN_DATA: Domain = 3;

pub type StreamId = i32;
pub type Note = i32;

pub type ServiceStateFlags = u8;
pub const SERVICE_STATE_AVAIL: ServiceStateFlags = 0x1;

pub const ALIGNMENT: usize = 8;

pub fn align(size: usize) -> usize {
    let mask = ALIGNMENT - 1;
    (size + mask) & !mask
}

pub fn pad_len(size: usize) -> isize {
    (align(size) - size) as isize
}

pub fn header_into(p: &mut [u8], size: usize, code: Code, domain: Domain) -> &mut [u8] {
    p[0..].as_mut().write(&(size as u32).to_le_bytes()).unwrap();
    p[4..].as_mut().write(&code.to_le_bytes()).unwrap();
    p[6] = domain;
    p[7] = 0;
    &mut p[HEADER_SIZE..]
}

#[inline]
pub fn size(p: &[u8]) -> usize {
    u32::from_le_bytes(p[0..4].try_into().unwrap()) as usize
}

#[inline]
pub fn code(p: &[u8]) -> Code {
    Code::from_le_bytes(p[4..6].try_into().unwrap())
}

#[inline]
pub fn domain(p: &[u8]) -> Domain {
    p[6]
}

#[inline]
pub fn flow_count(p: &[u8]) -> usize {
    (p.len() - HEADER_SIZE) / FLOW_SIZE
}

pub struct Flow {
    pub id: StreamId,
    pub increment: i32,
}

pub fn flow_into(p: &mut [u8], index: usize, id: StreamId, increment: i32) {
    let flow = &mut p[HEADER_SIZE + FLOW_SIZE * index..];
    flow[0..].as_mut().write(&id.to_le_bytes()).unwrap();
    flow[4..].as_mut().write(&increment.to_le_bytes()).unwrap();
}

pub fn flow(p: &[u8], index: usize) -> Flow {
    let flow = &p[HEADER_SIZE + FLOW_SIZE * index..];
    Flow {
        id: StreamId::from_le_bytes(flow[0..4].try_into().unwrap()),
        increment: i32::from_le_bytes(flow[4..8].try_into().unwrap()),
    }
}

pub fn data_header_into(p: &mut [u8], size: usize, code: Code, id: StreamId, note: Note) {
    let content = header_into(p, size, code, DOMAIN_DATA);
    content[0..].as_mut().write(&id.to_le_bytes()).unwrap();
    content[4..].as_mut().write(&note.to_le_bytes()).unwrap();
}

#[inline]
pub fn data_id(p: &[u8]) -> StreamId {
    StreamId::from_le_bytes(p[HEADER_SIZE..HEADER_SIZE + 4].try_into().unwrap())
}

pub fn services_header_into(p: &mut [u8], size: usize, count: u16) {
    let content = header_into(p, size, CODE_SERVICES, DOMAIN_CALL);
    content[0..].as_mut().write(&count.to_le_bytes()).unwrap();
}

pub fn service_states<'a>(p: &'a [u8]) -> &'a [ServiceStateFlags] {
    let count = u16::from_le_bytes(p[HEADER_SIZE..SERVICES_HEADER_SIZE].try_into().unwrap());
    &p[SERVICES_HEADER_SIZE..SERVICES_HEADER_SIZE + count as usize]
}
