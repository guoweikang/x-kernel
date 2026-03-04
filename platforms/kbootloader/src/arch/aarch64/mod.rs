// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! AArch64 position-independent boot implementation.

mod entry;
mod mmu;
mod relocate;

pub use entry::_start;

use crate::bootinfo::{BootInfo, BootProtocol};
use kbuild_config::PHYS_VIRT_OFFSET;

/// Static boot info populated before entering kernel.
static mut BOOT_INFO: BootInfo = BootInfo::new(BootProtocol::DeviceTree);

/// Constructs the standardized BootInfo and stores it in a static.
///
/// # Safety
///
/// Must be called exactly once during early boot before MMU switch.
pub(crate) unsafe fn construct_boot_info(dtb: usize, cpu_id: usize) -> &'static BootInfo {
    let actual_addr = relocate::get_actual_load_addr();

    unsafe {
        BOOT_INFO.kernel_load_paddr = actual_addr;
        BOOT_INFO.phys_virt_offset = PHYS_VIRT_OFFSET;
        BOOT_INFO.dtb_addr = dtb;
        BOOT_INFO.cpu_id = cpu_id;
    }

    unsafe { &*(&raw const BOOT_INFO) }
}

/// Entry point wrapper: extracts cpu_id and dtb from BootInfo and calls kplat::entry.
///
/// # Safety
///
/// Must be called with a valid `BootInfo` pointer after MMU is enabled.
pub(crate) unsafe extern "C" fn kbootloader_entry(boot_info: &'static BootInfo) -> ! {
    kplat::entry(boot_info.cpu_id, boot_info.dtb_addr)
}
