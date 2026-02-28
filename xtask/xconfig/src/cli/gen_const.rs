use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{KconfigError, Result};
use crate::kconfig::ast::{Entry, RangeType, RustType, SymbolType};
use crate::kconfig::Parser;

/// Generate Rust const definitions from .config file
pub fn gen_const_command(
    config: PathBuf,
    output_dir: PathBuf,
    kconfig: PathBuf,
    srctree: PathBuf,
) -> Result<()> {
    println!("📝 Generating Rust const definitions from .config...");
    println!("Config: {}", config.display());
    println!("Output: {}", output_dir.display());

    // Parse .config file
    let config_map = parse_config(&config)?;

    // Parse Kconfig file to obtain the authoritative symbol types
    let mut parser = Parser::new(&kconfig, &srctree)?;
    let ast = parser.parse()?;
    let type_map = build_type_map(&ast.entries);

    // Generate config.rs
    generate_config_rs(&output_dir, &config_map, &type_map)?;

    println!("✅ Generated config.rs successfully");

    Ok(())
}

/// Build a mapping from symbol name (without CONFIG_ prefix) to its SymbolType
/// by walking the Kconfig AST.
fn build_type_map(entries: &[Entry]) -> HashMap<String, SymbolType> {
    let mut map = HashMap::new();
    collect_types(entries, &mut map);
    map
}

fn collect_types(entries: &[Entry], map: &mut HashMap<String, SymbolType>) {
    for entry in entries {
        match entry {
            Entry::Config(config) => {
                let name = config.name.strip_prefix("CONFIG_").unwrap_or(&config.name);
                map.insert(name.to_string(), config.symbol_type.clone());
            }
            Entry::MenuConfig(mc) => {
                let name = mc.name.strip_prefix("CONFIG_").unwrap_or(&mc.name);
                map.insert(name.to_string(), mc.symbol_type.clone());
            }
            Entry::Choice(choice) => {
                for opt in &choice.options {
                    let name = opt.name.strip_prefix("CONFIG_").unwrap_or(&opt.name);
                    map.insert(name.to_string(), opt.symbol_type.clone());
                }
            }
            Entry::Menu(menu) => {
                collect_types(&menu.entries, map);
            }
            Entry::If(if_entry) => {
                collect_types(&if_entry.entries, map);
            }
            _ => {}
        }
    }
}

/// Return the Rust primitive type string for a RustType annotation.
fn rust_type_str(rt: &RustType) -> &'static str {
    match rt {
        RustType::U8 => "u8",
        RustType::U16 => "u16",
        RustType::U32 => "u32",
        RustType::U64 => "u64",
        RustType::U128 => "u128",
        RustType::Usize => "usize",
        RustType::I8 => "i8",
        RustType::I16 => "i16",
        RustType::I32 => "i32",
        RustType::I64 => "i64",
        RustType::I128 => "i128",
        RustType::Isize => "isize",
        RustType::Str => "&str",
        RustType::String => "String",
    }
}

/// Return the Rust primitive type string for an integer-class SymbolType.
fn symbol_type_primitive_str(st: &SymbolType) -> &'static str {
    match st {
        SymbolType::U8 => "u8",
        SymbolType::U16 => "u16",
        SymbolType::U32 => "u32",
        SymbolType::U64 => "u64",
        SymbolType::U128 => "u128",
        SymbolType::Usize => "usize",
        SymbolType::I8 => "i8",
        SymbolType::I16 => "i16",
        SymbolType::I32 => "i32",
        SymbolType::I64 => "i64",
        SymbolType::I128 => "i128",
        SymbolType::Isize => "isize",
        _ => "usize",
    }
}

/// Parse .config file
/// Now expects standardized format:
/// - Bool: CONFIG_X=y or # CONFIG_X is not set
/// - Int: CONFIG_X=123 (no quotes)
/// - Hex: CONFIG_X=0xff (no quotes)
/// - String: CONFIG_X="value" (with quotes)
fn parse_config(config_path: &Path) -> Result<HashMap<String, String>> {
    let content = fs::read_to_string(config_path)?;

    let mut config = HashMap::new();
    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        if let Some((key, value)) = line.split_once('=') {
            let key = key.trim();
            let value = value.trim();

            // Remove quotes if present (for backward compatibility)
            let value = if value.starts_with('"') && value.ends_with('"') {
                &value[1..value.len() - 1]
            } else {
                value
            };

            config.insert(key.to_string(), value.to_string());
        }
    }

    Ok(config)
}

fn split_tuples(s: &str) -> Result<Vec<&str>> {
    let mut tuples = Vec::new();
    let mut start = 0;
    let mut depth = 0;
    let mut in_quotes = false;
    let chars: Vec<char> = s.chars().collect();
    
    for i in 0..chars.len() {
        match chars[i] {
            '"' => {
                // Check if this quote is escaped by looking at the previous character
                let is_escaped = i > 0 && chars[i-1] == '\\';
                if !is_escaped {
                    in_quotes = !in_quotes;
                }
            }
            '(' if !in_quotes => {
                if depth == 0 {
                    start = i;
                }
                depth += 1;
            }
            ')' if !in_quotes => {
                depth -= 1;
                if depth == 0 {
                    tuples.push(&s[start..=i]);
                }
                if depth < 0 {
                    return Err(KconfigError::Config("Unmatched closing parenthesis".into()));
                }
            }
            _ => {}
        }
    }
    
    if depth != 0 {
        return Err(KconfigError::Config("Unmatched opening parenthesis".into()));
    }
    
    Ok(tuples)
}

/// Parse single tuple string and return element list
fn parse_single_tuple(tuple_str: &str) -> Result<Vec<String>> {
    let tuple_str = tuple_str.trim();
    if !tuple_str.starts_with('(') || !tuple_str.ends_with(')') {
        return Err(KconfigError::Config(format!("Invalid tuple format: {}", tuple_str)));
    }
    
    let inner = &tuple_str[1..tuple_str.len()-1];
    let mut elements = Vec::new();
    let mut current = String::new();
    let mut depth = 0;
    let mut in_quotes = false;
    
    for ch in inner.chars() {
        match ch {
            '"' => {
                in_quotes = !in_quotes;
                current.push(ch);
            }
            ',' if depth == 0 && !in_quotes => {
                elements.push(current.trim().to_string());
                current.clear();
            }
            '(' | '[' if !in_quotes => {
                depth += 1;
                current.push(ch);
            }
            ')' | ']' if !in_quotes => {
                depth -= 1;
                current.push(ch);
            }
            _ => current.push(ch),
        }
    }
    
    if !current.trim().is_empty() {
        elements.push(current.trim().to_string());
    }
    
    Ok(elements)
}

/// Infer Rust types for each element in a tuple
fn infer_tuple_types(elements: &[String]) -> Vec<String> {
    elements.iter().map(|elem| {
        let trimmed = elem.trim();
        
        // String (with quotes)
        if (trimmed.starts_with('"') && trimmed.ends_with('"')) 
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')) {
            return "&str".to_string();
        }
        
        // Hexadecimal - all hex values are usize
        if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
            return "usize".to_string();
        }
        
        // Negative number
        if trimmed.starts_with('-') {
            return "i64".to_string();
        }
        
        // Positive integer
        if trimmed.parse::<u64>().is_ok() {
            return "usize".to_string();
        }
        
        // Default to string
        "&str".to_string()
    }).collect()
}

/// Format tuple elements as Rust code
fn format_tuple_elements(elements: &[String], types: &[String]) -> Result<String> {
    if elements.len() != types.len() {
        return Err(KconfigError::Config(format!(
            "Element and type count mismatch: {} elements, {} types",
            elements.len(),
            types.len()
        )));
    }
    
    let formatted: Vec<String> = elements.iter().zip(types.iter()).map(|(elem, typ)| {
        let trimmed = elem.trim();
        
        if typ == "&str" {
            // String type
            if trimmed.starts_with('"') && trimmed.ends_with('"') {
                trimmed.to_string()
            } else {
                format!("\"{}\"", trimmed)
            }
        } else {
            // Numeric type, keep as-is
            trimmed.to_string()
        }
    }).collect();
    
    Ok(format!("({})", formatted.join(", ")))
}

/// Parse tuple array and return (tuple_type, rust_code)
fn parse_tuple_array(inner: &str) -> Result<(String, String)> {
    // 1. Split into individual tuples
    let tuples = split_tuples(inner)?;
    
    if tuples.is_empty() {
        return Err(KconfigError::Config("No tuples found".into()));
    }
    
    // 2. Parse first tuple to determine expected structure
    let first_tuple = parse_single_tuple(tuples[0])?;
    let expected_arity = first_tuple.len();
    let types = infer_tuple_types(&first_tuple);
    
    // 3. Validate all tuples have the same arity and consistent types
    for (idx, tuple_str) in tuples.iter().enumerate() {
        let elements = parse_single_tuple(tuple_str)?;
        
        // Check arity matches
        if elements.len() != expected_arity {
            return Err(KconfigError::Config(format!(
                "Tuple {} has {} elements, but expected {} (from first tuple)",
                idx, elements.len(), expected_arity
            )));
        }
        
        // Check types are consistent
        let tuple_types = infer_tuple_types(&elements);
        for (elem_idx, (expected_type, actual_type)) in types.iter().zip(tuple_types.iter()).enumerate() {
            if expected_type != actual_type {
                return Err(KconfigError::Config(format!(
                    "Type mismatch in tuple {} at position {}: expected {}, got {}",
                    idx, elem_idx, expected_type, actual_type
                )));
            }
        }
    }
    
    // 4. Generate Rust type string
    let tuple_type = format!("({})", types.join(", "));
    
    // 5. Generate all tuples' Rust code
    let mut rust_lines = Vec::new();
    for tuple_str in tuples {
        let elements = parse_single_tuple(tuple_str)?;
        let formatted = format_tuple_elements(&elements, &types)?;
        rust_lines.push(format!("    {}", formatted));
    }
    
    Ok((tuple_type, rust_lines.join(",\n")))
}

/// Generate config.rs file with constants
/// Handles three types:
/// - Int: decimal numbers (e.g., 123)
/// - Hex: 0x-prefixed numbers (e.g., 0xff)
/// - String: quoted strings (e.g., "hello")
fn generate_config_rs(
    output_dir: &Path,
    config: &HashMap<String, String>,
    type_map: &HashMap<String, SymbolType>,
) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let config_rs_path = output_dir.join("config.rs");

    let mut content = String::new();
    content.push_str("// Auto-generated by xtask xconfig gen-const from .config\n");
    content.push_str("// DO NOT EDIT MANUALLY\n\n");

    // Sort keys for stable output
    let mut sorted_keys: Vec<&String> = config.keys().collect();
    sorted_keys.sort();

    for key in sorted_keys {
        let value = &config[key];

        // Skip boolean configs (y/n/m) - handled via --cfg
        if value == "y" || value == "n" || value == "m" {
            continue;
        }

        content.push_str(&format!("#[allow(dead_code)]\n"));

        // Strip CONFIG_ prefix to look up in type_map
        let clean_key = key.strip_prefix("CONFIG_").unwrap_or(key.as_str());

        // Try to emit using the authoritative SymbolType from Kconfig
        if let Some(symbol_type) = type_map.get(clean_key) {
            if emit_typed_const(&mut content, key, value, symbol_type)? {
                continue;
            }
        }

        // Fallback: use heuristics when no type information is available
        emit_heuristic_const(&mut content, key, value)?;
    }

    fs::write(&config_rs_path, content)?;

    println!("📝 Generated config.rs at: {}", config_rs_path.display());

    Ok(())
}

/// Emit a `pub const` declaration using the authoritative SymbolType from Kconfig.
/// Returns `true` if the constant was emitted, `false` to fall back to heuristic.
fn emit_typed_const(
    content: &mut String,
    key: &str,
    value: &str,
    symbol_type: &SymbolType,
) -> Result<bool> {
    match symbol_type {
        // Bool/Tristate values are passed via --cfg flags; should already have been skipped above.
        SymbolType::Bool | SymbolType::Tristate => Ok(false),

        SymbolType::String => {
            content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
            Ok(true)
        }

        SymbolType::Hex => {
            // Hex type — emit as usize
            content.push_str(&format!("pub const {}: usize = {};\n\n", key, value));
            Ok(true)
        }

        // Specific integer types: use the exact declared Rust type
        st if st.is_integer_type() => {
            let rust_type = symbol_type_primitive_str(st);
            content.push_str(&format!("pub const {}: {} = {};\n\n", key, rust_type, value));
            Ok(true)
        }

        SymbolType::Range(range_type) => {
            emit_range_const(content, key, value, range_type)?;
            Ok(true)
        }

        _ => Ok(false),
    }
}

/// Emit a range (array) constant using the RangeType from Kconfig.
fn emit_range_const(
    content: &mut String,
    key: &str,
    value: &str,
    range_type: &RangeType,
) -> Result<()> {
    if !value.starts_with('[') || !value.ends_with(']') {
        // Unexpected format — fall back to raw string
        content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
        return Ok(());
    }

    let inner = &value[1..value.len() - 1];

    if inner.is_empty() {
        // Empty array
        match range_type {
            RangeType::StringArray => {
                content.push_str(&format!("pub const {}: &[&str] = &[];\n\n", key));
            }
            RangeType::Tuple(types) => {
                let tuple_type = types
                    .iter()
                    .map(|t| rust_type_str(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                content.push_str(&format!(
                    "pub const {}: &[({})] = &[];\n\n",
                    key, tuple_type
                ));
            }
            RangeType::Primitive(t) => {
                content.push_str(&format!(
                    "pub const {}: &[{}] = &[];\n\n",
                    key,
                    rust_type_str(t)
                ));
            }
            RangeType::Unknown => {
                content.push_str(&format!("pub const {}: &[&str] = &[];\n\n", key));
            }
        }
        return Ok(());
    }

    match range_type {
        RangeType::StringArray => {
            let items: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            let str_items: Vec<String> = items
                .iter()
                .map(|s| format!("\"{}\"", s.trim_matches('"')))
                .collect();
            content.push_str(&format!(
                "pub const {}: &[&str] = &[{}];\n\n",
                key,
                str_items.join(", ")
            ));
        }

        RangeType::Tuple(types) => {
            let tuple_type = types
                .iter()
                .map(|t| rust_type_str(t))
                .collect::<Vec<_>>()
                .join(", ");
            match parse_tuple_array(inner) {
                Ok((_, rust_code)) => {
                    content.push_str(&format!(
                        "pub const {}: &[({})] = &[\n{}\n];\n\n",
                        key, tuple_type, rust_code
                    ));
                }
                Err(e) => {
                    eprintln!(
                        "⚠️  Warning: Failed to parse tuple array for {}: {}",
                        key, e
                    );
                    content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
                }
            }
        }

        RangeType::Primitive(t) => {
            let elem_type = rust_type_str(t);
            let items: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            content.push_str(&format!(
                "pub const {}: &[{}] = &[{}];\n\n",
                key,
                elem_type,
                items.join(", ")
            ));
        }

        RangeType::Unknown => {
            // Unknown range type — use heuristic array logic
            emit_heuristic_array(content, key, inner)?;
        }
    }

    Ok(())
}

/// Emit a constant using value-based heuristics (fallback when no Kconfig type is known).
fn emit_heuristic_const(content: &mut String, key: &str, value: &str) -> Result<()> {
    // Check if it's a range value (starts with [ and ends with ])
    if value.starts_with('[') && value.ends_with(']') {
        let inner = &value[1..value.len() - 1];
        if inner.is_empty() {
            content.push_str(&format!("pub const {}: &[&str] = &[];\n\n", key));
            return Ok(());
        }
        emit_heuristic_array(content, key, inner)?;
        return Ok(());
    }

    // Hex value
    if value.starts_with("0x") || value.starts_with("0X") {
        match usize::from_str_radix(&value[2..], 16) {
            Ok(_) => {
                content.push_str(&format!("pub const {}: usize = {};\n\n", key, value));
            }
            Err(_) => {
                eprintln!("⚠️  Warning: Invalid hex value for {}: {}", key, value);
            }
        }
        return Ok(());
    }

    // Unsigned integer
    if let Ok(uint_val) = value.parse::<usize>() {
        content.push_str(&format!("pub const {}: usize = {};\n\n", key, uint_val));
        return Ok(());
    }

    // Signed integer
    if let Ok(int_val) = value.parse::<i64>() {
        content.push_str(&format!("pub const {}: i64 = {};\n\n", key, int_val));
        return Ok(());
    }

    // String fallback
    content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
    Ok(())
}

/// Emit an array constant using heuristics on the inner content (without brackets).
fn emit_heuristic_array(content: &mut String, key: &str, inner: &str) -> Result<()> {
    // Check if it's a tuple array: look for '(' character
    if inner.contains('(') && inner.contains(')') {
        match parse_tuple_array(inner) {
            Ok((tuple_type, rust_code)) => {
                content.push_str(&format!(
                    "pub const {}: &[{}] = &[\n{}\n];\n\n",
                    key, tuple_type, rust_code
                ));
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Warning: Failed to parse tuple array for {}: {}",
                    key, e
                );
                content.push_str(&format!("pub const {}: &str = \"[{}]\";\n\n", key, inner));
            }
        }
        return Ok(());
    }

    let items: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
    let first_item = items[0];

    if first_item.starts_with("0x") || first_item.starts_with("0X") {
        let mut valid_items: Vec<String> = Vec::new();
        let mut has_invalid = false;
        for s in &items {
            let trimmed = s.trim();
            if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                match usize::from_str_radix(&trimmed[2..], 16) {
                    Ok(_) => valid_items.push(trimmed.to_string()),
                    Err(_) => {
                        eprintln!(
                            "⚠️  Warning: Invalid hex value '{}' in array {}",
                            trimmed, key
                        );
                        has_invalid = true;
                    }
                }
            } else {
                eprintln!(
                    "⚠️  Warning: Skipping non-hex item '{}' in hex array {}",
                    trimmed, key
                );
                has_invalid = true;
            }
        }
        if has_invalid {
            eprintln!(
                "⚠️  Warning: {} has mixed types - only hex values will be included",
                key
            );
        }
        content.push_str(&format!(
            "pub const {}: &[usize] = &[{}];\n\n",
            key,
            valid_items.join(", ")
        ));
    } else if first_item.parse::<usize>().is_ok() {
        let mut valid_items: Vec<String> = Vec::new();
        let mut has_invalid = false;
        for s in &items {
            if s.trim().parse::<usize>().is_ok() {
                valid_items.push(s.trim().to_string());
            } else {
                eprintln!(
                    "⚠️  Warning: Skipping non-integer item '{}' in integer array {}",
                    s.trim(),
                    key
                );
                has_invalid = true;
            }
        }
        if has_invalid {
            eprintln!(
                "⚠️  Warning: {} has mixed types - only integer values will be included",
                key
            );
        }
        content.push_str(&format!(
            "pub const {}: &[usize] = &[{}];\n\n",
            key,
            valid_items.join(", ")
        ));
    } else {
        let str_items: Vec<String> = items
            .iter()
            .map(|s| format!("\"{}\"", s.trim().trim_matches('"')))
            .collect();
        content.push_str(&format!(
            "pub const {}: &[&str] = &[{}];\n\n",
            key,
            str_items.join(", ")
        ));
    }

    Ok(())
}
