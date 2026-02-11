use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

// Platform mapping table: (directory, arch_config, platform_config)
const PLATFORMS: &[(&str, &str, &str)] = &[
    ("aarch64-qemu-virt", "ARCH_AARCH64", "PLATFORM_AARCH64_QEMU_VIRT"),
    ("aarch64-crosvm-virt", "ARCH_AARCH64", "PLATFORM_AARCH64_CROSVM_VIRT"),
    ("aarch64-raspi", "ARCH_AARCH64", "PLATFORM_AARCH64_RASPI"),
    ("riscv64-qemu-virt", "ARCH_RISCV64", "PLATFORM_RISCV64_QEMU_VIRT"),
    ("x86_64-qemu-virt", "ARCH_X86_64", "PLATFORM_X86_64_QEMU_VIRT"),
    ("x86-csv", "ARCH_X86_64", "PLATFORM_X86_CSV"),
    ("loongarch64-qemu-virt", "ARCH_LOONGARCH64", "PLATFORM_LOONGARCH64_QEMU_VIRT"),
];

#[derive(Debug, Deserialize)]
struct PlatformConfig {
    #[serde(default)]
    arch: Option<String>,
    #[serde(default)]
    platform: Option<String>,
    #[serde(default)]
    package: Option<String>,
    #[serde(default)]
    plat: Option<HashMap<String, toml::Value>>,
    #[serde(default)]
    devices: Option<HashMap<String, toml::Value>>,
}

/// Convert TOML key (kebab-case) to Kconfig key (UPPER_SNAKE_CASE with PLATFORM_ prefix)
fn toml_key_to_config_key(key: &str) -> String {
    format!("PLATFORM_{}", key.replace("-", "_").to_uppercase())
}

/// Format a TOML value for defconfig output
fn format_value(value: &toml::Value) -> String {
    match value {
        toml::Value::Integer(i) => {
            // Use hex for addresses (>= 0x100 / 256) or large values
            // Keep small numbers like IRQ numbers in decimal
            if *i >= 256 {
                format!("0x{:x}", i)
            } else {
                i.to_string()
            }
        }
        toml::Value::String(s) => {
            // If the string looks like a hex number, keep it as is (without quotes for defconfig)
            if s.starts_with("0x") || s.starts_with("0X") {
                s.to_string()
            } else {
                format!("\"{}\"", s)
            }
        }
        toml::Value::Boolean(b) => (if *b { "y" } else { "n" }).to_string(),
        _ => panic!("Unsupported TOML type: {:?}", value),
    }
}

/// Generate defconfig content for a platform
fn generate_defconfig(
    arch_config: &str,
    platform_config: &str,
    config: &PlatformConfig,
) -> String {
    let mut output = String::new();
    
    // Architecture and platform selection
    output.push_str(&format!("{}=y\n", arch_config));
    output.push_str(&format!("{}=y\n", platform_config));
    output.push_str("\n");
    
    // Platform Basic Configuration
    if let Some(plat) = &config.plat {
        let mut basic_configs = Vec::new();
        let mut dma_power_configs = Vec::new();
        let mut other_configs = Vec::new();
        
        // Sort config keys into groups
        for (key, value) in plat {
            // Skip arrays for now (mmio-ranges, virtio-mmio-ranges, pci-ranges)
            if value.is_array() {
                continue;
            }
            
            let config_key = toml_key_to_config_key(key);
            let formatted_value = format_value(value);
            let config_line = format!("{}={}", config_key, formatted_value);
            
            // Group configs
            if key.starts_with("dma-") || key == "psci-method" {
                dma_power_configs.push((key.clone(), config_line));
            } else if matches!(
                key.as_str(),
                "cpu-num" | "phys-memory-base" | "phys-memory-size" | "low-memory-base" | "low-memory-size" 
                | "high-memory-base" | "kernel-base-paddr" | "kernel-base-vaddr" 
                | "phys-virt-offset" | "phys-bus-offset" | "phys-boot-offset"
                | "kernel-aspace-base" | "kernel-aspace-size" | "boot-stack-size"
            ) {
                basic_configs.push((key.clone(), config_line));
            } else {
                other_configs.push((key.clone(), config_line));
            }
        }
        
        // Output basic configs
        if !basic_configs.is_empty() {
            output.push_str("# Platform Basic Configuration\n");
            for (_, line) in basic_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
        
        // Output DMA and power management configs
        if !dma_power_configs.is_empty() {
            output.push_str("# Platform DMA and Power Management\n");
            for (key, line) in dma_power_configs {
                if key == "psci-method" {
                    // Convert psci-method to boolean config
                    if let Some(toml::Value::String(method)) = plat.get(&key) {
                        if method == "hvc" {
                            output.push_str("PLATFORM_PSCI_HVC=y\n");
                        } else if method == "smc" {
                            output.push_str("PLATFORM_PSCI_SMC=y\n");
                        }
                    }
                } else {
                    output.push_str(&line);
                    output.push('\n');
                }
            }
            output.push('\n');
        }
        
        // Output other configs
        if !other_configs.is_empty() {
            for (_, line) in other_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
    }
    
    // Device Configuration
    if let Some(devices) = &config.devices {
        let mut uart_configs = Vec::new();
        let mut timer_configs = Vec::new();
        let mut gic_configs = Vec::new();
        let mut rtc_configs = Vec::new();
        let mut pci_configs = Vec::new();
        let mut other_device_configs = Vec::new();
        
        for (key, value) in devices {
            // Skip arrays
            if value.is_array() {
                continue;
            }
            
            let config_key = toml_key_to_config_key(key);
            let formatted_value = format_value(value);
            let config_line = format!("{}={}", config_key, formatted_value);
            
            // Group device configs
            if key.starts_with("uart-") {
                uart_configs.push((key.clone(), config_line));
            } else if key.starts_with("timer-") || key == "ipi-irq" || key == "pmu-irq" {
                timer_configs.push((key.clone(), config_line));
            } else if key.starts_with("gic") {
                gic_configs.push((key.clone(), config_line));
            } else if key.starts_with("rtc-") {
                rtc_configs.push((key.clone(), config_line));
            } else if key.starts_with("pci-") {
                pci_configs.push((key.clone(), config_line));
            } else {
                other_device_configs.push((key.clone(), config_line));
            }
        }
        
        // Output UART configs
        if !uart_configs.is_empty() {
            output.push_str("# Platform Devices\n");
            for (_, line) in &uart_configs {
                output.push_str(&line);
                output.push('\n');
            }
        }
        
        // Output timer/irq configs
        if !timer_configs.is_empty() {
            for (_, line) in &timer_configs {
                output.push_str(&line);
                output.push('\n');
            }
        }
        
        if !uart_configs.is_empty() || !timer_configs.is_empty() {
            output.push('\n');
        }
        
        // Output GIC configs
        if !gic_configs.is_empty() {
            output.push_str("# GIC Configuration\n");
            for (_, line) in &gic_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
        
        // Output RTC configs
        if !rtc_configs.is_empty() {
            output.push_str("# RTC Configuration\n");
            // Check if we have rtc-paddr, which means RTC is present
            if rtc_configs.iter().any(|(key, _)| key == "rtc-paddr") {
                output.push_str("PLATFORM_RTC_PL031=y\n");
            }
            for (_, line) in &rtc_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
        
        // Output PCI configs
        if !pci_configs.is_empty() {
            output.push_str("# PCI Configuration\n");
            for (_, line) in &pci_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
        
        // Output other device configs
        if !other_device_configs.is_empty() {
            output.push_str("# Other Device Configuration\n");
            for (_, line) in &other_device_configs {
                output.push_str(&line);
                output.push('\n');
            }
            output.push('\n');
        }
    }
    
    // Remove trailing newlines
    output.trim_end().to_string() + "\n"
}

/// Discover all platforms with platconfig.toml
fn discover_platforms(platforms_dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut found = Vec::new();
    
    for (platform_dir, _, _) in PLATFORMS {
        let platconfig_path = platforms_dir.join(platform_dir).join("platconfig.toml");
        if platconfig_path.exists() {
            found.push((platform_dir.to_string(), platconfig_path));
        }
    }
    
    Ok(found)
}

fn main() -> Result<()> {
    println!("🔄 Migrating platconfig.toml to Kconfig defconfig\n");
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━\n");
    
    // Get the repository root (assuming we're run from xtask/migrate_to_kconfig)
    let repo_root = std::env::current_dir()?
        .ancestors()
        .nth(2)
        .context("Could not find repository root")?
        .to_path_buf();
    
    let platforms_dir = repo_root.join("platforms");
    
    // Discover platforms
    let platforms = discover_platforms(&platforms_dir)?;
    
    if platforms.is_empty() {
        println!("⚠️  No platforms with platconfig.toml found!");
        return Ok(());
    }
    
    // Process each platform
    for (platform_name, platconfig_path) in platforms {
        println!("📝 Processing: {}", platform_name);
        
        // Find platform config from mapping
        let platform_info = PLATFORMS
            .iter()
            .find(|(name, _, _)| *name == platform_name)
            .context(format!("Platform {} not in mapping table", platform_name))?;
        
        let (_, arch_config, platform_config) = platform_info;
        
        // Read and parse TOML
        let toml_content = fs::read_to_string(&platconfig_path)
            .context(format!("Failed to read {}", platconfig_path.display()))?;
        let config: PlatformConfig = toml::from_str(&toml_content)
            .context(format!("Failed to parse {}", platconfig_path.display()))?;
        
        // Generate defconfig
        let defconfig_content = generate_defconfig(arch_config, platform_config, &config);
        
        // Write defconfig
        let defconfig_path = platforms_dir.join(&platform_name).join("defconfig");
        fs::write(&defconfig_path, defconfig_content)
            .context(format!("Failed to write {}", defconfig_path.display()))?;
        
        println!("   ✅ Created: {}\n", defconfig_path.display());
    }
    
    println!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    println!("✨ Migration complete!\n");
    println!("📌 Next steps:");
    println!("   1. Review generated defconfig files:");
    println!("      ls platforms/*/defconfig\n");
    println!("   2. Test with one platform:");
    println!("      cp platforms/aarch64-qemu-virt/defconfig .config");
    println!("      make menuconfig");
    println!("      make build\n");
    println!("   3. If everything works:");
    println!("      # Delete old config system");
    println!("      rm platforms/*/platconfig.toml");
    println!("      # Update documentation");
    println!("      # Remove this migration tool");
    
    Ok(())
}
