// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Host-side compilation checks for relocation-sensitive patterns.
//!
//! These tests verify that global variables and function pointers compile and
//! link correctly with the PIC/PIE settings used by `kbootloader`. They run on
//! the host and serve as a quick sanity-check that the crate's build
//! configuration (e.g. `relocation-model=pic`) does not break ordinary Rust
//! constructs that would be subject to `R_AARCH64_RELATIVE` relocations on the
//! target.
//!
//! Full runtime verification of the relocation logic must be done by booting
//! the kernel in QEMU with the `kbootloader` feature enabled (see
//! `scripts/test_kbootloader.sh`).

/// Static global variable — would require an `R_AARCH64_RELATIVE` relocation
/// on the target; checks that the PIC build settings do not break this pattern.
#[test]
fn test_global_var_compiles_with_pic() {
    static TEST_VAR: usize = 0x12345678;
    assert_eq!(TEST_VAR, 0x12345678);
}

/// Function pointer — another common source of `R_AARCH64_RELATIVE` relocations;
/// checks that the PIC build settings do not break this pattern.
#[test]
fn test_function_pointer_compiles_with_pic() {
    fn dummy_fn() -> usize {
        42
    }

    let fn_ptr: fn() -> usize = dummy_fn;
    assert_eq!(fn_ptr(), 42);
}
