// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Unified position-independent boot layer for x-kernel (AArch64 support).

#![cfg_attr(target_os = "none", no_std)]
#![cfg(target_os = "none")]

pub use linkme::{
    distributed_slice as def_boot_init, distributed_slice as register_boot_init,
};

#[def_boot_init]
pub static INIT_TRAP: [fn()];

#[def_boot_init]
pub static PRIMARY_KERNEL_ENTRY: [fn(usize, usize) -> !];

#[def_boot_init]
pub static SECOND_KERNEL_ENTRY: [fn(usize) -> !];

macro_rules! call_init_trap {
    () => {{
        let mut iter = $crate::INIT_TRAP.iter();
        if let Some(func) = iter.next() {
            super::serial::boot_print_usize(*func as *const() as usize);
            func();
        }
    }}
}

macro_rules! call_kernel_entry {
    ($entry:ident, $($args:tt)*) => {{
        let mut iter = $crate::$entry.iter();
        if let Some(func) = iter.next() {
            func($($args)*)
        }
    }}
}

pub mod bootinfo;
pub mod size_const;
pub mod arch;


