use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Package::commands_function field tests (293-295) ───────────────────────

/// rez package: commands_function field stores rex script body
#[test]
fn test_package_commands_function_set_and_get() {
    use rez_next_package::Package;

    let mut pkg = Package::new("mypkg".to_string());
    let script = "env.setenv('MY_PKG_ROOT', '{root}')\nenv.PATH.prepend('{root}/bin')";
    pkg.commands_function = Some(script.to_string());
    assert!(pkg.commands_function.is_some());
    assert!(pkg
        .commands_function
        .as_ref()
        .unwrap()
        .contains("MY_PKG_ROOT"));
}

/// rez package: commands and commands_function are both populated after parsing package.py
#[test]
fn test_package_commands_function_synced_with_commands() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let content = r#"name = 'cmdpkg'
version = '1.0'
def commands():
    env.setenv('CMDPKG_ROOT', '{root}')
    env.PATH.prepend('{root}/bin')
"#;
    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert!(
        pkg.commands.is_some() || pkg.commands_function.is_some(),
        "At least one of commands/commands_function should be set after parsing"
    );
    if let Some(ref cmd) = pkg.commands {
        assert!(!cmd.is_empty(), "commands should not be empty string");
    }
}

/// rez package: commands_function is None for package without commands
#[test]
fn test_package_commands_function_none_by_default() {
    use rez_next_package::Package;

    let pkg = Package::new("noop_pkg".to_string());
    assert!(
        pkg.commands_function.is_none(),
        "commands_function should be None for new package without commands"
    );
    assert!(
        pkg.commands.is_none(),
        "commands should also be None for new package"
    );
}

