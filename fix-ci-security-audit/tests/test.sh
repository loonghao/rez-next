#!/bin/bash
set -uo pipefail

REWARD_DIR="/logs/verifier"
REWARD_FILE="$REWARD_DIR/reward.json"
mkdir -p "$REWARD_DIR"

cd /workspace

TOTAL=4
PASSED=0

echo "========================================="
echo "  Verifying: fix-ci-security-audit task"
echo "========================================="

# --- Test 1: cargo audit exits with code 0 (no vulnerabilities) ---
echo ""
echo "[1/$TOTAL] Running cargo audit..."
AUDIT_OUTPUT=$(cargo audit 2>&1)
AUDIT_EXIT=$?
if [ $AUDIT_EXIT -eq 0 ]; then
    echo "  PASS: cargo audit exits with code 0"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL: cargo audit exits with code $AUDIT_EXIT"
    echo "  Output (last 20 lines):"
    echo "$AUDIT_OUTPUT" | tail -20 | sed 's/^/    /'
fi

# --- Test 2: cargo check --workspace succeeds ---
echo ""
echo "[2/$TOTAL] Running cargo check --workspace..."
CHECK_OUTPUT=$(cargo check --workspace 2>&1)
CHECK_EXIT=$?
if [ $CHECK_EXIT -eq 0 ]; then
    echo "  PASS: cargo check --workspace succeeds"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL: cargo check --workspace failed with code $CHECK_EXIT"
    echo "  Output (last 20 lines):"
    echo "$CHECK_OUTPUT" | tail -20 | sed 's/^/    /'
fi

# --- Test 3: lru version in workspace Cargo.toml is >= 0.16 ---
echo ""
echo "[3/$TOTAL] Checking lru version in workspace Cargo.toml..."
LRU_LINE=$(grep '^lru' Cargo.toml | head -1)
echo "  Found: $LRU_LINE"
if echo "$LRU_LINE" | grep -qE '"0\.(1[6-9]|[2-9][0-9])'; then
    echo "  PASS: lru version is >= 0.16"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL: lru version is not >= 0.16"
fi

# --- Test 4: rez-next-package uses workspace lru (not pinned 0.12) ---
echo ""
echo "[4/$TOTAL] Checking rez-next-package uses workspace lru..."
PKG_LRU=$(grep 'lru' crates/rez-next-package/Cargo.toml | head -1)
echo "  Found: $PKG_LRU"
if echo "$PKG_LRU" | grep -q 'workspace = true\|workspace=true'; then
    echo "  PASS: rez-next-package uses workspace lru"
    PASSED=$((PASSED + 1))
elif echo "$PKG_LRU" | grep -qE '"0\.(1[6-9]|[2-9][0-9])'; then
    echo "  PASS: rez-next-package uses lru >= 0.16 directly"
    PASSED=$((PASSED + 1))
else
    echo "  FAIL: rez-next-package still uses old lru version"
fi

# --- Summary ---
echo ""
echo "========================================="
echo "  Results: $PASSED/$TOTAL tests passed"
echo "========================================="

# Write reward file
SCORE=$(echo "scale=2; $PASSED / $TOTAL" | bc)
cat > "$REWARD_FILE" <<EOF
{
    "score": $SCORE,
    "tests_passed": $PASSED,
    "tests_total": $TOTAL,
    "cargo_audit_exit_code": $AUDIT_EXIT,
    "cargo_check_exit_code": $CHECK_EXIT
}
EOF

# Also write simple reward.txt
if [ "$PASSED" -eq "$TOTAL" ]; then
    echo 1 > "$REWARD_DIR/reward.txt"
else
    echo 0 > "$REWARD_DIR/reward.txt"
fi

echo ""
echo "Reward written to $REWARD_FILE"
cat "$REWARD_FILE"
