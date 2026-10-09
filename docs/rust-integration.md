# Rust embedding API

Use `rez-next-runtime` to embed local repository loading, dependency resolution,
environment export and direct command execution. It is the supported high-level
integration boundary and reuses the repository manager, strict solver,
materialized packages and environment manager from rez-next.

The root `rez-next` crate also re-exports these types as `rez_core::runtime`.
New integrations can use the smaller standalone crate directly:

```toml
[dependencies]
rez-next-runtime = "0.3.9"
tokio = { version = "1", features = ["rt-multi-thread", "macros"] }
```

## Resolve an unpacked repository

```rust,no_run
use std::collections::HashMap;
use std::path::PathBuf;

use rez_next_runtime::RezRuntime;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let parent: HashMap<String, String> = std::env::vars().collect();
    let resolved = RezRuntime::new([PathBuf::from("/cache/unpacked")])?
        .with_target("linux", "x86_64")?
        .with_parent_environment(parent)
        .resolve(["rust-1.95.0"])
        .await?;

    let status = resolved.command("rustc").arg("--version").status()?;
    assert!(status.success());
    Ok(())
}
```

Repository paths must already exist. Downloading, archive extraction and cache
ownership belong to the embedding application. Package `{root}` expansion and
generated `<PACKAGE>_ROOT` variables use the real descriptor directory, including
the selected non-hashed variant subdirectory. No `/packages/<name>` layout is
assumed. Hashed-variant payload lookup is outside the current solver's supported
materialization behavior.

Package discovery uses a private read-only repository bridge over the canonical
`repository/family[/version]/package.*` layout. It inspects only family and
version directories, choosing `package.py`, then `package.yaml`, then
`package.yml` in each directory. A family-level descriptor stops discovery
below that family, so application payload directories are never traversed.
Malformed higher-priority definitions produce an error rather than falling
back to another format. Repository-root descriptors and deeper layouts are not
package locations. All formats share the same core solver.
Python uses `PackageSerializer`; YAML uses the public core `Package`
deserializer and validation because the legacy 0.3.9 YAML serializer omits Rex
commands. YAML `commands` is a string containing the same Rex operations used by
Python package definitions.
Relative repository paths become absolute at construction so a child's working
directory cannot change package root or PATH meaning.

`ResolvedEnvironment::context()` exposes the resolved packages and context.
`environment()` returns the generated map; `into_parts()` consumes the result
into both components. The context's `environment_vars` contains the same map.
`REZ_USED_REQUEST`, `REZ_USED_RESOLVE`, `REZ_USED_PACKAGES_NAMES`,
`REZ_USED_PACKAGES_PATH`, `REZ_USED_VERSION` and `REZ_USED_TIMESTAMP` record the
activation inputs and result.

## Parent environment and execution

The default context excludes the ambient parent environment. Calling
`with_parent_environment(map)` uses exactly that complete map as the base and
applies package actions afterward. An empty map explicitly isolates activation;
it never falls back to the process environment. Windows environment keys are
folded to avoid separate `Path` and `PATH` entries.

`with_context_config(ContextConfig)` exposes the existing core configuration,
including additional variables, unsets and PATH strategy. Supplying an explicit
parent continues to take precedence over ambient inheritance regardless of
builder call order. Selective parent-variable inheritance is outside the 0.3.9
core API.

`command(program)` returns `std::process::Command` with the ambient environment
cleared and the resolved map applied. Add arguments and `current_dir` through
the standard API. It invokes the program directly, preserving argument
boundaries and the child exit status. Windows `.bat` and `.cmd` files follow
the operating system's batch execution behavior; prefer native executables when
untrusted arguments require shell-independent forwarding.

## Explicit target constraints

No target is inferred by default. `with_target(platform, architecture)` adds
exact platform and architecture constraints before resolving package variants.
The repository must provide the corresponding `platform` and `arch` families.
Platforms are `windows`, `linux` and `osx`; `macos` and `darwin` normalize to
`osx`. Architecture values use the repository's Rez version spelling, such as
`x86_64`, `AMD64` or `aarch64`.

Compatible explicit positive requests are intersected with the exact target.
Contradictory requests and explicit weak/conflict requests for either target
family return `IncompatibleTargetRequirement`. For example, a Windows target
rejects `platform-linux`, `~platform-windows` and `!platform-linux`, instead of
returning a context whose declaration disagrees with its packages. Selecting
another target does not make its executables runnable on the current host.

## Errors and compatibility

`RezRuntimeError` classifies invalid repositories and requirements, unsupported
targets, incompatible explicit target requests, missing package families,
repository failures, solver failures and environment failures. Lower-level
failures remain available as error sources. A missing transitive dependency or
unsatisfied version produces `Resolve`; no partial environment is returned.
`with_solver_config` changes preferences while retaining `strict_mode = true`.

The facade supports the package and solver behavior shipped by the 0.3.9 core
crates. Ephemeral resolution and selective parent-variable policy are not part
of this initial embedding API. No parallel dependency solver is introduced.

The public types and methods in `rez_next_runtime` are the supported embedding
surface. Patch releases do not intentionally break it. Before 1.0, a required
breaking change belongs in a minor release with migration guidance.

The exposed context retains its 0.3.9 core behavior: its low-level `validate()`
does not understand weak/conflict requests, and formatting its
`PackageRequirement` values does not round-trip operator syntax. Resolution
validates the original request strings through the strict solver. Preserve
those strings when re-resolving; `REZ_USED_REQUEST` records them for inspection.

## SDK verification

```bash
vx just runtime-test
vx just runtime-package-check
# After the required core versions are public on crates.io:
vx just runtime-registry-check
```

The package check stages and builds the SDK together with its complete core
dependency closure using Cargo's multi-package support. This permits checking
unpublished core API changes without pretending those APIs already exist in the
public registry. The normalized SDK manifest must contain only crates.io
dependencies, including target-specific dependencies.

The separate registry check packages the SDK alone and runs all its external
contracts from the normalized archive manifest against public dependencies.
Run it from a clean checkout after the required core versions are published;
it must pass before publishing or adopting the SDK in another package.
