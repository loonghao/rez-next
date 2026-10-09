# rez-next-runtime

High-level Rust embedding API for local Rez repositories. The facade reuses
rez-next's repository manager, strict dependency solver, materialized packages
and environment manager.

```rust,no_run
use rez_next_runtime::RezRuntime;

async fn resolve() -> Result<(), Box<dyn std::error::Error>> {
    let resolved = RezRuntime::new(["/cache/unpacked"])?
        .with_target("linux", "x86_64")?
        .with_parent_environment(std::env::vars().collect())
        .resolve(["rust-1.95.0"])
        .await?;
    let status = resolved.command("rustc").arg("--version").status()?;
    assert!(status.success());
    Ok(())
}
```

No target is inferred by default. Explicit targets always constrain the
`platform` and `arch` package families, including variant selection. Injecting
an empty parent environment isolates the result from ambient variables.

For a binary package builder, the same facade provides a read-only installation
plan. It parses the original definition through Core, resolves its actual
variant with the strict solver, and returns Core's canonical repository paths.
It does not execute package commands or modify the definition.

```rust,no_run
use rez_next_runtime::InstallationPlan;

async fn plan() -> Result<(), Box<dyn std::error::Error>> {
    let plan = InstallationPlan::from_definition("package.py", "windows", "x86_64").await?;
    assert_eq!(plan.package_relative_path, "witr/0.3.4");
    assert_eq!(plan.variant_relative_path, "witr/0.3.4/platform-windows/arch-x86_64");
    Ok(())
}
```

The crate also installs a thin CLI for builders written in other languages:

```bash
vx cargo install rez-next-runtime --locked
vx rez-next-runtime installation-plan --definition package.py --platform windows --arch x86_64 --json
```

JSON contains `schema_version: 1`, verified `platform` and `arch`, `name`,
nullable `version`, nullable `variant_index`, ordered
`variant_requirements`, `package_relative_path`, and `variant_relative_path`.
Platform aliases `macos` and `darwin` normalize to `osx`; architecture values
retain their exact Rez version text. Paths always use `/` separators. Keep
`package.py` at the package base and place
its selected payload at the variant path; `{root}` then identifies that payload
through the normal runtime resolver. Missing dependencies fail strictly. Supply
actual local dependency repositories with repeated `--repository` arguments,
or use `InstallationPlan::from_definition_with_repositories` in Rust.

See the [Rust integration guide](https://github.com/loonghao/rez-next/blob/main/docs/rust-integration.md)
for configuration, errors, supported materialization behavior and compatibility.
