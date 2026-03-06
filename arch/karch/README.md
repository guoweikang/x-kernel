# karch

Lightweight architecture-specific low-level operations for the x-kernel project.

This crate provides a uniform API across all supported architectures (AArch64, x86_64, RISC-V, LoongArch64) for:

- **TLB flush**: `flush_tlb(vaddr: Option<VirtAddr>)`
- **Cache maintenance** (AArch64): `flush_icache_all()`, `flush_dcache_line(vaddr)`
- **CPU control**: `stop_cpu()`, `await_interrupts()`
- **Local interrupt management**: `enable_irq()`, `disable_irq()`, `irq_enabled()`
- **Thread pointer (TLS)**: `read_thread_pointer()`, `write_thread_pointer(val)`
- **FP/SIMD enable** (AArch64, LoongArch64): `enable_fp()`
- **LSX extension** (LoongArch64): `enable_lsx()`

## Design

`karch` is intentionally kept lightweight: it only depends on `memaddr`, `cfg-if`, and
architecture-specific register libraries (`aarch64-cpu`, `x86`/`x86_64`, `riscv`,
`loongArch64`). It has **no** OS-level dependencies, making it suitable as a low-level
building block for other crates.

## Features

- `arm-el2`: Enable AArch64 EL2 (hypervisor) variants of TLB flush and related operations.
