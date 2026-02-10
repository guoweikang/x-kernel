use crate::error::Result;
use crate::kconfig::SymbolTable;
use std::fs::File;
use std::io::Write;
use std::path::Path;

pub struct ConfigWriter;

impl ConfigWriter {
    pub fn write(path: impl AsRef<Path>, symbols: &SymbolTable) -> Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "#")?;
        writeln!(file, "# Automatically generated file; DO NOT EDIT.")?;
        writeln!(file, "# Rust Kbuild Configuration")?;
        writeln!(file, "#")?;

        for (name, symbol) in symbols.all_symbols() {
            let clean_name = name.strip_prefix("CONFIG_").unwrap_or(name);
            
            if let Some(value) = &symbol.value {
                match value.as_str() {
                    "y" | "m" => {
                        writeln!(file, "{}={}", clean_name, value)?;
                    }
                    "n" => {
                        writeln!(file, "# {} is not set", clean_name)?;
                    }
                    _ => {
                        use crate::kconfig::ast::SymbolType;
                        match symbol.symbol_type {
                            SymbolType::Hex => {
                                // Hex: NO quotes, normalize to 0x format
                                let normalized_hex = if value.starts_with("0x") || value.starts_with("0X") {
                                    format!("0x{}", value[2..].to_lowercase())
                                } else {
                                    match value.parse::<i64>() {
                                        Ok(num) if num >= 0 => format!("0x{:x}", num),
                                        Ok(num) => format!("-0x{:x}", num.unsigned_abs()),
                                        Err(_) => value.to_string(),
                                    }
                                };
                                writeln!(file, "{}={}", clean_name, normalized_hex)?;
                            }
                            SymbolType::Int => {
                                // Int: NO quotes, decimal format
                                writeln!(file, "{}={}", clean_name, value)?;
                            }
                            SymbolType::String => {
                                // String: Keep quotes
                                writeln!(file, "{}=\"{}\"", clean_name, value)?;
                            }
                            _ => {
                                // Fallback for other types
                                writeln!(file, "{}=\"{}\"", clean_name, value)?;
                            }
                        }
                    }
                }
            } else {
                writeln!(file, "# {} is not set", clean_name)?;
            }
        }

        Ok(())
    }
}
