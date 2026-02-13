// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Platform configuration module (generated from Kconfig).
//!
//! This module maps Kconfig-generated configuration constants to platform-specific
//! namespaces expected by the platform code.

/// Platform-level configuration constants
pub mod plat {
    pub const BOOT_STACK_SIZE: usize = 0x40000;
    pub const DMA_MEM_BASE: usize = 0x4000_0000;
    pub const DMA_MEM_SIZE: usize = 0x200_000;
    pub const KERNEL_ASPACE_BASE: usize = kbuild_config::KERNEL_ASPACE_BASE as _;
    pub const KERNEL_ASPACE_SIZE: usize = kbuild_config::KERNEL_ASPACE_SIZE as _;
    pub const KERNEL_BASE_PADDR: usize = kbuild_config::KERNEL_BASE_PADDR as _;
    pub const PHYS_MEMORY_BASE: usize = 0x4000_0000;
    pub const PHYS_MEMORY_SIZE: usize = 0x4000_0000;
    pub const PHYS_VIRT_OFFSET: usize = 0xffff_0000_0000_0000;
    pub const PSCI_METHOD: &str = "hvc";
}

/// Device-related configuration constants
pub mod devices {
    pub const GICC_PADDR: usize = 0x0801_0000;
    pub const GICD_PADDR: usize = 0x0800_0000;
    pub const IPI_IRQ: usize = 1;

    pub const MMIO_RANGES: &[(usize, usize)] = &[
        (0x0900_0000, 0x1000),
        (0x0910_0000, 0x1000),
        (0x0800_0000, 0x2_0000),
        (0x0a00_0000, 0x4000),
        (0x1000_0000, 0x2eff_0000),
        (0x40_1000_0000, 0x1000_0000),
    ];

    pub const PMU_IRQ: usize = 23;
    pub const RTC_PADDR: usize = 0x901_0000;
    pub const TIMER_IRQ: usize = 30;
    pub const UART_IRQ: usize = 33;
    pub const UART_PADDR: usize = 0x0900_0000;
    pub const VIRTIO_MMIO_RANGES: &[(usize, usize)] = &[
        (0x0a00_0000, 0x200),
        (0x0a00_0200, 0x200),
        (0x0a00_0400, 0x200),
        (0x0a00_0600, 0x200),
        (0x0a00_0800, 0x200),
        (0x0a00_0a00, 0x200),
        (0x0a00_0c00, 0x200),
        (0x0a00_0e00, 0x200),
        (0x0a00_1000, 0x200),
        (0x0a00_1200, 0x200),
        (0x0a00_1400, 0x200),
        (0x0a00_1600, 0x200),
        (0x0a00_1800, 0x200),
        (0x0a00_1a00, 0x200),
        (0x0a00_1c00, 0x200),
        (0x0a00_1e00, 0x200),
        (0x0a00_3000, 0x200),
        (0x0a00_2200, 0x200),
        (0x0a00_2400, 0x200),
        (0x0a00_2600, 0x200),
        (0x0a00_2800, 0x200),
        (0x0a00_2a00, 0x200),
        (0x0a00_2c00, 0x200),
        (0x0a00_2e00, 0x200),
        (0x0a00_3000, 0x200),
        (0x0a00_3200, 0x200),
        (0x0a00_3400, 0x200),
        (0x0a00_3600, 0x200),
        (0x0a00_3800, 0x200),
        (0x0a00_3a00, 0x200),
        (0x0a00_3c00, 0x200),
        (0x0a00_3e00, 0x200),
    ];
}
