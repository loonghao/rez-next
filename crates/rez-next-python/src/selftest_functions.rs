//! Self-test suite exposed to Python.
//!
//! Equivalent to `rez selftest` — verifies core functionality at runtime.

use pyo3::prelude::*;

/// Run basic self-tests and return (passed, failed, total) counts.
/// Equivalent to `rez selftest`
#[pyfunction]
pub fn selftest() -> PyResult<(usize, usize, usize)> {
    let mut passed = 0usize;
    let mut failed = 0usize;

    macro_rules! test {
        ($name:expr, $body:expr) => {
            if { $body } {
                passed += 1;
            } else {
                eprintln!("selftest FAIL: {}", $name);
                failed += 1;
            }
        };
    }

    // ── Version system ────────────────────────────────────────────────────────
    test!("version_parse_basic", {
        let cases = [
            "1.0.0",
            "2.1.3",
            "1.0.0-alpha1",
            "3.2.1",
            "0.0.1",
            "100.200.300",
        ];
        cases
            .iter()
            .all(|s| rez_next_version::Version::parse(s).is_ok())
    });

    test!("version_range_parse", {
        let cases = ["1.0+<2.0", ">=3.9", "<2.0", "3.9", "1.2.3+<1.3", ""];
        cases
            .iter()
            .all(|s| rez_next_version::VersionRange::parse(s).is_ok())
    });

    test!("version_comparison", {
        use rez_next_version::Version;
        let v1 = Version::parse("1.0.0").unwrap();
        let v2 = Version::parse("2.0.0").unwrap();
        let v3 = Version::parse("1.0.0").unwrap();
        v1 < v2 && v1 == v3 && v2 > v3
    });

    test!("version_range_contains", {
        use rez_next_version::{Version, VersionRange};
        let range = VersionRange::parse(">=3.9").unwrap();
        let v39 = Version::parse("3.9").unwrap();
        let v311 = Version::parse("3.11").unwrap();
        let v38 = Version::parse("3.8").unwrap();
        range.contains(&v39) && range.contains(&v311) && !range.contains(&v38)
    });

    // ── Config ────────────────────────────────────────────────────────────────
    test!("config_loads", {
        let cfg = rez_next_common::config::RezCoreConfig::load();
        !cfg.version.is_empty()
    });

    // ── Package requirements ──────────────────────────────────────────────────
    test!("package_requirement_parse", {
        use rez_next_package::PackageRequirement;
        PackageRequirement::parse("python-3.9").is_ok()
            && PackageRequirement::parse("maya").is_ok()
            && PackageRequirement::parse("houdini>=19.5").is_ok()
            && PackageRequirement::parse("python-3+<4").is_ok()
    });

    test!("package_requirement_satisfied_by", {
        use rez_next_package::PackageRequirement;
        use rez_next_version::Version;
        let req = PackageRequirement::parse("python-3.9").unwrap();
        req.satisfied_by(&Version::parse("3.9").unwrap())
    });

    test!("package_build_fields", {
        use rez_next_package::Package;
        use rez_next_version::Version;
        let mut pkg = Package::new("testpkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.commands = Some("env.setenv('MY_ROOT', '{root}')".to_string());
        pkg.tools = vec!["mytool".to_string()];
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.version.is_some() && !pkg.tools.is_empty() && pkg.commands.is_some()
    });

    // ── Rex DSL ───────────────────────────────────────────────────────────────
    test!("rex_parse_setenv", {
        use rez_next_rex::RexParser;
        let parser = RexParser::new();
        parser
            .parse("env.setenv('MY_VAR', 'value')")
            .map(|a| a.len() == 1)
            .unwrap_or(false)
    });

    test!("rex_parse_prepend_path", {
        use rez_next_rex::RexParser;
        let parser = RexParser::new();
        parser
            .parse("env.prepend_path('PATH', '{root}/bin')")
            .map(|a| a.len() == 1)
            .unwrap_or(false)
    });

    test!("rex_execute_maya_commands", {
        use rez_next_rex::RexExecutor;
        let commands = "env.setenv('MAYA_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')";
        let mut exec = RexExecutor::new();
        exec.execute_commands(commands, "maya", Some("/opt/maya/2024.1"), Some("2024.1"))
            .map(|env| {
                env.vars
                    .get("MAYA_ROOT")
                    .map(|v| v.contains("/opt/maya"))
                    .unwrap_or(false)
            })
            .unwrap_or(false)
    });

    test!("rex_resetenv_info_stop", {
        use rez_next_rex::RexExecutor;
        let commands = "info('test message')\nresetenv('OLD_VAR')\nstop('done')";
        let mut exec = RexExecutor::new();
        exec.execute_commands(commands, "pkg", None, None)
            .map(|env| env.stopped && !env.info_messages.is_empty())
            .unwrap_or(false)
    });

    // ── Shell generation ──────────────────────────────────────────────────────
    test!("shell_bash_generation", {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.vars
            .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
        env.aliases
            .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());
        let script = generate_shell_script(&env, &ShellType::Bash);
        script.contains("export MY_ROOT=") && script.contains("alias pkg=")
    });

    test!("shell_powershell_generation", {
        use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};
        let mut env = RexEnvironment::new();
        env.vars
            .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
        let script = generate_shell_script(&env, &ShellType::PowerShell);
        script.contains("$env:MY_ROOT")
    });

    // ── Suite management ──────────────────────────────────────────────────────
    test!("suite_create_and_save", {
        use rez_next_suites::Suite;
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("test_suite");
        let mut suite = Suite::new().with_description("rez-next selftest suite");
        suite
            .add_context("dev", vec!["python-3.9".to_string()])
            .is_ok()
            && suite.save(&suite_path).is_ok()
            && Suite::is_suite(&suite_path)
    });

    test!("suite_load_roundtrip", {
        use rez_next_suites::Suite;
        let dir = tempfile::tempdir().unwrap();
        let suite_path = dir.path().join("roundtrip_suite");
        let mut suite = Suite::new().with_description("roundtrip");
        suite
            .add_context("ctx", vec!["python-3.10".to_string()])
            .unwrap();
        suite.save(&suite_path).unwrap();
        Suite::load(&suite_path)
            .map(|s| s.description == Some("roundtrip".to_string()) && s.len() == 1)
            .unwrap_or(false)
    });

    // ── Repository ────────────────────────────────────────────────────────────
    test!("repository_manager_create", {
        use rez_next_repository::simple_repository::RepositoryManager;
        let mgr = RepositoryManager::new();
        mgr.repository_count() == 0
    });

    let total = passed + failed;
    Ok((passed, failed, total))
}
