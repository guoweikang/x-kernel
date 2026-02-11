# Platform Configuration Migration Tool

## Purpose

This is a **one-time use tool** to migrate platform configuration from the old `platconfig.toml` format to new Kconfig-based `defconfig` files.

## Usage

```bash
# From the xtask/migrate_to_kconfig directory:
cargo run

# Or from the repository root:
cargo run --manifest-path xtask/migrate_to_kconfig/Cargo.toml
```

The tool will:
1. Discover all platforms by scanning `platforms/*/platconfig.toml`
2. Parse TOML files and convert them to defconfig format
3. Generate `defconfig` files in each platform directory

## What it does

### Platform Discovery
Scans for all `platforms/*/platconfig.toml` files and processes these platforms:
- `aarch64-qemu-virt`
- `aarch64-crosvm-virt`
- `aarch64-raspi`
- `riscv64-qemu-virt`
- `x86_64-qemu-virt`
- `x86-csv`
- `loongarch64-qemu-virt`

### Conversion Rules

1. **Key Mapping**: Converts kebab-case to UPPER_SNAKE_CASE with `PLATFORM_` prefix
   - `cpu-num` → `PLATFORM_CPU_NUM`
   - `phys-memory-base` → `PLATFORM_PHYS_MEM_BASE`

2. **Value Formatting**:
   - Integers >= 256 → hex format (e.g., `0x40000000`)
   - Small integers (< 256) → decimal (e.g., `33` for IRQ numbers)
   - Strings with hex → hex without underscores (e.g., `"0xffff_0000_0000_0000"` → `0xffff000000000000`)
   - Other strings → quoted (e.g., `"value"`)
   - Booleans → `y` or `n`

3. **Special Handling**:
   - `psci-method: "hvc"` → `PLATFORM_PSCI_HVC=y`
   - `psci-method: "smc"` → `PLATFORM_PSCI_SMC=y`
   - Arrays (mmio-ranges, virtio-mmio-ranges, pci-ranges) are skipped

4. **Output Grouping**:
   - Architecture and platform selection
   - Platform Basic Configuration
   - Platform DMA and Power Management
   - Platform Devices
   - GIC Configuration
   - RTC Configuration
   - PCI Configuration
   - Other Device Configuration

## Output Format

Example generated defconfig file:

```
ARCH_AARCH64=y
PLATFORM_AARCH64_QEMU_VIRT=y

# Platform Basic Configuration
PLATFORM_CPU_NUM=4
PLATFORM_PHYS_MEM_BASE=0x40000000
PLATFORM_PHYS_MEM_SIZE=0x8000000
...

# Platform DMA and Power Management
PLATFORM_DMA_MEM_BASE=0x40000000
PLATFORM_DMA_MEM_SIZE=0x200000
PLATFORM_PSCI_HVC=y
...
```

## Post-Migration Steps

After successful migration and testing:

1. **Review generated defconfig files**:
   ```bash
   ls platforms/*/defconfig
   cat platforms/aarch64-qemu-virt/defconfig
   ```

2. **Test with one platform**:
   ```bash
   cp platforms/aarch64-qemu-virt/defconfig .config
   make menuconfig
   make build
   ```

3. **If everything works**:
   - Delete old platconfig.toml files: `rm platforms/*/platconfig.toml`
   - Update documentation
   - Remove this migration tool: `rm -rf xtask/migrate_to_kconfig`
   - Update build system to use defconfig by default

## Dependencies

- `toml = "0.8"` - TOML parsing
- `serde = { version = "1.0", features = ["derive"] }` - Deserialization
- `anyhow = "1.0"` - Error handling

## Notes

- **NO `CONFIG_` prefix** is used in defconfig files (confirmed from codebase)
- Handles missing keys gracefully (some platforms may not have all configs)
- Preserves hex format for memory addresses (without underscores for Kconfig compatibility)
- Can be run from any directory (automatically finds repository root)
