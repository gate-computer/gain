// Copyright (c) 2020 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

//! Types and traits for working with asynchronous tasks.

use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};

use async_task::Task;

use crate::core;
use crate::core::YieldFuture;
use crate::threadunsafe::{ThreadUnsafeCell, ThreadUnsafeFuture, ThreadUnsafeRefCell};

lazy_static! {
    static ref TASKS: ThreadUnsafeRefCell<VecDeque<Task<()>>> = Default::default();
}

/// A handle that awaits the result of a task.
pub type JoinHandle<T> = async_task::JoinHandle<T, ()>;

/// Spawn a task and block the program on its result.
#[inline(always)]
pub fn block_on<F, T>(future: F) -> T
where
    F: Future<Output = T>,
{
    block_on_boxed(Box::pin(future))
}

fn block_on_boxed<F, T>(mut future: Pin<Box<F>>) -> T
where
    F: Future<Output = T>,
{
    let rerun = Arc::new(ThreadUnsafeCell::new(false));
    let wakerun = rerun.clone();
    let waker = async_task::waker_fn(move || wakerun.set(true));
    let cx = &mut Context::from_waker(&waker);

    loop {
        if let Poll::Ready(result) = future.as_mut().poll(cx) {
            return result;
        }

        loop {
            // Can't access TASKS in the same for/if with run().
            let task = TASKS.borrow_mut().pop_front();
            if let Some(task) = task {
                task.run();
            } else {
                break;
            }
        }

        if !rerun.replace(false) {
            core::io();
        }
    }
}

/// Spawn a new task.
pub fn spawn<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + Send + 'static,
    T: Send + 'static,
{
    let (task, handle) = async_task::spawn(future, |task| TASKS.borrow_mut().push_back(task), ());
    task.schedule();
    handle
}

/// Spawn a new local task.
///
/// ```
/// // TODO: T shouldn't require Send trait.
/// ```
#[inline(always)]
pub fn spawn_local<F, T>(future: F) -> JoinHandle<T>
where
    F: Future<Output = T> + 'static,
    T: Send + 'static,
{
    spawn(ThreadUnsafeFuture(future))
}

/// Yield execution back to the runtime.
pub async fn yield_now() {
    YieldFuture::new().await;
}
