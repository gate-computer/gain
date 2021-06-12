// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! I/O stream bindings.

use std::cell::RefCell;
use std::rc::Rc;

use gain::stream::buf::{Buf, Read, ReadStream, ReadWriteStream};
use gain::stream::{Close, Write, WriteStream};
use lep::{obj, Domain, Obj, Pair, Res};

use crate::future::future_obj;

/// Bidirectional stream object.
pub type ReadWriteStreamRef = RefCell<ReadWriteStream>;

/// Input stream object.
pub type ReadStreamRef = RefCell<ReadStream>;

/// Output stream object.
pub type WriteStreamRef = RefCell<WriteStream>;

/// Register all stream function.
pub fn register(d: &mut Domain) {
    register_receive(d);
    register_send(d);
    register_close(d);
}

/// Register the `<-` function.
pub fn register_receive(d: &mut Domain) {
    d.register("<-", receive);
}

/// Register the `->` function.
pub fn register_send(d: &mut Domain) {
    d.register("->", send);
}

/// Register the `close` function.
pub fn register_close(d: &mut Domain) {
    d.register("close", close);
}

/// The `<-` function.
pub fn receive(args: &Obj) -> Res {
    async fn read_vec<T: Read>(stream: &RefCell<T>) -> Res {
        let mut v = Vec::new();
        match stream
            .borrow_mut()
            .buf_read(1, |b: &mut Buf| v.extend_from_slice(b.as_slice()))
            .await
        {
            Ok(()) => Ok(Rc::new(v) as Obj),
            Err(e) => Err(format!("read: {}", e)),
        }
    }

    if let Some(pair) = args.downcast_ref::<Pair>() {
        if pair.1.is::<()>() {
            let arg = pair.0.clone();

            if arg.is::<ReadWriteStreamRef>() {
                return Ok(future_obj(async move {
                    read_vec(arg.downcast_ref::<ReadWriteStreamRef>().unwrap()).await
                }));
            }

            if arg.is::<ReadStreamRef>() {
                return Ok(future_obj(async move {
                    read_vec(arg.downcast_ref::<ReadStreamRef>().unwrap()).await
                }));
            }

            return Err("not an input stream".to_string());
        }
    }

    crate::wrong_number_of_arguments()
}

/// The `->` function.
pub fn send(args: &Obj) -> Res {
    async fn write_all<T: Write>(stream: &RefCell<T>, data: &String) -> Res {
        match stream.borrow_mut().write_all(data.as_bytes()).await {
            Ok(_) => Ok(obj::nil()),
            Err(e) => Err(format!("write: {}", e)),
        }
    }

    if let Some(pair0) = args.downcast_ref::<Pair>() {
        if let Some(pair1) = pair0.1.downcast_ref::<Pair>() {
            if pair1.1.is::<()>() {
                let arg0 = pair0.0.clone();
                let arg1 = pair1.0.clone();

                if !arg1.is::<String>() {
                    return Err("not a string".to_string());
                }

                if arg0.is::<ReadWriteStreamRef>() {
                    return Ok(future_obj(async move {
                        let data = arg1.downcast_ref::<String>().unwrap();
                        write_all(arg0.downcast_ref::<ReadWriteStreamRef>().unwrap(), data).await
                    }));
                }

                if arg0.is::<WriteStreamRef>() {
                    return Ok(future_obj(async move {
                        let data = arg1.downcast_ref::<String>().unwrap();
                        write_all(arg0.downcast_ref::<WriteStreamRef>().unwrap(), data).await
                    }));
                }

                return Err("not an output stream".to_string());
            }
        }
    }

    crate::wrong_number_of_arguments()
}

/// The `close` function.
pub fn close(args: &Obj) -> Res {
    async fn close<T: Close>(stream: &RefCell<T>) -> Res {
        stream.borrow_mut().close().await;
        Ok(obj::nil())
    }

    if let Some(pair) = args.downcast_ref::<Pair>() {
        if pair.1.is::<()>() {
            let arg = pair.0.clone();

            if arg.is::<ReadWriteStreamRef>() {
                return Ok(future_obj(async move {
                    close(arg.downcast_ref::<ReadWriteStreamRef>().unwrap()).await
                }));
            }

            if arg.is::<ReadStreamRef>() {
                return Ok(future_obj(async move {
                    close(arg.downcast_ref::<ReadStreamRef>().unwrap()).await
                }));
            }

            if arg.is::<WriteStreamRef>() {
                return Ok(future_obj(async move {
                    close(arg.downcast_ref::<WriteStreamRef>().unwrap()).await
                }));
            }

            return Err("not a stream".to_string());
        }
    }

    crate::wrong_number_of_arguments()
}
