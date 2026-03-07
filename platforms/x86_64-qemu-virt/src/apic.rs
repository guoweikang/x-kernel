// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Local APIC and IO APIC setup for x86_64-qemu-virt.

use core::{
    mem::MaybeUninit,
    sync::atomic::{AtomicU8, Ordering},
};

use kplat::memory::{PhysAddr, p2v, pa};
use kspin::SpinNoIrq;
use lazyinit::LazyInit;
use x2apic::{
    ioapic::{IoApic, IrqFlags},
    lapic::{LocalApic, LocalApicBuilder, xapic_base},
};
use x86_64::instructions::port::Port;

use self::vectors::*;
/// APIC vector assignments.
pub(super) mod vectors {
    pub const APIC_TIMER_VECTOR: u8 = 0xf0;
    pub const APIC_SPURIOUS_VECTOR: u8 = 0xf1;
    pub const APIC_ERROR_VECTOR: u8 = 0xf2;
    /// First CPU vector reserved for MSI-X. Vectors 0x40–0xEF are available
    /// for MSI-X (above the IO-APIC range 0x20–0x3F, below APIC_TIMER_VECTOR).
    pub const MSIX_VECTOR_BASE: u8 = 0x40;
}

const IO_APIC_BASE: PhysAddr = pa!(0xFEC0_0000);
static mut LOCAL_APIC: MaybeUninit<LocalApic> = MaybeUninit::uninit();
static mut IS_X2APIC: bool = false;
static IO_APIC: LazyInit<SpinNoIrq<IoApic>> = LazyInit::new();

/// Counter used to dynamically allocate MSI-X CPU vectors.
/// Starts at MSIX_VECTOR_BASE and increments on each allocation.
static MSIX_VECTOR_COUNTER: AtomicU8 = AtomicU8::new(MSIX_VECTOR_BASE);

/// Allocates the next available MSI-X CPU vector.
///
/// Returns `None` when all vectors in the range
/// `[MSIX_VECTOR_BASE, APIC_TIMER_VECTOR)` are exhausted.
#[unsafe(export_name = "__kplat_alloc_msix_vector")]
pub fn alloc_msix_vector() -> Option<u8> {
    // Use a compare-exchange loop to atomically check-and-increment,
    // avoiding any risk of the counter wrapping past APIC_TIMER_VECTOR when
    // called concurrently (e.g. from multiple CPUs during boot).
    loop {
        let current = MSIX_VECTOR_COUNTER.load(Ordering::Relaxed);
        if current >= APIC_TIMER_VECTOR {
            return None;
        }
        match MSIX_VECTOR_COUNTER.compare_exchange(
            current,
            current + 1,
            Ordering::Relaxed,
            Ordering::Relaxed,
        ) {
            Ok(_) => return Some(current),
            Err(_) => continue,
        }
    }
}

/// Returns the APIC ID of the current logical CPU.
#[unsafe(export_name = "__kplat_current_apic_id")]
pub fn current_apic_id() -> u8 {
    raw_cpuid::CpuId::new()
        .get_feature_info()
        .map_or(0, |f| f.initial_local_apic_id())
}

/// Enables or disables the IO APIC line for the given IRQ number.
///
/// MSI-X vectors (>= MSIX_VECTOR_BASE) bypass the IO-APIC entirely and are
/// delivered directly by the Local APIC, so they are ignored here.
pub fn enable(irq: usize, enabled: bool) {
    // MSI-X vectors are not routed through the IO-APIC.
    if irq >= MSIX_VECTOR_BASE as usize {
        return;
    }

    let vector = 0x20 + irq;

    if vector < APIC_TIMER_VECTOR as usize {
        unsafe {
            let mut io_apic = IO_APIC.lock();

            if irq <= io_apic.max_table_entry() as usize {
                // RTE 已在 init_primary() 中配置好 vector、dest、mode、
                // trigger 等字段，此处只需切换 mask bit 即可。
                if enabled {
                    io_apic.enable_irq(irq as u8);
                } else {
                    io_apic.disable_irq(irq as u8);
                }
            }
        }
    }
}

/// Returns a mutable reference to the local APIC.
#[allow(static_mut_refs)]
pub fn local_apic<'a>() -> &'a mut LocalApic {
    unsafe { LOCAL_APIC.assume_init_mut() }
}
/// Converts an APIC ID into a raw APIC register format.
#[cfg(feature = "smp")]
pub fn raw_apic_id(id_u8: u8) -> u32 {
    if unsafe { IS_X2APIC } {
        id_u8 as u32
    } else {
        (id_u8 as u32) << 24
    }
}
/// Detects whether the CPU supports x2APIC.
fn cpu_has_x2apic() -> bool {
    match raw_cpuid::CpuId::new().get_feature_info() {
        Some(finfo) => finfo.has_x2apic(),
        None => false,
    }
}
/// Initializes local and IO APIC on the boot CPU.
pub fn init_primary() {
    info!("Initialize Local APIC...");
    unsafe {
        Port::<u8>::new(0x21).write(0xff);
        Port::<u8>::new(0xA1).write(0xff);
    }
    let mut builder = LocalApicBuilder::new();
    builder
        .timer_vector(APIC_TIMER_VECTOR as _)
        .error_vector(APIC_ERROR_VECTOR as _)
        .spurious_vector(APIC_SPURIOUS_VECTOR as _);
    if cpu_has_x2apic() {
        info!("Using x2APIC.");
        unsafe { IS_X2APIC = true };
    } else {
        info!("Using xAPIC.");
        let base_vaddr = p2v(pa!(unsafe { xapic_base() } as usize));
        builder.set_xapic_base(base_vaddr.as_usize() as u64);
    }
    let mut lapic = builder.build().unwrap();
    unsafe {
        lapic.enable();
        #[allow(static_mut_refs)]
        LOCAL_APIC.write(lapic);
    }

    let mut io_apic = unsafe { IoApic::new(p2v(IO_APIC_BASE).as_usize() as u64) };

    unsafe {
        use x2apic::ioapic::{IrqMode, RedirectionTableEntry};

        let max_entry = io_apic.max_table_entry();
        info!(
            "  IO-APIC supports {} IRQ inputs (0-{})",
            max_entry + 1,
            max_entry
        );

        // 为所有 IRQ line 创建默认 RTE (masked 状态)。
        // ISA IRQ 0-9 使用 edge-triggered, high-active（PC/AT 惯例）。
        // IRQ 10 及以上可能被 PCI INTx 使用，PCI 规范规定 INTx 为
        // level-triggered, low-active，此处按此默认配置，确保 legacy
        // fallback 路径（设备无 MSI-X 时）能正确工作。
        for irq in 0..=max_entry {
            let mut entry = RedirectionTableEntry::default();
            entry.set_vector((0x20 + irq) as u8);
            entry.set_dest(0);
            entry.set_mode(IrqMode::Fixed);
            if irq >= 10 {
                // PCI INTx: level-triggered, low-active, masked
                entry
                    .set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE | IrqFlags::MASKED);
            } else {
                // ISA: edge-triggered, high-active, masked
                entry.set_flags(IrqFlags::MASKED);
            }
            io_apic.set_table_entry(irq, entry);
        }
        info!("IO-APIC initialized and masked");
    }

    IO_APIC.init_once(SpinNoIrq::new(io_apic));
}
/// Initializes local APIC on a secondary CPU.
#[cfg(feature = "smp")]
pub fn init_secondary() {
    unsafe { local_apic().enable() };
}
mod irq_impl {
    use kplat::interrupts::{Handler, HandlerTable, IntrManager, TargetCpu};

    use super::*;

    const MAX_IRQ_COUNT: usize = 256;
    const IO_APIC_VECTOR_BASE: usize = 0x20;

    static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();
    struct IntrManagerImpl;

    #[impl_dev_interface]
    impl IntrManager for IntrManagerImpl {
        fn enable(irq: usize, enabled: bool) {
            super::enable(irq, enabled);
        }

        fn reg_handler(irq: usize, handler: Handler) -> bool {
            if IRQ_HANDLER_TABLE.register_handler(irq, handler) {
                Self::enable(irq, true);
                return true;
            }
            warn!("reg_handler handler for IRQ {} failed", irq);
            false
        }

        fn unreg_handler(irq: usize) -> Option<Handler> {
            Self::enable(irq, false);
            IRQ_HANDLER_TABLE.unregister_handler(irq)
        }

        // 外部中断进来的是 CPU Vector，转换回 IRQ 号传给框架
        fn dispatch_irq(vector: usize) -> Option<usize> {
            let irq = if vector >= APIC_TIMER_VECTOR as usize {
                // Local APIC 内部中断 (Timer/Spurious/Error)，直接透传
                vector
            } else if vector >= MSIX_VECTOR_BASE as usize {
                // MSI-X vector range: the vector IS the IRQ identifier.
                // MSI-X is edge-triggered, so no masking is needed on dispatch.
                let irq = vector;
                trace!("MSI-X IRQ {}", irq);
                IRQ_HANDLER_TABLE.handle(irq);
                unsafe { super::local_apic().end_of_interrupt() };
                return Some(irq);
            } else if vector >= IO_APIC_VECTOR_BASE {
                // IO-APIC 外设中断，还原为 IRQ 号
                vector - IO_APIC_VECTOR_BASE
            } else {
                return None;
            };

            trace!("IRQ {}", irq);
            if !IRQ_HANDLER_TABLE.handle(irq) {
                // 对于 level-triggered 的 IO-APIC 中断（如 PCI INTx），设备
                // 会持续拉低中断线直到 driver 显式 ack。如果没有注册 handler
                // 消费该中断，必须在 EOI 之前 mask 该 IRQ line，否则 EOI 后
                // 设备中断线仍然 asserted，IO-APIC 会立即重新触发，造成中断风暴。
                //
                // 异步 poll 机制通过 irq_hook 唤醒任务，任务处理完数据后会在
                // register_irq_waker() 中调用 khal::irq::enable(irq, true)
                // 重新使能该 IRQ line。
                super::enable(irq, false);
            }
            unsafe { super::local_apic().end_of_interrupt() };
            Some(irq)
        }

        fn notify_cpu(interrupt_id: usize, target: TargetCpu) {
            match target {
                TargetCpu::Self_ => {
                    unsafe {
                        super::local_apic().send_ipi_self(interrupt_id as _);
                    };
                }
                TargetCpu::Specific(cpu_id) => {
                    unsafe {
                        super::local_apic().send_ipi(interrupt_id as _, cpu_id as _);
                    };
                }
                TargetCpu::AllButSelf { me: _, total: _ } => {
                    use x2apic::lapic::IpiAllShorthand;
                    unsafe {
                        super::local_apic()
                            .send_ipi_all(interrupt_id as _, IpiAllShorthand::AllExcludingSelf);
                    };
                }
            }
        }

        fn set_prio(_irq: usize, _priority: u8) {
            todo!()
        }
    }
}
