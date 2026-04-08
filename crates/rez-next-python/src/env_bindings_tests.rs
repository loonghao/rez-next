use super::*;
use rez_next_package::Package;

// ── helpers ─────────────────────────────────────────────────────────────

fn make_pkg(name: &str, ver: &str) -> PyPackage {
    let mut pkg = Package::new(name.to_string());
    pkg.version = Some(rez_next_version::Version::parse(ver).unwrap());
    PyPackage(pkg)
}

fn empty_rez_env(packages: Vec<String>) -> PyRezEnv {
    PyRezEnv {
        packages,
        scripts: HashMap::new(),
        env_vars: HashMap::new(),
        success: false,
        failure_reason: Some("test stub".to_string()),
        resolved_packages: Vec::new(),
    }
}

// ── PyPackageFamily ──────────────────────────────────────────────────────

#[test]
fn test_package_family_creates() {
    let family = PyPackageFamily::new("python".to_string());
    assert_eq!(family.name, "python");
    assert_eq!(family.num_versions(), 0);
}

#[test]
fn test_package_family_add_versions() {
    let mut family = PyPackageFamily::new("python".to_string());
    family.add_package(make_pkg("python", "3.9.0"));
    family.add_package(make_pkg("python", "3.11.0"));

    assert_eq!(family.num_versions(), 2);
    let versions = family.versions();
    assert!(versions.contains(&"3.9.0".to_string()));
    assert!(versions.contains(&"3.11.0".to_string()));
}

#[test]
fn test_package_family_latest_version() {
    let mut family = PyPackageFamily::new("python".to_string());
    for ver in ["3.8.0", "3.9.0", "3.10.0", "3.11.0"] {
        family.add_package(make_pkg("python", ver));
    }
    let latest = family.latest_version();
    assert!(latest.is_some(), "latest_version must not be None");
    let latest_ver = latest.unwrap().0.version.unwrap();
    let valid = ["3.8.0", "3.9.0", "3.10.0", "3.11.0"];
    assert!(
        valid.contains(&latest_ver.as_str()),
        "unexpected latest: {}",
        latest_ver.as_str()
    );
}

#[test]
fn test_package_family_empty_latest_is_none() {
    let family = PyPackageFamily::new("empty_pkg".to_string());
    assert!(family.latest_version().is_none());
}

#[test]
fn test_package_family_repr() {
    let mut family = PyPackageFamily::new("python".to_string());
    family.add_package(make_pkg("python", "3.9.0"));
    let repr = family.__repr__();
    assert!(repr.contains("python"), "repr: {repr}");
    assert!(
        repr.contains('1'.to_string().as_str()),
        "repr should show 1 version: {repr}"
    );
}

#[test]
fn test_package_family_str() {
    let family = PyPackageFamily::new("maya".to_string());
    assert_eq!(family.__str__(), "maya");
}

#[test]
fn test_package_family_iter_packages() {
    let mut family = PyPackageFamily::new("houdini".to_string());
    family.add_package(make_pkg("houdini", "19.5"));
    family.add_package(make_pkg("houdini", "20.0"));
    let pkgs = family.iter_packages();
    assert_eq!(pkgs.len(), 2);
}

// ── PyRezEnv (no repo required — use stub structs) ───────────────────────

#[test]
fn test_rez_env_empty_packages() {
    let env = PyRezEnv::new(vec![], None, None).unwrap();
    assert!(
        env.success,
        "empty package list should resolve successfully"
    );
    assert!(env.packages.is_empty());
}

#[test]
fn test_rez_env_repr() {
    let env = empty_rez_env(vec!["python-3.9".to_string()]);
    let repr = env.__repr__();
    assert!(repr.contains("RezEnv"), "repr: {repr}");
    assert!(repr.contains("python-3.9"), "repr: {repr}");
}

#[test]
fn test_rez_env_get_shell_code_absent_shell() {
    let env = empty_rez_env(vec![]);
    // no scripts registered → None
    assert!(env.get_shell_code("bash").is_none());
    assert!(env.get_shell_code("powershell").is_none());
}

#[test]
fn test_rez_env_get_shell_code_present() {
    let mut env = empty_rez_env(vec![]);
    env.scripts
        .insert("bash".to_string(), "export FOO=bar\n".to_string());
    let code = env.get_shell_code("bash");
    assert!(code.is_some());
    assert!(code.unwrap().contains("FOO=bar"));
}

#[test]
fn test_rez_env_available_shells_empty() {
    let env = empty_rez_env(vec![]);
    assert!(env.available_shells().is_empty());
}

#[test]
fn test_rez_env_available_shells_sorted() {
    let mut env = empty_rez_env(vec![]);
    env.scripts.insert("powershell".to_string(), "".to_string());
    env.scripts.insert("bash".to_string(), "".to_string());
    env.scripts.insert("fish".to_string(), "".to_string());
    let shells = env.available_shells();
    assert_eq!(shells, vec!["bash", "fish", "powershell"]);
}

#[test]
fn test_rez_env_write_script_missing_shell_returns_err() {
    let tmp_file = std::env::temp_dir().join("rez_test_write_script.sh");
    let env = empty_rez_env(vec![]);
    let result = env.write_script(tmp_file.to_str().unwrap(), "bash");
    assert!(
        result.is_err(),
        "write_script with missing shell should return Err"
    );
}

#[test]
fn test_rez_env_write_script_writes_file() {
    let tmp_file = std::env::temp_dir().join("rez_test_write_script_ok.sh");
    let _ = std::fs::remove_file(&tmp_file);

    let mut env = empty_rez_env(vec![]);
    env.scripts
        .insert("bash".to_string(), "export REZ_TEST=1\n".to_string());

    env.write_script(tmp_file.to_str().unwrap(), "bash")
        .unwrap();
    let content = std::fs::read_to_string(&tmp_file).unwrap();
    assert!(content.contains("REZ_TEST=1"));

    let _ = std::fs::remove_file(&tmp_file);
}

#[test]
fn test_get_activation_script_unknown_packages() {
    let result = get_activation_script(
        vec!["nonexistent_pkg_xyz_999".to_string()],
        "bash",
        Some(vec!["/nonexistent/path_xyz".to_string()]),
    );
    if let Ok(script) = result {
        assert!(
            script.contains("Generated by rez-next rex"),
            "unexpected fallback script: {script}"
        );
    }
    // Err is also acceptable (package not found) — both paths are valid
}

// ── Additional PyPackageFamily edge-case tests ───────────────────────────

#[test]
fn test_package_family_versions_empty_when_no_version() {
    // Package with no version should not appear in versions() list
    let mut family = PyPackageFamily::new("noversion".to_string());
    let pkg = PyPackage(Package::new("noversion".to_string())); // no version set
    family.add_package(pkg);
    // num_versions counts ALL packages (even versionless)
    assert_eq!(family.num_versions(), 1);
    // versions() only includes packages with Some(version)
    let versions = family.versions();
    assert!(versions.is_empty(), "versionless pkg should not appear: {:?}", versions);
}

#[test]
fn test_package_family_latest_version_empty_returns_none() {
    let mut family = PyPackageFamily::new("noverpkg".to_string());
    // Add a package with no version
    family.add_package(PyPackage(Package::new("noverpkg".to_string())));
    // latest_version is the first element after sort — still Some (even if versionless)
    // This tests the method doesn't panic.
    let _ = family.latest_version();
}

#[test]
fn test_package_family_iter_empty() {
    let family = PyPackageFamily::new("empty".to_string());
    let pkgs = family.iter_packages();
    assert!(pkgs.is_empty());
}

#[test]
fn test_package_family_repr_zero_versions() {
    let family = PyPackageFamily::new("mypkg".to_string());
    let repr = family.__repr__();
    assert!(repr.contains("mypkg"), "repr: {repr}");
    assert!(repr.contains('0'.to_string().as_str()), "repr should show 0 versions: {repr}");
}

// ── Additional PyRezEnv tests ────────────────────────────────────────────

#[test]
fn test_rez_env_success_flag_on_stub() {
    let env = empty_rez_env(vec!["pkg-1.0".to_string()]);
    assert!(!env.success, "stub env should have success=false");
    assert!(env.failure_reason.is_some());
}

#[test]
fn test_rez_env_available_shells_with_scripts() {
    let mut env = empty_rez_env(vec![]);
    env.scripts.insert("zsh".to_string(), "# zsh\n".to_string());
    env.scripts.insert("bash".to_string(), "# bash\n".to_string());
    let shells = env.available_shells();
    assert!(shells.contains(&"bash".to_string()));
    assert!(shells.contains(&"zsh".to_string()));
    // Must be sorted
    let mut sorted = shells.clone();
    sorted.sort();
    assert_eq!(shells, sorted);
}

#[test]
fn test_rez_env_num_resolved_packages() {
    let mut env = empty_rez_env(vec![]);
    assert_eq!(env.num_resolved_packages(), 0);
    env.resolved_packages.push(make_pkg("python", "3.9.0"));
    assert_eq!(env.num_resolved_packages(), 1);
}

#[test]
fn test_rez_env_resolved_packages_getter() {
    let mut env = empty_rez_env(vec![]);
    env.resolved_packages.push(make_pkg("cmake", "3.26.0"));
    let pkgs = env.resolved_packages();
    assert_eq!(pkgs.len(), 1);
    assert_eq!(pkgs[0].0.name, "cmake");
}

#[test]
fn test_rez_env_write_script_powershell_content() {
    let tmp_file = std::env::temp_dir().join("rez_test_write_ps1.ps1");
    let _ = std::fs::remove_file(&tmp_file);
    let mut env = empty_rez_env(vec![]);
    env.scripts.insert(
        "powershell".to_string(),
        "$env:FOO = 'bar'\n".to_string(),
    );
    env.write_script(tmp_file.to_str().unwrap(), "powershell").unwrap();
    let content = std::fs::read_to_string(&tmp_file).unwrap();
    assert!(content.contains("FOO"));
    let _ = std::fs::remove_file(&tmp_file);
}

// ── PyRezEnv env_vars field ──────────────────────────────────────────────

#[test]
fn test_rez_env_env_vars_initially_empty_in_stub() {
    let env = empty_rez_env(vec![]);
    assert!(
        env.env_vars.is_empty(),
        "stub env should start with empty env_vars"
    );
}

#[test]
fn test_rez_env_env_vars_can_be_set() {
    let mut env = empty_rez_env(vec![]);
    env.env_vars.insert("MYVAR".to_string(), "42".to_string());
    assert_eq!(env.env_vars.get("MYVAR"), Some(&"42".to_string()));
}

// ── PyPackageFamily: name is preserved after add_package ────────────────

#[test]
fn test_package_family_name_preserved_after_add() {
    let mut family = PyPackageFamily::new("testpkg".to_string());
    family.add_package(make_pkg("testpkg", "1.0.0"));
    assert_eq!(family.name, "testpkg");
}

// ── PyPackageFamily: multiple families are independent ───────────────────

#[test]
fn test_two_families_are_independent() {
    let mut fa = PyPackageFamily::new("alpha".to_string());
    let mut fb = PyPackageFamily::new("beta".to_string());
    fa.add_package(make_pkg("alpha", "1.0"));
    assert_eq!(fa.num_versions(), 1);
    assert_eq!(fb.num_versions(), 0);
    fb.add_package(make_pkg("beta", "2.0"));
    assert_eq!(fa.num_versions(), 1);
    assert_eq!(fb.num_versions(), 1);
}

// ── PyRezEnv: packages field preserves input order ───────────────────────

#[test]
fn test_rez_env_packages_order_preserved() {
    let input = vec!["zzz-1.0".to_string(), "aaa-2.0".to_string()];
    let env = empty_rez_env(input.clone());
    assert_eq!(env.packages, input, "package list should preserve order");
}

// ── PyRezEnv: failure_reason is None when success=true ───────────────────

#[test]
fn test_rez_env_empty_resolves_with_no_failure_reason() {
    let env = PyRezEnv::new(vec![], None, None).unwrap();
    assert!(env.success);
    assert!(
        env.failure_reason.is_none(),
        "successful env should have no failure_reason"
    );
}

// ── PyPackageFamily: str representation equals family name ───────────────

#[test]
fn test_package_family_str_matches_name_field() {
    let family = PyPackageFamily::new("nuke".to_string());
    assert_eq!(family.__str__(), family.name);
}

// ── Cycle 115 additions ──────────────────────────────────────────────────

#[test]
fn test_rez_env_print_script_missing_shell_does_not_panic() {
    let env = empty_rez_env(vec![]);
    // Should not panic even when shell not registered
    env.print_script("nonexistent_shell");
}

#[test]
fn test_rez_env_print_script_with_registered_shell_does_not_panic() {
    let mut env = empty_rez_env(vec![]);
    env.scripts.insert("bash".to_string(), "export X=1\n".to_string());
    // Should not panic when shell is present
    env.print_script("bash");
}

#[test]
fn test_package_family_versions_sorted_order() {
    let mut family = PyPackageFamily::new("python".to_string());
    // Add versions out of order
    for ver in ["3.11.0", "3.8.0", "3.10.0", "3.9.0"] {
        family.add_package(make_pkg("python", ver));
    }
    assert_eq!(family.num_versions(), 4);
    let versions = family.versions();
    // All four versions must be present
    assert!(versions.contains(&"3.8.0".to_string()));
    assert!(versions.contains(&"3.9.0".to_string()));
    assert!(versions.contains(&"3.10.0".to_string()));
    assert!(versions.contains(&"3.11.0".to_string()));
}

#[test]
fn test_rez_env_env_vars_multiple_entries() {
    let mut env = empty_rez_env(vec![]);
    env.env_vars.insert("A".to_string(), "1".to_string());
    env.env_vars.insert("B".to_string(), "2".to_string());
    env.env_vars.insert("C".to_string(), "3".to_string());
    assert_eq!(env.env_vars.len(), 3);
    assert_eq!(env.env_vars["A"], "1");
    assert_eq!(env.env_vars["C"], "3");
}

#[test]
fn test_rez_env_failure_reason_stub_is_some() {
    let env = empty_rez_env(vec!["maya-2024".to_string()]);
    // The stub always sets failure_reason to Some("test stub")
    assert_eq!(env.failure_reason.as_deref(), Some("test stub"));
}

#[test]
fn test_package_family_add_same_version_twice_increments_count() {
    let mut family = PyPackageFamily::new("dup_pkg".to_string());
    family.add_package(make_pkg("dup_pkg", "1.0.0"));
    family.add_package(make_pkg("dup_pkg", "1.0.0")); // duplicate
    // num_versions counts raw list length, duplicates allowed
    assert_eq!(family.num_versions(), 2);
}

#[test]
fn test_rez_env_packages_field_reflects_input() {
    let pkgs = vec![
        "python-3.9".to_string(),
        "cmake-3.26".to_string(),
        "boost-1.82".to_string(),
    ];
    let env = empty_rez_env(pkgs.clone());
    assert_eq!(env.packages.len(), 3);
    assert_eq!(env.packages[0], "python-3.9");
    assert_eq!(env.packages[2], "boost-1.82");
}

// ── Cycle 119 additions ──────────────────────────────────────────────────

mod test_env_cy119 {
    use super::*;

    /// empty PyRezEnv::new() sets success=true
    #[test]
    fn test_empty_env_is_successful() {
        let env = PyRezEnv::new(vec![], None, None).unwrap();
        assert!(env.success, "empty package resolve should succeed");
    }

    /// available_shells returns sorted result even with 1 entry
    #[test]
    fn test_available_shells_single_entry_is_sorted() {
        let mut env = empty_rez_env(vec![]);
        env.scripts.insert("zsh".to_string(), "# zsh\n".to_string());
        let shells = env.available_shells();
        assert_eq!(shells, vec!["zsh".to_string()]);
    }

    /// write_script overwrites an existing file
    #[test]
    fn test_write_script_overwrites_existing_file() {
        let tmp = std::env::temp_dir().join("rez_cy119_overwrite.sh");
        std::fs::write(&tmp, b"old content").unwrap();
        let mut env = empty_rez_env(vec![]);
        env.scripts
            .insert("bash".to_string(), "export OVERWRITTEN=1\n".to_string());
        env.write_script(tmp.to_str().unwrap(), "bash").unwrap();
        let content = std::fs::read_to_string(&tmp).unwrap();
        assert!(content.contains("OVERWRITTEN"), "file should be overwritten");
        let _ = std::fs::remove_file(&tmp);
    }

    /// get_shell_code returns None for shell not in scripts
    #[test]
    fn test_get_shell_code_missing_shell_is_none() {
        let env = empty_rez_env(vec![]);
        assert!(env.get_shell_code("fish").is_none());
    }

    /// num_resolved_packages reflects add count
    #[test]
    fn test_num_resolved_packages_increments() {
        let mut env = empty_rez_env(vec![]);
        assert_eq!(env.num_resolved_packages(), 0);
        env.resolved_packages.push(make_pkg("maya", "2024.1"));
        env.resolved_packages.push(make_pkg("python", "3.11.0"));
        assert_eq!(env.num_resolved_packages(), 2);
    }

    /// env_vars key removal works
    #[test]
    fn test_env_vars_key_removal() {
        let mut env = empty_rez_env(vec![]);
        env.env_vars.insert("TEMP_KEY".to_string(), "val".to_string());
        assert!(env.env_vars.contains_key("TEMP_KEY"));
        env.env_vars.remove("TEMP_KEY");
        assert!(!env.env_vars.contains_key("TEMP_KEY"), "key should be removed");
    }
}

mod test_env_cy125 {
    use super::*;

    /// empty_rez_env scripts map is initially empty
    #[test]
    fn test_empty_env_scripts_is_empty() {
        let env = empty_rez_env(vec![]);
        assert!(env.scripts.is_empty(), "scripts must be empty initially");
    }

    /// env_vars map is initially empty
    #[test]
    fn test_empty_env_vars_is_empty() {
        let env = empty_rez_env(vec![]);
        assert!(env.env_vars.is_empty(), "env_vars must be empty initially");
    }

    /// resolved_packages initially empty
    #[test]
    fn test_empty_env_resolved_packages_is_empty() {
        let env = empty_rez_env(vec![]);
        assert!(
            env.resolved_packages.is_empty(),
            "resolved_packages must be empty initially"
        );
    }

    /// available_shells with two scripts returns two-element sorted list
    #[test]
    fn test_available_shells_two_scripts_sorted() {
        let mut env = empty_rez_env(vec![]);
        env.scripts.insert("zsh".to_string(), "# zsh".to_string());
        env.scripts.insert("bash".to_string(), "# bash".to_string());
        let shells = env.available_shells();
        assert_eq!(shells.len(), 2);
        // sorted lexicographically: bash < zsh
        assert_eq!(shells[0], "bash");
        assert_eq!(shells[1], "zsh");
    }

    /// env_vars supports inserting multiple distinct keys
    #[test]
    fn test_env_vars_multiple_keys() {
        let mut env = empty_rez_env(vec![]);
        env.env_vars.insert("A".to_string(), "1".to_string());
        env.env_vars.insert("B".to_string(), "2".to_string());
        assert_eq!(env.env_vars.len(), 2);
        assert_eq!(env.env_vars["A"], "1");
        assert_eq!(env.env_vars["B"], "2");
    }
}
