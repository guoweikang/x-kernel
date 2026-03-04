// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! AArch64 position-independent boot entry.

use core::arch::naked_asm;

use kbuild_config::{BOOT_STACK_SIZE, PHYS_VIRT_OFFSET};

#[unsafe(link_section = ".bss.stack")]
static mut BOOT_STACK: [u8; BOOT_STACK_SIZE] = [0; BOOT_STACK_SIZE];

/// Linux ARM64 Image format boot entry.
///
/// Implements the ARM64 Linux boot protocol header so this image can be loaded
/// by UEFI firmware, U-Boot, QEMU, and other standard ARM64 bootloaders.
#[unsafe(naked)]
#[unsafe(no_mangle)]
#[unsafe(link_section = ".text.boot")]
pub unsafe extern "C" fn _start() -> ! {
    const FLAG_LE: usize = 0b0;
    const FLAG_PAGE_SIZE_4K: usize = 0b10;
    const FLAG_ANY_MEM: usize = 0b1000;
    naked_asm!(
        // Linux ARM64 Boot Protocol Header
        "add     x13, x18, #0x16",       // 'MZ' magic
        "b       {entry}",               // branch to primary entry
        ".quad   0",                     // image load offset from start of RAM
        ".quad   _ekernel - _start",     // effective size of kernel image
        ".quad   {flags}",               // kernel flags (LE + 4K pages + any mem)
        ".quad   0",                     // reserved
        ".quad   0",                     // reserved
        ".quad   0",                     // reserved
        ".ascii  \"ARM\\x64\"",          // magic number
        ".long   0",                     // reserved (PE COFF offset)
        flags = const FLAG_LE | FLAG_PAGE_SIZE_4K | FLAG_ANY_MEM,
        entry = sym _start_primary,
    )
}

#[unsafe(naked)]
unsafe extern "C" fn _start_primary() -> ! {
    naked_asm!(
        // 1. Save boot parameters
        "mrs     x19, mpidr_el1",
        "and     x19, x19, #0xffffff",  // CPU ID from MPIDR
        "mov     x20, x0",              // save DTB pointer

        // 2. Set up physical boot stack (using ADRP for PIE)
        "adrp    x8, {boot_stack}",
        "add     x8, x8, :lo12:{boot_stack}",
        "add     x8, x8, {stack_size}",
        "mov     sp, x8",

        // 3. Apply ELF relocations (must be done before any abs-address access)
        "bl      {apply_relocations}",

        // 4. Switch to EL1
        "bl      {switch_to_el1}",

        // 5. Enable FP/SIMD
        "bl      {enable_fp}",

        // 6. Initialize early boot page table
        "bl      {init_boot_page_table}",

        // 7. Enable MMU (pass L0 page table physical address)
        "adrp    x0, {boot_pt}",
        "add     x0, x0, :lo12:{boot_pt}",
        "bl      {init_mmu}",

        // 8. Switch stack to high virtual address
        "mov     x8, {phys_virt_offset}",
        "add     sp, sp, x8",

        // 9. Construct BootInfo (dtb=x20, cpu_id=x19)
        "mov     x0, x20",              // dtb
        "mov     x1, x19",              // cpu_id
        "bl      {construct_boot_info}",

        // 10. x0 = &BootInfo; call into kernel entry
        "ldr     x8, ={kbootloader_entry}",
        "blr     x8",
        "b       .",

        boot_stack = sym BOOT_STACK,
        stack_size = const BOOT_STACK_SIZE,
        apply_relocations = sym super::relocate::apply_relocations,
        switch_to_el1 = sym kcpu::boot::switch_to_el1,
        enable_fp = sym enable_fp,
        init_boot_page_table = sym super::mmu::init_boot_page_table,
        boot_pt = sym super::mmu::BOOT_PT_L0,
        init_mmu = sym kcpu::boot::init_mmu,
        phys_virt_offset = const PHYS_VIRT_OFFSET,
        construct_boot_info = sym super::construct_boot_info,
        kbootloader_entry = sym super::kbootloader_entry,
    )
}

unsafe fn enable_fp() {
    #[cfg(feature = "fp-simd")]
    kcpu::instrs::enable_fp();
}
