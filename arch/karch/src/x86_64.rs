// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! x86_64 low-level architecture operations.

use core::arch::asm;

use memaddr::VirtAddr;
use x86::{msr, tlb};
use x86_64::instructions::interrupts;

/// Flushes the TLB.
///
/// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
/// entry that maps the given virtual address.
#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    if let Some(vaddr) = vaddr {
        unsafe { tlb::flush(vaddr.into()) }
    } else {
        unsafe { tlb::flush_all() }
    }
}

/// Halt the current CPU.
#[inline]
pub fn stop_cpu() {
    disable_irq();
    await_interrupts(); // should never return
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn await_interrupts() {
    if cfg!(target_os = "none") {
        unsafe { asm!("hlt") }
    } else {
        core::hint::spin_loop()
    }
}

/// Allows the current CPU to respond to interrupts.
#[inline]
pub fn enable_irq() {
    #[cfg(target_os = "none")]
    interrupts::enable()
}

/// Makes the current CPU ignore interrupts.
#[inline]
pub fn disable_irq() {
    #[cfg(target_os = "none")]
    interrupts::disable()
}

/// Returns whether the current CPU is allowed to respond to interrupts.
#[inline]
pub fn irq_enabled() -> bool {
    interrupts::are_enabled()
}

/// Reads the thread pointer of the current CPU (`FS_BASE`).
///
/// It is used to implement TLS (Thread Local Storage).
#[inline]
pub fn read_thread_pointer() -> usize {
    unsafe { msr::rdmsr(msr::IA32_FS_BASE) as usize }
}

/// Writes the thread pointer of the current CPU (`FS_BASE`).
///
/// It is used to implement TLS (Thread Local Storage).
///
/// # Safety
///
/// This function is unsafe as it changes the CPU states.
#[inline]
pub unsafe fn write_thread_pointer(val: usize) {
    unsafe { msr::wrmsr(msr::IA32_FS_BASE, val as u64) }
}
