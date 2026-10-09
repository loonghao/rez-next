# vx Extension: `rez-release`

`rez-next` ships a [vx](https://github.com/vx-org/vx) extension that makes the
release workflow drivable from vx:

```bash
vx x rez-release release --dry-run
```

The extension is a thin wrapper: it calls
`rez_next.release.release_package` and adds no release logic of its own, so
release semantics (release / local / dry-run modes, VCS validation, tag
behaviour) are exactly those of the underlying API.

## Files

| File | Purpose |
|------|---------|
| `vx-extension.toml` | Extension manifest (name, runtime, entrypoint, arg definitions) |
| `scripts/release.py` | Entrypoint script; parses arguments and calls `release_package` |

## Requirements

- [`vx`](https://github.com/vx-org/vx) on `PATH`
- The `rez-next` Python package importable (`pip install rez-next`, or
  `vx just py-build` from a checkout). The extension exits `3` with an
  explanatory message when the import fails.

## Installation

vx discovers extensions in this priority order:

1. `~/.vx/extensions-dev/<name>` — created by `vx ext dev <dir>` (a symlink;
   use this while developing the extension)
2. `<project>/.vx/extensions/<name>` — project-local
3. `~/.vx/extensions/<name>` — user-level

### Develop against this checkout

```bash
cd /path/to/rez-next
vx ext dev .
vx ext list        # rez-release should appear with SOURCE = dev
vx ext dev . --unlink   # remove the link when done
```

### User-level install

Only the two files are needed:

```bash
mkdir -p ~/.vx/extensions/rez-release/scripts
cp vx-extension.toml ~/.vx/extensions/rez-release/
cp scripts/release.py ~/.vx/extensions/rez-release/scripts/
```

### Project-local install

Copy the same two files into `<project>/.vx/extensions/rez-release/`.

## Usage

```bash
vx x rez-release [PATH] [options]        # bare entrypoint
vx x rez-release release [PATH] [options]
vx x rez-release check   [PATH]          # alias for --dry-run
```

All three forms accept the same options: the subcommand is optional, and the
bare entrypoint forwards its arguments to the script unchanged.

| Option | Effect |
|--------|--------|
| `PATH` (positional) | Package source directory; defaults to the caller's directory |
| `-n`, `--dry-run` | Validate only — no build, no install, no VCS writes |
| `-l`, `--local` | Install into `local_packages_path` instead of `release_packages_path` |
| `-m`, `--message` | Release message used for the VCS tag |
| `--ignore-existing-tag` | Release even if the release tag already exists (default: refuse) |
| `--json` | Emit the raw `ReleaseResult` as JSON on stdout |

When `PATH` is omitted, the directory is taken from `VX_PROJECT_DIR`, which vx
sets to the directory it was invoked from. Run from inside a package directory
to release it; pass an explicit path to release a package from elsewhere.

The extension passes `ignore_existing_tag=False` unless `--ignore-existing-tag`
is given, matching the `rez-next release` CLI default. Re-releasing an existing
version therefore fails instead of overwriting the installed package.

With `--json`, progress lines go to stderr so stdout carries only the JSON
document.

### Examples

```bash
cd ~/dev/mypkg
vx x rez-release --dry-run                  # check this package (bare entrypoint)
vx x rez-release release -m "mypkg 1.2.0"   # release it
vx x rez-release release ~/dev/other --dry-run --json
```

## Exit codes

| Code | Meaning |
|------|---------|
| `0` | Release succeeded |
| `1` | Release failed (see below) |
| `2` | Usage error (package directory not found, bad arguments) |
| `3` | `rez-next` Python package is not importable |

Failure paths are passed through, not swallowed. When a release fails the
extension prints each entry from `ReleaseResult.errors` to stderr and exits
non-zero, covering:

- **Uncommitted changes** — `VCS validation failed: ... Repository is not clean`
- **No package definition** — `No package.py or package.yaml found`
- **Existing release tag** — `Release tag '...' already exists. Use
  --ignore-existing-tag to override.` Nothing is installed or overwritten.
- **Build or test failure** — propagated from the Rust layer as a
  `RuntimeError`

Build, test, and install-path failures are ordinary release failures and exit
`1`; there is no separate "unexpected failure" code.

## Notes

- **Do not add `[[entrypoint.arguments]]` to `vx-extension.toml`.** When an
  entrypoint declares arguments, vx parses bare-entrypoint flags itself and
  passes them as `VX_ARG_*` environment variables *instead of* forwarding argv
  to the script. The script then sees an empty argv, every argparse flag falls
  back to its default, and `vx x rez-release --dry-run` silently becomes a real
  release that builds, installs, and creates a VCS tag. Neither `--` nor
  unknown flags escape this — vx rejects undeclared flags before the script
  runs. The script's own argparse is the single source of truth for flags, and
  bad usage still exits `2`. A regression test guards this.
- vx runs the entrypoint with the **extension directory** as the working
  directory, which is why the package path comes from the `PATH` argument or
  `VX_PROJECT_DIR` rather than the process cwd.
- `--dry-run` returns before the build, install, and tag steps, so it touches
  neither the filesystem nor the VCS.
- **Why re-releasing an existing version is refused.** With the permissive
  legacy behaviour, re-running a release overwrites the installed `package.py`
  and rewrites its `vcs_metadata.json` to the new commit, while the git tag
  stays on the old commit. The same version then reports two different source
  commits depending on whether you read the tag or the installed provenance —
  and the result still comes back successful. The tag check runs before
  anything is installed, so a rejected release changes nothing.

  The tag itself is created only after the package has been built, tested and
  installed, which keeps the invariant **tag exists ⟺ release succeeded**. A
  build or test failure therefore leaves no tag behind, so re-running the
  release after fixing it works without passing `--ignore-existing-tag`.

  When a release does go through over an existing tag (`--ignore-existing-tag`
  or the legacy default), the existing tag is **kept as is** — never
  re-created, never moved onto the new commit. The release reports success and
  keeps the `Tag '...' already exists` warning.
- Any real release imports `package.py`, which creates `__pycache__/` in the
  **source** directory. If that directory is a git work tree without a
  `.gitignore` entry for it, the next release is rejected as
  `Repository is not clean`. Add `__pycache__/` to `.gitignore`.
