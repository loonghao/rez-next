#!/bin/bash
set -euo pipefail

cd /workspace

echo "=== Step 1: Verify initial state has vulnerabilities ==="
if cargo audit 2>&1; then
    echo "ERROR: Expected cargo audit to fail, but it passed"
    exit 1
fi
echo "Confirmed: vulnerabilities exist in initial state"

echo ""
echo "=== Step 2: Upgrade lru in workspace Cargo.toml (0.14.0 -> 0.16) ==="
sed -i 's/^lru = "0\.14\.0"/lru = "0.16"/' Cargo.toml

echo "=== Step 3: Fix rez-next-package to use workspace lru (0.12 -> workspace) ==="
sed -i 's/^lru = "0\.12"/lru.workspace = true/' crates/rez-next-package/Cargo.toml

echo "=== Step 4: Fix rez-next-cache to use workspace lru (0.14 -> workspace) ==="
sed -i 's/^lru = "0\.14"/lru.workspace = true/' crates/rez-next-cache/Cargo.toml

echo "=== Step 5: Update Cargo.lock (upgrades bytes, slab, lru, etc.) ==="
cargo update

echo "=== Step 6: Verify compilation ==="
cargo check --workspace
echo "Compilation: OK"

echo "=== Step 7: Verify cargo audit passes ==="
cargo audit
echo ""
echo "=== All vulnerabilities resolved ==="
