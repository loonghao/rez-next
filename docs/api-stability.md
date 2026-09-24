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

A symbol is covered by SemVer only if it satisfies **both** conditions:

1. **Reachability.** It is a `pub use` re-export at the root of a contract
   crate's `lib.rs`. A `pub mod` leaf path is never part of the contract, even
   when the same type is reachable both ways.

   ```rust
   use rez_next_version::Version;          // contract
   use rez_next_version::version::Version; // NOT contract - implementation detail
   ```

   Both paths currently resolve to the same type, but only the first one is
   promised. Without this condition, every internal refactor would be a breaking
   change.

2. **Closure.** It is an entry point listed under
   [Stable surface](#stable-surface), or a type reachable from one of those
   entry points' signatures - their parameter types, return types, and the types
   of their public fields.

Condition 1 alone is not enough. Several crates re-export internal modules at the
root through a glob (`pub use cache::*;`, `pub use shell::*;`), so a root
`pub use` can still be an implementation detail. Condition 2 is what decides:
**whether a symbol is in the contract depends on whether it appears in the type
closure of a promised signature, not on which module it happens to live in.**

Anything reachable from a crate root that fails condition 2 is listed under
[Not part of the contract](#not-part-of-the-contract).

## Stable surface

The items below are the entry points of the contract. Every type in their
signature closure is covered as well; the list calls out the non-obvious ones.
The full public surface of each crate is what CI compares mechanically; see the
crate rustdoc for the complete list.

A symbol belongs to exactly one of this list and
[Not part of the contract](#not-part-of-the-contract) - never to both.

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
- `get_reverse_dependency_tree`, `get_plugins`, `ResourceSearchResult`

Scanning and caching types are re-exported at this crate's root but are **not**
covered - see [Not part of the contract](#not-part-of-the-contract). No promised
signature depends on them: `Repository` and `PackageRepository` only return
`Package`, `Version`, `String`, `bool`, `RepositoryMetadata`, `RepositoryStats`
and `RezCoreError`.

### `rez-next-solver`

- `SolverRequest`, `SolverConfig`, `SolverStatus`, `ConflictStrategy`
- `DependencySolver`, `DependencyResolver`, `SolverStats`
- `ResolutionResult`, `DetailedResolutionResult`, `ResolvedPackageInfo`,
  `ResolutionConflict`, `ResolutionStats`

`DependencySolver::resolve` takes a `SolverRequest` and returns a
`ResolutionResult`, and neither reaches the A* search internals, so those
internals stay out of the contract.

### `rez-next-context`

- `RezResolvedContext` and its `new` / `get_environ` / `get_tools` /
  `get_package` / `save` / `load` / `get_summary` entry points
- `ResolvedPackage` - returned by `RezResolvedContext::get_package`
- `ResolvedContextSummary` - returned by `get_summary`

## Not part of the contract

These are reachable from crate roots today but are explicitly **not** promised.
Each entry fails condition 2 of [The stability rule](#the-stability-rule): it is
not an entry point, and no promised signature depends on it.

- `rez-next-repository`: the scanning and telemetry types re-exported from
  `scanner_types` - `ScanResult`, `PackageScanResult`, `ScanError`,
  `ScanErrorType`, `ScannerConfig`, `ScanPerformanceMetrics`, `CacheStatistics`,
  `REZ_PACKAGE_FILENAMES`. Also `RepositoryScanner` and the
  `high_performance_scanner` module (`HighPerformanceScanner`,
  `HighPerformanceConfig`, `SIMDPatternMatcher`, `PerformanceStats`), and the
  `cache` module (`CacheEntry`, `CacheConfig`, `RepositoryCache`, `CacheStats`).
- `rez-next-solver`: the A* search internals - `astar::*`, `heuristics`,
  `SearchState`, `Reduction`, `TotalReduction`, `SolverState`
- `rez-next-context`: `shell` and `execution` internals. Both are private `mod`s
  behind a glob `pub use`, so the `pub mod` leaf wording of condition 1 does not
  apply to them - condition 2 excludes them instead. The promised
  `RezResolvedContext` methods only return `HashMap`, `PathBuf`, `Version`,
  `ResolvedPackage`, `ResolvedContextSummary` and `RezCoreError`; none of
  `ShellType`, `ShellExecutor`, `ShellInfo`, `CommandResult`, `ExecutionConfig`,
  `ContextExecutor`, `SpawnedProcess`, `ProcessResult`, `ExecutionStats` or
  `ContextExecutionBuilder` appears there. The rest of the crate's root
  re-exports that no promised signature reaches (`environment`, `serialization`)
  are excluded for the same reason.
- Anything reached through a `pub mod` path instead of a root `pub use`
- Any crate not listed in [Which surface is the contract](#which-surface-is-the-contract)

## Known gaps

These are open decisions, not promises in either direction. Resolve them before
publishing to crates.io.

- **`ResolvedContext` vs `RezResolvedContext`.** `rez-next-context` exports two
  context types at its root. This document used to name `ResolvedContext` while
  listing `new` / `get_environ` / `get_tools` / `get_package` / `save` / `load` /
  `get_summary` - all seven actually live on `RezResolvedContext`, so the name
  was corrected and only `RezResolvedContext` is promised today.
  `ResolvedContext` (from `context`, with `from_requirements` / `get_package` /
  `generate_environment`) is used by the CLI and by `llms-full.txt`, so deciding
  which type is canonical, and what to promise for it, needs its own discussion.
- **`RepositoryManager`.** `DependencyResolver::new` takes an
  `Arc<RepositoryManager>`, and `RepositoryManager` is re-exported from
  `rez-next-repository`'s root but is not in that crate's stable surface. As
  written, the `DependencyResolver` promise cannot be used without depending on
  an unstable type. Promoting `RepositoryManager` is a widening decision and is
  out of scope here; it needs the same discussion as any other API addition.

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

Moving a symbol between [Stable surface](#stable-surface) and
[Not part of the contract](#not-part-of-the-contract) needs both lists edited in
the same commit. Decide it by the type-closure test in
[The stability rule](#the-stability-rule), not by which module the symbol lives
in.
