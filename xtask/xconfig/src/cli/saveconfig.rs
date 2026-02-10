use crate::config::{ConfigGenerator, ConfigWriter};
use crate::error::Result;
use crate::kconfig::Parser;
use std::path::PathBuf;

pub fn saveconfig_command(
    output: PathBuf,
    kconfig: PathBuf,
    srctree: PathBuf,
) -> Result<()> {
    println!("Saving configuration...");
    println!("Kconfig: {}", kconfig.display());
    println!("Output: {}", output.display());
    
    // Parse Kconfig to get symbol definitions
    let mut parser = Parser::new(&kconfig, &srctree)?;
    let ast = parser.parse()?;
    
    // Build symbol table from Kconfig
    let mut symbols = crate::kconfig::SymbolTable::new();
    
    // Extract symbols from AST and apply defaults
    for entry in &ast.entries {
        if let crate::kconfig::ast::Entry::Config(config) = entry {
            // Strip CONFIG_ prefix if present
            let clean_name = config.name.strip_prefix("CONFIG_").unwrap_or(&config.name);
            symbols.add_symbol(clean_name.to_string(), config.symbol_type.clone());
            
            // Apply default value if present
            // Note: This is a simplified implementation that only handles simple
            // constant and symbol expressions. Full expression evaluation
            // (with dependencies and conditional defaults) is not yet implemented.
            if let Some(default_expr) = &config.properties.default {
                // Extract value from simple expressions
                if let crate::kconfig::ast::Expr::Const(val) = default_expr {
                    symbols.set_value(clean_name, val.clone());
                } else if let crate::kconfig::ast::Expr::Symbol(sym) = default_expr {
                    // Handle default values like 'y' or 'n'
                    symbols.set_value(clean_name, sym.clone());
                }
                // Complex expressions (e.g., conditional defaults with 'if')
                // would require full expression evaluation and are not handled here
            } else {
                // Set to 'n' if no default
                match config.symbol_type {
                    crate::kconfig::ast::SymbolType::Bool | 
                    crate::kconfig::ast::SymbolType::Tristate => {
                        symbols.set_value(clean_name, "n".to_string());
                    }
                    _ => {}
                }
            }
        }
    }
    
    // Write .config file
    ConfigWriter::write(&output, &symbols)?;
    println!("✅ Saved .config to {}", output.display());
    
    // Generate auto.conf
    let auto_conf = output.parent().unwrap_or(std::path::Path::new(".")).join("auto.conf");
    ConfigGenerator::generate_auto_conf(&auto_conf, &symbols)?;
    println!("✅ Generated {}", auto_conf.display());
    
    // Generate autoconf.h
    let autoconf_h = output.parent().unwrap_or(std::path::Path::new(".")).join("autoconf.h");
    ConfigGenerator::generate_autoconf_h(&autoconf_h, &symbols)?;
    println!("✅ Generated {}", autoconf_h.display());
    
    Ok(())
}
