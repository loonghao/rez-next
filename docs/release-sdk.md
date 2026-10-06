# Release SDK (Rust)

`rez-next-build` exposes the package release workflow as a Rust API, so an
external crate can drive a release in-process — including with its own version
control implementation instead of the git/hg/svn detection built in.

This is the API to use when you want to embed releasing in another tool. If you
only need to run a release from the command line, use `rez-next release` or the
[vx extension](./vx-extension.md); both sit on top of the same workflow.

## Quick start

```rust
use rez_next_build::{ReleaseManager, ReleaseMode};

let manager = ReleaseManager::new(ReleaseMode::Local, false, true);
let result = manager.release(std::path::Path::new("."), Some("1.2.0"))?;
if !result.success {
    for e in &result.errors {
        eprintln!("release failed: {e}");
    }
}
```

`release()` detects the VCS by looking for `.git`, `.hg`, or `.svn` in the
source directory. Pass `vcs: None` explicitly through `release_with_vcs()` for
the same behaviour.

## Injecting your own VCS

Implement [`ReleaseVCS`] and hand it to `release_with_vcs()`:

```rust,no_run
use rez_next_build::{ReleaseManager, ReleaseMode, ReleaseVCS, VCSMetadata};
use rez_next_common::RezCoreError;
use std::path::PathBuf;
use std::sync::Arc;

struct MyVCS;

impl ReleaseVCS for MyVCS {
    fn get_type_name(&self) -> &str { "my-vcs" }
    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> { todo!() }
    fn is_clean(&self) -> Result<bool, RezCoreError> { todo!() }
    fn get_current_branch(&self) -> Result<String, RezCoreError> { todo!() }
    fn get_latest_commit(&self) -> Result<String, RezCoreError> { todo!() }
    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> { todo!() }
    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> { todo!() }
    fn get_changelog(&self, from: Option<&str>, to: Option<&str>)
        -> Result<String, RezCoreError> { todo!() }
    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> { todo!() }
}

let manager = ReleaseManager::new(ReleaseMode::Local, false, true);
let result = manager.release_with_vcs(
    std::path::Path::new("."),
    None,
    Some(Arc::new(MyVCS)),
)?;
# Ok::<(), RezCoreError>(())
```

Nine methods are required. `validate_repo_state()`, `is_releasable_branch()`,
`get_current_revision()`, and `export()` have default implementations.

Two behaviours the release flow relies on:

- **`create_tag` must not move an existing tag.** The flow never asks it to —
  see [Tag policy](#tag-policy).
- **A returned `Err` is never fatal.** Metadata, changelog, and tag-check
  failures are downgraded to warnings so a degraded VCS cannot block a release.
  The one exception is `validate_repo_state()` failing while VCS validation is
  enabled, which aborts before anything is built.

An injected VCS is still validated, so supplying one cannot bypass the
repository-state check.

## Reading the result

`ReleaseResult` is returned as `Ok` even when the release was rejected. **Check
`success` (equivalently, `errors.is_empty()`) before anything else** — `Ok` does
not mean the release shipped.

| Field | Meaning |
| --- | --- |
| `success` | `errors.is_empty()` |
| `package_name`, `version` | From the package definition |
| `install_path` | Where the package was installed; prefixed `[dry-run] ` in dry-run mode |
| `vcs_metadata` | VCS metadata; also written to `vcs_metadata.json` in `install_path` |
| `changelog` | From the VCS, when one was available |
| `errors` | Human-readable failures; non-empty implies `success == false` |
| `warnings` | Non-fatal notes. A release can succeed with warnings. |

`errors` and `warnings` are diagnostic text for logs. Match on `success` and
treat the strings as display-only — their wording is not a stable contract.

`Err(RezCoreError)` means the workflow itself could not run, not that the
release was rejected.

## Release modes

| Mode | Behaviour |
| --- | --- |
| `Release` | Install into `release_packages_path`; falls back to `local_packages_path` when unset or still the default |
| `Local` | Install into `local_packages_path` |
| `DryRun` | Validate only — no build, no install, no VCS writes |

Dry run returns after the package definition and repository state are validated,
so it touches neither the filesystem nor the VCS.

## Tag policy

The release tag is named `{name}-{version}`. The workflow checks it **before**
building (step 3) and creates it **last**, after the package is installed
(step 9). That ordering maintains the invariant:

> **tag exists ⟺ release succeeded**

A build or test failure therefore leaves no tag behind, and the same version can
be retried without an override.

`set_ignore_existing_tag()` selects how an existing tag is treated:

| Value | When the tag already exists |
| --- | --- |
| `Some(false)` | **Hard failure** — errors before anything is built or installed |
| `Some(true)` | Warn, install, report success. The existing tag is **kept** — never re-created, never moved |
| `None` *(default)* | Legacy: warn and continue, same as `Some(true)` |

What this means for a caller:

- A successful release guarantees the tag exists, but **not that this run
  created it**. Check `warnings` for `Tag '<name>' already exists` to tell the
  two apart.
- Under `Some(false)`, a successful release always created the tag itself.
- When a release does go through over an existing tag, the tag stays on its
  original commit. Re-creating it would detach a released version from its
  provenance, which is exactly what strict mode prevents.

## Stability

`rez-next-build` is not in the [API stability contract](./api-stability.md)
crate list, so it is not covered by `cargo-semver-checks` in CI. Pin a released
version and read the changelog before upgrading.

[`ReleaseVCS`]: https://docs.rs/rez-next-build/latest/rez_next_build/vcs/trait.ReleaseVCS.html
