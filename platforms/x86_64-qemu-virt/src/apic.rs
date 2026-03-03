// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Local APIC and IO APIC setup for x86_64-qemu-virt.

use core::mem::MaybeUninit;

use kplat::memory::{PhysAddr, p2v, pa};
use kspin::SpinNoIrq;
use lazyinit::LazyInit;
use x2apic::{
    ioapic::IoApic,
    lapic::{LocalApic, LocalApicBuilder, xapic_base},
    ioapic::IrqFlags,
};
use x86_64::instructions::port::Port;

use self::vectors::*;
/// APIC vector assignments.
pub(super) mod vectors {
    pub const APIC_TIMER_VECTOR: u8 = 0xf0;
    pub const APIC_SPURIOUS_VECTOR: u8 = 0xf1;
    pub const APIC_ERROR_VECTOR: u8 = 0xf2;
}

const IO_APIC_BASE: PhysAddr = pa!(0xFEC0_0000);
static mut LOCAL_APIC: MaybeUninit<LocalApic> = MaybeUninit::uninit();
static mut IS_X2APIC: bool = false;
static IO_APIC: LazyInit<SpinNoIrq<IoApic>> = LazyInit::new();
/// Enables or disables the IO APIC line for the given irq number.


pub fn enable(irq: usize, enabled: bool) {
    info!("X86_64 enable IRQ: {}, enabled={}", irq, enabled);
    // 注意：传进来的是 IRQ，我们计算出 Vector 用于越界保护
    let vector = 0x20 + irq;

    if vector < APIC_TIMER_VECTOR as usize {
        unsafe {
            let mut io_apic = IO_APIC.lock();

            if irq <= io_apic.max_table_entry() as usize {
                // 在 Enable 阶段动态覆写配置（解决 x86 的 Level/Edge 配置差异）
                // 我们统一将所有开启的 IO-APIC 引脚配制为 边沿触发、低电平
                let mut entry = io_apic.table_entry(irq as u8);
                entry.set_flags(IrqFlags::LOW_ACTIVE); // 注意不要带 LEVEL_TRIGGERED

                if enabled {
                    io_apic.set_table_entry(irq as u8, entry); // 写入配置
                    io_apic.enable_irq(irq as u8);             // 解除掩码
                } else {
                    io_apic.disable_irq(irq as u8);            // 加上掩码
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
        use x2apic::ioapic::IrqMode;
        use x2apic::ioapic::RedirectionTableEntry;

        let max_entry = io_apic.max_table_entry();
        info!("  IO-APIC supports {} IRQ inputs (0-{})", max_entry + 1, max_entry);

        // ✅ 为所有IRQ line创建默认RTE (masked状态)
        for irq in 0..=max_entry {
            let mut entry = RedirectionTableEntry::default();
            entry.set_vector((0x20 + irq) as u8);        // CPU vector
            entry.set_dest(0);                           // 发送给CPU 0
            entry.set_mode(IrqMode::Fixed);              // Fixed模式
            entry.set_flags(IrqFlags::MASKED);           // 默认mask
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
    const IO_APIC_VECTOR_BASE: usize = 0x20; // 收敛到底层 外部只看到IRQ编号，不暴露CPU Vector细节

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

        // 外部中断进来的是 CPU Vector，我们需要在这里转换回 IRQ 抛给框架
        fn dispatch_irq(vector: usize) -> Option<usize> {
            let irq = if vector >= APIC_TIMER_VECTOR as usize {
                // 1. Local APIC 内部中断 (Timer=240, Spurious=241, Error=242 等)
                // 它们不经过 IO-APIC，不需要减去偏移，直接把 Vector 当作逻辑 IRQ 传递
                vector
            } else if vector >= IO_APIC_VECTOR_BASE {
                // 2. IO-APIC 外设中断 (如 PCI 网络、块设备)
                // 它们的 Vector 是我们在初始化时通过 0x20 + irq 映射的，需要还原
                vector - IO_APIC_VECTOR_BASE
            } else {
                // 3. 异常 (0-31) 不应该进入外设中断派发器
                return None;
            };

            if irq != APIC_TIMER_VECTOR as usize {
                // 屏蔽高频的 Timer 日志，只打印外设 IRQ
                trace!("IRQ {}", irq);
            }

            if !IRQ_HANDLER_TABLE.handle(irq) {
                // 对于异步 poll 机制，因为走的是 IRQ_HOOK，这里如果打印 warning 会刷屏，所以静默或调低日志级别即可
                warn!("Unhandled IRQ {}", irq);
            }

            unsafe { super::local_apic().end_of_interrupt() };
            Some(irq) // 向框架传递真实的逻辑 IRQ 号
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
