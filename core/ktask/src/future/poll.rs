// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Async helpers built on top of pollable I/O and IRQ wakers.

use core::{
    future::poll_fn,
    sync::atomic::{AtomicU32, Ordering},
    task::Poll,
};

use kerrno::{KError, KResult};
use kpoll::{IoEvents, Pollable};

/// Bitmask of IRQ numbers whose enable has been deferred.
/// `register_irq_waker` sets bits here instead of calling `enable(irq, true)`
/// directly, so that the IRQ is only unmasked when the task is about to block
/// (with local interrupts disabled), preventing the level-triggered IRQ from
/// firing during the poll phase and causing a spurious wakeup.
static PENDING_IRQ_ENABLES: AtomicU32 = AtomicU32::new(0);

/// Tracks the last IRQ that caused a wakeup via irq_hook (diagnostic).
static LAST_WAKE_IRQ: AtomicU32 = AtomicU32::new(0);

/// Returns and clears the last IRQ that woke via irq_hook (diagnostic).
pub fn take_last_wake_irq() -> u32 {
    LAST_WAKE_IRQ.swap(0, Ordering::Relaxed)
}

/// Enable all IRQs that were deferred by `register_irq_waker`.
///
/// Must be called with local interrupts disabled (e.g. while holding
/// a `SpinNoIrq` guard) so that the newly-unmasked IRQs are not delivered
/// until the task has transitioned to Blocked state.
pub fn flush_deferred_irq_enables() {
    let mask = PENDING_IRQ_ENABLES.swap(0, Ordering::Relaxed);
    let mut m = mask;
    let mut irq = 0usize;
    while m != 0 {
        if m & 1 != 0 {
            khal::irq::enable(irq, true);
        }
        irq += 1;
        m >>= 1;
    }
}

/// Discard all deferred IRQ enables (used when "immediately woken").
pub fn clear_deferred_irq_enables() {
    PENDING_IRQ_ENABLES.store(0, Ordering::Relaxed);
}

/// A helper to wrap a synchronous non-blocking I/O function into an
/// asynchronous function.
///
/// # Arguments
///
/// * `pollable`: The pollable object to register for I/O events.
/// * `events`: The I/O events to wait for.
/// * `non_blocking`: If true, the function will return `KError::WouldBlock`
///   immediately when the I/O operation would block.
/// * `f`: The synchronous non-blocking I/O function to be wrapped. It should
///   return `KError::WouldBlock` when the operation would block.
pub async fn poll_io<P: Pollable, F: FnMut() -> KResult<T>, T>(
    pollable: &P,
    events: IoEvents,
    non_blocking: bool,
    mut f: F,
) -> KResult<T> {
    super::interruptible(poll_fn(move |cx| match f() {
        Ok(value) => Poll::Ready(Ok(value)),
        Err(KError::WouldBlock) => {
            if non_blocking {
                return Poll::Ready(Err(KError::WouldBlock));
            }
            pollable.register(cx, events);
            match f() {
                Ok(value) => Poll::Ready(Ok(value)),
                Err(KError::WouldBlock) => Poll::Pending,
                Err(e) => Poll::Ready(Err(e)),
            }
        }
        Err(e) => Poll::Ready(Err(e)),
    }))
    .await?
}

/// Registers a waker for the given IRQ number.
pub fn register_irq_waker(irq: usize, waker: &core::task::Waker) {
    use alloc::collections::{BTreeMap, btree_map::Entry};

    use kpoll::PollSet;
    use kspin::SpinNoIrq;

    static POLL_IRQ: SpinNoIrq<BTreeMap<usize, PollSet>> = SpinNoIrq::new(BTreeMap::new());

    fn irq_hook(irq: usize) {
        if let Some(s) = POLL_IRQ.lock().get(&irq) {
            LAST_WAKE_IRQ.store(irq as u32, Ordering::Relaxed);
            s.wake();
        }
    }

    match POLL_IRQ.lock().entry(irq) {
        Entry::Vacant(e) => {
            khal::irq::register_irq_hook(irq_hook);
            e.insert(PollSet::new())
        }
        Entry::Occupied(e) => e.into_mut(),
    }
    .register(waker);

    // Defer the IRQ enable instead of doing it immediately.
    // For level-triggered interrupts (e.g. PCI IRQ 11), calling
    // enable(irq, true) here would unmask the IRQ while the task is still
    // in its poll() phase. If the IRQ line is asserted (e.g. another device
    // sharing the same IRQ, or ISR re-set by TX completion), the IRQ fires
    // immediately, irq_hook sets woke=true, and block_on sees "immediately
    // woken" — causing a tight busyloop without ever blocking.
    //
    // By deferring to flush_deferred_irq_enables() (called from block_on
    // with local interrupts disabled, right before blocked_resched), the
    // IRQ is only unmasked when the task is about to enter Blocked state.
    // The IRQ will be delivered after the SpinNoIrq guard is dropped, at
    // which point the task is properly Blocked and can be correctly woken.
    debug_assert!(irq < 32, "IRQ number {} out of range for deferred enable bitmask", irq);
    PENDING_IRQ_ENABLES.fetch_or(1u32 << irq, Ordering::Relaxed);
}
