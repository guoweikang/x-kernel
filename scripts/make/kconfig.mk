# Extract ARCH and PLAT from .config if it exists

ifneq ($(wildcard .config),)
  # Extract ARCH based on ARCH_* in .config (checking both with and without CONFIG_ prefix)
  ifeq ($(shell grep -q "^\(CONFIG_\)\?ARCH_AARCH64=y" .config && echo y),y)
    ARCH_FROM_CONFIG := aarch64
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?ARCH_RISCV64=y" .config && echo y),y)
    ARCH_FROM_CONFIG := riscv64
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?ARCH_X86_64=y" .config && echo y),y)
    ARCH_FROM_CONFIG := x86_64
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?ARCH_LOONGARCH64=y" .config && echo y),y)
    ARCH_FROM_CONFIG := loongarch64
  endif
  
  # Extract PLAT based on PLATFORM_* in .config (checking both with and without CONFIG_ prefix)
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_AARCH64_QEMU_VIRT=y" .config && echo y),y)
    PLAT_FROM_CONFIG := aarch64-qemu-virt
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_AARCH64_CROSVM_VIRT=y" .config && echo y),y)
    PLAT_FROM_CONFIG := aarch64-crosvm-virt
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_AARCH64_RASPI=y" .config && echo y),y)
    PLAT_FROM_CONFIG := aarch64-raspi
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_RISCV64_QEMU_VIRT=y" .config && echo y),y)
    PLAT_FROM_CONFIG := riscv64-qemu-virt
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_X86_64_QEMU_VIRT=y" .config && echo y),y)
    PLAT_FROM_CONFIG := x86_64-qemu-virt
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_X86_CSV=y" .config && echo y),y)
    PLAT_FROM_CONFIG := x86-csv
  endif
  ifeq ($(shell grep -q "^\(CONFIG_\)\?PLATFORM_LOONGARCH64_QEMU_VIRT=y" .config && echo y),y)
    PLAT_FROM_CONFIG := loongarch64-qemu-virt
  endif
  
  # Use config values as defaults, but allow command line override
  ARCH ?= $(ARCH_FROM_CONFIG)
  PLAT ?= $(PLAT_FROM_CONFIG)
endif

# Final defaults if not set from config or command line
ARCH ?= aarch64
PLAT ?= $(ARCH)-qemu-virt

export ARCH PLAT
