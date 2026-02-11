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

    // Default to HVC if neither is explicitly set
    #[cfg(not(any(PLATFORM_PSCI_HVC, PLATFORM_PSCI_SMC)))]
    pub const PSCI_METHOD: &str = "hvc";

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

    // MMIO region sizes (private constants)
    const UART_SIZE: usize = 0x1000; // 4KB
    const GIC_SIZE: usize = 0x10000; // 64KB
    const RTC_SIZE: usize = 0x1000;  // 4KB
    const VIRTIO_BASE: usize = 0x0a00_0000;
    const VIRTIO_SIZE: usize = 0x4000;
    const PCI_MEM_BASE: usize = 0x1000_0000;
    const PCI_MEM_SIZE: usize = 0x2eff_0000;
    const PCI_CFG_BASE: usize = 0x40_1000_0000;
    const PCI_CFG_SIZE: usize = 0x1000_0000;

    // MMIO ranges for device mapping (RTC enabled)
    #[cfg(RTC)]
    pub const MMIO_RANGES: [(usize, usize); 7] = [
        (UART_PADDR, UART_SIZE),
        (GICD_PADDR, GIC_SIZE),
        (GICC_PADDR, GIC_SIZE),
        (RTC_PADDR, RTC_SIZE),
        (VIRTIO_BASE, VIRTIO_SIZE),
        (PCI_MEM_BASE, PCI_MEM_SIZE),
        (PCI_CFG_BASE, PCI_CFG_SIZE),
    ];

    // MMIO ranges for device mapping (RTC disabled)
    #[cfg(not(RTC))]
    pub const MMIO_RANGES: [(usize, usize); 6] = [
        (UART_PADDR, UART_SIZE),
        (GICD_PADDR, GIC_SIZE),
        (GICC_PADDR, GIC_SIZE),
        (VIRTIO_BASE, VIRTIO_SIZE),
        (PCI_MEM_BASE, PCI_MEM_SIZE),
        (PCI_CFG_BASE, PCI_CFG_SIZE),
    ];
}
