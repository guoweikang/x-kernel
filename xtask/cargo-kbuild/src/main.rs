use clap::Parser;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::process::{self, Command};

#[derive(Parser, Debug)]
#[command(name = "cargo-kbuild", bin_name = "cargo")]
#[command(about = "Build with Kconfig features from .config")]
struct Cli {
    /// Subcommand name (always "kbuild")
    #[arg(value_name = "kbuild")]
    _subcommand: String,
    
    /// Path to .config file
    #[arg(long, default_value = ".config")]
    kconfig: PathBuf,
    
    /// Change directory before build
    #[arg(short = 'C', value_name = "PATH")]
    directory: Option<PathBuf>,
    
    /// Additional cargo arguments (pass-through)
    #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
    cargo_args: Vec<String>,
}

fn main() {
    let cli = Cli::parse();
    
    // Parse .config and extract enabled CONFIG_* options
    let kconfig_features = match parse_kconfig(&cli.kconfig) {
        Ok(features) => features,
        Err(e) => {
            eprintln!("Error parsing {}: {}", cli.kconfig.display(), e);
            process::exit(1);
        }
    };
    
    // Build cargo command
    let mut cmd = Command::new("cargo");
    cmd.arg("build");
    
    if let Some(dir) = cli.directory {
        cmd.current_dir(dir);
    }
    
    // Append Kconfig features (merge with existing --features if present)
    if !kconfig_features.is_empty() {
        let existing_features = extract_features_arg(&cli.cargo_args);
        let all_features = if let Some(existing) = existing_features {
            format!("{},{}", existing, kconfig_features.join(","))
        } else {
            kconfig_features.join(",")
        };
        
        // Remove existing --features argument to avoid duplication
        let filtered_args: Vec<String> = remove_features_arg(&cli.cargo_args);
        
        cmd.args(&filtered_args);
        cmd.arg("--features");
        cmd.arg(all_features);
    } else {
        // No kconfig features, just pass through all args
        cmd.args(&cli.cargo_args);
    }
    
    // Execute cargo build
    let status = cmd.status()
        .expect("Failed to execute cargo build");
    
    process::exit(status.code().unwrap_or(1));
}

fn parse_kconfig(path: &PathBuf) -> Result<Vec<String>, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read file: {}", e))?;
    
    let mut features = Vec::new();
    let mut seen = HashSet::new();
    
    for line in content.lines() {
        let line = line.trim();
        
        // Skip comments and empty lines
        if line.is_empty() || line.starts_with('#') {
            continue;
        }
        
        // Parse lines like: CONFIG_XXX=y or CONFIG_XXX=m or XXX=y
        let config_line = if let Some(stripped) = line.strip_prefix("CONFIG_") {
            stripped
        } else if line.contains('=') {
            // Handle lines without CONFIG_ prefix (like MAX_CPUS=102)
            line
        } else {
            continue;
        };
        
        if let Some((name, value)) = config_line.split_once('=') {
            if value == "y" || value == "m" {
                // Convert to feature name (lowercase)
                let feature_name = name.to_lowercase();
                // Avoid duplicates
                if seen.insert(feature_name.clone()) {
                    features.push(feature_name);
                }
            }
        }
    }
    
    Ok(features)
}

fn extract_features_arg(args: &[String]) -> Option<String> {
    // Find --features argument in cargo_args
    for (i, arg) in args.iter().enumerate() {
        if arg == "--features" && i + 1 < args.len() {
            return Some(args[i + 1].clone());
        }
    }
    None
}

fn remove_features_arg(args: &[String]) -> Vec<String> {
    // Remove --features and its value from args
    let mut filtered = Vec::new();
    let mut skip_next = false;
    
    for arg in args {
        if skip_next {
            skip_next = false;
            continue;
        }
        
        if arg == "--features" {
            skip_next = true;
            continue;
        }
        
        filtered.push(arg.clone());
    }
    
    filtered
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_parse_kconfig_basic() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "CONFIG_SMP=y").unwrap();
        writeln!(tmpfile, "CONFIG_NET=y").unwrap();
        writeln!(tmpfile, "CONFIG_DEBUG=n").unwrap();
        
        let features = parse_kconfig(&tmpfile.path().to_path_buf()).unwrap();
        assert_eq!(features, vec!["smp", "net"]);
    }

    #[test]
    fn test_parse_kconfig_with_comments() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "# This is a comment").unwrap();
        writeln!(tmpfile, "CONFIG_SMP=y").unwrap();
        writeln!(tmpfile, "# CONFIG_NET is not set").unwrap();
        writeln!(tmpfile, "").unwrap();
        writeln!(tmpfile, "CONFIG_MM=y").unwrap();
        
        let features = parse_kconfig(&tmpfile.path().to_path_buf()).unwrap();
        assert_eq!(features, vec!["smp", "mm"]);
    }

    #[test]
    fn test_parse_kconfig_with_module() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "CONFIG_SMP=y").unwrap();
        writeln!(tmpfile, "CONFIG_NET=m").unwrap();
        writeln!(tmpfile, "CONFIG_DEBUG=n").unwrap();
        
        let features = parse_kconfig(&tmpfile.path().to_path_buf()).unwrap();
        assert_eq!(features, vec!["smp", "net"]);
    }

    #[test]
    fn test_parse_kconfig_no_duplicates() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "CONFIG_SMP=y").unwrap();
        writeln!(tmpfile, "CONFIG_SMP=y").unwrap();
        
        let features = parse_kconfig(&tmpfile.path().to_path_buf()).unwrap();
        assert_eq!(features, vec!["smp"]);
    }

    #[test]
    fn test_parse_kconfig_without_prefix() {
        let mut tmpfile = NamedTempFile::new().unwrap();
        writeln!(tmpfile, "ARM64=y").unwrap();
        writeln!(tmpfile, "ARM=y").unwrap();
        writeln!(tmpfile, "MAX_CPUS=102").unwrap();
        
        let features = parse_kconfig(&tmpfile.path().to_path_buf()).unwrap();
        assert_eq!(features, vec!["arm64", "arm"]);
    }

    #[test]
    fn test_extract_features_arg() {
        let args = vec![
            "--release".to_string(),
            "--features".to_string(),
            "extra,test".to_string(),
            "--target".to_string(),
            "aarch64".to_string(),
        ];
        
        let features = extract_features_arg(&args);
        assert_eq!(features, Some("extra,test".to_string()));
    }

    #[test]
    fn test_extract_features_arg_none() {
        let args = vec![
            "--release".to_string(),
            "--target".to_string(),
            "aarch64".to_string(),
        ];
        
        let features = extract_features_arg(&args);
        assert_eq!(features, None);
    }

    #[test]
    fn test_remove_features_arg() {
        let args = vec![
            "--release".to_string(),
            "--features".to_string(),
            "extra,test".to_string(),
            "--target".to_string(),
            "aarch64".to_string(),
        ];
        
        let filtered = remove_features_arg(&args);
        assert_eq!(filtered, vec!["--release", "--target", "aarch64"]);
    }

    #[test]
    fn test_remove_features_arg_none() {
        let args = vec![
            "--release".to_string(),
            "--target".to_string(),
            "aarch64".to_string(),
        ];
        
        let filtered = remove_features_arg(&args);
        assert_eq!(filtered, vec!["--release", "--target", "aarch64"]);
    }
}
