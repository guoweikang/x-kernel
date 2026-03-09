// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! ELF RELA relocation support for position-independent AArch64 boot.
//!
//! When the kernel is loaded at an address different from its linked address,
//! absolute references (R_AARCH64_RELATIVE relocations) must be patched before
//! any Rust code that uses static variables or function pointers can run.

/// ELF64 RELA relocation entry.
#[repr(C)]
struct Elf64Rela {
    /// Offset (relative to load address) of the location to patch.
    r_offset: u64,
    /// Relocation type and symbol index.
    r_info: u64,
    /// Addend: `*target = load_offset + r_addend`.
    r_addend: i64,
}

/// AArch64 R_AARCH64_RELATIVE relocation type.
const R_AARCH64_RELATIVE: u32 = 1027;

/// Returns the actual (runtime) physical load address of `_start`.
///
/// Uses `ADRP` which is PC-relative and therefore works correctly regardless
/// of where the image is loaded in physical memory.
pub fn get_actual_load_addr() -> usize {
    let actual_addr: usize;
    unsafe {
        core::arch::asm!(
            "adrp {0}, _start",
            "add  {0}, {0}, :lo12:_start",
            out(reg) actual_addr,
            options(pure, nomem, nostack),
        );
    }
    actual_addr
}

/// Apply all `R_AARCH64_RELATIVE` relocations from the `.rela.dyn` section.
///
/// This patches every absolute pointer in the kernel image so that Rust
/// code can use statics, vtables, and function pointers safely at runtime.
///
/// # Safety
///
/// - Must be called before any Rust code accesses statics.
/// - Must be called with the MMU disabled (physical addresses).
/// - Must be called exactly once during early boot.
pub unsafe fn apply_relocations() {
    unsafe extern "C" {
        fn _start();
        fn __rela_dyn_start();
        fn __rela_dyn_end();
    }

    let linked_addr = _start as usize;
    let actual_addr = get_actual_load_addr();
    let load_offset = (actual_addr as isize) - (linked_addr as isize);

    // No relocation needed if loaded at the linked address.
    if load_offset == 0 {
        return;
    }

    let start = __rela_dyn_start as *mut Elf64Rela;
    let end = __rela_dyn_end as *const Elf64Rela;
    let count = (end as usize - start as usize) / core::mem::size_of::<Elf64Rela>();

    let relocs = unsafe { core::slice::from_raw_parts_mut(start, count) };

    for reloc in relocs {
        let ty = (reloc.r_info & 0xFFFF_FFFF) as u32;
        if ty == R_AARCH64_RELATIVE {
            // target_phys = r_offset + load_offset
            // *target_phys = r_addend + load_offset
            let target = ((reloc.r_offset as isize) + load_offset) as *mut usize;
            unsafe {
                *target = ((reloc.r_addend as isize) + load_offset) as usize;
            }
        }
    }

    // Ensure all stores are visible before subsequent instruction fetches.
    unsafe {
        core::arch::asm!("dsb sy", "isb", options(nostack));
    }
}
