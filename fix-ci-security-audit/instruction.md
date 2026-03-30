# Fix CI Security Audit Failures in a Rust Project

## Context

You are working on `rez-next`, a Rust workspace project (next-generation Rez package manager).
The project has a GitHub Actions CI pipeline that includes a **Security Audit** job using `rustsec/audit-check`.

A Pull Request (PR #72) has been opened on branch `fix/ci-custom-workflow-v3` which replaces
the reusable CI workflow with a custom one. However, the CI is **failing** because `cargo audit`
reports security vulnerabilities in the project's dependencies.

## Your Task

1. **Clone the repository** from `https://github.com/loonghao/rez-next.git`
2. **Check out branch** `fix/ci-custom-workflow-v3`
3. **Run `cargo audit`** to identify the security vulnerabilities
4. **Fix all vulnerabilities** by upgrading the affected dependencies:
   - Upgrade direct dependencies in `Cargo.toml` files as needed
   - Run `cargo update` to update transitive dependencies in `Cargo.lock`
   - Ensure the project still compiles (`cargo check --workspace`)
5. **Verify** that `cargo audit` exits with code 0 (no vulnerabilities)
6. **Commit and push** the fix to the `fix/ci-custom-workflow-v3` branch

## Known Vulnerabilities (as of 2026-03-28)

- **RUSTSEC-2026-0007**: `bytes` <= 1.10.x — Integer overflow in `BytesMut::reserve` (transitive dependency)
- **RUSTSEC-2026-0002**: `lru` 0.12.x and 0.14.x — `IterMut` violates Stacked Borrows (direct dependency)

## Key Constraints

- The `lru` crate is used in multiple workspace crates with different versions:
  - `Cargo.toml` (workspace): `lru = "0.14.0"`
  - `crates/rez-next-cache/Cargo.toml`: `lru = "0.14"` (not using workspace)
  - `crates/rez-next-package/Cargo.toml`: `lru = "0.12"` (old version, not using workspace)
  - `crates/rez-next-repository/Cargo.toml`: `lru.workspace = true`
  - `crates/rez-next-version/Cargo.toml`: `lru.workspace = true`
- Only `lru::LruCache` is used in the codebase — the API is stable across versions
- `bytes` is a transitive dependency (via `reqwest` → `hyper` → etc.) — `cargo update` is sufficient
- After fixing, `cargo audit` may still show **warnings** (unmaintained crates from `rustpython-parser`) — these are acceptable and do not cause CI failure
- The project must compile successfully after changes (`cargo check --workspace`)

## Success Criteria

1. `cargo audit` exits with code 0 (zero vulnerabilities)
2. `cargo check --workspace` succeeds
3. Changes are committed to the `fix/ci-custom-workflow-v3` branch
4. Modified files: `Cargo.toml`, `Cargo.lock`, `crates/rez-next-cache/Cargo.toml`, `crates/rez-next-package/Cargo.toml`
