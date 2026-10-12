//! Unsupported script generation must never perform a build or install.

use std::fs;
use std::process::Command;

#[test]
fn scripts_mode_never_executes_builder_or_installs() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    let repository = temporary.path().join("packages");
    fs::create_dir_all(&source).unwrap();
    fs::create_dir_all(&repository).unwrap();
    let recipe = "name = 'scripts_safety'\nversion = '1.0.0'\nbuild_command = 'echo executed > marker.txt'\n";
    fs::write(source.join("package.py"), recipe).unwrap();
    fs::write(repository.join("sentinel.txt"), "unchanged").unwrap();

    let result = Command::new(env!("CARGO_BIN_EXE_rez-next"))
        .args(["build", "--scripts", "--install", "--prefix"])
        .arg(&repository)
        .current_dir(&source)
        .env("REZ_DISABLE_HOME_CONFIG", "1")
        .output()
        .unwrap();

    // This safety gate is explicitly not an implementation of build-env
    // generation. It must fail honestly until that feature is implemented.
    assert!(!result.status.success());
    assert!(String::from_utf8_lossy(&result.stderr).contains("not yet supported"));
    assert!(!source.join("marker.txt").exists());
    assert!(!source.join(".rez_build").exists());
    assert_eq!(
        fs::read_to_string(source.join("package.py")).unwrap(),
        recipe
    );
    assert_eq!(
        fs::read_to_string(repository.join("sentinel.txt")).unwrap(),
        "unchanged"
    );
    assert_eq!(fs::read_dir(repository).unwrap().count(), 1);
}
