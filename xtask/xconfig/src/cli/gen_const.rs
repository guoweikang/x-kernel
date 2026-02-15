use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use crate::error::{KconfigError, Result};

/// Generate Rust const definitions from .config file
pub fn gen_const_command(config: PathBuf, output_dir: PathBuf) -> Result<()> {
    println!("📝 Generating Rust const definitions from .config...");
    println!("Config: {}", config.display());
    println!("Output: {}", output_dir.display());

    // Parse .config file
    let config_map = parse_config(&config)?;

    // Generate config.rs
    generate_config_rs(&output_dir, &config_map)?;

    println!("✅ Generated config.rs successfully");

    Ok(())
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
) -> Result<()> {
    fs::create_dir_all(output_dir)?;

    let config_rs_path = output_dir.join("config.rs");

    let mut content = String::new();
    content.push_str("// Auto-generated by xtask xconfig gen-const from .config\n");
    content.push_str("// DO NOT EDIT MANUALLY\n\n");

    for (key, value) in config {
        // Skip boolean configs (y/n/m) - handled via --cfg
        if value == "y" || value == "n" || value == "m" {
            continue;
        }

        content.push_str(&format!("#[allow(dead_code)]\n"));

        // Check if it's a range value (starts with [ and ends with ])
        if value.starts_with('[') && value.ends_with(']') {
            let inner = &value[1..value.len()-1];
            
            if inner.is_empty() {
                // Empty array
                content.push_str(&format!("pub const {}: &[&str] = &[];\n\n", key));
                continue;
            }
            
            // Check if it's a tuple array: look for '(' character
            if inner.contains('(') && inner.contains(')') {
                // This is a tuple array
                match parse_tuple_array(inner) {
                    Ok((tuple_type, rust_code)) => {
                        content.push_str(&format!(
                            "pub const {}: &[{}] = &[\n{}\n];\n\n",
                            key, tuple_type, rust_code
                        ));
                    }
                    Err(e) => {
                        eprintln!("⚠️  Warning: Failed to parse tuple array for {}: {}", key, e);
                        // Fallback to string
                        content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
                    }
                }
                continue;
            }
            
            let items: Vec<&str> = inner.split(',').map(|s| s.trim()).collect();
            
            // Determine element type from first item (guaranteed to exist since inner is not empty)
            let first_item = items[0];
            
            if first_item.starts_with("0x") || first_item.starts_with("0X") {
                // Hex array - parse as usize values
                let mut valid_items: Vec<String> = Vec::new();
                let mut has_invalid = false;
                for s in &items {
                    let trimmed = s.trim();
                    if trimmed.starts_with("0x") || trimmed.starts_with("0X") {
                        // Parse hex string to validate and keep original format
                        match usize::from_str_radix(&trimmed[2..], 16) {
                            Ok(_) => {
                                // Keep the hex format (0x...)
                                valid_items.push(trimmed.to_string());
                            }
                            Err(_) => {
                                eprintln!("⚠️  Warning: Invalid hex value '{}' in array {}", trimmed, key);
                                has_invalid = true;
                            }
                        }
                    } else {
                        eprintln!("⚠️  Warning: Skipping non-hex item '{}' in hex array {}", trimmed, key);
                        has_invalid = true;
                    }
                }
                if has_invalid {
                    eprintln!("⚠️  Warning: {} has mixed types - only hex values will be included", key);
                }
                content.push_str(&format!("pub const {}: &[usize] = &[{}];\n\n", 
                    key, valid_items.join(", ")));
            } else if first_item.parse::<usize>().is_ok() {
                // Integer array - validate all items are unsigned integers
                let mut valid_items: Vec<String> = Vec::new();
                let mut has_invalid = false;
                for s in &items {
                    if s.trim().parse::<usize>().is_ok() {
                        valid_items.push(s.trim().to_string());
                    } else {
                        eprintln!("⚠️  Warning: Skipping non-integer item '{}' in integer array {}", s.trim(), key);
                        has_invalid = true;
                    }
                }
                if has_invalid {
                    eprintln!("⚠️  Warning: {} has mixed types - only integer values will be included", key);
                }
                content.push_str(&format!("pub const {}: &[usize] = &[{}];\n\n", 
                    key, valid_items.join(", ")));
            } else {
                // String array - all items treated as strings
                let str_items: Vec<String> = items.iter()
                    .map(|s| format!("\"{}\"", s.trim().trim_matches('"')))
                    .collect();
                content.push_str(&format!("pub const {}: &[&str] = &[{}];\n\n", 
                    key, str_items.join(", ")));
            }
            continue;
        }

        // Check if it's a hex value (starts with 0x or 0X)
        if value.starts_with("0x") || value.starts_with("0X") {
            // Parse and validate as usize hex
            match usize::from_str_radix(&value[2..], 16) {
                Ok(_) => {
                    content.push_str(&format!("pub const {}: usize = {};\n\n", key, value));
                }
                Err(_) => {
                    eprintln!("⚠️  Warning: Invalid hex value for {}: {}", key, value);
                }
            }
        }
        // Try parsing as unsigned integer
        else if let Ok(uint_val) = value.parse::<usize>() {
            content.push_str(&format!("pub const {}: usize = {};\n\n", key, uint_val));
        }
        // Then try parsing as signed integer
        else if let Ok(int_val) = value.parse::<i64>() {
            content.push_str(&format!("pub const {}: i64 = {};\n\n", key, int_val));
        }
        // Otherwise treat as string
        else {
            content.push_str(&format!("pub const {}: &str = \"{}\";\n\n", key, value));
        }
    }

    fs::write(&config_rs_path, content)?;

    println!("📝 Generated config.rs at: {}", config_rs_path.display());

    Ok(())
}
