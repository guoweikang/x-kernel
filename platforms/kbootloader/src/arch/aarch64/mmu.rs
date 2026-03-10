// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Early boot page table setup and MMU initialisation for AArch64.
//!
//! All code in this module that runs before the MMU is enabled lives in
//! `.idmap.text` and uses only PC-relative addressing to obtain physical
//! addresses of data.

use aarch64_cpu::{asm::barrier, registers::*};
use memaddr::PhysAddr;
use page_table::{
    PageTableEntry,
    PagingFlags,
    aarch64::{A64PageEntry, Arm64MemAttr},
};

/// A page-aligned wrapper used to place page table arrays in the correct
/// linker section with the required 4 KiB alignment.
#[repr(C, align(4096))]
struct PageAligned<T>(T);

impl<T: Copy> PageAligned<T> {
    const fn new(val: T) -> Self {
        Self(val)
    }
}

impl<T, const N: usize> core::ops::Index<usize> for PageAligned<[T; N]> {
    type Output = T;
    fn index(&self, idx: usize) -> &T {
        &self.0[idx]
    }
}

impl<T, const N: usize> core::ops::IndexMut<usize> for PageAligned<[T; N]> {
    fn index_mut(&mut self, idx: usize) -> &mut T {
        &mut self.0[idx]
    }
}

/// Level-0 boot page table (shared between TTBR0 and TTBR1).
#[unsafe(link_section = ".data.boot_page_table")]
static mut BOOT_PT_L0: PageAligned<[A64PageEntry; 512]> =
    PageAligned::new([A64PageEntry::empty(); 512]);

/// Level-1 page table for the identity map / kernel map.
///
/// Because TTBR0 and TTBR1 share the same L0 table, a single L1 table
/// covers both the low (identity) and high (virtual kernel) windows.
#[unsafe(link_section = ".data.boot_page_table")]
static mut BOOT_PT_L1: PageAligned<[A64PageEntry; 512]> =
    PageAligned::new([A64PageEntry::empty(); 512]);

/// Build the minimal boot page tables required to switch the MMU on.
///
/// The two tables form a two-level walk:
/// ```text
/// TTBR0/TTBR1 → BOOT_PT_L0[0] → BOOT_PT_L1
///                  L1[0] → 0x0000_0000  (1 GiB, Device)
///                  L1[1] → 0x4000_0000  (1 GiB, Normal RWX)
/// ```
///
/// The same L0 table is used for both TTBR0 (low addresses) and TTBR1
/// (high addresses) because the kernel virtual base has the same L0/L1
/// indices as its physical address when the top 16 bits are masked off.
///
/// # Safety
///
/// Must be called before the MMU is enabled.  All memory accesses use
/// physical addresses obtained via `adrp`/`add` (PC-relative).
#[unsafe(link_section = ".idmap.text")]
pub unsafe fn create_boot_page_tables() {
    // Obtain physical addresses of the static arrays via PC-relative
    // addressing – mandatory before the MMU is on.
    let l0_pa: usize;
    let l1_pa: usize;

    unsafe {
        core::arch::asm!(
            "adrp {out}, {sym}",
            "add  {out}, {out}, :lo12:{sym}",
            sym = sym BOOT_PT_L0,
            out = out(reg) l0_pa,
            options(pure, nomem, nostack),
        );
        core::arch::asm!(
            "adrp {out}, {sym}",
            "add  {out}, {out}, :lo12:{sym}",
            sym = sym BOOT_PT_L1,
            out = out(reg) l1_pa,
            options(pure, nomem, nostack),
        );
    }

    // Safety: raw pointer writes to physical addresses obtained above.
    let l0 = unsafe { &mut *(l0_pa as *mut [A64PageEntry; 512]) };
    let l1 = unsafe { &mut *(l1_pa as *mut [A64PageEntry; 512]) };

    // L0[0] → L1 table
    l0[0] = A64PageEntry::new_table(PhysAddr::from(l1_pa));

    // L1[0]: identity-map the first 1 GiB as Device memory.
    // Device memory needs both read and write access so that early boot code
    // can interact with UART and other MMIO peripherals in this range.
    l1[0] = A64PageEntry::new_page(
        PhysAddr::from(0usize),
        PagingFlags::READ | PagingFlags::WRITE | PagingFlags::DEVICE,
        true, // 1 GiB block
    );

    // L1[1]: map the second 1 GiB (0x4000_0000 … 0x8000_0000) as Normal
    // memory with full permissions.  The kernel image lives here.
    l1[1] = A64PageEntry::new_page(
        PhysAddr::from(0x4000_0000usize),
        PagingFlags::READ | PagingFlags::WRITE | PagingFlags::EXECUTE,
        true, // 1 GiB block
    );

    // Ensure all page table writes are visible before enabling the MMU.
    barrier::dsb(barrier::SY);
}

/// Configure MMU registers and enable the MMU.
///
/// Sets `MAIR_EL1`, `TCR_EL1`, `TTBR0_EL1`, `TTBR1_EL1` and then turns
/// the MMU on via `SCTLR_EL1`.
///
/// # Safety
///
/// Must be called after [`create_boot_page_tables`] and before any code
/// that relies on virtual addresses.
#[unsafe(link_section = ".idmap.text")]
pub unsafe fn init_mmu() {
    // Obtain physical address of L0 page table via PC-relative addressing.
    let root_pa: usize;
    unsafe {
        core::arch::asm!(
            "adrp {out}, {sym}",
            "add  {out}, {out}, :lo12:{sym}",
            sym = sym BOOT_PT_L0,
            out = out(reg) root_pa,
            options(pure, nomem, nostack),
        );
    }

    // Program memory attributes.
    MAIR_EL1.set(Arm64MemAttr::MAIR_VALUE);

    // Configure TCR_EL1: 4 KiB granule, 48-bit VA, 48-bit PA, inner-shareable
    // write-back cacheable walks for both TTBR0 (T0SZ=16) and TTBR1 (T1SZ=16).
    let tcr_flags0 = TCR_EL1::EPD0::EnableTTBR0Walks
        + TCR_EL1::TG0::KiB_4
        + TCR_EL1::SH0::Inner
        + TCR_EL1::ORGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::IRGN0::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::T0SZ.val(16);
    let tcr_flags1 = TCR_EL1::EPD1::EnableTTBR1Walks
        + TCR_EL1::TG1::KiB_4
        + TCR_EL1::SH1::Inner
        + TCR_EL1::ORGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::IRGN1::WriteBack_ReadAlloc_WriteAlloc_Cacheable
        + TCR_EL1::T1SZ.val(16);
    TCR_EL1.write(TCR_EL1::IPS::Bits_48 + tcr_flags0 + tcr_flags1);
    barrier::isb(barrier::SY);

    // Point both TTBR0 and TTBR1 at the same L0 table so that low (identity)
    // and high (kernel) virtual addresses are both accessible right after the
    // MMU is enabled.
    let root_pa_u64 = root_pa as u64;
    TTBR0_EL1.set(root_pa_u64);
    TTBR1_EL1.set(root_pa_u64);

    // Flush the entire TLB before enabling the MMU.
    karch::flush_tlb(None);

    // Enable the MMU and turn on I-cache and D-cache.
    SCTLR_EL1.modify(SCTLR_EL1::M::Enable + SCTLR_EL1::C::Cacheable + SCTLR_EL1::I::Cacheable);
    // Disable SPAN
    SCTLR_EL1.set(SCTLR_EL1.get() | (1 << 23));
    barrier::isb(barrier::SY);
}
