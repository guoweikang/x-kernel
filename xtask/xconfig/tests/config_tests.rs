use xconfig::config::{ConfigReader, ConfigWriter};
use xconfig::kconfig::{SymbolTable, SymbolType};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_config_reader() {
    let config_path = "tests/fixtures/basic/expected.config";
    let config = ConfigReader::read(config_path).unwrap();

    assert_eq!(config.get("TEST_BOOL"), Some(&"y".to_string()));
    assert_eq!(config.get("TEST_STRING"), Some(&"hello".to_string()));
    assert_eq!(config.get("TEST_INT"), Some(&"42".to_string()));
}

#[test]
fn test_config_reader_backward_compat() {
    // Test reading config with CONFIG_ prefix (backward compatibility)
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.config");

    fs::write(&config_path, "CONFIG_TEST1=y\nCONFIG_TEST2=\"value\"\n# CONFIG_TEST3 is not set\n").unwrap();

    let config = ConfigReader::read(&config_path).unwrap();
    assert_eq!(config.get("TEST1"), Some(&"y".to_string()));
    assert_eq!(config.get("TEST2"), Some(&"value".to_string()));
    assert_eq!(config.get("TEST3"), Some(&"n".to_string()));
}

#[test]
fn test_config_writer() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.config");

    let mut symbols = SymbolTable::new();
    symbols.add_symbol("TEST1".to_string(), SymbolType::Bool);
    symbols.set_value("TEST1", "y".to_string());
    symbols.add_symbol("TEST2".to_string(), SymbolType::String);
    symbols.set_value("TEST2", "value".to_string());

    ConfigWriter::write(&config_path, &symbols).unwrap();

    // Read back and verify - should NOT have CONFIG_ prefix
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("TEST1=y"));
    assert!(content.contains("TEST2=\"value\""));
    assert!(!content.contains("CONFIG_"));
}

#[test]
fn test_config_roundtrip() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = temp_dir.path().join("test.config");

    // Write config
    let mut symbols = SymbolTable::new();
    symbols.add_symbol("A".to_string(), SymbolType::Bool);
    symbols.set_value("A", "y".to_string());
    symbols.add_symbol("B".to_string(), SymbolType::Bool);
    symbols.set_value("B", "n".to_string());

    ConfigWriter::write(&config_path, &symbols).unwrap();

    // Read back
    let config = ConfigReader::read(&config_path).unwrap();
    assert_eq!(config.get("A"), Some(&"y".to_string()));
    assert_eq!(config.get("B"), Some(&"n".to_string()));
}
