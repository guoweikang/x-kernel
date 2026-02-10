# X-Kernel Kconfig System

## Overview

X-Kernel uses Kconfig for kernel configuration, similar to Linux kernel.
This provides a hierarchical menu-driven interface for selecting features
and platform parameters.

## Quick Start

### 1. Interactive Configuration

Use the TUI menuconfig to configure kernel:

```bash
make menuconfig
# or directly:
xconf menuconfig
```

### 2. Load Default Configuration

```bash
make defconfig
```

### 3. Update Configuration After Changes

```bash
make oldconfig
```

### 4. Build with Configuration

```bash
make build-kbuild
```

## Configuration Sections

### Architecture
- **ARCH_AARCH64**: ARM 64-bit (ARMv8)
- **ARCH_RISCV64**: RISC-V 64-bit
- **ARCH_X86_64**: x86 64-bit
- **ARCH_LOONGARCH64**: LoongArch 64-bit

### Platform Selection
Platform choices depend on selected architecture:
- **QEMU Virt**: Virtual machine for development
- **Raspberry Pi 4**: ARM physical hardware
- **CrosVM**: Chrome OS virtualization

### Platform Parameters
Hardware-specific addresses and sizes:
- CPU count
- Memory layout
- Device addresses
- Interrupt numbers

### Kernel Features
- **SMP**: Multi-core support
- **Preemption model**: None/Voluntary/Full
- **Timer frequency**: HZ value

### Device Drivers
- VirtIO devices (block, network)
- ARM GIC interrupt controller
- UART controllers

### Debugging
- Debug symbols
- Log levels
- Verbose output

## Configuration Files

- `.config`: Generated configuration (do NOT commit)
- `.config.example`: Example default configuration
- `Kconfig`: Main configuration definition
- `platforms/Kconfig`: Platform-specific options

## Migration from platconfig

**Status**: In Progress

The old `platconfig` (TOML-based) system is being migrated to Kconfig.
During migration, both systems coexist using a compatibility layer.

See `docs/MIGRATION.md` for details.
