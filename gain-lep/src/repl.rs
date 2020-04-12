// Copyright (c) 2020 Timo Savola. All rights reserved.
// Use of this source code is governed by a BSD-style
// license that can be found in the LICENSE file.

use gain::stream::buf::{Buf, Read, ReadWriteStream};
use gain::stream::{RecvWriteStream, Write};
use lep::{eval_stmt, obj, Domain, State};

use crate::{obj_future, stringify};

/// Read, evaluate and print in a loop.
pub async fn repl<'a>(
    conn: RecvWriteStream,
    domain: Domain<'a>,
    state: State,
) -> (Domain<'a>, State) {
    repl_default(conn, domain, state, || vec!['\r' as u8]).await
}

/// Read, evaluate and print in a loop.
///
/// The default reply function is invoked when an empty line is entered.
pub async fn repl_default<'a, DefaultFn: Fn() -> Vec<u8>>(
    conn: RecvWriteStream,
    mut domain: Domain<'a>,
    mut state: State,
    default_reply: DefaultFn,
) -> (Domain<'a>, State) {
    let mut conn = ReadWriteStream::new(conn);

    let mut buf = Vec::new();
    let mut eof = false;
    let mut read = true;

    loop {
        if !eof && read {
            eof = match conn
                .buf_read(1, |b: &mut Buf| {
                    buf.extend_from_slice(b.as_slice());
                    b.consume_all();
                    true
                })
                .await
            {
                Ok(not_eof) => !not_eof,
                Err(e) => {
                    println!("receive error: {}", e);
                    return (domain, state);
                }
            };

            read = false;
        }

        let mut input: Option<String> = None;
        let mut output = String::new();
        for i in 0..buf.len() {
            if buf[i] == '\n' as u8 {
                let tail = buf.split_off(i + 1);
                buf.resize(i, 0);

                match String::from_utf8(buf) {
                    Ok(s) => input = Some(s),
                    Err(e) => output = format!("parse error: {}\n", e),
                }

                buf = tail;
                break;
            }
        }

        if let Some(input) = input {
            if input.trim_start().is_empty() {
                output = String::from_utf8_lossy(&default_reply().as_slice()).to_string();
            } else {
                match eval_stmt(&mut domain, state.clone(), &input) {
                    Ok(new_state) => {
                        let value = &new_state.result.value;
                        let name = &new_state.result.name;
                        if name == "_" {
                            match obj_future(value).await {
                                Ok(value) => {
                                    output = format!(
                                        "{}\n",
                                        stringify(&value).unwrap_or("?".to_string())
                                    );
                                    state = new_state.with_result(value);
                                }
                                Err(e) => {
                                    output = format!("error: {}\n", e);
                                    state = new_state.with_result(obj::nil());
                                }
                            }
                        } else {
                            output = format!(
                                "{} = {}\n",
                                name,
                                stringify(value).unwrap_or("?".to_string())
                            );
                            state = new_state;
                        }
                    }
                    Err(e) => output = format!("error: {}\n", e),
                }
            }
        } else if eof {
            return (domain, state);
        } else {
            read = true;
        }

        if !output.is_empty() {
            if let Err(e) = conn.write_all(output.as_bytes()).await {
                println!("write error: {}", e);
                return (domain, state);
            }
        }
    }
}
