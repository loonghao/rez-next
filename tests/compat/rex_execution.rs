use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Rex execution tests ────────────────────────────────────────────────────

#[test]
fn test_rex_typical_maya_setup() {
    let mut exec = RexExecutor::new();
    let commands = r#"env.setenv('MAYA_VERSION', '2024')
env.setenv('MAYA_LOCATION', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('LD_LIBRARY_PATH', '{root}/lib')
alias('maya', '{root}/bin/maya')
alias('mayapy', '{root}/bin/mayapy')
"#;
    let env = exec
        .execute_commands(
            commands,
            "maya",
            Some("/opt/autodesk/maya/2024"),
            Some("2024"),
        )
        .unwrap();

    assert_eq!(env.vars.get("MAYA_VERSION"), Some(&"2024".to_string()));
    assert_eq!(
        env.vars.get("MAYA_LOCATION"),
        Some(&"/opt/autodesk/maya/2024".to_string())
    );
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/opt/autodesk/maya/2024/bin"))
        .unwrap_or(false));
    assert_eq!(
        env.aliases.get("maya"),
        Some(&"/opt/autodesk/maya/2024/bin/maya".to_string())
    );
}

#[test]
fn test_rex_python_package_setup() {
    let mut exec = RexExecutor::new();
    let commands = r#"env.setenv('PYTHONHOME', '{root}')
env.prepend_path('PATH', '{root}/bin')
env.prepend_path('PYTHONPATH', '{root}/lib/python3.11/site-packages')
"#;
    let env = exec
        .execute_commands(commands, "python", Some("/usr/local"), Some("3.11.5"))
        .unwrap();

    assert_eq!(env.vars.get("PYTHONHOME"), Some(&"/usr/local".to_string()));
    assert!(env
        .vars
        .get("PATH")
        .map(|v| v.contains("/usr/local/bin"))
        .unwrap_or(false));
    assert!(env
        .vars
        .get("PYTHONPATH")
        .map(|v| v.contains("site-packages"))
        .unwrap_or(false));
}

#[test]
fn test_rex_generates_valid_bash_script() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv('TEST_VAR', 'test_value')
env.prepend_path('PATH', '/opt/test/bin')
alias('test_cmd', '/opt/test/bin/test')
"#,
            "test_pkg",
            Some("/opt/test"),
            Some("1.0"),
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("export TEST_VAR="),
        "bash script missing export"
    );
    assert!(script.contains("export PATH="), "bash script missing PATH");
    assert!(
        script.contains("alias test_cmd="),
        "bash script missing alias"
    );
}

#[test]
fn test_rex_generates_valid_powershell_script() {
    let mut exec = RexExecutor::new();
    let env = exec
        .execute_commands(
            r#"env.setenv('MY_APP', '{root}')
alias('myapp', '{root}/myapp.exe')
"#,
            "myapp",
            Some("C:\\Program Files\\MyApp"),
            Some("2.0"),
        )
        .unwrap();

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    assert!(
        script.contains("$env:MY_APP"),
        "PowerShell script missing $env:"
    );
    assert!(
        script.contains("Set-Alias"),
        "PowerShell script missing Set-Alias"
    );
}

