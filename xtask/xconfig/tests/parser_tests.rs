use xconfig::kconfig::Parser;
use std::path::PathBuf;

#[test]
fn test_parse_basic_config() {
    let kconfig_path = PathBuf::from("tests/fixtures/basic/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/basic");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let ast = parser.parse().unwrap();

    // Should parse 3 config entries
    assert_eq!(ast.entries.len(), 3);
}

#[test]
fn test_parse_source_directive() {
    let kconfig_path = PathBuf::from("tests/fixtures/source/Kconfig");
    let srctree = PathBuf::from("tests/fixtures/source");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    assert!(result.is_ok());
    let ast = result.unwrap();

    // Should have mainmenu, config, source, and entries from sourced file
    assert!(ast.entries.len() >= 2);
}

#[test]
fn test_parse_example_project() {
    let kconfig_path = PathBuf::from("examples/sample_project/Kconfig");
    let srctree = PathBuf::from("examples/sample_project");

    let mut parser = Parser::new(&kconfig_path, &srctree).unwrap();
    let result = parser.parse();

    if let Err(e) = &result {
        eprintln!("Parse error: {}", e);
    }
    assert!(result.is_ok());
    let ast = result.unwrap();

    // Should parse all entries including sourced files
    assert!(ast.entries.len() > 0);
}
