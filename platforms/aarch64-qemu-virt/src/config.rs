// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Platform configuration module (generated from Kconfig).
//!
//! This module maps Kconfig-generated configuration constants to platform-specific
//! namespaces expected by the platform code.

// Import all Kconfig-generated constants
use kbuild_config::*;

/// Platform-level configuration constants
pub mod plat {
    use super::*;
    pub const BOOT_STACK_SIZE: usize = PLATFORM_BOOT_STACK_SIZE as usize;
    pub const PHYS_VIRT_OFFSET: usize = PLATFORM_PHYS_VIRT_OFFSET as usize;

    // PSCI method (conditional compilation based on Kconfig)
    #[cfg(PLATFORM_PSCI_HVC)]
    pub const PSCI_METHOD: &str = "hvc";

    #[cfg(PLATFORM_PSCI_SMC)]
    pub const PSCI_METHOD: &str = "smc";

    // Memory layout
    pub const PHYS_MEMORY_BASE: usize = PLATFORM_PHYS_MEM_BASE as usize;
    pub const PHYS_MEMORY_SIZE: usize = PLATFORM_PHYS_MEM_SIZE as usize;
    pub const KERNEL_BASE_PADDR: usize = PLATFORM_KERNEL_BASE_PADDR as usize;
    pub const KERNEL_BASE_VADDR: usize = PLATFORM_KERNEL_BASE_VADDR as usize;
    pub const KERNEL_ASPACE_BASE: usize = PLATFORM_KERNEL_ASPACE_BASE as usize;
    pub const KERNEL_ASPACE_SIZE: usize = PLATFORM_KERNEL_ASPACE_SIZE as usize;
    pub const PHYS_BUS_OFFSET: usize = PLATFORM_PHYS_BUS_OFFSET as usize;

    // DMA memory
    pub const DMA_MEM_BASE: usize = PLATFORM_DMA_MEM_BASE as usize;
    pub const DMA_MEM_SIZE: usize = PLATFORM_DMA_MEM_SIZE as usize;

}

/// Device-related configuration constants
pub mod devices {
    use super::*;

    // GIC (Generic Interrupt Controller)
    pub const GICD_PADDR: usize = PLATFORM_GICD_PADDR as usize;
    pub const GICC_PADDR: usize = PLATFORM_GICC_PADDR as usize;

    // RTC (Real-Time Clock) - conditional compilation
    #[cfg(RTC)]
    pub const RTC_PADDR: usize = PLATFORM_RTC_PADDR as usize;

    // Interrupts
    pub const TIMER_IRQ: usize = PLATFORM_TIMER_IRQ as usize;
    pub const IPI_IRQ: usize = PLATFORM_IPI_IRQ as usize;
    pub const PMU_IRQ: usize = PLATFORM_PMU_IRQ as usize;

    // UART
    pub const UART_PADDR: usize = PLATFORM_UART_PADDR as usize;
    pub const UART_IRQ: usize = PLATFORM_UART_IRQ as usize;

    // MMIO region sizes
    const UART_SIZE: usize = 0x1000; // 4KB
    const GIC_SIZE: usize = 0x10000; // 64KB
    const RTC_SIZE: usize = 0x1000;  // 4KB
    // MMIO ranges for device mapping
    // Note: Array size is fixed, using (0, 0) as placeholder when RTC is disabled
    pub const MMIO_RANGES: [(usize, usize); 7] = [
        // UART
        (UART_PADDR, UART_SIZE),
        // GIC Distributor
        (GICD_PADDR, GIC_SIZE),
        // GIC CPU Interface
        (GICC_PADDR, GIC_SIZE),
        // RTC (if enabled, otherwise placeholder)
        #[cfg(RTC)]
        (RTC_PADDR, RTC_SIZE),
        #[cfg(not(RTC))]
        (0, 0), // Placeholder when RTC is disabled
        (0x0a00_0000, 0x4000),      // VirtIO
        (0x1000_0000, 0x2eff_0000),     // PCI memory ranges (ranges 1: 32-bit MMIO space)
        (0x40_1000_0000, 0x1000_0000), // PCI config space
    ];
}
