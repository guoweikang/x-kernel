// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Early boot page table initialization for AArch64 PIE boot.

use kplat::memory::{PageAligned, pa};
use page_table::{PageTableEntry, PagingFlags, aarch64::A64PageEntry as A64PTE};

#[unsafe(link_section = ".data")]
pub static mut BOOT_PT_L0: PageAligned<[A64PTE; 512]> = PageAligned::new([A64PTE::empty(); 512]);

#[unsafe(link_section = ".data")]
static mut BOOT_PT_L1: PageAligned<[A64PTE; 512]> = PageAligned::new([A64PTE::empty(); 512]);

/// Initialize the early boot page table.
///
/// Sets up a minimal identity map covering the first 2 GiB of physical memory,
/// which is sufficient to run early boot code before the full kernel page table
/// is established.
///
/// # Safety
///
/// Must be called before MMU is enabled. Not safe to call more than once.
pub unsafe fn init_boot_page_table() {
    unsafe {
        // L0[0] -> L1 table (covers 0..512 GiB in user space via TTBR0)
        BOOT_PT_L0[0] = A64PTE::new_table(pa!(&raw const BOOT_PT_L1 as usize));

        // Identity map 0x0000_0000..0x4000_0000 (1 GiB, device memory)
        BOOT_PT_L1[0] = A64PTE::new_page(
            pa!(0),
            PagingFlags::READ | PagingFlags::WRITE | PagingFlags::DEVICE,
            true,
        );
        // Identity map 0x4000_0000..0x8000_0000 (1 GiB, normal memory — where kernel loads)
        BOOT_PT_L1[1] = A64PTE::new_page(
            pa!(0x4000_0000),
            PagingFlags::READ | PagingFlags::WRITE | PagingFlags::EXECUTE,
            true,
        );
    }
}
