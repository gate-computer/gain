// Copyright (c) 2019 Timo Savola.
// Use of this source code is governed by the MIT
// license that can be found in the LICENSE file.

use std::cell::{Cell, Ref, RefCell, RefMut};
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

pub struct ThreadUnsafeCell<T>(Cell<T>);

unsafe impl<T> Send for ThreadUnsafeCell<T> {}
unsafe impl<T> Sync for ThreadUnsafeCell<T> {}

impl<T: Copy> ThreadUnsafeCell<T> {
    #[inline]
    pub fn new(val: T) -> Self {
        Self(Cell::new(val))
    }

    #[inline]
    pub fn set(&self, val: T) {
        self.0.set(val)
    }

    #[inline]
    pub fn replace(&self, val: T) -> T {
        self.0.replace(val)
    }
}

pub struct ThreadUnsafeRefCell<T>(RefCell<T>);

unsafe impl<T> Send for ThreadUnsafeRefCell<T> {}
unsafe impl<T> Sync for ThreadUnsafeRefCell<T> {}

impl<T> ThreadUnsafeRefCell<T> {
    #[inline]
    pub fn borrow(&self) -> Ref<T> {
        self.0.borrow()
    }

    #[inline]
    pub fn borrow_mut(&self) -> RefMut<T> {
        self.0.borrow_mut()
    }
}

impl<T> Default for ThreadUnsafeRefCell<T>
where
    T: Default,
{
    #[inline]
    fn default() -> Self {
        Self(Default::default())
    }
}

pub struct ThreadUnsafeFuture<F, R>(pub F)
where
    F: Future<Output = R> + 'static;

unsafe impl<F, R> Send for ThreadUnsafeFuture<F, R> where F: Future<Output = R> + 'static {}

impl<F, R> Future for ThreadUnsafeFuture<F, R>
where
    F: Future<Output = R> + 'static,
{
    type Output = R;

    fn poll(self: Pin<&mut Self>, cx: &mut Context) -> Poll<R> {
        unsafe { self.map_unchecked_mut(|c| &mut c.0).poll(cx) }
    }
}
