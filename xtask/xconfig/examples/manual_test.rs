use xconfig::kconfig::Parser;
use xconfig::ui::state::ConfigState;
use std::path::PathBuf;

fn main() {
    println!("Testing with actual Kconfig...\n");
    
    let kconfig_path = PathBuf::from("Kconfig");
    let srctree = PathBuf::from(".");
    
    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let ast = parser.parse().unwrap();
    
    let config_state = ConfigState::build_from_entries(&ast.entries);
    
    println!("Total items collected: {}", config_state.all_items.len());
    
    // Check for architecture options
    let has_arch_aarch64 = config_state.all_items.iter().any(|i| i.id == "ARCH_AARCH64");
    let has_arch_riscv64 = config_state.all_items.iter().any(|i| i.id == "ARCH_RISCV64");
    let has_arch_x86_64 = config_state.all_items.iter().any(|i| i.id == "ARCH_X86_64");
    
    println!("\nArchitecture options found:");
    println!("  ARCH_AARCH64: {}", has_arch_aarch64);
    println!("  ARCH_RISCV64: {}", has_arch_riscv64);
    println!("  ARCH_X86_64: {}", has_arch_x86_64);
    
    // Check for platform options (these are inside if blocks)
    let has_platform_aarch64_qemu = config_state.all_items.iter().any(|i| i.id == "PLATFORM_AARCH64_QEMU_VIRT");
    let has_platform_aarch64_crosvm = config_state.all_items.iter().any(|i| i.id == "PLATFORM_AARCH64_CROSVM_VIRT");
    let has_platform_riscv64_qemu = config_state.all_items.iter().any(|i| i.id == "PLATFORM_RISCV64_QEMU_VIRT");
    let has_platform_x86_64_qemu = config_state.all_items.iter().any(|i| i.id == "PLATFORM_X86_64_QEMU_VIRT");
    
    println!("\nPlatform options found (inside if blocks):");
    println!("  PLATFORM_AARCH64_QEMU_VIRT: {}", has_platform_aarch64_qemu);
    println!("  PLATFORM_AARCH64_CROSVM_VIRT: {}", has_platform_aarch64_crosvm);
    println!("  PLATFORM_RISCV64_QEMU_VIRT: {}", has_platform_riscv64_qemu);
    println!("  PLATFORM_X86_64_QEMU_VIRT: {}", has_platform_x86_64_qemu);
    
    // Check Platform Selection menu
    if let Some(platform_menu_items) = config_state.menu_tree.get("menu_Platform Selection") {
        println!("\nPlatform Selection menu contains {} items:", platform_menu_items.len());
        for item in platform_menu_items {
            println!("  - {} (depth: {}, kind: {:?})", item.id, item.depth, 
                     match &item.kind {
                         xconfig::ui::state::MenuItemKind::Choice { .. } => "Choice",
                         xconfig::ui::state::MenuItemKind::Config { .. } => "Config",
                         xconfig::ui::state::MenuItemKind::Menu { .. } => "Menu",
                         _ => "Other",
                     });
        }
    } else {
        println!("\n⚠️  Platform Selection menu not found in menu_tree!");
    }
    
    println!("\n✓ Test completed successfully!");
}
