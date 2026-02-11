# Extract ARCH and PLAT from .config if it exists
# BUT: Skip for Kconfig configuration targets (menuconfig, defconfig, etc.)

# Define targets that DON'T need .config
# Note: These lists are duplicated in the main Makefile for early validation
# Keep both lists in sync when adding new targets
KCONFIG_TARGETS := menuconfig defconfig saveconfig oldconfig
CLEAN_TARGETS := clean clean_c distclean
UTILITY_TARGETS := clippy doc doc_check_missing fmt unittest unittest_no_fail_fast

# Check if current goal is a non-build target
SKIP_CONFIG_CHECK := $(filter $(KCONFIG_TARGETS) $(CLEAN_TARGETS) $(UTILITY_TARGETS),$(MAKECMDGOALS))

# Only read .config if:
# 1. .config exists AND
# 2. We're not running a Kconfig/utility target
ifeq ($(SKIP_CONFIG_CHECK),)
  ifneq ($(wildcard .config),)
    # Read .config once and extract ARCH and PLAT in a single pass
    CONFIG_VALUES := $(shell awk '/^(CONFIG_)?ARCH_[A-Z0-9_]+=y/ { print $$0 } /^(CONFIG_)?PLATFORM_[A-Z0-9_]+=y/ { print $$0 }' .config 2>/dev/null)

    # Parse architecture (only if CONFIG_VALUES is not empty)
    ifneq ($(CONFIG_VALUES),)
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

	  # Parse mode: BUILD_TYPE_RELEASE or BUILD_TYPE_DEBUG
	  ifeq ($(findstring BUILD_TYPE_DEBUG=y,$(CONFIG_VALUES)),BUILD_TYPE_DEBUG=y)
		BUILD_TYPE_FROM_CONFIG := debug
	  else ifeq ($(findstring BUILD_TYPE_RELEASE=y,$(CONFIG_VALUES)),BUILD_TYPE_RELEASE=y)
		BUILD_TYPE_FROM_CONFIG := release
	  endif

	  # parse PLATFORM_KERNEL_BASE_PADDR=0x40200000
	  ifneq ($(shell grep -E '^PLATFORM_KERNEL_BASE_PADDR=' .config 2>/dev/null),)
	  		KERNEL_BASE_PADDR_FROM_CONFIG := $(shell grep -E '^PLATFORM_KERNEL_BASE_PADDR=' .config 2>/dev/null | cut -d '=' -f2)
	  endif

      # Use config values as defaults, but allow command line override
      ARCH ?= $(ARCH_FROM_CONFIG)
      PLAT ?= $(PLAT_FROM_CONFIG)
	  MODE ?= $(BUILD_TYPE_FROM_CONFIG)
	  KERNEL_BASE_PADDR ?= $(KERNEL_BASE_PADDR_FROM_CONFIG)
    endif
  endif
endif



# Final defaults if not set from config or command line
ARCH ?= aarch64
PLAT ?= $(ARCH)-qemu-virt

export ARCH PLAT
