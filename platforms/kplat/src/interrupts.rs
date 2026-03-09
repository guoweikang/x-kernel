// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Platform interrupt controller interface.

pub use handler_table::HandlerTable;
use kplat_macros::device_interface;

/// IRQ handler type.
pub type Handler = handler_table::Handler;

/// Target CPU(s) for inter-processor interrupts.
pub enum TargetCpu {
    /// Target the current CPU.
    Self_,
    /// Target a specific CPU by ID.
    Specific(usize),
    /// Target all CPUs except the caller.
    AllButSelf { me: usize, total: usize },
}

#[device_interface]
pub trait IntrManager {
    /// Enables or disables the given interrupt.
    fn enable(id: usize, on: bool);
    /// Registers a handler for the given interrupt.
    fn reg_handler(id: usize, h: Handler) -> bool;
    /// Unregisters the handler for the given interrupt.
    fn unreg_handler(id: usize) -> Option<Handler>;
    /// Dispatches a hardware IRQ and returns a logical IRQ number if any.
    fn dispatch_irq(id: usize) -> Option<usize>;

    /// Sends an IPI or interrupt notification to a target CPU.
    fn notify_cpu(id: usize, target: TargetCpu);
    /// Sets the priority for the given interrupt.
    fn set_prio(id: usize, prio: u8);
}

// Platform-provided MSI-X helpers (x86_64 only).
// The implementations live in the platform crate (e.g. x86_64-qemu-virt)
// and are linked in via the exported symbol names below.
#[cfg(target_arch = "x86_64")]
unsafe extern "Rust" {
    #[link_name = "__kplat_alloc_msix_vector"]
    fn __alloc_msix_vector_impl() -> Option<u8>;
    #[link_name = "__kplat_current_apic_id"]
    fn __current_apic_id_impl() -> u8;
}

/// Allocates the next available MSI-X CPU vector.
///
/// Returns `None` when all vectors are exhausted.
#[cfg(target_arch = "x86_64")]
pub fn alloc_msix_vector() -> Option<u8> {
    unsafe { __alloc_msix_vector_impl() }
}

/// Returns the APIC ID of the current logical CPU.
#[cfg(target_arch = "x86_64")]
pub fn current_apic_id() -> u8 {
    unsafe { __current_apic_id_impl() }
}
