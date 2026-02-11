# Extract ARCH and PLAT from .config if it exists

ifneq ($(wildcard .config),)
  # Read .config once and extract ARCH and PLAT in a single pass
  CONFIG_VALUES := $(shell awk '/^(CONFIG_)?ARCH_[A-Z0-9_]+=y/ { print $$0 } /^(CONFIG_)?PLATFORM_[A-Z0-9_]+=y/ { print $$0 }' .config)
  
  # Parse architecture
  ifeq ($(findstring ARCH_AARCH64=y,$(CONFIG_VALUES)),ARCH_AARCH64=y)
    ARCH_FROM_CONFIG := aarch64
  else ifeq ($(findstring ARCH_RISCV64=y,$(CONFIG_VALUES)),ARCH_RISCV64=y)
    ARCH_FROM_CONFIG := riscv64
  else ifeq ($(findstring ARCH_X86_64=y,$(CONFIG_VALUES)),ARCH_X86_64=y)
    ARCH_FROM_CONFIG := x86_64
  else ifeq ($(findstring ARCH_LOONGARCH64=y,$(CONFIG_VALUES)),ARCH_LOONGARCH64=y)
    ARCH_FROM_CONFIG := loongarch64
  endif
  
  # Parse platform
  ifeq ($(findstring PLATFORM_AARCH64_QEMU_VIRT=y,$(CONFIG_VALUES)),PLATFORM_AARCH64_QEMU_VIRT=y)
    PLAT_FROM_CONFIG := aarch64-qemu-virt
  else ifeq ($(findstring PLATFORM_AARCH64_CROSVM_VIRT=y,$(CONFIG_VALUES)),PLATFORM_AARCH64_CROSVM_VIRT=y)
    PLAT_FROM_CONFIG := aarch64-crosvm-virt
  else ifeq ($(findstring PLATFORM_AARCH64_RASPI=y,$(CONFIG_VALUES)),PLATFORM_AARCH64_RASPI=y)
    PLAT_FROM_CONFIG := aarch64-raspi
  else ifeq ($(findstring PLATFORM_RISCV64_QEMU_VIRT=y,$(CONFIG_VALUES)),PLATFORM_RISCV64_QEMU_VIRT=y)
    PLAT_FROM_CONFIG := riscv64-qemu-virt
  else ifeq ($(findstring PLATFORM_X86_64_QEMU_VIRT=y,$(CONFIG_VALUES)),PLATFORM_X86_64_QEMU_VIRT=y)
    PLAT_FROM_CONFIG := x86_64-qemu-virt
  else ifeq ($(findstring PLATFORM_X86_CSV=y,$(CONFIG_VALUES)),PLATFORM_X86_CSV=y)
    PLAT_FROM_CONFIG := x86-csv
  else ifeq ($(findstring PLATFORM_LOONGARCH64_QEMU_VIRT=y,$(CONFIG_VALUES)),PLATFORM_LOONGARCH64_QEMU_VIRT=y)
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
