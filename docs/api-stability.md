# API Stability Contract

This document defines which parts of rez-next are covered by SemVer, and how that
promise is enforced in CI.

rez-next is pre-1.0 and every crate currently sits at `0.3.6`. Under the
[0.x SemVer rules](https://doc.rust-lang.org/cargo/reference/semver.html), a
breaking change requires a **minor** bump (`0.3.x` -> `0.4.0`) and an additive
change requires a **patch** bump.

## Which surface is the contract

| Surface | Contract | Rationale |
| --- | --- | --- |
| Fine-grained crates: `rez-next-common`, `rez-next-version`, `rez-next-package`, `rez-next-repository`, `rez-next-solver`, `rez-next-context` | **Yes** | This is the library contract. Downstream Rust consumers depend on these crates directly and get in-process resolution and package scanning. |
| Top-level `rez-next` crate | **No** | CLI facade. It pulls in the whole CLI/TUI stack (`ratatui`, `crossterm`, `tokio`, ...) and re-exports only a subset of the workspace. Do not depend on it as a library. |
| PyO3 module `rez_next` | **No** | The Python-facing API. Making a Rust consumer depend on a Python runtime is not acceptable; the wheel is versioned by its own release. |
| `rez-next` / `rez` CLI output | **No** | Human-readable and unversioned. Use the Rust crates or the Python API for programmatic access. |

## The stability rule

**Only the `pub use` re-exports at the root of a crate's `lib.rs` are part of the
stability contract. `pub mod` leaf modules are implementation details and may
change in any release without a version bump.**

```rust
use rez_next_version::Version;          // contract
use rez_next_version::version::Version; // NOT contract - implementation detail
```

Both paths currently resolve to the same type, but only the first one is
promised. This rule is what makes the boundary real: without it, "stable" would
only describe the crate root, and every internal refactor would be a breaking
change.

## Stable surface

The items below are the entry points of the contract. The full public surface of
each crate is what CI compares mechanically; see the crate rustdoc for the
complete list.

### `rez-next-common`

- `RezCoreError`, `RezCoreResult`, `RezCoreConfig`

### `rez-next-version`

- `Version` - parsing, comparison, ordering
- `VersionRange` - range algebra and `contains`
- `VersionParser`, `StateMachineParser` - the underlying parsers

### `rez-next-package`

- `Package`, `PackageRequirement`, `Requirement`, `VersionConstraint`
- `PackageFormat`, `PackageSerializer`

### `rez-next-repository`

- `PackageRepository` trait and `FilesystemPackageRepository`
- `Repository`, `RepositoryStats`, `RepositoryMetadata`, `RepositoryType`,
  `PackageSearchCriteria`, `deduplicate_packages`
- Resource model: `PackageResource`, `PackageFamilyResource`, `VariantResource`,
  `ResourceHandle`, `ResourcePool`
- Scanning: `ScanResult`, `PackageScanResult`, `ScanError`, `ScanErrorType`,
  `ScannerConfig`, `ScanPerformanceMetrics`, `CacheStatistics`,
  `REZ_PACKAGE_FILENAMES`
- `get_reverse_dependency_tree`, `get_plugins`, `ResourceSearchResult`

### `rez-next-solver`

- `SolverRequest`, `SolverConfig`, `SolverStatus`, `ConflictStrategy`
- `DependencySolver`, `DependencyResolver`, `SolverStats`
- `ResolutionResult`, `DetailedResolutionResult`, `ResolvedPackageInfo`,
  `ResolutionConflict`, `ResolutionStats`

### `rez-next-context`

- `ResolvedContext` and its `new` / `get_environ` / `get_tools` / `get_package` /
  `save` / `load` / `get_summary` entry points
- `ResolvedContextSummary`

## Not part of the contract

These are reachable from crate roots today but are explicitly **not** promised:

- `rez-next-solver`: the A* search internals - `astar::*`, `heuristics`,
  `SearchState`, `Reduction`, `TotalReduction`, `SolverState`
- `rez-next-repository`: `high_performance_scanner`, `scanner_types`, `cache`
- `rez-next-context`: `shell`, `execution` internals
- Anything reached through a `pub mod` path instead of a root `pub use`
- Any crate not listed in [Which surface is the contract](#which-surface-is-the-contract)

## Enforcement

The `API Stability` job in `.github/workflows/ci.yml` runs
[`cargo-semver-checks`](https://github.com/obi1kenobi/cargo-semver-checks) over
the six contract crates. Run it locally with:

```bash
vx just semver-check           # compare against origin/main
vx just semver-check <rev>     # compare against an explicit revision
```

The baseline revision must be reachable locally, so `git fetch` first.

### Baseline selection

No crate is published to crates.io yet, so a registry baseline is not available.
The gate compares against a **git revision** instead: the merge base with the
target branch on pull requests, and `HEAD~1` on pushes. Once the crates are
published, the gate can move to registry baselines, which is the more accurate
comparison - a git baseline also sees changes that were never released.

### When the gate fails

The gate compares the *whole* public API of each crate, so it will also flag
changes inside `pub mod` leaves that this document already excludes. Resolve a
finding in one of three ways, in order of preference:

1. **Make the change additive.** Add the new API, keep the old one working, and
   mark it `#[deprecated]`. Nothing breaks and no bump is needed.
2. **Bump the crate's minor version in the same PR** if the break is intended and
   unavoidable. Keep `.release-please-manifest.json`, the workspace, and every
   `x-release-please-version` marker in sync - the `Release Version Consistency`
   job enforces this on every PR.
3. **Narrow the lint** for a finding that only touches a non-contract leaf, using
   `[package.metadata.cargo-semver-checks.lints]` in that crate's `Cargo.toml`.
   Say so explicitly in the PR description.

## Changing this document

Adding a crate to the contract means three edits, and all of them are required:

1. Add the crate to `api_crates` in the `justfile`.
2. Document its stable surface here.
3. Re-run `vx just semver-check` so the first run is a clean baseline.

Removing a crate from the contract is a breaking change for its consumers and
needs the same discussion as any other API break.
