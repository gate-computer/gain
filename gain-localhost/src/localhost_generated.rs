// automatically generated by the FlatBuffers compiler, do not modify



use std::mem;
use std::cmp::Ordering;

extern crate flatbuffers;
use self::flatbuffers::EndianScalar;

#[allow(unused_imports, dead_code)]
pub mod localhost {

  use std::mem;
  use std::cmp::Ordering;

  extern crate flatbuffers;
  use self::flatbuffers::EndianScalar;
#[allow(unused_imports, dead_code)]
pub mod flat {

  use std::mem;
  use std::cmp::Ordering;

  extern crate flatbuffers;
  use self::flatbuffers::EndianScalar;

#[allow(non_camel_case_types)]
#[repr(u8)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Function {
  NONE = 0,
  Request = 1,

}

const ENUM_MIN_FUNCTION: u8 = 0;
const ENUM_MAX_FUNCTION: u8 = 1;

impl<'a> flatbuffers::Follow<'a> for Function {
  type Inner = Self;
  #[inline]
  fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
    flatbuffers::read_scalar_at::<Self>(buf, loc)
  }
}

impl flatbuffers::EndianScalar for Function {
  #[inline]
  fn to_little_endian(self) -> Self {
    let n = u8::to_le(self as u8);
    let p = &n as *const u8 as *const Function;
    unsafe { *p }
  }
  #[inline]
  fn from_little_endian(self) -> Self {
    let n = u8::from_le(self as u8);
    let p = &n as *const u8 as *const Function;
    unsafe { *p }
  }
}

impl flatbuffers::Push for Function {
    type Output = Function;
    #[inline]
    fn push(&self, dst: &mut [u8], _rest: &[u8]) {
        flatbuffers::emplace_scalar::<Function>(dst, *self);
    }
}

#[allow(non_camel_case_types)]
const ENUM_VALUES_FUNCTION:[Function; 2] = [
  Function::NONE,
  Function::Request
];

#[allow(non_camel_case_types)]
const ENUM_NAMES_FUNCTION:[&'static str; 2] = [
    "NONE",
    "Request"
];

pub fn enum_name_function(e: Function) -> &'static str {
  let index = e as u8;
  ENUM_NAMES_FUNCTION[index as usize]
}

pub struct FunctionUnionTableOffset {}
pub enum RequestOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct Request<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Request<'a> {
    type Inner = Request<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> Request<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        Request {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args RequestArgs<'args>) -> flatbuffers::WIPOffset<Request<'bldr>> {
      let mut builder = RequestBuilder::new(_fbb);
      if let Some(x) = args.body { builder.add_body(x); }
      if let Some(x) = args.content_type { builder.add_content_type(x); }
      if let Some(x) = args.uri { builder.add_uri(x); }
      if let Some(x) = args.method { builder.add_method(x); }
      builder.finish()
    }

    pub const VT_METHOD: flatbuffers::VOffsetT = 4;
    pub const VT_URI: flatbuffers::VOffsetT = 6;
    pub const VT_CONTENT_TYPE: flatbuffers::VOffsetT = 8;
    pub const VT_BODY: flatbuffers::VOffsetT = 10;

  #[inline]
  pub fn method(&self) -> Option<&'a str> {
    self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Request::VT_METHOD, None)
  }
  #[inline]
  pub fn uri(&self) -> Option<&'a str> {
    self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Request::VT_URI, None)
  }
  #[inline]
  pub fn content_type(&self) -> Option<&'a str> {
    self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Request::VT_CONTENT_TYPE, None)
  }
  #[inline]
  pub fn body(&self) -> Option<&'a [u8]> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, u8>>>(Request::VT_BODY, None).map(|v| v.safe_slice())
  }
}

pub struct RequestArgs<'a> {
    pub method: Option<flatbuffers::WIPOffset<&'a  str>>,
    pub uri: Option<flatbuffers::WIPOffset<&'a  str>>,
    pub content_type: Option<flatbuffers::WIPOffset<&'a  str>>,
    pub body: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a ,  u8>>>,
}
impl<'a> Default for RequestArgs<'a> {
    #[inline]
    fn default() -> Self {
        RequestArgs {
            method: None,
            uri: None,
            content_type: None,
            body: None,
        }
    }
}
pub struct RequestBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> RequestBuilder<'a, 'b> {
  #[inline]
  pub fn add_method(&mut self, method: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Request::VT_METHOD, method);
  }
  #[inline]
  pub fn add_uri(&mut self, uri: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Request::VT_URI, uri);
  }
  #[inline]
  pub fn add_content_type(&mut self, content_type: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Request::VT_CONTENT_TYPE, content_type);
  }
  #[inline]
  pub fn add_body(&mut self, body: flatbuffers::WIPOffset<flatbuffers::Vector<'b , u8>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Request::VT_BODY, body);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> RequestBuilder<'a, 'b> {
    let start = _fbb.start_table();
    RequestBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Request<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

pub enum ResponseOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct Response<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Response<'a> {
    type Inner = Response<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> Response<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        Response {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args ResponseArgs<'args>) -> flatbuffers::WIPOffset<Response<'bldr>> {
      let mut builder = ResponseBuilder::new(_fbb);
      if let Some(x) = args.body { builder.add_body(x); }
      if let Some(x) = args.content_type { builder.add_content_type(x); }
      builder.add_status_code(args.status_code);
      builder.finish()
    }

    pub const VT_STATUS_CODE: flatbuffers::VOffsetT = 4;
    pub const VT_CONTENT_TYPE: flatbuffers::VOffsetT = 6;
    pub const VT_BODY: flatbuffers::VOffsetT = 8;

  #[inline]
  pub fn status_code(&self) -> u16 {
    self._tab.get::<u16>(Response::VT_STATUS_CODE, Some(0)).unwrap()
  }
  #[inline]
  pub fn content_type(&self) -> Option<&'a str> {
    self._tab.get::<flatbuffers::ForwardsUOffset<&str>>(Response::VT_CONTENT_TYPE, None)
  }
  #[inline]
  pub fn body(&self) -> Option<&'a [u8]> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Vector<'a, u8>>>(Response::VT_BODY, None).map(|v| v.safe_slice())
  }
}

pub struct ResponseArgs<'a> {
    pub status_code: u16,
    pub content_type: Option<flatbuffers::WIPOffset<&'a  str>>,
    pub body: Option<flatbuffers::WIPOffset<flatbuffers::Vector<'a ,  u8>>>,
}
impl<'a> Default for ResponseArgs<'a> {
    #[inline]
    fn default() -> Self {
        ResponseArgs {
            status_code: 0,
            content_type: None,
            body: None,
        }
    }
}
pub struct ResponseBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> ResponseBuilder<'a, 'b> {
  #[inline]
  pub fn add_status_code(&mut self, status_code: u16) {
    self.fbb_.push_slot::<u16>(Response::VT_STATUS_CODE, status_code, 0);
  }
  #[inline]
  pub fn add_content_type(&mut self, content_type: flatbuffers::WIPOffset<&'b  str>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Response::VT_CONTENT_TYPE, content_type);
  }
  #[inline]
  pub fn add_body(&mut self, body: flatbuffers::WIPOffset<flatbuffers::Vector<'b , u8>>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Response::VT_BODY, body);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> ResponseBuilder<'a, 'b> {
    let start = _fbb.start_table();
    ResponseBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Response<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

pub enum CallOffset {}
#[derive(Copy, Clone, Debug, PartialEq)]

pub struct Call<'a> {
  pub _tab: flatbuffers::Table<'a>,
}

impl<'a> flatbuffers::Follow<'a> for Call<'a> {
    type Inner = Call<'a>;
    #[inline]
    fn follow(buf: &'a [u8], loc: usize) -> Self::Inner {
        Self {
            _tab: flatbuffers::Table { buf: buf, loc: loc },
        }
    }
}

impl<'a> Call<'a> {
    #[inline]
    pub fn init_from_table(table: flatbuffers::Table<'a>) -> Self {
        Call {
            _tab: table,
        }
    }
    #[allow(unused_mut)]
    pub fn create<'bldr: 'args, 'args: 'mut_bldr, 'mut_bldr>(
        _fbb: &'mut_bldr mut flatbuffers::FlatBufferBuilder<'bldr>,
        args: &'args CallArgs) -> flatbuffers::WIPOffset<Call<'bldr>> {
      let mut builder = CallBuilder::new(_fbb);
      if let Some(x) = args.function { builder.add_function(x); }
      builder.add_function_type(args.function_type);
      builder.finish()
    }

    pub const VT_FUNCTION_TYPE: flatbuffers::VOffsetT = 4;
    pub const VT_FUNCTION: flatbuffers::VOffsetT = 6;

  #[inline]
  pub fn function_type(&self) -> Function {
    self._tab.get::<Function>(Call::VT_FUNCTION_TYPE, Some(Function::NONE)).unwrap()
  }
  #[inline]
  pub fn function(&self) -> Option<flatbuffers::Table<'a>> {
    self._tab.get::<flatbuffers::ForwardsUOffset<flatbuffers::Table<'a>>>(Call::VT_FUNCTION, None)
  }
  #[inline]
  #[allow(non_snake_case)]
  pub fn function_as_request(&self) -> Option<Request<'a>> {
    if self.function_type() == Function::Request {
      self.function().map(|u| Request::init_from_table(u))
    } else {
      None
    }
  }

}

pub struct CallArgs {
    pub function_type: Function,
    pub function: Option<flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>>,
}
impl<'a> Default for CallArgs {
    #[inline]
    fn default() -> Self {
        CallArgs {
            function_type: Function::NONE,
            function: None,
        }
    }
}
pub struct CallBuilder<'a: 'b, 'b> {
  fbb_: &'b mut flatbuffers::FlatBufferBuilder<'a>,
  start_: flatbuffers::WIPOffset<flatbuffers::TableUnfinishedWIPOffset>,
}
impl<'a: 'b, 'b> CallBuilder<'a, 'b> {
  #[inline]
  pub fn add_function_type(&mut self, function_type: Function) {
    self.fbb_.push_slot::<Function>(Call::VT_FUNCTION_TYPE, function_type, Function::NONE);
  }
  #[inline]
  pub fn add_function(&mut self, function: flatbuffers::WIPOffset<flatbuffers::UnionWIPOffset>) {
    self.fbb_.push_slot_always::<flatbuffers::WIPOffset<_>>(Call::VT_FUNCTION, function);
  }
  #[inline]
  pub fn new(_fbb: &'b mut flatbuffers::FlatBufferBuilder<'a>) -> CallBuilder<'a, 'b> {
    let start = _fbb.start_table();
    CallBuilder {
      fbb_: _fbb,
      start_: start,
    }
  }
  #[inline]
  pub fn finish(self) -> flatbuffers::WIPOffset<Call<'a>> {
    let o = self.fbb_.end_table(self.start_);
    flatbuffers::WIPOffset::new(o.value())
  }
}

}  // pub mod flat
}  // pub mod localhost
