# cargo-kbuild User Guide

A build tool that brings Linux Kconfig-style global configuration to Rust/Cargo projects.

## Installation

Install from source:

```bash
git clone https://github.com/guoweikang/cargo-test.git
cd cargo-test
cargo install --path cargo-kbuild
```

Verify installation:

```bash
cargo-kbuild --version
```

## Quick Start

### 1. Create Configuration File

Create a `.config` file in your workspace root using external Kconfig tools (like Linux's `make menuconfig`) or manually:

```bash
# Boolean configs (y = enabled, n = disabled)
CONFIG_SMP=y
CONFIG_NET=y
CONFIG_PREEMPT=y

# Integer configs
CONFIG_LOG_LEVEL=3
CONFIG_MAX_CPUS=8

# String configs
CONFIG_DEFAULT_SCHEDULER="cfs"
```

### 2. Enable Kbuild in Crates

Add to your crate's `Cargo.toml`:

```toml
[package.metadata.kbuild]
enabled = true
```

### 3. Use Like Cargo

```bash
# Any cargo command works
cargo-kbuild build
cargo-kbuild test
cargo-kbuild run
cargo-kbuild check
```

With custom config file:

```bash
cargo-kbuild build --kconfig custom.config
cargo-kbuild test --kconfig .config.debug
```

## Cargo Wrapper Mode

cargo-kbuild works as a transparent wrapper around cargo. ANY cargo command can be used:

### Standard Cargo Commands

```bash
# Development
cargo-kbuild check              # Fast compilation check
cargo-kbuild build              # Build project
cargo-kbuild build --release    # Release build

# Testing
cargo-kbuild test               # Run all tests
cargo-kbuild test --lib         # Test library only
cargo-kbuild test integration   # Run specific test

# Running
cargo-kbuild run                # Run default binary
cargo-kbuild run --bin demo     # Run specific binary
cargo-kbuild run -- --help      # Pass args to binary

# Code Quality
cargo-kbuild clippy             # Run clippy linter
cargo-kbuild clippy -- -D warnings  # Fail on warnings
cargo-kbuild fmt                # Format code

# Documentation
cargo-kbuild doc                # Build documentation
cargo-kbuild doc --open         # Build and open docs

# Benchmarks
cargo-kbuild bench              # Run benchmarks
```

### Custom Configuration

```bash
# Use different config file
cargo-kbuild test --kconfig .config.debug
cargo-kbuild run --kconfig configs/production.config
```

### Passing Arguments

```bash
# Arguments before -- go to cargo
cargo-kbuild test --release --lib

# Arguments after -- go to the test binary
cargo-kbuild test -- --nocapture --test-threads=1

# Combined
cargo-kbuild run --release -- --verbose --input data.txt
```

## Architecture Overview

### Core Principle

**`.config` is EXTERNAL** - cargo-kbuild reads it, never creates it.

```
External Kconfig Tool → .config → cargo-kbuild → Compiled Project
(make menuconfig)       (read)    (apply)       (ready to run)
```

### What cargo-kbuild Does

1. **Read** existing `.config` file
2. **Generate** `target/kbuild/config.rs` with constants
3. **Generate** `.cargo/config.toml` for zero warnings
4. **Set** RUSTFLAGS with `--cfg` flags
5. **Validate** dependency relationships
6. **Call** `cargo build` with appropriate flags

### What cargo-kbuild Does NOT Do

- ❌ Generate `.config` files
- ❌ Provide interactive configuration UI
- ❌ Manage config templates

## Using Configurations

### Boolean Configurations

Use directly in code with `#[cfg]` attributes:

```rust
#[cfg(CONFIG_SMP)]
fn init_smp() {
    println!("SMP mode enabled");
}

#[cfg(not(CONFIG_SMP))]
fn init_single_core() {
    println!("Single-core mode");
}
```

**No declaration needed in Cargo.toml** - cargo-kbuild handles it automatically.

### Integer and String Configurations

First, add the `kbuild_config` dependency:

```toml
[dependencies]
kbuild_config = { path = "../kbuild_config" }
```

Then use in code:

```rust
use kbuild_config::*;

fn init() {
    println!("Log level: {}", CONFIG_LOG_LEVEL);
    println!("Max CPUs: {}", CONFIG_MAX_CPUS);
    println!("Scheduler: {}", CONFIG_DEFAULT_SCHEDULER);
}
```

## Feature Declaration Rules

### When to Declare CONFIG_* Features

**ONLY when you have optional dependencies:**

```toml
# ✅ CORRECT: Has optional dependency
[dependencies]
kernel_net = { path = "crates/kernel_net", optional = true }

[features]
CONFIG_NET = ["kernel_net"]  # Enables the optional dependency
```

### When NOT to Declare Features

**When using configs in code without optional dependencies:**

```toml
# ✅ CORRECT: Using CONFIG_SMP in code, no optional deps
[package.metadata.kbuild]
enabled = true

# No [features] section needed
```

```rust
// Code can use CONFIG_SMP directly
#[cfg(CONFIG_SMP)]
fn init_smp() {
    println!("SMP enabled");
}
```

### Comparison

❌ **WRONG** (old approach):
```toml
[features]
CONFIG_SMP = []       # Don't do this
CONFIG_PREEMPT = []   # No optional dependency
CONFIG_LOGGING = []   # Just using in code
```

✅ **CORRECT** (new architecture):
```toml
# Option 1: No optional dependencies
[package.metadata.kbuild]
enabled = true
# No [features] section

# Option 2: Has optional dependencies
[dependencies]
tokio = { version = "1.0", optional = true }

[features]
CONFIG_ASYNC = ["tokio"]  # Only because of optional dep
```

## Dependency Validation

cargo-kbuild enforces strict validation rules:

### ✅ Allowed Dependencies

1. **Kbuild-enabled crate without sub-features**:
   ```toml
   [features]
   CONFIG_NET = ["network_utils"]  # ✅ Correct
   ```

2. **Third-party library with sub-features**:
   ```toml
   [features]
   CONFIG_LOGGING = ["log/std"]    # ✅ Correct
   CONFIG_ASYNC = ["tokio/rt"]     # ✅ Correct
   ```

3. **Non-kbuild internal crate with sub-features**:
   ```toml
   [features]
   CONFIG_LEGACY = ["legacy_driver/usb"]  # ✅ Correct
   ```

### ❌ Prohibited Dependencies

Cannot specify sub-features for kbuild-enabled dependencies:

```toml
[features]
# ❌ WRONG! network_utils is kbuild-enabled
CONFIG_NET = ["network_utils/async"]
```

**Why?** Kbuild-enabled crates read their own configs from `.config`. Parent crates cannot control them via sub-features.

## Configuration File Format

The `.config` file uses simple key-value pairs:

```bash
# Comments start with #

# Boolean values: y = enabled, n = disabled
CONFIG_SMP=y
CONFIG_DEBUG=n

# Integer values
CONFIG_LOG_LEVEL=3
CONFIG_MAX_CPUS=8

# String values: use double quotes
CONFIG_DEFAULT_SCHEDULER="cfs"
CONFIG_ARCH="x86_64"

# Disabled features can be commented out
# CONFIG_EXPERIMENTAL=y
```

## Auto-Generated Files

cargo-kbuild generates the following files (do NOT commit to git):

### 1. `.cargo/config.toml`

Declares all `CONFIG_*` options to avoid "unexpected cfg" warnings:

```toml
# Auto-generated by cargo-kbuild
[build]
rustflags = [
    "--check-cfg=cfg(CONFIG_SMP)",
    "--check-cfg=cfg(CONFIG_NET)",
    # ... all CONFIG_* from .config
]
```

### 2. `target/kbuild/config.rs`

Contains integer and string constants:

```rust
// Auto-generated by cargo-kbuild from .config
pub const CONFIG_LOG_LEVEL: i32 = 3;
pub const CONFIG_MAX_CPUS: i32 = 8;
pub const CONFIG_DEFAULT_SCHEDULER: &str = "cfs";
```

These files are regenerated on every build.

## Common Scenarios

### Scenario 1: Adding a New Feature

1. Use the config in your code:
   ```rust
   #[cfg(CONFIG_NEW_FEATURE)]
   fn new_feature() {
       println!("New feature enabled");
   }
   ```

2. Enable in `.config`:
   ```bash
   CONFIG_NEW_FEATURE=y
   ```

3. Build:
   ```bash
   cargo-kbuild build
   ```

No Cargo.toml changes needed (unless you have optional dependencies).

### Scenario 2: Multiple Configuration Files

Maintain different configs for different environments:

```bash
# Development
cargo-kbuild build --kconfig .config.dev

# Production
cargo-kbuild build --kconfig .config.prod

# Testing
cargo-kbuild build --kconfig .config.test
```

### Scenario 3: Debugging Configuration

1. Check enabled features:
   ```bash
   grep "=y" .config
   ```

2. View generated rustflags:
   ```bash
   cat .cargo/config.toml
   ```

3. View generated constants:
   ```bash
   cat target/kbuild/config.rs
   ```

## Commands

### `cargo-kbuild build`

Build the project with current configuration.

**Options**:
- `--kconfig <path>`: Specify config file (default: `.config`)

**Examples**:
```bash
# Use default .config
cargo-kbuild build

# Use custom config file
cargo-kbuild build --kconfig custom.config
```

### `cargo-kbuild --help`

Display help information.

```bash
cargo-kbuild --help
```

### `cargo-kbuild --version`

Display version information.

```bash
cargo-kbuild --version
```

## How It Works

### Build Flow

```
1. Parse workspace
   ├─ Read Cargo.toml files
   ├─ Identify kbuild-enabled crates
   └─ Build dependency graph

2. Read .config file
   └─ Extract all CONFIG_* options

3. Generate .cargo/config.toml
   └─ Declare all CONFIG_* for check-cfg

4. Generate target/kbuild/config.rs
   └─ Create constants for int/string values

5. Validate dependencies
   ├─ Check kbuild-enabled crates
   └─ Prevent sub-features on kbuild deps

6. Build project
   ├─ Set RUSTFLAGS with --cfg flags
   ├─ Pass features to cargo (only declared ones)
   └─ Execute cargo build
```

### Global Configuration Model

Unlike Cargo features (tree-based propagation), kbuild uses global sharing:

```
Cargo Features:              cargo-kbuild:

    root                         .config
     |                              |
     +-- child1                ├── crate1
     |   |                     ├── crate2
     +-- child2                ├── crate3
         |                     └── crate4
         +-- child3
                              All crates read
Tree propagation             from same .config
```

## Troubleshooting

### Error: .config file not found

**Solution**: Create `.config` file manually or use external Kconfig tools.

### Error: Cannot specify sub-feature for kbuild dependency

```
❌ Error in crate 'my-crate':
Feature 'CONFIG_NET' specifies sub-feature: 'network_utils/async'
```

**Solution**: Remove the sub-feature, let the dependency read its own config:

```toml
# Before
CONFIG_NET = ["network_utils/async"]

# After
CONFIG_NET = ["network_utils"]
```

Then enable the config in `.config`:
```bash
CONFIG_ASYNC=y
```

### Warning: unexpected cfg condition

```
warning: unexpected `cfg` condition name: `CONFIG_XXX`
```

**Solution**: Always use `cargo-kbuild build` instead of `cargo build`. This ensures `.cargo/config.toml` is generated.

## Best Practices

1. **Always use cargo-kbuild commands**
   - Use `cargo-kbuild build` not `cargo build`
   - Ensures `.cargo/config.toml` is up-to-date

2. **Commit .config to git**
   - Provides default configuration
   - Facilitates team collaboration

3. **Do NOT commit auto-generated files**
   - `.cargo/config.toml`
   - `target/kbuild/`

4. **Use consistent naming**
   - All global configs use `CONFIG_` prefix
   - Use uppercase and underscores: `CONFIG_MY_FEATURE`

5. **Document your configurations**
   - Add comments in `.config` explaining each option
   - List available configs in README

## Example Projects

The repository's `crates/` directory contains complete examples:

- **kernel_irq** - Basic kbuild usage, no features
- **kernel_task** - Depends on other kbuild crates
- **kernel_schedule** - Multiple CONFIG_* usage
- **kernel_net** - Used as optional dependency
- **demo_mixed_deps** - Using integer/string constants
- **legacy_driver** - Non-kbuild crate example

Check their `Cargo.toml` files to learn configuration patterns.

## Technical Details

For implementation details, see:
- [IMPLEMENTATION_DETAILS.md](../IMPLEMENTATION_DETAILS.md) - Technical architecture
- [IMPLEMENTATION_SUMMARY.md](../IMPLEMENTATION_SUMMARY.md) - Feature overview

## Support

Found an issue or have a suggestion? Please open an issue on GitHub.
