// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Wrapper functions for assembly instructions.

use memaddr::{MemoryAddr, PhysAddr};
use x86::controlregs;

pub use karch::{
    await_interrupts, flush_tlb, read_thread_pointer, stop_cpu, write_thread_pointer,
};
// Re-exported with legacy names for backward compatibility.
pub use karch::{disable_irq as disable_local, enable_irq as enable_local, irq_enabled as is_enabled};

/// Reads the current page table root register for user space (`CR3`).
///
/// x86_64 does not have a separate page table root register for user and
/// kernel space, so this operation is the same as [`read_kernel_page_table`].
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_user_page_table() -> PhysAddr {
    pa!(unsafe { controlregs::cr3() } as usize).align_down_4k()
}

/// Reads the current page table root register for kernel space (`CR3`).
///
/// x86_64 does not have a separate page table root register for user and
/// kernel space, so this operation is the same as [`read_user_page_table`].
///
/// Returns the physical address of the page table root.
#[inline]
pub fn read_kernel_page_table() -> PhysAddr {
    read_user_page_table()
}

/// Writes the register to update the current page table root for user space
/// (`CR3`).
///
/// x86_64 does not have a separate page table root register for user
/// and kernel space, so this operation is the same as [`write_kernel_page_table`].
///
/// Note that the TLB will be **flushed** after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_user_page_table(root_paddr: PhysAddr) {
    unsafe { controlregs::cr3_write(root_paddr.as_usize() as _) }
}

/// Writes the register to update the current page table root for kernel space
/// (`CR3`).
///
/// x86_64 does not have a separate page table root register for user
/// and kernel space, so this operation is the same as [`write_user_page_table`].
///
/// Note that the TLB will be **flushed** after this operation.
///
/// # Safety
///
/// This function is unsafe as it changes the virtual memory address space.
#[inline]
pub unsafe fn write_kernel_page_table(root_paddr: PhysAddr) {
    unsafe { write_user_page_table(root_paddr) }
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
    pub fn user_copy(dst: *mut u8, src: *const u8, size: usize) -> usize;
}

/// Performs a hypercall to the hypervisor using the `vmmcall` instruction.
///
/// This is used on AMD/Hygon platforms for KVM hypercalls.
/// For Intel platforms, `vmcall` would be used instead.
///
/// # Arguments
/// * `nr` - Hypercall number (passed in RAX)
/// * `a0` - First argument (passed in RBX)
/// * `a1` - Second argument (passed in RCX)
///
/// # Returns
/// The return value from the hypervisor (from RAX).
#[inline]
pub fn hypercall(nr: u64, a0: u64, a1: u64) -> i64 {
    let ret: i64;
    unsafe {
        // Note: rbx is reserved by LLVM, so we need to save/restore it manually
        core::arch::asm!(
            "push rbx",
            "mov rbx, {a0}",
            "vmmcall",
            "pop rbx",
            a0 = in(reg) a0,
            inout("rax") nr => ret,
            in("rcx") a1,
            options()
        );
    }
    ret
}

