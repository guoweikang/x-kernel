// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! AArch64 low-level architecture operations.

use core::arch::asm;

use aarch64_cpu::{asm::barrier, registers::*};
use memaddr::VirtAddr;

/// Flushes the TLB.
///
/// If `vaddr` is [`None`], flushes the entire TLB. Otherwise, flushes the TLB
/// entry that maps the given virtual address.
#[inline]
pub fn flush_tlb(vaddr: Option<VirtAddr>) {
    if let Some(vaddr) = vaddr {
        const VA_MASK: usize = (1 << 44) - 1; // VA[55:12] => bits[43:0]
        let operand = (vaddr.as_usize() >> 12) & VA_MASK;

        #[cfg(not(feature = "arm-el2"))]
        unsafe {
            // TLB Invalidate by VA, All ASID, EL1, Inner Shareable
            asm!("tlbi vaae1is, {}; dsb sy; isb", in(reg) operand)
        }
        #[cfg(feature = "arm-el2")]
        unsafe {
            // TLB Invalidate by VA, EL2, Inner Shareable
            asm!("tlbi vae2is, {}; dsb sy; isb", in(reg) operand)
        }
    } else {
        // flush the entire TLB
        #[cfg(not(feature = "arm-el2"))]
        unsafe {
            // TLB Invalidate by VMID, All at stage 1, EL1
            asm!("dsb sy; isb; tlbi vmalle1; dsb sy; isb")
        }
        #[cfg(feature = "arm-el2")]
        unsafe {
            // TLB Invalidate All, EL2
            asm!("tlbi alle2; dsb sy; isb")
        }
    }
}

/// Flushes the entire instruction cache.
#[inline]
pub fn flush_icache_all() {
    unsafe { asm!("ic iallu; dsb sy; isb") };
}

/// Flushes the data cache line at the given virtual address.
///
/// Uses the `DC IVAC` instruction (Data Cache Invalidate by Virtual Address to
/// Point of Coherency). The cache line size is implementation-defined; 64 bytes
/// is typical for AArch64 but may vary across CPU implementations.
#[inline]
pub fn flush_dcache_line(vaddr: VirtAddr) {
    unsafe { asm!("dc ivac, {0:x}; dsb sy; isb", in(reg) vaddr.as_usize()) };
}

/// Halt the current CPU.
///
/// Disables interrupts then executes WFI. Since interrupts are disabled,
/// this should stop execution until reset.
#[inline]
pub fn stop_cpu() {
    disable_irq();
    aarch64_cpu::asm::wfi();
}

/// Relaxes the current CPU and waits for interrupts.
///
/// It must be called with interrupts enabled, otherwise it will never return.
#[inline]
pub fn await_interrupts() {
    aarch64_cpu::asm::wfi();
}

/// Allows the current CPU to respond to interrupts (clears DAIF.I).
#[inline]
pub fn enable_irq() {
    DAIF.write(DAIF::I::Unmasked);
}

/// Makes the current CPU ignore interrupts (sets DAIF.I).
#[inline]
pub fn disable_irq() {
    DAIF.write(DAIF::I::Masked);
}

/// Returns whether the current CPU is allowed to respond to interrupts.
#[inline]
pub fn irq_enabled() -> bool {
    !DAIF.is_set(DAIF::I)
}

/// Reads the thread pointer of the current CPU (`TPIDR_EL0`).
///
/// It is used to implement TLS (Thread Local Storage).
#[inline]
pub fn read_thread_pointer() -> usize {
    TPIDR_EL0.get() as usize
}

/// Writes the thread pointer of the current CPU (`TPIDR_EL0`).
///
/// It is used to implement TLS (Thread Local Storage).
///
/// # Safety
///
/// This function is unsafe as it changes the current CPU states.
#[inline]
pub unsafe fn write_thread_pointer(val: usize) {
    TPIDR_EL0.set(val as _)
}

/// Enable FP/SIMD instructions by setting the `FPEN` field in `CPACR_EL1`.
#[inline]
pub fn enable_fp() {
    CPACR_EL1.write(CPACR_EL1::FPEN::TrapNothing);
    barrier::isb(barrier::SY);
}
