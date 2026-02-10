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
            // Strip CONFIG_ prefix if present
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
                        writeln!(file, "{}=\"{}\"", clean_name, value)?;
                    }
                }
            } else {
                writeln!(file, "# {} is not set", clean_name)?;
            }
        }

        Ok(())
    }
}
