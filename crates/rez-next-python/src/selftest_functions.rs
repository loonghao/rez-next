//! Self-test suite exposed to Python.
//!
//! Equivalent to `rez selftest` — verifies core functionality at runtime.

use pyo3::prelude::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct SelftestCheckResult {
    pub(crate) name: &'static str,
    pub(crate) passed: bool,
}

impl SelftestCheckResult {
    pub(crate) const fn new(name: &'static str, passed: bool) -> Self {
        Self { name, passed }
    }
}

pub(crate) fn collect_selftest_results() -> Vec<SelftestCheckResult> {
    vec![
        SelftestCheckResult::new("version_parse_basic", check_version_parse_basic()),
        SelftestCheckResult::new("version_range_parse", check_version_range_parse()),
        SelftestCheckResult::new("version_comparison", check_version_comparison()),
        SelftestCheckResult::new("version_range_contains", check_version_range_contains()),
        SelftestCheckResult::new("config_loads", check_config_loads()),
        SelftestCheckResult::new(
            "package_requirement_parse",
            check_package_requirement_parse(),
        ),
        SelftestCheckResult::new(
            "package_requirement_satisfied_by",
            check_package_requirement_satisfied_by(),
        ),
        SelftestCheckResult::new("package_build_fields", check_package_build_fields()),
        SelftestCheckResult::new("rex_parse_setenv", check_rex_parse_setenv()),
        SelftestCheckResult::new("rex_parse_prepend_path", check_rex_parse_prepend_path()),
        SelftestCheckResult::new(
            "rex_execute_maya_commands",
            check_rex_execute_maya_commands(),
        ),
        SelftestCheckResult::new("rex_resetenv_info_stop", check_rex_resetenv_info_stop()),
        SelftestCheckResult::new("shell_bash_generation", check_shell_bash_generation()),
        SelftestCheckResult::new(
            "shell_powershell_generation",
            check_shell_powershell_generation(),
        ),
        SelftestCheckResult::new("suite_create_and_save", check_suite_create_and_save()),
        SelftestCheckResult::new("suite_load_roundtrip", check_suite_load_roundtrip()),
        SelftestCheckResult::new(
            "repository_manager_create",
            check_repository_manager_create(),
        ),
    ]
}

pub(crate) fn summarize_selftest_results(results: &[SelftestCheckResult]) -> (usize, usize, usize) {
    let passed = results.iter().filter(|result| result.passed).count();
    let total = results.len();
    (passed, total - passed, total)
}

pub(crate) fn check_version_parse_basic() -> bool {
    let cases = ["1.0.0", "2.1.3", "1.0.0-alpha1", "3.2.1", "0.0.1", "100.200.300"];
    cases
        .iter()
        .all(|version| rez_next_version::Version::parse(version).is_ok())
}

pub(crate) fn check_version_range_parse() -> bool {
    let cases = ["1.0+<2.0", ">=3.9", "<2.0", "3.9", "1.2.3+<1.3", ""];
    cases
        .iter()
        .all(|range| rez_next_version::VersionRange::parse(range).is_ok())
}

pub(crate) fn check_version_comparison() -> bool {
    use rez_next_version::Version;

    let Ok(v1) = Version::parse("1.0.0") else {
        return false;
    };
    let Ok(v2) = Version::parse("2.0.0") else {
        return false;
    };
    let Ok(v3) = Version::parse("1.0.0") else {
        return false;
    };

    v1 < v2 && v1 == v3 && v2 > v3
}

pub(crate) fn check_version_range_contains() -> bool {
    use rez_next_version::{Version, VersionRange};

    let Ok(range) = VersionRange::parse(">=3.9") else {
        return false;
    };
    let Ok(v39) = Version::parse("3.9") else {
        return false;
    };
    let Ok(v311) = Version::parse("3.11") else {
        return false;
    };
    let Ok(v38) = Version::parse("3.8") else {
        return false;
    };

    range.contains(&v39) && range.contains(&v311) && !range.contains(&v38)
}

pub(crate) fn check_config_loads() -> bool {
    let cfg = rez_next_common::config::RezCoreConfig::load();
    !cfg.version.is_empty()
}

pub(crate) fn check_package_requirement_parse() -> bool {
    use rez_next_package::PackageRequirement;

    PackageRequirement::parse("python-3.9").is_ok()
        && PackageRequirement::parse("maya").is_ok()
        && PackageRequirement::parse("houdini>=19.5").is_ok()
        && PackageRequirement::parse("python-3+<4").is_ok()
}

pub(crate) fn check_package_requirement_satisfied_by() -> bool {
    use rez_next_package::PackageRequirement;
    use rez_next_version::Version;

    let Ok(req) = PackageRequirement::parse("python-3.9") else {
        return false;
    };
    let Ok(version) = Version::parse("3.9") else {
        return false;
    };

    req.satisfied_by(&version)
}

pub(crate) fn check_package_build_fields() -> bool {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let Ok(version) = Version::parse("1.0.0") else {
        return false;
    };

    let mut pkg = Package::new("testpkg".to_string());
    pkg.version = Some(version);
    pkg.commands = Some("env.setenv('MY_ROOT', '{root}')".to_string());
    pkg.tools = vec!["mytool".to_string()];
    pkg.requires = vec!["python-3.9".to_string()];

    pkg.version.is_some() && !pkg.tools.is_empty() && pkg.commands.is_some()
}

pub(crate) fn check_rex_parse_setenv() -> bool {
    use rez_next_rex::RexParser;

    RexParser::new()
        .parse("env.setenv('MY_VAR', 'value')")
        .map(|actions| actions.len() == 1)
        .unwrap_or(false)
}

pub(crate) fn check_rex_parse_prepend_path() -> bool {
    use rez_next_rex::RexParser;

    RexParser::new()
        .parse("env.prepend_path('PATH', '{root}/bin')")
        .map(|actions| actions.len() == 1)
        .unwrap_or(false)
}

pub(crate) fn check_rex_execute_maya_commands() -> bool {
    use rez_next_rex::RexExecutor;

    let commands = "env.setenv('MAYA_ROOT', '{root}')\nenv.prepend_path('PATH', '{root}/bin')";
    let mut exec = RexExecutor::new();
    exec.execute_commands(commands, "maya", Some("/opt/maya/2024.1"), Some("2024.1"))
        .map(|env| {
            env.vars
                .get("MAYA_ROOT")
                .map(|value| value.contains("/opt/maya"))
                .unwrap_or(false)
        })
        .unwrap_or(false)
}

pub(crate) fn check_rex_resetenv_info_stop() -> bool {
    use rez_next_rex::RexExecutor;

    let commands = "info('test message')\nresetenv('OLD_VAR')\nstop('done')";
    let mut exec = RexExecutor::new();
    exec.execute_commands(commands, "pkg", None, None)
        .map(|env| env.stopped && !env.info_messages.is_empty())
        .unwrap_or(false)
}

pub(crate) fn check_shell_bash_generation() -> bool {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());
    env.aliases
        .insert("pkg".to_string(), "/opt/pkg/bin/pkg".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);
    script.contains("export MY_ROOT=") && script.contains("alias pkg=")
}

pub(crate) fn check_shell_powershell_generation() -> bool {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("MY_ROOT".to_string(), "/opt/pkg".to_string());

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    script.contains("$env:MY_ROOT")
}

pub(crate) fn check_suite_create_and_save() -> bool {
    use rez_next_suites::Suite;

    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let suite_path = dir.path().join("test_suite");

    let mut suite = Suite::new().with_description("rez-next selftest suite");
    if suite
        .add_context("dev", vec!["python-3.9".to_string()])
        .is_err()
    {
        return false;
    }

    suite.save(&suite_path).is_ok() && Suite::is_suite(&suite_path)
}

pub(crate) fn check_suite_load_roundtrip() -> bool {
    use rez_next_suites::Suite;

    let Ok(dir) = tempfile::tempdir() else {
        return false;
    };
    let suite_path = dir.path().join("roundtrip_suite");

    let mut suite = Suite::new().with_description("roundtrip");
    if suite
        .add_context("ctx", vec!["python-3.10".to_string()])
        .is_err()
    {
        return false;
    }
    if suite.save(&suite_path).is_err() {
        return false;
    }

    Suite::load(&suite_path)
        .map(|suite| suite.description == Some("roundtrip".to_string()) && suite.len() == 1)
        .unwrap_or(false)
}

pub(crate) fn check_repository_manager_create() -> bool {
    use rez_next_repository::simple_repository::RepositoryManager;

    let mgr = RepositoryManager::new();
    mgr.repository_count() == 0
}

/// Run basic self-tests and return (passed, failed, total) counts.
/// Equivalent to `rez selftest`
#[pyfunction]
pub fn selftest() -> PyResult<(usize, usize, usize)> {
    Ok(summarize_selftest_results(&collect_selftest_results()))
}

/// Run basic self-tests and return each check result as (name, passed) pairs.
///
/// Unlike [`selftest()`] which only returns a summary tuple, this function
/// returns the full breakdown so callers can identify exactly which checks
/// failed without relying on stderr output.
///
/// Returns a list of `(name: str, passed: bool)` tuples, one per check.
#[pyfunction]
pub fn selftest_verbose() -> PyResult<Vec<(String, bool)>> {
    Ok(collect_selftest_results()
        .into_iter()
        .map(|r| (r.name.to_string(), r.passed))
        .collect())
}

#[cfg(test)]
#[path = "selftest_functions_tests.rs"]
mod tests;
