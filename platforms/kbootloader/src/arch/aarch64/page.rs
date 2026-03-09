// SPDX-License-Identifier: Apache-2.0
// Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
// See LICENSES for license details.

pub const VA_BITS : usize = 48;
pub const PG_VA_BITS: usize = 48;
pub const PAGE_LEVELS: usize = 4;
pub const PAGE_SHIFT: usize = 12;
pub const PAGE_SIZE: usize = 1usize << PAGE_SHIFT;

const fn _page_offset(va: usize) -> usize {
    !((1usize << va) - 1)
}

const fn _page_end(va: usize) -> usize {
    !((1usize << va) - 1)
}
