//! A library for polling I/O events and waking up tasks.

#![no_std]
#![deny(missing_docs)]

#[cfg(feature = "alloc")]
extern crate alloc;

use core::{
    task::{Context, Waker},
};

use bitflags::bitflags;
use linux_raw_sys::general::*;
use kspin::SpinNoIrq;

#[cfg(feature = "alloc")]
use alloc::vec::Vec;

bitflags! {
    /// I/O events.
    #[derive(Debug, Clone, Copy)]
    pub struct IoEvents: u32 {
        /// Available for read
        const IN     = POLLIN;
        /// Urgent data for read
        const PRI    = POLLPRI;
        /// Available for write
        const OUT    = POLLOUT;

        /// Error condition
        const ERR    = POLLERR;
        /// Hang up
        const HUP    = POLLHUP;
        /// Invalid request
        const NVAL   = POLLNVAL;

        /// Equivalent to [`IN`](Self::IN)
        const RDNORM = POLLRDNORM;
        /// Priority band data can be read
        const RDBAND = POLLRDBAND;
        /// Equivalent to [`OUT`](Self::OUT)
        const WRNORM = POLLWRNORM;
        /// Priority data can be written
        const WRBAND = POLLWRBAND;

        /// Message
        const MSG    = POLLMSG;
        /// Remove
        const REMOVE = POLLREMOVE;
        /// Stream socket peer closed connection, or shut down writing half of connection.
        const RDHUP  = POLLRDHUP;

        /// Events that are always polled even without specifying them.
        const ALWAYS_POLL = Self::ERR.bits() | Self::HUP.bits();
    }
}

/// Trait for types that can be polled for I/O events.
pub trait Pollable {
    /// Polls for I/O events.
    fn poll(&self) -> IoEvents;

    /// Registers wakers for I/O events.
    fn register(&self, context: &mut Context<'_>, events: IoEvents);
}

#[cfg(feature = "alloc")]
struct Inner {
    wakers: Vec<Waker>,
}

#[cfg(feature = "alloc")]
impl Inner {
    const fn new() -> Self {
        Self {
            wakers: Vec::new(),
        }
    }

    fn register(&mut self, waker: &Waker) {
        self.wakers.push(waker.clone());
    }

    fn take_wakers(&mut self) -> Vec<Waker> {
        core::mem::take(&mut self.wakers)
    }

    fn is_empty(&self) -> bool {
        self.wakers.is_empty()
    }

    fn len(&self) -> usize {
        self.wakers.len()
    }
}

/// A data structure for waking up tasks that are waiting for I/O events.
#[cfg(feature = "alloc")]
pub struct PollSet(SpinNoIrq<Inner>);

#[cfg(feature = "alloc")]
impl Default for PollSet {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(feature = "alloc")]
impl PollSet {
    /// Creates a new empty [`PollSet`].
    pub const fn new() -> Self {
        Self(SpinNoIrq::new(Inner::new()))
    }

    /// Registers a waker.
    pub fn register(&self, waker: &Waker) {
        self.0.lock().register(waker);
    }

    /// Wakes up all registered wakers.
    ///
    /// Returns the number of wakers that were woken up.
    pub fn wake(&self) -> usize {
        // Collect all wakers while holding the lock, then release the lock
        // before calling wake() to avoid potential deadlock
        let wakers = {
            let mut guard = self.0.lock();
            if guard.is_empty() {
                return 0;
            }
            let count = guard.len();
            let wakers = guard.take_wakers();
            drop(guard);  // Explicitly release the lock
            (count, wakers)
        };

        let (count, wakers) = wakers;
        
        // Call wake() outside the lock to avoid reentry issues
        for waker in wakers {
            waker.wake();
        }
        
        count
    }
}

#[cfg(feature = "alloc")]
impl Drop for PollSet {
    fn drop(&mut self) {
        // Ensure all wakers are woken when dropped
        self.wake();
    }
}

#[cfg(feature = "alloc")]
impl alloc::task::Wake for PollSet {
    fn wake(self: alloc::sync::Arc<Self>) {
        self.as_ref().wake();
    }

    fn wake_by_ref(self: &alloc::sync::Arc<Self>) {
        self.as_ref().wake();
    }
}
