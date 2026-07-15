# Production readiness

rez-next is production-ready for the supported workflows below when consumers
pin a released version and validate their own package corpus. It is a pre-1.0,
curated Rez-compatible implementation, not a drop-in replacement for Rez's
internal Python modules.

## Supported production surface

- CLI package discovery, search, view, dependency solving, context creation,
  and environment activation.
- `rez env` activation through generated scripts for POSIX shells, PowerShell,
  and Windows `cmd.exe`.
- Source and released builds, variant selection, build/private-build
  requirements, package tests, and post-install tests.
- Git-backed releases with repository-state validation, dependency resolution,
  tag creation, remote push verification, and rollback of a local tag when the
  push fails.
- Filesystem package ignore, unignore, remove, family removal, ignored-package
  cleanup, payload paths, and parent-resource lookup.
- Curated top-level Python APIs for versions, package queries, solving, and
  resolved contexts.
- Checksum-verified self-update: release assets are installed only after an
  exact SHA-256 entry is found and verified.

## Explicit exclusions

Unsupported operations fail with an error instead of reporting success or a
repository miss:

- Rez internal implementation namespaces are not mirrored.
- In-process Python CLI dispatch is not supported; invoke `rez` or `rez-next`.
- Variant lookup by URI and direct filesystem `install_variant` are not yet
  supported.
- The pure-Python `Resolver` does not provide a solve cache until safe
  repository invalidation is implemented.
- CLI release currently supports Git. Mercurial and SVN are available through
  the lower-level VCS library but are not part of the CLI release contract.
- Advanced copy options such as variant selection, destination renaming,
  timestamp preservation, and dry-run are rejected until implemented.

Rez `package.py` files are executable Python by design. Treat package
repositories as trusted code sources.

## Release gates

Every production change must pass:

```bash
vx just ci
vx just py-ci
```

CI runs Rust checks on Linux, macOS, and Windows; exercises PowerShell and
`cmd.exe` activation; verifies release-version consistency; builds Python
wheels on all three platforms; and tests the abi3 wheels across supported
Python versions. Release assets include SHA-256 checksums consumed by
`self-update`.

## Adoption checklist

1. Pin an exact rez-next release; do not deploy from `main`.
2. Run representative resolves, builds, package tests, and environment
   activation against a copy of the studio package repository.
3. Compare resolved package sets and critical environment variables with Rez.
4. Canary the release with a small user group and retain the previous binary
   and package-path configuration for rollback.
5. Promote only after the canary corpus and DCC launch checks pass.

Pre-1.0 releases may intentionally change unsupported or newly curated APIs.
Read the changelog before upgrading.
