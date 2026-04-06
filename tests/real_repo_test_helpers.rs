//! Shared package-fixture helpers for real repository integration tests.
//!
//! Used by:
//! - `real_repo_integration.rs`
//! - `real_repo_context_tests.rs`
//! - `real_repo_resolve_tests.rs`
//!
//! Each consumer must include:
//!   `#[path = "real_repo_test_helpers.rs"] mod real_repo_test_helpers;`
//! and then call `real_repo_test_helpers::create_package(...)`.

use std::fs;
use std::path::Path;


/// Create a minimal package.py in a temp repo at `<repo>/<name>/<version>/package.py`.
pub fn create_package(
    repo_dir: &Path,
    name: &str,
    version: &str,
    requires: &[&str],
    tools: &[&str],
    commands: Option<&str>,
) {
    let pkg_dir = repo_dir.join(name).join(version);
    fs::create_dir_all(&pkg_dir).unwrap();

    let requires_str = requires
        .iter()
        .map(|requirement| format!("    \"{}\",", requirement))
        .collect::<Vec<_>>()
        .join("\n");

    let tools_str = tools
        .iter()
        .map(|tool| format!("    \"{}\",", tool))
        .collect::<Vec<_>>()
        .join("\n");

    let cmd_block = if let Some(cmd) = commands {
        format!(
            r#"
def commands():
    {}
"#,
            cmd
        )
    } else {
        format!(
            r#"
def commands():
    env.{upper}_ROOT.set("{{{{root}}}}")
    env.PATH.prepend("{{{{root}}}}/bin")
"#,
            upper = name.to_uppercase()
        )
    };

    let requires_block = if requires.is_empty() {
        String::new()
    } else {
        format!("requires = [\n{}\n]\n", requires_str)
    };

    let tools_block = if tools.is_empty() {
        String::new()
    } else {
        format!("tools = [\n{}\n]\n", tools_str)
    };

    let content = format!(
        r#"name = "{name}"
version = "{version}"
description = "Test package {name}-{version}"
{requires_block}{tools_block}{cmd_block}"#,
        name = name,
        version = version,
        requires_block = requires_block,
        tools_block = tools_block,
        cmd_block = cmd_block,
    );

    fs::write(pkg_dir.join("package.py"), content).unwrap();
}


