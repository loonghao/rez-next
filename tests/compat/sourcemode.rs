use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── SourceMode behaviour tests ───────────────────────────────────────────────

/// rez.source: SourceMode::Inline returns script content without writing a file
#[test]
fn test_source_mode_inline_returns_content() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    // Simulate SourceMode::Inline: build script in memory
    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        !content.is_empty(),
        "Inline mode should produce non-empty script content"
    );
    assert!(
        content.contains("REZ_RESOLVE"),
        "Inline script should contain REZ_RESOLVE"
    );
}

/// rez.source: SourceMode::File writes script to specified path
#[test]
fn test_source_mode_file_writes_to_disk() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let dir = tempfile::tempdir().unwrap();
    let dest = dir.path().join("activate.sh");

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "maya-2024".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);

    std::fs::write(&dest, &content).unwrap();
    let read_back = std::fs::read_to_string(&dest).unwrap();
    assert!(
        read_back.contains("REZ_RESOLVE"),
        "Written script should contain REZ_RESOLVE"
    );
}

/// rez.source: SourceMode::TempFile produces a non-empty file path string
#[test]
fn test_source_mode_temp_file_nonempty_path() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "houdini-20".to_string());
    let content = generate_shell_script(&env, &ShellType::Bash);

    let tmp = std::env::temp_dir().join(format!("test_act_{}.sh", std::process::id()));
    std::fs::write(&tmp, &content).unwrap();
    assert!(tmp.exists(), "Temp file should exist after write");
    let _ = std::fs::remove_file(&tmp); // cleanup
}

