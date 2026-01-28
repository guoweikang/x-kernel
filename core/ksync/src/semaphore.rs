//! A counting semaphore implementation.

use core::sync::atomic::{AtomicUsize, Ordering};
use axtask::future::block_on;
use event_listener::{Event, listener};

/// A counting semaphore.
///
/// Allows a specified number of permits to be acquired.
pub struct Semaphore {
    count: AtomicUsize,
    event: Event,
}

impl Semaphore {
    /// Creates a new semaphore with the given number of permits.
    pub const fn new(permits: usize) -> Self {
        Self {
            count: AtomicUsize::new(permits),
            event: Event::new(),
        }
    }
    
    /// Acquires a permit, blocking until one is available.
    pub fn acquire(&self) {
        loop {
            let count = self.count.load(Ordering::Relaxed);
            
            if count == 0 {
                listener!(self.event => listener);
                if self.count.load(Ordering::Acquire) == 0 {
                    block_on(listener);
                }
                continue;
            }
            
            match self.count.compare_exchange_weak(
                count,
                count - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return,
                Err(_) => continue,
            }
        }
    }
    
    /// Tries to acquire a permit without blocking.
    ///
    /// Returns `true` if a permit was acquired, `false` otherwise.
    pub fn try_acquire(&self) -> bool {
        loop {
            let count = self.count.load(Ordering::Relaxed);
            
            if count == 0 {
                return false;
            }
            
            match self.count.compare_exchange_weak(
                count,
                count - 1,
                Ordering::Acquire,
                Ordering::Relaxed,
            ) {
                Ok(_) => return true,
                Err(_) => continue,
            }
        }
    }
    
    /// Releases a permit.
    pub fn release(&self) {
        self.count.fetch_add(1, Ordering::Release);
        self.event.notify(1);
    }
    
    /// Returns the current number of available permits.
    pub fn available_permits(&self) -> usize {
        self.count.load(Ordering::Relaxed)
    }

    /// Acquires a permit and returns a guard.
    ///
    /// The permit is automatically released when the guard is dropped.
    pub fn acquire_guard(&self) -> SemaphoreGuard<'_> {
        self.acquire();
        SemaphoreGuard { sem: self }
    }
}

/// RAII guard for a semaphore permit.
///
/// The permit is automatically released when the guard is dropped.
pub struct SemaphoreGuard<'a> {
    sem: &'a Semaphore,
}

impl Drop for SemaphoreGuard<'_> {
    fn drop(&mut self) {
        self.sem.release();
    }
}
