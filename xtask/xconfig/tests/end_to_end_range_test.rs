use std::fs;
use tempfile::TempDir;
use xconfig::config::{ConfigReader, ConfigWriter};
use xconfig::kconfig::{Parser, SymbolTable, SymbolType};
use xconfig::kconfig::ast::{Entry, Expr};

#[test]
fn test_hex_underscore_end_to_end() {
    let temp_dir = TempDir::new().unwrap();
    
    // 1. Create test Kconfig file with hex underscores
    let kconfig_content = r#"
mainmenu "Test"

config TEST_MMIO_BASE
    rangetype "MMIO Base Address"
    default [0x1000_0000, 0x2000_0000, 0xfe00_0000]
"#;
    let kconfig_path = temp_dir.path().join("Kconfig");
    fs::write(&kconfig_path, kconfig_content).unwrap();
    
    // 2. Parse Kconfig
    let mut parser = Parser::new(&kconfig_path, temp_dir.path()).unwrap();
    let parse_result = parser.parse();
    
    if let Err(e) = &parse_result {
        eprintln!("Parse error: {}", e);
    }
    assert!(parse_result.is_ok(), "Failed to parse Kconfig with hex underscores");
    let ast = parse_result.unwrap();
    
    // 3. Extract default values to SymbolTable
    let mut symbols = SymbolTable::new();
    
    for entry in &ast.entries {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_MMIO_BASE" {
                symbols.add_symbol(config.name.clone(), SymbolType::Range);
                
                // Extract default value if present
                if !config.properties.defaults.is_empty() {
                    if let Expr::Const(default_value) = &config.properties.defaults[0].value {
                        // Normalize spaces for consistency: the parser adds spaces after commas
                        // in array literals "[0x1000_0000, 0x2000_0000]" but we want to store
                        // and compare without spaces "[0x1000_0000,0x2000_0000]"
                        let normalized = default_value.replace(" ", "");
                        symbols.set_value("TEST_MMIO_BASE", normalized);
                    }
                }
            }
        }
    }
    
    // 4. Write to .config
    let config_path = temp_dir.path().join(".config");
    ConfigWriter::write(&config_path, &symbols).unwrap();
    
    // 5. Verify .config content
    let config_content = fs::read_to_string(&config_path).unwrap();
    assert!(
        config_content.contains("TEST_MMIO_BASE=[0x1000_0000,0x2000_0000,0xfe00_0000]"),
        "Config file should contain hex values with underscores without extra quotes. Got: {}",
        config_content
    );
    assert!(!config_content.contains("\"["), "Config should not have extra quotes around arrays");
    
    // 6. Read .config back
    let config = ConfigReader::read(&config_path).unwrap();
    assert_eq!(
        config.get("TEST_MMIO_BASE"),
        Some(&"[0x1000_0000,0x2000_0000,0xfe00_0000]".to_string())
    );
}

#[test]
fn test_hex_array_mixed_formats() {
    let temp_dir = TempDir::new().unwrap();
    
    // Test array with mix of hex values with and without underscores
    let kconfig_content = r#"
mainmenu "Test"

config TEST_HEX_MIXED
    rangetype "Hex Mixed Format"
    default [0x10, 0x1000_0000, 0xff, 0xfe00_0000]
"#;
    let kconfig_path = temp_dir.path().join("Kconfig");
    fs::write(&kconfig_path, kconfig_content).unwrap();
    
    let mut parser = Parser::new(&kconfig_path, temp_dir.path()).unwrap();
    let parse_result = parser.parse();
    
    if let Err(e) = &parse_result {
        eprintln!("Parse error: {}", e);
    }
    assert!(parse_result.is_ok(), "Failed to parse Kconfig with mixed hex formats");
    let ast = parse_result.unwrap();
    
    // Extract config
    let config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_HEX_MIXED" {
                return Some(config);
            }
        }
        None
    });
    
    assert!(config.is_some(), "TEST_HEX_MIXED config not found");
    let config = config.unwrap();
    
    // Verify default value is preserved with underscores
    assert_eq!(config.properties.defaults.len(), 1);
    if let Expr::Const(default_value) = &config.properties.defaults[0].value {
        assert!(default_value.contains("0x1000_0000"), "Should preserve underscores in hex");
        assert!(default_value.contains("0xfe00_0000"), "Should preserve underscores in hex");
    } else {
        panic!("Default value should be a constant expression");
    }
}
