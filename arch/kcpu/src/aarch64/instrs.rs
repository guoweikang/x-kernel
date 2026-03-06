// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Wrapper functions for assembly instructions.

use aarch64_cpu::registers::*;
use memaddr::PhysAddr;

pub use karch::{
    await_interrupts, enable_fp, flush_dcache_line, flush_icache_all, flush_tlb,
    read_thread_pointer, stop_cpu, write_thread_pointer,
};
// Re-exported with legacy names for backward compatibility.
pub use karch::{disable_irq as disable_local, enable_irq as enable_local, irq_enabled as is_enabled};

/// Reads the current page table root register for kernel space (`TTBR1_EL1`).
///
/// When the "arm-el2" feature is enabled,
/// TTBR0_EL2 is dedicated to the Hypervisor's Stage-2 page table base address.
///
/// Returns the physical address of the page table root.
#[inline]
pub fn kernel_pt_root() -> PhysAddr {
    let pt_root_reg: usize;

    #[cfg(not(feature = "arm-el2"))]
    {
        pt_root_reg = TTBR1_EL1.get() as usize;
    }

    #[cfg(feature = "arm-el2")]
    {
        pt_root_reg = TTBR0_EL2.get() as usize;
    }

    pa!(pt_root_reg)
}

/// Reads the current page table root register for user space (`TTBR0_EL1`).
///
/// When the "arm-el2" feature is enabled, for user-mode programs,
/// virtualization is completely transparent to them, so there is no need to modify
///
/// Returns the physical address of the page table root.
#[inline]
pub fn user_pt_root() -> PhysAddr {
    let val = TTBR0_EL1.get();
    pa!(val as usize)
}

/// Writes the register to update the current page table root for kernel space
/// (`TTBR1_EL1`).
///
/// When the "arm-el2" feature is enabled,
/// TTBR0_EL2 is dedicated to the Hypervisor's Stage-2 page table base address.
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_kernel_page_table(root_paddr: PhysAddr) {
    #[cfg(not(feature = "arm-el2"))]
    {
        // kernel space page table use TTBR1 (0xffff_0000_0000_0000..0xffff_ffff_ffff_ffff)
        TTBR1_EL1.set(root_paddr.as_usize() as _);
    }

    #[cfg(feature = "arm-el2")]
    {
        // kernel space page table at EL2 use TTBR0_EL2 (0x0000_0000_0000_0000..0x0000_ffff_ffff_ffff)
        TTBR0_EL2.set(root_paddr.as_usize() as _);
    }
}

/// Writes the register to update the current page table root for user space
/// (`TTBR1_EL0`).
/// When the "arm-el2" feature is enabled, for user-mode programs,
/// virtualization is completely transparent to them, so there is no need to modify
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_user_page_table(root_paddr: PhysAddr) {
    TTBR0_EL1.set(root_paddr.as_usize() as _);
}

/// Writes exception vector base address register (`VBAR_EL1`).
///
/// # Safety
///
/// This function is unsafe as it changes the exception handling behavior of the
/// current CPU.
#[inline]
pub unsafe fn write_exception_vector_base(vbar: usize) {
    #[cfg(not(feature = "arm-el2"))]
    VBAR_EL1.set(vbar as _);
    #[cfg(feature = "arm-el2")]
    VBAR_EL2.set(vbar as _);
}

#[cfg(feature = "uspace")]
core::arch::global_asm!(include_str!("copy_user.S"));

#[cfg(feature = "uspace")]
unsafe extern "C" {
    /// Copies data from source to destination, where addresses may be in user
    /// space. Equivalent to memcpy.
    ///
    /// # Safety
    /// This function is unsafe because it performs raw memory operations.
    ///
    /// # Returns
    /// Returns the number of bytes not copied. This means 0 indicates success,
    /// while a value > 0 indicates failure.
    pub fn raw_copy_from_user(dst: *mut u8, src: *const u8, size: usize) -> usize;
}

/// Alias for compatibility with other architectures
#[cfg(feature = "uspace")]
pub use raw_copy_from_user as user_copy;

