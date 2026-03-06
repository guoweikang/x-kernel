// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! RISC-V low-level architecture operations.

use memaddr::VirtAddr;
use riscv::{asm, register::sstatus};

/// Flushes the TLB.
///
/// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
/// entry that maps the given virtual address.
#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    if let Some(vaddr) = vaddr {
        asm::sfence_vma(0, vaddr.as_usize())
    } else {
        asm::sfence_vma_all();
    }
}

/// Halt the current CPU.
#[inline]
pub fn stop_cpu() {
    disable_irq();
    riscv::asm::wfi(); // should never return
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn await_interrupts() {
    riscv::asm::wfi()
}

/// Allows the current CPU to respond to interrupts.
#[inline]
pub fn enable_irq() {
    unsafe { sstatus::set_sie() }
}

/// Makes the current CPU ignore interrupts.
#[inline]
pub fn disable_irq() {
    unsafe { sstatus::clear_sie() }
}

/// Returns whether the current CPU is allowed to respond to interrupts.
#[inline]
pub fn irq_enabled() -> bool {
    sstatus::read().sie()
}

/// Reads the thread pointer of the current CPU (`tp`).
///
/// It is used to implement TLS (Thread Local Storage).
#[inline]
pub fn read_thread_pointer() -> usize {
    let tp;
    unsafe { core::arch::asm!("mv {}, tp", out(reg) tp) };
    tp
}

/// Writes the thread pointer of the current CPU (`tp`).
///
/// It is used to implement TLS (Thread Local Storage).
///
/// # Safety
///
/// This function is unsafe as it changes the CPU states.
#[inline]
pub unsafe fn write_thread_pointer(val: usize) {
    unsafe { core::arch::asm!("mv tp, {}", in(reg) val) }
}
