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
/// Enables or disables the IO APIC line for the given vector.
pub fn enable(vector: usize, enabled: bool) {
    info!("X86_64 enable IRQ: vector={} (0x{:x}), enabled={}", vector, vector, enabled);
    
    if vector < APIC_TIMER_VECTOR as _ {
        const IO_APIC_VECTOR_BASE: usize = 0x20;
        
        if vector >= IO_APIC_VECTOR_BASE && vector < APIC_TIMER_VECTOR as usize {
            let irq_line = vector - IO_APIC_VECTOR_BASE;
            
            unsafe {
                let mut io_apic = IO_APIC.lock();
                
                if irq_line <= io_apic.max_table_entry() as usize {
                    let before = io_apic.table_entry(irq_line as u8);
                    info!("  Before: RTE[{}] vector={}, flags={:?}", 
                          irq_line, before.vector(), before.flags());
                    
                    // ✅ 关键：先检查是否有handler
                    if enabled {
                        // 检查handler是否已注册
                        if !IRQ_HANDLER_TABLE.has_handler(vector) {
                            warn!("  WARNING: Enabling IRQ {} before handler registered!", vector);
                            // 仍然允许enable,但记录警告
                        }
                        io_apic.enable_irq(irq_line as u8);
                    } else {
                        io_apic.disable_irq(irq_line as u8);
                    }
                    
                    let after = io_apic.table_entry(irq_line as u8);
                    info!("  After:  RTE[{}] vector={}, flags={:?}", 
                          irq_line, after.vector(), after.flags());
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

        // ✅ 配置PCI IRQ的触发模式
        for irq in [10, 11] {
            let mut entry = RedirectionTableEntry::default();
            entry.set_vector((0x20 + irq) as u8);
            entry.set_dest(0);
            entry.set_mode(IrqMode::Fixed);
            // Level-triggered, Low-active, Masked
            entry.set_flags(IrqFlags::LEVEL_TRIGGERED | IrqFlags::LOW_ACTIVE | IrqFlags::MASKED);

            io_apic.set_table_entry(irq, entry);

            // 验证
            let verify = io_apic.table_entry(irq);
            info!("  IRQ {}: vector={} (0x{:x}), dest={}, mode={:?}, flags={:?}", 
                  irq, verify.vector(), verify.vector(), verify.dest(), verify.mode(), verify.flags());
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
    const MAX_IRQ_COUNT: usize = 256;
    static IRQ_HANDLER_TABLE: HandlerTable<MAX_IRQ_COUNT> = HandlerTable::new();
    struct IntrManagerImpl;
    #[impl_dev_interface]
    impl IntrManager for IntrManagerImpl {
        fn enable(vector: usize, enabled: bool) {
            super::enable(vector, enabled);
        }

        fn reg_handler(vector: usize, handler: Handler) -> bool {
            if IRQ_HANDLER_TABLE.register_handler(vector, handler) {
                Self::enable(vector, true);
                return true;
            }
            warn!("reg_handler handler for IRQ {} failed", vector);
            false
        }

        fn unreg_handler(vector: usize) -> Option<Handler> {
            Self::enable(vector, false);
            IRQ_HANDLER_TABLE.unregister_handler(vector)
        }

        fn dispatch_irq(vector: usize) -> Option<usize> {
            if vector != 240 {
                info!("X86_64 IRQ vector: {} (0x{:x})", vector, vector);  // ← 添加这行
            }

            trace!("IRQ {}", vector);
            if !IRQ_HANDLER_TABLE.handle(vector) {
                warn!("Unhandled IRQ {vector}");
            }
            unsafe { super::local_apic().end_of_interrupt() };
            Some(vector)
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
