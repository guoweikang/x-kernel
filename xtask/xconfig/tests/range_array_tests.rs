use std::path::PathBuf;
use xconfig::kconfig::ast::{Entry, Expr};
use xconfig::kconfig::Parser;

#[test]
fn test_parse_range_array_numbers() {
    let kconfig_path = PathBuf::from("tests/fixtures/range_arrays/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/range_arrays");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    if let Err(e) = &result {
        eprintln!("Parse error: {}", e);
    }
    assert!(result.is_ok());
    let ast = result.unwrap();

    // Find the TEST_RANGE_NUMBERS config
    let range_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_RANGE_NUMBERS" {
                return Some(config);
            }
        }
        None
    });

    assert!(range_config.is_some(), "TEST_RANGE_NUMBERS config not found");
    let range_config = range_config.unwrap();

    // Verify it has a default with array value
    assert_eq!(range_config.properties.defaults.len(), 1);
    let default = &range_config.properties.defaults[0];
    assert!(matches!(&default.value, Expr::Const(s) if s == "[1, 2, 3, 4, 5]"));
}

#[test]
fn test_parse_range_array_hex() {
    let kconfig_path = PathBuf::from("tests/fixtures/range_arrays/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/range_arrays");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = result.unwrap();

    // Find the TEST_RANGE_HEX config
    let hex_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_RANGE_HEX" {
                return Some(config);
            }
        }
        None
    });

    assert!(hex_config.is_some(), "TEST_RANGE_HEX config not found");
    let hex_config = hex_config.unwrap();

    // Verify it has a default with hex array value (preserved format)
    assert_eq!(hex_config.properties.defaults.len(), 1);
    let default = &hex_config.properties.defaults[0];
    assert!(matches!(&default.value, Expr::Const(s) if s == "[0x10, 0x20, 0x30]"));
}

#[test]
fn test_parse_range_array_identifiers() {
    let kconfig_path = PathBuf::from("tests/fixtures/range_arrays/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/range_arrays");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = result.unwrap();

    // Find the TEST_RANGE_IDENTIFIERS config
    let id_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_RANGE_IDENTIFIERS" {
                return Some(config);
            }
        }
        None
    });

    assert!(id_config.is_some(), "TEST_RANGE_IDENTIFIERS config not found");
    let id_config = id_config.unwrap();

    // Verify it has a default with identifier array value
    assert_eq!(id_config.properties.defaults.len(), 1);
    let default = &id_config.properties.defaults[0];
    assert!(matches!(&default.value, Expr::Const(s) if s == "[apple, banana, cherry]"));
}

#[test]
fn test_parse_range_array_empty() {
    let kconfig_path = PathBuf::from("tests/fixtures/range_arrays/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/range_arrays");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = result.unwrap();

    // Find the TEST_RANGE_EMPTY config
    let empty_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_RANGE_EMPTY" {
                return Some(config);
            }
        }
        None
    });

    assert!(empty_config.is_some(), "TEST_RANGE_EMPTY config not found");
    let empty_config = empty_config.unwrap();

    // Verify it has a default with empty array value
    assert_eq!(empty_config.properties.defaults.len(), 1);
    let default = &empty_config.properties.defaults[0];
    assert!(matches!(&default.value, Expr::Const(s) if s == "[]"));
}

#[test]
fn test_parse_range_array_mixed() {
    let kconfig_path = PathBuf::from("tests/fixtures/range_arrays/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/range_arrays");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = result.unwrap();

    // Find the TEST_RANGE_MIXED config
    let mixed_config = ast.entries.iter().find_map(|entry| {
        if let Entry::Config(config) = entry {
            if config.name == "TEST_RANGE_MIXED" {
                return Some(config);
            }
        }
        None
    });

    assert!(mixed_config.is_some(), "TEST_RANGE_MIXED config not found");
    let mixed_config = mixed_config.unwrap();

    // Verify it has a default with mixed array value
    assert_eq!(mixed_config.properties.defaults.len(), 1);
    let default = &mixed_config.properties.defaults[0];
    assert!(matches!(&default.value, Expr::Const(s) if s == "[1, 0x20, value]"));
}
