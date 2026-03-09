// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

//! Unified position-independent boot layer for x-kernel (AArch64 support).

#![cfg_attr(target_os = "none", no_std)]
#![cfg(target_os = "none")]

pub mod bootinfo;
pub mod size_const;

mod arch;
