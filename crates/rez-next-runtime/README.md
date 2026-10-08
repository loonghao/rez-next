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

See the [Rust integration guide](https://github.com/loonghao/rez-next/blob/main/docs/rust-integration.md)
for configuration, errors, supported materialization behavior and compatibility.
