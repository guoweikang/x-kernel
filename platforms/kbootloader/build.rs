use std::{env, fs, path::PathBuf};

fn out_dir() -> PathBuf {
    PathBuf::from(env::var("OUT_DIR").unwrap())
}

fn main() {
    let arch = env::var("CARGO_CFG_TARGET_ARCH").unwrap();

    println!("cargo:rerun-if-changed=linker.lds.S");
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rustc-link-search={}", out_dir().display());

    let target = std::env::var("TARGET").unwrap();
    println!("cargo::rustc-check-cfg=cfg(hard_float)");
    if target.as_str() == "aarch64-unknown-none" {
        println!("cargo::rustc-cfg=hard_float");
    }

    let out_dir = out_dir();

    // Read linker script template
    let linker_script_src =
        fs::read_to_string("linker.lds.S").expect("Failed to read linker.lds.S");

    // Replace placeholders with build config
    let kernel_base = env::var("KERNEL_BASE_VADDR").unwrap_or_else(|_| match arch.as_str() {
        "aarch64" => "0xffff000000200000".to_string(),
        "riscv64" => "0xffffffc000200000".to_string(),
        "x86_64" => "0xffff800000200000".to_string(),
        "loongarch64" => "0x9000000000200000".to_string(),
        _ => panic!("Unsupported architecture: {}", arch),
    });

    let linker_script = linker_script_src
        .replace("%ARCH%", &arch)
        .replace("%KERNEL_BASE%", &kernel_base);

    // Write processed linker script
    let linker_script_path = out_dir.join("linker.lds");
    fs::write(&linker_script_path, linker_script).expect("Failed to write linker script");

    println!("cargo:rustc-link-arg=-T{}", linker_script_path.display());
    println!("cargo:rustc-link-arg=-pie");
    println!("cargo:rustc-link-arg=-Bsymbolic");
    println!("cargo:rustc-link-arg=--no-dynamic-linker");
}
