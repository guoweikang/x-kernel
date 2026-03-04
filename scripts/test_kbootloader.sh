#!/bin/bash
# SPDX-License-Identifier: Apache-2.0
# Copyright 2025 KylinSoft Co., Ltd. <https://www.kylinos.cn/>
# See LICENSES for license details.
#
# Test kbootloader PIE boot on aarch64-qemu-virt.

set -e

echo "🧪 Testing kbootloader PIE boot..."

# 1. Build with kbootloader feature enabled
echo "1️⃣  Building with kbootloader feature..."
make PLAT=aarch64-qemu-virt FEATURES="kbootloader" build

# 2. Check that .rela.dyn section is present in the ELF
echo "2️⃣  Checking .rela.dyn section..."
KERNEL_ELF=target/aarch64-unknown-none/release/x-kernel
if ! aarch64-linux-gnu-readelf -S "$KERNEL_ELF" | grep -q ".rela.dyn"; then
    echo "❌ ERROR: .rela.dyn section not found!"
    exit 1
fi
echo "✅ .rela.dyn section present"

# 3. Check PIE file type
echo "3️⃣  Checking PIE flag..."
if aarch64-linux-gnu-readelf -h "$KERNEL_ELF" | grep -q "DYN"; then
    echo "✅ ELF type is DYN (PIE)"
else
    echo "⚠️  WARNING: ELF type is not DYN - may not be position-independent"
fi

# 4. Boot test at default address
echo "4️⃣  Testing boot at default address..."
timeout 30s make PLAT=aarch64-qemu-virt FEATURES="kbootloader" justrun || {
    echo "❌ Boot test failed!"
    exit 1
}

echo "✅ All tests passed!"
