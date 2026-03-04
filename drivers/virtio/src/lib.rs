// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Wrappers of some devices in the [`virtio-drivers`][1] crate, that implement
//! traits in the [`driver_base`][2] series crates.
//!
//! Like the [`virtio-drivers`][1] crate, you must implement the [`VirtIoHal`]
//! trait (alias of [`virtio-drivers::Hal`][3]), to allocate DMA regions and
//! translate between physical addresses (as seen by devices) and virtual
//! addresses (as seen by your program).
//!
//! [1]: https://docs.rs/virtio-drivers/latest/virtio_drivers/
//! [2]: https://docs.rs/virtio-drivers/latest/virtio_drivers/trait.Hal.html

#![no_std]
#![cfg_attr(doc, feature(doc_cfg))]

#[cfg(feature = "alloc")]
extern crate alloc;
#[cfg(feature = "net")]
extern crate net as driver_net;

#[cfg(feature = "block")]
mod blk;
#[cfg(feature = "block")]
pub use self::blk::VirtIoBlkDev;

#[cfg(feature = "gpu")]
mod gpu;
#[cfg(feature = "gpu")]
pub use self::gpu::VirtIoGpuDev;

#[cfg(feature = "input")]
mod input;
#[cfg(feature = "input")]
pub use self::input::VirtIoInputDev;

#[cfg(feature = "net")]
mod net;
#[cfg(feature = "net")]
pub use self::net::VirtIoNetDev;

#[cfg(unittest)]
pub mod mock_virtio;
#[cfg(feature = "socket")]
mod socket;
use driver_base::{DeviceKind, DriverError};
use virtio_drivers::transport::DeviceType as VirtIoDevType;
pub use virtio_drivers::{
    BufferDirection, Hal as VirtIoHal, PhysAddr,
    transport::{
        Transport,
        mmio::MmioTransport,
        pci::{PciTransport, bus as pci},
    },
};

use self::pci::{DeviceFunction, DeviceFunctionInfo, PciRoot};
#[cfg(feature = "socket")]
pub use self::socket::VirtIoSocketDev;

/// Try to probe a VirtIO MMIO device from the given memory region.
///
/// If the device is recognized, returns the device type and a transport object
/// for later operations. Otherwise, returns [`None`].
pub fn probe_mmio_device(
    reg_base: *mut u8,
    _reg_size: usize,
) -> Option<(DeviceKind, MmioTransport)> {
    use core::ptr::NonNull;

    use virtio_drivers::transport::mmio::VirtIOHeader;

    let header = NonNull::new(reg_base as *mut VirtIOHeader).unwrap();
    let transport = unsafe { MmioTransport::new(header) }.ok()?;
    let dev_kind = as_device_kind(transport.device_type())?;
    Some((dev_kind, transport))
}

/// Try to probe a VirtIO PCI device from the given PCI address.
///
/// If the device is recognized, returns the device type and a transport object
/// for later operations. Otherwise, returns [`None`].
pub fn probe_pci_device<H: VirtIoHal>(
    root: &mut PciRoot,
    bdf: DeviceFunction,
    dev_info: &DeviceFunctionInfo,
) -> Option<(DeviceKind, PciTransport, usize)> {
    use virtio_drivers::transport::pci::virtio_device_type;

    let dev_kind = virtio_device_type(dev_info).and_then(as_device_kind)?;
    let transport = PciTransport::new::<H>(root, bdf).ok()?;

    #[cfg(target_arch = "x86_64")]
    let irq = {
        // 读取 PCI 配置空间偏移 0x3C 处的值 (Interrupt Line)
        // 注意：根据你的 pci crate 实现，如果原生支持 read8，可以直接调用
        // 这里假设底层提供 32 位读取能力，0x3C 的最低 8 位即为 Interrupt Line
        // let config_data = root.read(bdf, 0x3C);
        // let irq_line = (config_data & 0xFF) as usize;
        // irq_line // 直接返回 IRQ (例如 10 或是 11)，不要在这里加 CPU Vector 偏移！

        // 由于 virtio-drivers 封装了 PciRoot 的配置空间读取能力，
        // 这里暂时硬编码 QEMU i440fx 主板的默认 PCI IRQ 路由 (PIRQ 轮转)。
        // 注意：这里只返回纯 IRQ 号 (10, 11)，绝对不要加上 CPU Vector 的偏移！
        match bdf.device {
                0..=3 => 11,
                4..=7 => 10,
            _ => 11,
        }
    };

    #[cfg(not(target_arch = "x86_64"))]
    let irq = {
        #[cfg(target_arch = "loongarch64")]
        const PCI_IRQ_BASE: usize = 0x10;
        #[cfg(target_arch = "aarch64")]
        const PCI_IRQ_BASE: usize = 0x23;
        #[cfg(target_arch = "riscv64")]
        const PCI_IRQ_BASE: usize = 0x20;
        PCI_IRQ_BASE + (bdf.device & 3) as usize
    };

    log::info!("PCI virtio device at {:?}: IRQ = {}", bdf, irq);
    Some((dev_kind, transport, irq))
}

const fn as_device_kind(t: VirtIoDevType) -> Option<DeviceKind> {
    use VirtIoDevType::*;
    match t {
        Block => Some(DeviceKind::Block),
        Network => Some(DeviceKind::Net),
        GPU => Some(DeviceKind::Display),
        Input => Some(DeviceKind::Input),
        Socket => Some(DeviceKind::Vsock),
        _ => None,
    }
}

#[allow(dead_code)]
pub(crate) const fn as_driver_error(e: virtio_drivers::Error) -> DriverError {
    use virtio_drivers::{Error::*, device::socket::SocketError::*};
    match e {
        QueueFull => DriverError::BadState,
        NotReady => DriverError::WouldBlock,
        WrongToken => DriverError::BadState,
        AlreadyUsed => DriverError::AlreadyExists,
        InvalidParam => DriverError::InvalidInput,
        DmaError => DriverError::NoMemory,
        IoError => DriverError::Io,
        Unsupported => DriverError::Unsupported,
        ConfigSpaceTooSmall => DriverError::BadState,
        ConfigSpaceMissing => DriverError::BadState,
        SocketDeviceError(e) => match e {
            ConnectionExists => DriverError::AlreadyExists,
            NotConnected => DriverError::BadState,
            InvalidOperation | InvalidNumber | UnknownOperation(_) => DriverError::InvalidInput,
            OutputBufferTooShort(_) | BufferTooShort | BufferTooLong(..) => {
                DriverError::InvalidInput
            }
            UnexpectedDataInPacket | PeerSocketShutdown | NoResponseReceived | ConnectionFailed => {
                DriverError::Io
            }
            InsufficientBufferSpaceInPeer => DriverError::WouldBlock,
            RecycledWrongBuffer => DriverError::BadState,
        },
    }
}
