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

Anything reachable from a crate root that fails condition 2 is not covered. The
[Not part of the contract](#not-part-of-the-contract) list names the non-obvious
cases and is not exhaustive.

## Stable surface

The items below are the entry points of the contract. Every type in their
signature closure is covered as well; the list calls out the non-obvious ones.
The full public surface of each crate is what CI compares mechanically; see the
crate rustdoc for the complete list.

A symbol never appears on both this list and
[Not part of the contract](#not-part-of-the-contract). Neither list is
exhaustive: a root-reachable symbol on neither list is not covered.

### `rez-next-common`

- `RezCoreError`, `RezCoreResult`, `RezCoreConfig` (the last is
  `#[non_exhaustive]` - see [Current exemptions](#current-exemptions))

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
  `RezResolvedContext` methods only return `Self`, `HashMap`, `PathBuf`,
  `String`, `ResolvedPackage`, `ResolvedContextSummary` and `RezCoreError`; its
  public fields add only `bool`, `Vec<Requirement>` and `chrono::DateTime<Utc>`
  (for `timestamp`). None of
  `ShellType`, `ShellExecutor`, `ShellInfo`, `CommandResult`, `ExecutionConfig`,
  `ContextExecutor`, `SpawnedProcess`, `ProcessResult`, `ExecutionStats` or
  `ContextExecutionBuilder` appears there. The rest of the crate's root
  re-exports that no promised signature reaches (`environment`, `serialization`,
  `context`) are excluded for the same reason.
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

The crates are published to crates.io, but the gate still compares against a
**git revision**: the merge base with the target branch on pull requests, and
`HEAD~1` on pushes. A git baseline also sees changes that were never released,
so it is the stricter of the two; moving to registry baselines remains an option
once the release cadence settles.

Note that publishing and the gate interact: `just runtime-package-check` runs
`cargo package`, which resolves internal dependencies through the crates.io
index rather than the local path. A workspace-wide bump therefore fails on
`Runtime SDK` until every dependency is published at the new version - see
[When the gate fails](#when-the-gate-fails).

### When the gate fails

The gate compares the *whole* public API of each crate, so it will also flag
changes inside `pub mod` leaves that this document already excludes. Resolve a
finding in one of three ways, in order of preference:

1. **Make the change additive.** Add the new API, keep the old one working, and
   mark it `#[deprecated]`. Nothing breaks and no bump is needed.
2. **Bump the crate's minor version in the same PR** if the break is intended and
   unavoidable. Keep `.release-please-manifest.json`, the workspace, and every
   `x-release-please-version` marker in sync - the `Release Version Consistency`
   job enforces this on every PR. The release tooling manages a single root
   version, so a bump moves the whole workspace.

   This route is unavailable while a dependency that the bumped crate requires
   is not yet published at the new version: `just runtime-package-check` resolves
   internal dependencies through the crates.io index, so a bump fails there
   until the dependency is available. Publish dependencies first, or pick
   another route.
3. **Narrow the lint** for a finding that only touches a non-contract leaf, using
   `[package.metadata.cargo-semver-checks.lints]` in that crate's `Cargo.toml`.
   Say so explicitly in the PR description.

### Current exemptions

Every exemption is recorded here so the contract never drifts silently.

`rez-next-common` narrows `struct_marked_non_exhaustive` to a warning.
`RezCoreConfig` is marked `#[non_exhaustive]` because it keeps growing: modelling
a further upstream setting adds a field, and the attribute keeps that additive
instead of breaking every downstream struct literal. Marking the struct is itself
a one-off major break, which is the debt `rez-next-common` owes until its next
minor bump; the attribute disables `constructible_struct_adds_field` permanently,
so no exemption is needed for field growth. Construct via
`RezCoreConfig::default()` and assign fields rather than writing a struct
literal, and pattern matches must keep a wildcard arm. Exemptions use `warn`
rather than `allow` so a regression stays visible in CI output.

`rez-next-common` narrows `enum_marked_non_exhaustive` to a warning for the same
reason. `RezCoreError` is marked `#[non_exhaustive]` because it gains a variant
whenever a subsystem needs to report a distinct failure — the CLI already maps
variants to exit codes, so growth is expected rather than hypothetical. The
attribute keeps that growth additive instead of breaking every downstream match,
which must now keep a wildcard arm. Marking the enum is itself a one-off major
break, so the lint is downgraded to `warn` rather than `allow`, keeping it
visible in CI output.

### Known follow-ups

Config-related debt that is tracked but not yet resolved, recorded here so it is
visible alongside the exemptions above.

- **`RezCoreConfig` and `RezCoreError` need a minor bump, which now means
  0.4.0.** The `#[non_exhaustive]` attributes above are one-off major breaks
  that landed without a version bump, so a consumer running `cargo update` would
  receive them silently. `RezCoreConfig` shipped that way in 0.3.12, so 0.3.x
  can no longer absorb the bump; both must land in the next minor release that
  includes `rez-next-common`. Tracked separately.
- **String-list settings are not path lists, on either input channel.**
  `implicit_packages`, `parent_variables` and `resetting_variables` are
  registered as `KeyType::StrList`. Upstream rez separates `StrList` values with
  commas and whitespace; only `PathList` splits on `os.pathsep`. Two code paths
  must both honour this:
  - config-file values go through `assign_str_list`, which keeps a bare string
    as a single element;
  - plain `REZ_*` environment values are coerced by `coerce_env_value` before
    any assignment helper sees them, and use `split_env_str_list` (commas and
    whitespace).
    Coercing them with `split_env_paths` would break package requests
    containing `:` on unix.

  A fix applied to only one of the two leaves the other reachable route broken.
- **Upstream default values are asserted against an independent snapshot.** The
  alignment test transcribes rez 3.4.0's defaults separately from `rez_default`
  so the two are compared, not merely restated. Extending `rez_default` means
  extending that snapshot too.

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
