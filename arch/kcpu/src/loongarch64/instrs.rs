// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Wrapper functions for assembly instructions.

use core::arch::asm;

use loongArch64::register::{ecfg, eentry, pgdh, pgdl};
use memaddr::PhysAddr;

pub use karch::{
    await_interrupts, enable_fp, enable_lsx, flush_tlb, read_thread_pointer, stop_cpu,
    write_thread_pointer,
};
// Re-exported with legacy names for backward compatibility.
pub use karch::{disable_irq as disable_local, enable_irq as enable_local, irq_enabled as is_enabled};

/// Reads the current page table root register for user space (`PGDL`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_user_page_table() -> PhysAddr {
    PhysAddr::from(pgdl::read().base())
}

/// Reads the current page table root register for kernel space (`PGDH`).
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_kernel_page_table() -> PhysAddr {
    PhysAddr::from(pgdh::read().base())
}

/// Writes the register to update the current page table root for user space
/// (`PGDL`).
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
pub unsafe fn write_user_page_table(root_paddr: PhysAddr) {
    pgdl::set_base(root_paddr.as_usize() as _);
}

/// Writes the register to update the current page table root for kernel space
/// (`PGDH`).
///
/// Note that the TLB is **NOT** flushed after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
pub unsafe fn write_kernel_page_table(root_paddr: PhysAddr) {
    pgdh::set_base(root_paddr.as_usize());
}

/// Writes the Exception Entry Base Address register (`EENTRY`).
///
/// It also set the Exception Configuration register (`ECFG`) to `VS=0`.
///
/// - ECFG: <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#exception-configuration>
/// - EENTRY: <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#exception-entry-base-address>
///
/// # Safety
///
/// This function is unsafe as it changes the exception handling behavior of the
/// current CPU.
#[inline]
pub unsafe fn write_exception_entry_base(eentry: usize) {
    ecfg::set_vs(0);
    eentry::set_eentry(eentry);
}

/// Writes the Page Walk Controller registers (`PWCL` and `PWCH`).
///
/// # Safety
///
/// This function is unsafe as it changes the page walk configuration such as
/// levels and starting bits.
///
/// - `PWCL`: <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#page-walk-controller-for-lower-half-address-space>
/// - `PWCH`: <https://loongson.github.io/LoongArch-Documentation/LoongArch-Vol1-EN.html#page-walk-controller-for-higher-half-address-space>
#[inline]
pub unsafe fn write_pwc(pwcl: u32, pwch: u32) {
    unsafe {
        asm!(
            include_asm_macros!(),
            "csrwr {}, LA_CSR_PWCL",
            "csrwr {}, LA_CSR_PWCH",
            in(reg) pwcl,
            in(reg) pwch
        )
    }
}

#[cfg(feature = "uspace")]
core::arch::global_asm!(include_asm_macros!(), include_str!("copy_user.S"));

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
    pub fn user_copy(dst: *mut u8, src: *const u8, size: usize) -> usize;
}

