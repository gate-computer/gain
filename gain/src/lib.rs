// Copyright (c) 2018 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! # Gate interface
//!
//! ## Asynchronous execution
//!
//! The [`task`](task) module provides a framework for spawning and running
//! asynchronous tasks.
//!
//! A typical program runs a single top-level task:
//!
//! ```
//! use gain::task::{block_on, spawn};
//!
//! fn main() {
//!     block_on(async {
//!         spawn(concurrent_work());
//!         do_something().await;
//!     })
//! }
//!
//! async fn concurrent_work() {
//!     do_stuff().await;
//! }
//! ```
//!
//! Concurrency is achieved by spawning more tasks.  The program exits when the
//! top-level task returns.
//!
//! ## Service APIs
//!
//! The [`catalog`](catalog), [`identity`](identity), [`origin`](origin),
//! [`peer`](peer), [`peerindex`](peerindex) and [`random`](random) modules
//! provide access to the built-in Gate services.
//!
//! Common I/O stream types are defined in the [`stream`](stream) module.
//!
//! ## Service implementation
//!
//! Additional service bindings can be implemented using the
//! [`service`](service) module.

#[macro_use]
extern crate lazy_static;

pub mod catalog;
mod core;
mod gate;
pub mod identity;
pub mod origin;
mod packet;
pub mod peer;
pub mod peerindex;
pub mod random;
pub mod scope;
pub mod service;
pub mod stream;
pub mod task;
mod threadunsafe;
