use crate::error::Result;
use crate::kconfig::SymbolTable;
use std::fs::File;
use std::io::Write;
use std::path::Path;

/// ConfigGenerator generates build system configuration files.
/// 
/// Note: Modules (tristate 'm' values) are treated as 'y' in this implementation
/// as the rust-kbuild project does not currently support loadable modules.
/// This is a deviation from the Linux kernel Kconfig which generates separate
/// _MODULE defines for module options.
pub struct ConfigGenerator;

impl ConfigGenerator {
    pub fn generate_auto_conf(path: impl AsRef<Path>, symbols: &SymbolTable) -> Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "#")?;
        writeln!(file, "# Automatically generated file; DO NOT EDIT.")?;
        writeln!(file, "#")?;

        for (name, symbol) in symbols.all_symbols() {
            // Strip CONFIG_ prefix if present
            let clean_name = name.strip_prefix("CONFIG_").unwrap_or(name);
            
            if let Some(value) = &symbol.value {
                if value != "n" {
                    writeln!(file, "{}={}", clean_name, value)?;
                }
            }
        }

        Ok(())
    }

    pub fn generate_autoconf_h(path: impl AsRef<Path>, symbols: &SymbolTable) -> Result<()> {
        let mut file = File::create(path)?;

        writeln!(file, "/*")?;
        writeln!(file, " * Automatically generated file; DO NOT EDIT.")?;
        writeln!(file, " */")?;
        writeln!(file)?;

        for (name, symbol) in symbols.all_symbols() {
            // Strip CONFIG_ prefix if present
            let clean_name = name.strip_prefix("CONFIG_").unwrap_or(name);
            
            if let Some(value) = &symbol.value {
                match value.as_str() {
                    "y" => {
                        writeln!(file, "#define {} 1", clean_name)?;
                    }
                    "m" => {
                        // Treat modules as 'y' (no module support in rust-kbuild)
                        // This is a simplification compared to Linux Kconfig which would
                        // generate a separate _MODULE define
                        writeln!(file, "#define {} 1", clean_name)?;
                    }
                    "n" => {
                        // Don't define anything for disabled options
                    }
                    _ => {
                        writeln!(file, "#define {} \"{}\"", clean_name, value)?;
                    }
                }
            }
        }

        Ok(())
    }
}
