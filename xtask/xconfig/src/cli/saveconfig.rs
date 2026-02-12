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
    extract_symbols_from_entries(&ast.entries, &mut symbols);
    
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

fn extract_symbols_from_entries(entries: &[crate::kconfig::ast::Entry], symbols: &mut crate::kconfig::SymbolTable) {
    use crate::kconfig::ast::Entry;
    use crate::kconfig::Expr;
    
    for entry in entries {
        match entry {
            Entry::Config(config) => {
                // Strip CONFIG_ prefix if present
                let clean_name = config.name.strip_prefix("CONFIG_").unwrap_or(&config.name);
                symbols.add_symbol(clean_name.to_string(), config.symbol_type.clone());
                
                // Apply default value if present
                if let Some(default_expr) = &config.properties.default {
                    if let Expr::Const(val) = default_expr {
                        symbols.set_value(clean_name, val.clone());
                    } else if let Expr::Symbol(sym) = default_expr {
                        // Handle default values like 'y' or 'n'
                        symbols.set_value(clean_name, sym.clone());
                    } else if let Expr::ShellExpr(shell_expr) = default_expr {
                        // Evaluate shell expression
                        if let Ok(value) = crate::kconfig::shell_expr::evaluate_shell_expr(shell_expr, symbols) {
                            if !value.is_empty() {
                                symbols.set_value(clean_name, value);
                            }
                        }
                    }
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
            Entry::MenuConfig(menuconfig) => {
                let clean_name = menuconfig.name.strip_prefix("CONFIG_").unwrap_or(&menuconfig.name);
                symbols.add_symbol(clean_name.to_string(), menuconfig.symbol_type.clone());
            }
            Entry::Choice(choice) => {
                for option in &choice.options {
                    let clean_name = option.name.strip_prefix("CONFIG_").unwrap_or(&option.name);
                    symbols.add_symbol(clean_name.to_string(), option.symbol_type.clone());
                }
                
                // Apply choice default if specified
                if let Some(default_name) = &choice.default {
                    let clean_default = default_name.strip_prefix("CONFIG_").unwrap_or(default_name);
                    symbols.set_value(clean_default, "y".to_string());
                } else if let Some(first_option) = choice.options.first() {
                    // No default specified, select first option (standard Kconfig behavior)
                    let clean_name = first_option.name.strip_prefix("CONFIG_").unwrap_or(&first_option.name);
                    symbols.set_value(clean_name, "y".to_string());
                }
            }
            Entry::Menu(menu) => {
                extract_symbols_from_entries(&menu.entries, symbols);
            }
            Entry::If(if_entry) => {
                extract_symbols_from_entries(&if_entry.entries, symbols);
            }
            _ => {}
        }
    }
}
