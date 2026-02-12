use xconfig::kconfig::Parser;
use std::path::PathBuf;

#[test]
fn test_parse_real_kconfig() {
    let kconfig_path = PathBuf::from("../../Kconfig");
    let srctree = PathBuf::from("../..");
    
    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();
    
    assert!(result.is_ok(), "Failed to parse real Kconfig: {:?}", result.err());
    
    let ast = result.unwrap();
    println!("✓ Parse succeeded! Total entries: {}", ast.entries.len());
    
    // Check for ARCH config with conditional defaults
    use xconfig::kconfig::ast::Entry;
    
    let arch_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "ARCH" {
                return Some(config);
            }
        }
        None
    });
    
    assert!(arch_config.is_some(), "ARCH config not found");
    let arch_config = arch_config.unwrap();
    
    println!("ARCH config found:");
    println!("  Type: {:?}", arch_config.symbol_type);
    println!("  Number of defaults: {}", arch_config.properties.defaults.len());
    
    assert_eq!(
        arch_config.properties.defaults.len(), 
        4, 
        "ARCH should have 4 conditional defaults"
    );
    
    // Check for PLATFORM config (inside Platform Selection menu)
    let mut platform_config = None;
    for entry in &ast.entries {
        if let Entry::Menu(menu) = entry {
            if menu.title == "Platform Selection" {
                platform_config = menu.entries.iter().find_map(|entry| {
                    if let Entry::Config(config) = entry {
                        if config.name == "PLATFORM" {
                            return Some(config);
                        }
                    }
                    None
                });
                break;
            }
        }
    }
    
    assert!(platform_config.is_some(), "PLATFORM config not found");
    let platform_config = platform_config.unwrap();
    
    println!("PLATFORM config found:");
    println!("  Type: {:?}", platform_config.symbol_type);
    println!("  Number of defaults: {}", platform_config.properties.defaults.len());
    
    assert_eq!(
        platform_config.properties.defaults.len(), 
        7, 
        "PLATFORM should have 7 conditional defaults"
    );
    
    // Check for Platform Selection menu
    let platform_menu = ast.entries.iter().find_map(|entry| {
        if let Entry::Menu(menu) = entry {
            if menu.title == "Platform Selection" {
                return Some(menu);
            }
        }
        None
    });
    
    assert!(platform_menu.is_some(), "Platform Selection menu not found");
    let platform_menu = platform_menu.unwrap();
    println!("Platform Selection menu found with {} entries", platform_menu.entries.len());
    
    // Check for Kernel Features menu
    let kernel_features_menu = ast.entries.iter().find_map(|entry| {
        if let Entry::Menu(menu) = entry {
            if menu.title == "Kernel Features" {
                return Some(menu);
            }
        }
        None
    });
    
    assert!(kernel_features_menu.is_some(), "Kernel Features menu not found");
    println!("Kernel Features menu found");
    
    // Check for drivers/Kconfig source
    let has_drivers_source = ast.entries.iter().any(|entry| {
        if let Entry::Source(source) = entry {
            source.path.to_str().unwrap_or("").contains("drivers/Kconfig")
        } else {
            false
        }
    });
    
    assert!(has_drivers_source, "drivers/Kconfig source directive not found");
    println!("drivers/Kconfig source found");
}

#[test]
fn test_symbol_table_with_conditional_defaults() {
    use xconfig::kconfig::{Parser, SymbolTable};
    use xconfig::kconfig::ast::Entry;
    use xconfig::kconfig::expr::evaluate_expr;
    
    let kconfig_path = PathBuf::from("../../Kconfig");
    let srctree = PathBuf::from("../..");
    
    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let ast = parser.parse().unwrap();
    
    let mut symbol_table = SymbolTable::new();
    
    // Simulate symbol extraction with conditional defaults
    for entry in &ast.entries {
        match entry {
            Entry::Config(config) => {
                symbol_table.add_symbol(config.name.clone(), config.symbol_type.clone());
                
                // Process conditional defaults
                for (default_value, condition) in &config.properties.defaults {
                    let should_apply = if let Some(cond) = condition {
                        evaluate_expr(cond, &symbol_table).unwrap_or(false)
                    } else {
                        true
                    };
                    
                    if should_apply {
                        if let xconfig::kconfig::ast::Expr::Const(val) = default_value {
                            symbol_table.set_value(&config.name, val.clone());
                            break;
                        }
                    }
                }
            }
            Entry::Choice(choice) => {
                for option in &choice.options {
                    symbol_table.add_symbol(option.name.clone(), option.symbol_type.clone());
                }
                
                if let Some(default_name) = &choice.default {
                    symbol_table.set_value(default_name, "y".to_string());
                } else if let Some(first_option) = choice.options.first() {
                    symbol_table.set_value(&first_option.name, "y".to_string());
                }
            }
            _ => {}
        }
    }
    
    // Verify ARCH gets a default value
    let arch_value = symbol_table.get_value("ARCH");
    println!("ARCH value after processing defaults: {:?}", arch_value);
    
    // Check if ARCH_AARCH64 is enabled (it's the default)
    let arch_aarch64_enabled = symbol_table.is_enabled("ARCH_AARCH64");
    println!("ARCH_AARCH64 enabled: {}", arch_aarch64_enabled);
    
    if arch_aarch64_enabled {
        // ARCH should be "aarch64"
        assert_eq!(
            arch_value,
            Some("aarch64".to_string()),
            "ARCH should be aarch64 when ARCH_AARCH64 is enabled"
        );
    }
}

#[test]
fn debug_parse_structure() {
    use xconfig::kconfig::Parser;
    use xconfig::kconfig::ast::Entry;
    use std::path::PathBuf;
    
    let kconfig_path = PathBuf::from("../../Kconfig");
    let srctree = PathBuf::from("../..");
    
    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let ast = parser.parse().unwrap();
    
    println!("\nTotal top-level entries: {}", ast.entries.len());
    
    for (i, entry) in ast.entries.iter().enumerate() {
        match entry {
            Entry::Config(config) => {
                println!("{}: Config: {}", i, config.name);
            }
            Entry::MenuConfig(mc) => {
                println!("{}: MenuConfig: {}", i, mc.name);
            }
            Entry::Menu(menu) => {
                println!("{}: Menu: \"{}\" with {} sub-entries", i, menu.title, menu.entries.len());
                // Show first few sub-entries
                for (j, sub_entry) in menu.entries.iter().take(3).enumerate() {
                    match sub_entry {
                        Entry::Config(config) => {
                            println!("  {}.{}: Config: {}", i, j, config.name);
                        }
                        Entry::Choice(choice) => {
                            println!("  {}.{}: Choice: {} options", i, j, choice.options.len());
                        }
                        Entry::If(if_entry) => {
                            println!("  {}.{}: If block with {} entries", i, j, if_entry.entries.len());
                        }
                        _ => {
                            println!("  {}.{}: Other entry type", i, j);
                        }
                    }
                }
            }
            Entry::Choice(choice) => {
                println!("{}: Choice: {:?} with {} options", i, choice.prompt, choice.options.len());
            }
            Entry::If(if_entry) => {
                println!("{}: If: with {} entries", i, if_entry.entries.len());
            }
            Entry::Source(source) => {
                println!("{}: Source: {:?}", i, source.path);
            }
            Entry::MainMenu(title) => {
                println!("{}: MainMenu: \"{}\"", i, title);
            }
            Entry::Comment(comment) => {
                println!("{}: Comment: \"{}\"", i, comment.text);
            }
        }
    }
}
