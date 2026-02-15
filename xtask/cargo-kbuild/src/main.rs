use clap::{Args, Parser, Subcommand};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process;

#[derive(Debug, Deserialize)]
struct CargoToml {
    package: Package,
    #[serde(default)]
    features: HashMap<String, Vec<String>>,
    // Note: dependencies field kept for potential future feature validation
    #[serde(default)]
    #[allow(dead_code)]
    dependencies: HashMap<String, toml::Value>,
}

#[derive(Debug, Deserialize)]
struct Package {
    name: String,
    #[serde(default)]
    metadata: Metadata,
}

#[derive(Debug, Deserialize, Default)]
struct Metadata {
    #[serde(default)]
    kbuild: KbuildMetadata,
}

#[derive(Debug, Deserialize, Default)]
struct KbuildMetadata {
    #[serde(default)]
    enabled: bool,
}

#[derive(Debug)]
struct CrateInfo {
    name: String,
    // Note: path field kept for potential future features (e.g., detailed error reporting)
    #[allow(dead_code)]
    path: PathBuf,
    has_kbuild: bool,
    features: HashMap<String, Vec<String>>,
}

impl CrateInfo {
    fn is_kbuild_enabled(&self) -> bool {
        // A crate is kbuild-enabled if metadata.kbuild.enabled is set
        // or if it has any features (since non-kbuild crates typically don't declare features)
        // This is a heuristic that works for the current codebase
        self.has_kbuild || !self.features.is_empty()
    }
}

#[derive(Debug)]
struct Workspace {
    // Note: root field kept for potential future features (e.g., relative path resolution)
    #[allow(dead_code)]
    root: PathBuf,
    crates: Vec<CrateInfo>,
}

impl Workspace {
    fn new(root: PathBuf) -> Result<Self, String> {
        let mut crates = Vec::new();

        // Read workspace Cargo.toml
        let workspace_toml_path = root.join("Cargo.toml");
        let workspace_toml_content = fs::read_to_string(&workspace_toml_path)
            .map_err(|e| format!("Failed to read workspace Cargo.toml: {}", e))?;

        let workspace_toml: toml::Value = toml::from_str(&workspace_toml_content)
            .map_err(|e| format!("Failed to parse workspace Cargo.toml: {}", e))?;

        // Parse root package if it exists
        if workspace_toml.get("package").is_some() {
            if let Ok(root_crate) = Self::parse_crate(&root) {
                crates.push(root_crate);
            }
        }

        // Get workspace members
        let members = workspace_toml
            .get("workspace")
            .and_then(|w| w.get("members"))
            .and_then(|m| m.as_array())
            .ok_or("No workspace members found")?;

        // Parse each member crate
        for member in members {
            let member_path = member.as_str().ok_or("Invalid member path")?;
            let crate_path = root.join(member_path);

            if let Ok(crate_info) = Self::parse_crate(&crate_path) {
                crates.push(crate_info);
            }
        }

        Ok(Workspace { root, crates })
    }

    fn parse_crate(crate_path: &Path) -> Result<CrateInfo, String> {
        let cargo_toml_path = crate_path.join("Cargo.toml");
        let cargo_toml_content = fs::read_to_string(&cargo_toml_path)
            .map_err(|e| format!("Failed to read {}: {}", cargo_toml_path.display(), e))?;

        let cargo_toml: CargoToml = toml::from_str(&cargo_toml_content)
            .map_err(|e| format!("Failed to parse {}: {}", cargo_toml_path.display(), e))?;

        Ok(CrateInfo {
            name: cargo_toml.package.name.clone(),
            path: crate_path.to_path_buf(),
            has_kbuild: cargo_toml.package.metadata.kbuild.enabled,
            features: cargo_toml.features,
        })
    }

    // Note: find_crate method kept for potential future features (e.g., dependency graph analysis)
    #[allow(dead_code)]
    fn find_crate(&self, name: &str) -> Option<&CrateInfo> {
        self.crates.iter().find(|c| c.name == name)
    }
}

/// Check if a dependency package supports kbuild
/// Note: Function kept for potential future validation features
#[allow(dead_code)]
fn is_dependency_kbuild_enabled(workspace: &Workspace, pkg_name: &str) -> bool {
    if let Some(dep_crate) = workspace.find_crate(pkg_name) {
        // Check metadata.kbuild.enabled - this is the primary indicator
        if dep_crate.has_kbuild {
            return true;
        }

        // As a fallback heuristic, check if it has any features
        // (works for current codebase where non-kbuild crates don't use features)
        if !dep_crate.features.is_empty() {
            return true;
        }
    }

    false
}

/// Validate features for all kbuild-enabled crates
fn validate_features(workspace: &Workspace) -> Result<(), String> {
    println!("🔍 Validating feature dependencies...\n");

    // 1. Build a set of kbuild-enabled packages for performance
    let kbuild_packages: HashSet<String> = workspace
        .crates
        .iter()
        .filter(|c| c.is_kbuild_enabled())
        .map(|c| c.name.clone())
        .collect();

    // 2. Build a set of all workspace packages
    let workspace_packages: HashSet<String> =
        workspace.crates.iter().map(|c| c.name.clone()).collect();

    // 3. Validate each kbuild-enabled crate's features
    for crate_info in workspace.crates.iter().filter(|c| c.is_kbuild_enabled()) {
        for (feature_name, deps) in &crate_info.features {
            for dep in deps {
                // Check if sub-feature is specified
                if let Some((pkg_name, sub_feature)) = dep.split_once('/') {
                    // Key decision: Does the dependency support kbuild?
                    if kbuild_packages.contains(pkg_name) {
                        // ❌ Error: kbuild-enabled workspace crate cannot specify sub-feature
                        return Err(format!(
                            "❌ Error in crate '{}':\n\
                             \n\
                             Feature '{}' specifies sub-feature: '{}'\n\
                             \n\
                             Dependency '{}' is kbuild-enabled:\n\
                             - It reads CONFIG_* from .config directly\n\
                             - Cannot be controlled by parent crate\n\
                             \n\
                             Solution:\n\
                             1. Change to: {} = [\"{}\"]\n\
                             2. Enable {} in .config file\n\
                             \n\
                             Note: Third-party crates (e.g., log/std, tokio/rt) are allowed sub-features.\n",
                            crate_info.name,
                            feature_name,
                            dep,
                            pkg_name,
                            feature_name,
                            pkg_name,
                            sub_feature
                        ));
                    } else if workspace_packages.contains(pkg_name) {
                        // ℹ️ Info: Non-kbuild workspace crate - sub-feature allowed
                        eprintln!(
                            "ℹ️  '{}' is not kbuild-enabled, sub-feature allowed: {}\n",
                            pkg_name, dep
                        );
                    } else {
                        // ℹ️ Info: Third-party library - sub-feature allowed
                        eprintln!(
                            "ℹ️  '{}' is third-party, sub-feature allowed: {}\n",
                            pkg_name, dep
                        );
                    }
                }
            }
        }
    }

    println!("✅ Feature validation passed!\n");
    Ok(())
}

/// Collect all CONFIG_* names from .config file
fn collect_all_configs_from_file(config: &HashMap<String, String>) -> HashSet<String> {
    let mut configs = HashSet::new();

    for key in config.keys() {
        configs.insert(key.clone());
    }

    configs
}

/// Collect all CONFIG_* feature names from workspace crates (including root package)
fn collect_all_configs(workspace: &Workspace) -> HashSet<String> {
    let mut configs = HashSet::new();

    // Collect from all crates (not just kbuild-enabled) to include root package features
    for crate_info in workspace.crates.iter() {
        for feature_name in crate_info.features.keys() {
            configs.insert(feature_name.clone());
        }
    }

    configs
}

/// Generate .cargo/config.toml with check-cfg declarations
fn generate_cargo_config(workspace_root: &Path, configs: &HashSet<String>) -> Result<(), String> {
    let cargo_dir = workspace_root.join(".cargo");
    fs::create_dir_all(&cargo_dir)
        .map_err(|e| format!("Failed to create .cargo directory: {}", e))?;

    let config_path = cargo_dir.join("config.toml");

    let mut content = String::from("# Auto-generated by cargo-kbuild\n");
    content.push_str("# This file declares all conditional compilation flags\n");
    content.push_str("# Run 'cargo-kbuild build' to regenerate this file\n");
    content.push_str("# DO NOT commit this file to git\n\n");
    content.push_str("[build]\n");
    content.push_str("rustflags = [\n");

    let mut sorted_configs: Vec<_> = configs.iter().collect();
    sorted_configs.sort();

    for config in sorted_configs {
        content.push_str(&format!("    \"--check-cfg=cfg({})\",\n", config));
    }

    content.push_str("]\n");

    fs::write(&config_path, content)
        .map_err(|e| format!("Failed to write .cargo/config.toml: {}", e))?;

    println!(
        "✅ Generated .cargo/config.toml with {} config declarations",
        configs.len()
    );
    Ok(())
}

/// Parse .config file
/// Now expects standardized format:
/// - Bool: CONFIG_X=y or # CONFIG_X is not set
/// - Int: CONFIG_X=123 (no quotes)
/// - Hex: CONFIG_X=0xff (no quotes)
/// - String: CONFIG_X="value" (with quotes)
fn parse_config(config_path: &Path) -> Result<HashMap<String, String>, String> {
    let content =
        fs::read_to_string(config_path).map_err(|e| format!("Failed to read .config: {}", e))?;

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

/// Generate features based on .config
fn generate_features(config: &HashMap<String, String>) -> Vec<String> {
    let mut features = Vec::new();

    for (key, value) in config {
        if value == "y" || value == "m" {
            features.push(key.clone());
        }
    }

    features
}

/// Split string into individual tuples, handling nested parentheses
fn split_tuples(s: &str) -> Result<Vec<&str>, String> {
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
                    return Err("Unmatched closing parenthesis".to_string());
                }
            }
            _ => {}
        }
    }
    
    if depth != 0 {
        return Err("Unmatched opening parenthesis".to_string());
    }
    
    Ok(tuples)
}

/// Parse single tuple string and return element list
fn parse_single_tuple(tuple_str: &str) -> Result<Vec<String>, String> {
    let tuple_str = tuple_str.trim();
    if !tuple_str.starts_with('(') || !tuple_str.ends_with(')') {
        return Err(format!("Invalid tuple format: {}", tuple_str));
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
fn format_tuple_elements(elements: &[String], types: &[String]) -> Result<String, String> {
    if elements.len() != types.len() {
        return Err(format!(
            "Element and type count mismatch: {} elements, {} types",
            elements.len(),
            types.len()
        ));
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
fn parse_tuple_array(inner: &str) -> Result<(String, String), String> {
    // 1. Split into individual tuples
    let tuples = split_tuples(inner)?;
    
    if tuples.is_empty() {
        return Err("No tuples found".to_string());
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
            return Err(format!(
                "Tuple {} has {} elements, but expected {} (from first tuple)",
                idx, elements.len(), expected_arity
            ));
        }
        
        // Check types are consistent
        let tuple_types = infer_tuple_types(&elements);
        for (elem_idx, (expected_type, actual_type)) in types.iter().zip(tuple_types.iter()).enumerate() {
            if expected_type != actual_type {
                return Err(format!(
                    "Type mismatch in tuple {} at position {}: expected {}, got {}",
                    idx, elem_idx, expected_type, actual_type
                ));
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
/// - String: everything else with quotes
fn generate_config_rs(
    workspace_root: &Path,
    _config: &HashMap<String, String>,
) -> Result<(), String> {
    // Call xtask xconfig gen-const to generate config.rs
    println!("📝 Calling xconfig gen-const to generate config.rs...");
    
    let status = process::Command::new("cargo")
        .args(&["run", "-p", "xconfig", "--bin", "xconf", "--", "gen-const"])
        .current_dir(workspace_root)
        .status()
        .map_err(|e| format!("Failed to execute xconfig gen-const: {}", e))?;
    
    if !status.success() {
        return Err("xconfig gen-const command failed".to_string());
    }
    
    Ok(())
}

/// Apply kbuild configuration and run cargo command
///
/// # Arguments
/// * `workspace_root` - Root directory of the workspace
/// * `config_path` - Path to the .config file
/// * `cargo_cmd` - The cargo command to run (e.g., "build", "test", "check")
/// * `extra_args` - Additional arguments passed to cargo
///
/// # Returns
/// * `Ok(())` on success
/// * `Err(String)` with error message on failure
fn apply_kbuild_config(
    workspace_root: &Path,
    config_path: &Path,
    cargo_cmd: &str,
    extra_args: &[String],
) -> Result<(), String> {
    println!(
        "🔨 Running cargo {} with kbuild configuration...\n",
        cargo_cmd
    );

    // Parse workspace
    let workspace = Workspace::new(workspace_root.to_path_buf())?;

    // Validate features
    validate_features(&workspace)?;

    // Parse .config to get all CONFIG_* options
    let config = parse_config(config_path)?;

    // Generate config.rs file with constants
    generate_config_rs(workspace_root, &config)?;
    println!();

    // Collect all CONFIG_* names from .config file and generate .cargo/config.toml
    let all_configs = collect_all_configs_from_file(&config);
    generate_cargo_config(workspace_root, &all_configs)?;
    println!();

    // Generate features - only include features that are declared in Cargo.toml
    let features = generate_features(&config);
    let declared_features = collect_all_configs(&workspace);

    // Filter to only features that are actually declared in Cargo.toml
    let filtered_features: Vec<String> = features
        .into_iter()
        .filter(|f| declared_features.contains(f))
        .collect();

    println!("📋 Enabled features from .config:");
    for feature in &filtered_features {
        println!("  - {}", feature);
    }
    if filtered_features.is_empty() {
        println!("  (none - all CONFIG_* used via cfg flags)");
    }
    println!();

    // Build cargo command
    let mut cargo_args = vec![cargo_cmd.to_string()];

    if !filtered_features.is_empty() {
        cargo_args.push("--features".to_string());
        cargo_args.push(filtered_features.join(","));
    }

    // Add extra arguments
    cargo_args.extend_from_slice(extra_args);

    // Extract top-level cargo options like -C that must come before the subcommand
    let mut top_level_args: Vec<String> = Vec::new();
    let mut subcommand_args: Vec<String> = Vec::new();
    let mut i = 0;
    while i < cargo_args.len() {
        if i == 0 {
            // First element is the subcommand (build, test, etc.)
            subcommand_args.push(cargo_args[i].clone());
            i += 1;
            continue;
        }

        // Check for -C option (directory change)
        if cargo_args[i] == "-C" && i + 1 < cargo_args.len() {
            top_level_args.push(cargo_args[i].clone());
            top_level_args.push(cargo_args[i + 1].clone());
            i += 2;
            continue;
        }

        // Check for -Z option (unstable features)
        if cargo_args[i] == "-Z" && i + 1 < cargo_args.len() {
            top_level_args.push(cargo_args[i].clone());
            top_level_args.push(cargo_args[i + 1].clone());
            i += 2;
            continue;
        }

        // All other args go after the subcommand
        subcommand_args.push(cargo_args[i].clone());
        i += 1;
    }

    // Rebuild cargo_args with proper order: top_level_args, subcommand, subcommand_args
    let mut final_cargo_args: Vec<String> = Vec::new();
    final_cargo_args.extend(top_level_args);
    final_cargo_args.extend(subcommand_args);

    println!("🚀 Running: cargo {}\n", final_cargo_args.join(" "));

    let mut rustflags = env::var("RUSTFLAGS").unwrap_or_default();

    println!("🔍 [DEBUG] Original RUSTFLAGS from env: {}", rustflags);

    if !rustflags.is_empty() && !rustflags.ends_with(' ') {
        rustflags.push(' ');
    }

    // Add check-cfg declarations for all config options from .config
    for config_name in all_configs.iter() {
        if !rustflags.is_empty() && !rustflags.ends_with(' ') {
            rustflags.push(' ');
        }
        rustflags.push_str(&format!("--check-cfg=cfg({})", config_name));
    }

    // Add --cfg flags for ALL enabled configs from .config (not just features)
    for (key, value) in &config {
        if value == "y" || value == "m" {
            if !rustflags.is_empty() {
                rustflags.push(' ');
            }
            rustflags.push_str(&format!("--cfg {}", key));
        }
    }

    let mut cmd = process::Command::new("cargo");
    cmd.args(&final_cargo_args);
    cmd.current_dir(workspace_root);

    if !rustflags.is_empty() {
        println!("🔍 [DEBUG] Final RUSTFLAGS passed to cargo: {}", rustflags);
        cmd.env("RUSTFLAGS", rustflags);
    }

    let status = cmd
        .status()
        .map_err(|e| format!("Failed to run cargo: {}", e))?;

    if !status.success() {
        return Err(format!("cargo {} failed", cargo_cmd));
    }

    println!("\n✅ Command completed successfully!");
    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    bin_name = "cargo",
    version,
    about = "Kconfig-style configuration for Cargo"
)]
enum Cargo {
    Kbuild(KbuildCommand),
}

#[derive(Args, Debug)]
struct KbuildCommand {
    /// Path to .config file
    #[arg(long, default_value = ".config")]
    kconfig: PathBuf,

    #[command(subcommand)]
    command: Option<KbuildSubcommand>,
}

#[derive(Subcommand, Debug)]
enum KbuildSubcommand {
    /// Build the project
    Build {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run tests
    Test {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run a binary
    Run {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Check the project
    Check {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Run clippy
    Clippy {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Any other cargo command
    #[command(external_subcommand)]
    External(Vec<String>),
}

fn run_cargo_with_kbuild(
    workspace_root: &Path,
    kconfig_path: &Path,
    cargo_cmd: &str,
    extra_args: &[String],
) {
    if let Err(e) = apply_kbuild_config(workspace_root, kconfig_path, cargo_cmd, extra_args) {
        eprintln!("❌ Error: {}", e);
        process::exit(1);
    }
}

/// Print help message
fn print_help() {
    println!("cargo-kbuild");
    println!("Kconfig-style configuration system for Cargo");
    println!();
    println!("USAGE:");
    println!("    cargo kbuild [OPTIONS] <COMMAND>");
    println!();
    println!("OPTIONS:");
    println!("    --kconfig <FILE>    Path to .config file [default: .config]");
    println!();
    println!("COMMANDS:");
    println!("    build               Build the project");
    println!("    test                Run tests");
    println!("    run                 Run a binary");
    println!("    check               Check the project");
    println!("    clippy              Run clippy");
    println!("    <any-cargo-cmd>     Any other cargo command");
    println!();
    println!("EXAMPLES:");
    println!("    cargo kbuild build");
    println!("    cargo kbuild test --lib");
    println!("    cargo kbuild run --release");
    println!("    cargo kbuild check --all-targets");
    println!("    cargo kbuild clippy -- -D warnings");
    println!("    cargo kbuild build --kconfig custom.config");
}

/// Print version information
fn print_version() {
    println!("cargo-kbuild {}", env!("CARGO_PKG_VERSION"));
}

/// Extract --kconfig <path> from arguments and return (path, remaining_args)
fn extract_kconfig_arg(args: &[String]) -> (Option<PathBuf>, Vec<String>) {
    let mut kconfig_path = None;
    let mut remaining = Vec::new();
    let mut iter = args.iter();

    while let Some(arg) = iter.next() {
        if arg == "--kconfig" {
            if let Some(path) = iter.next() {
                kconfig_path = Some(PathBuf::from(path));
            }
        } else {
            remaining.push(arg.clone());
        }
    }

    (kconfig_path, remaining)
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Handle both `cargo-kbuild` and `cargo kbuild` invocation patterns
    let is_cargo_subcommand = args.len() > 1 && args[1] == "kbuild";

    // For cargo subcommand pattern, try using clap
    if is_cargo_subcommand {
        match Cargo::try_parse() {
            Ok(Cargo::Kbuild(kbuild)) => {
                let workspace_root = env::current_dir().expect("Failed to get current directory");

                let kconfig_path = if kbuild.kconfig.is_absolute() {
                    kbuild.kconfig
                } else {
                    workspace_root.join(kbuild.kconfig)
                };

                match kbuild.command {
                    Some(KbuildSubcommand::Build { args }) => {
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, "build", &args);
                    }
                    Some(KbuildSubcommand::Test { args }) => {
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, "test", &args);
                    }
                    Some(KbuildSubcommand::Run { args }) => {
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, "run", &args);
                    }
                    Some(KbuildSubcommand::Check { args }) => {
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, "check", &args);
                    }
                    Some(KbuildSubcommand::Clippy { args }) => {
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, "clippy", &args);
                    }
                    Some(KbuildSubcommand::External(args)) => {
                        if args.is_empty() {
                            eprintln!("Error: No command specified");
                            print_help();
                            process::exit(1);
                        }
                        let cmd = &args[0];
                        let cmd_args = &args[1..];
                        run_cargo_with_kbuild(&workspace_root, &kconfig_path, cmd, cmd_args);
                    }
                    None => {
                        print_help();
                    }
                }
                return;
            }
            Err(e) => {
                eprintln!("{}", e);
                process::exit(1);
            }
        }
    }

    // Legacy cargo-kbuild invocation (without "kbuild" as first arg)
    // This maintains backward compatibility
    let command_args = if args.len() > 1 { &args[1..] } else { &[] };

    if command_args.is_empty() {
        print_help();
        process::exit(1);
    }

    let workspace_root = env::current_dir().expect("Failed to get current directory");

    // Extract --kconfig if present
    let (kconfig_path, remaining_args) = extract_kconfig_arg(command_args);
    let kconfig_path = kconfig_path.unwrap_or_else(|| workspace_root.join(".config"));

    match remaining_args.get(0).map(|s| s.as_str()) {
        Some("--help") | Some("-h") | Some("help") => print_help(),
        Some("--version") | Some("-v") | Some("version") => print_version(),
        Some(cmd) => {
            // Forward ANY command to cargo with kbuild config
            run_cargo_with_kbuild(&workspace_root, &kconfig_path, cmd, &remaining_args[1..]);
        }
        None => {
            // If only --kconfig was provided, show help
            print_help();
            process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_split_tuples_basic() {
        let input = r#"("key1", 123), ("key2", 456)"#;
        let result = split_tuples(input);
        
        assert!(result.is_ok());
        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2);
        assert_eq!(tuples[0], r#"("key1", 123)"#);
        assert_eq!(tuples[1], r#"("key2", 456)"#);
    }

    #[test]
    fn test_split_tuples_with_quoted_parens() {
        let input = r#"("(test)", 123), ("normal", 456)"#;
        let result = split_tuples(input);
        
        assert!(result.is_ok());
        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2);
        assert_eq!(tuples[0], r#"("(test)", 123)"#);
        assert_eq!(tuples[1], r#"("normal", 456)"#);
    }

    #[test]
    fn test_split_tuples_nested_parens() {
        let input = r#"(1, (2, 3)), (4, 5)"#;
        let result = split_tuples(input);
        
        assert!(result.is_ok());
        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2);
        assert_eq!(tuples[0], r#"(1, (2, 3))"#);
        assert_eq!(tuples[1], r#"(4, 5)"#);
    }

    #[test]
    fn test_split_tuples_unmatched_opening() {
        let input = r#"("key", 123"#;
        let result = split_tuples(input);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unmatched opening parenthesis");
    }

    #[test]
    fn test_split_tuples_unmatched_closing() {
        let input = r#"("key", 123))"#;
        let result = split_tuples(input);
        
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), "Unmatched closing parenthesis");
    }

    #[test]
    fn test_split_tuples_with_escaped_quotes() {
        let input = r#"("key with \" quote", 123), ("normal", 456)"#;
        let result = split_tuples(input);
        
        assert!(result.is_ok());
        let tuples = result.unwrap();
        assert_eq!(tuples.len(), 2);
        assert_eq!(tuples[0], r#"("key with \" quote", 123)"#);
        assert_eq!(tuples[1], r#"("normal", 456)"#);
    }
}
