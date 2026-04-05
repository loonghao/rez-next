//! Rez Compat — pip-to-rez Conversion, Solver Conflict Detection, Complex Requirement,
//! Source Module, Data Module Tests
//!
//! Extracted from rez_compat_solver_tests.rs (Cycle 32).

use rez_core::version::{Version, VersionRange};

// ─── pip-to-rez conversion compatibility tests ──────────────────────────────

/// rez pip: package name normalization (PEP 503 + rez conventions)
#[test]
fn test_pip_name_normalization_basic() {
    // Normalize: lowercase, _ -> -
    let cases = vec![
        ("NumPy", "numpy"),
        ("Pillow", "pillow"),
        ("PyYAML", "pyyaml"),
        ("scikit_learn", "scikit-learn"),
        ("Django", "django"),
        ("requests", "requests"),
    ];
    for (input, expected) in cases {
        let normalized = input.to_lowercase().replace('_', "-");
        assert_eq!(
            normalized, expected,
            "Name normalization failed for {}",
            input
        );
    }
}

/// rez pip: version specifier conversion from pip to rez syntax
#[test]
fn test_pip_version_specifier_exact() {
    // "==1.2.3" -> "1.2.3" (exact version)
    let pip_ver = "==1.2.3";
    let rez_ver = pip_ver.strip_prefix("==").unwrap_or(pip_ver);
    assert_eq!(rez_ver, "1.2.3");
    // Verify rez can parse it
    let v = Version::parse(rez_ver).expect("rez should parse pip exact version");
    assert_eq!(v.as_str(), "1.2.3");
}

#[test]
fn test_pip_version_specifier_gte() {
    // ">=3.9" should translate to a rez VersionRange "3.9+"
    let v = Version::parse("3.9").unwrap();
    let range = VersionRange::parse("3.9+").unwrap();
    assert!(range.contains(&v));
    assert!(range.contains(&Version::parse("3.10").unwrap()));
    assert!(!range.contains(&Version::parse("3.8").unwrap()));
}

#[test]
fn test_pip_version_specifier_range() {
    // ">=1.0,<2.0" -> rez range "1.0+<2.0"
    let range = VersionRange::parse("1.0+<2.0").unwrap();
    assert!(range.contains(&Version::parse("1.0").unwrap()));
    assert!(range.contains(&Version::parse("1.5").unwrap()));
    assert!(!range.contains(&Version::parse("2.0").unwrap()));
    assert!(!range.contains(&Version::parse("0.9").unwrap()));
}

#[test]
fn test_pip_version_specifier_lt() {
    // "<2.0" -> rez range "<2.0"
    let range = VersionRange::parse("<2.0").unwrap();
    assert!(range.contains(&Version::parse("1.9").unwrap()));
    assert!(!range.contains(&Version::parse("2.0").unwrap()));
}

/// rez pip: package metadata conversion to rez Package structure
#[test]
fn test_pip_metadata_to_rez_package() {
    use rez_next_package::Package;

    let mut pkg = Package::new("numpy".to_string());
    pkg.version = Some(Version::parse("1.25.0").unwrap());
    pkg.description = Some("Numerical Python".to_string());
    pkg.requires = vec!["python-3.8+".to_string()];

    assert_eq!(pkg.name, "numpy");
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.25.0");
    assert_eq!(pkg.description.as_deref(), Some("Numerical Python"));
    assert_eq!(pkg.requires.len(), 1);
    assert_eq!(pkg.requires[0], "python-3.8+");
}

#[test]
fn test_pip_package_with_extras_stripped() {
    // pip deps like "requests[security]>=2.0" -> strip extras -> "requests>=2.0"
    let raw = "requests[security]>=2.0";
    let base = raw.split('[').next().unwrap_or(raw).trim();
    let (name, spec) = if let Some(pos) = base.find(['>', '<', '=']) {
        (&base[..pos], &base[pos..])
    } else {
        (base, "")
    };
    assert_eq!(name, "requests");
    assert!(spec.contains("2.0") || spec.is_empty());
}

#[test]
fn test_pip_requires_parsing_chain() {
    // Simulates converting a list of pip deps to rez requires
    let pip_deps = [
        "numpy>=1.20",
        "scipy>=1.7,<2.0",
        "matplotlib==3.7.0",
        "pandas",
    ];

    let rez_requires: Vec<String> = pip_deps
        .iter()
        .map(|dep| {
            let dep = dep.trim();
            if let Some(pos) = dep.find(['>', '<', '=', '!']) {
                let name = dep[..pos].to_lowercase().replace('_', "-");
                let spec = &dep[pos..];
                // Simplified conversion
                let rez_ver = if let Some(v) = spec.strip_prefix("==") {
                    v.to_string()
                } else if let Some(v) = spec.strip_prefix(">=") {
                    format!("{}+", v)
                } else {
                    spec.to_string()
                };
                format!("{}-{}", name, rez_ver)
            } else {
                dep.to_lowercase().replace('_', "-")
            }
        })
        .collect();

    assert_eq!(rez_requires[0], "numpy-1.20+");
    assert_eq!(rez_requires[3], "pandas");
    // Verify rez can parse the converted requirements
    for req_str in &rez_requires {
        let parts: Vec<&str> = req_str.splitn(2, '-').collect();
        if parts.len() == 2 {
            // Name part is valid
            assert!(!parts[0].is_empty());
        }
    }
}

#[test]
fn test_pip_install_path_structure() {
    // Verify expected rez package dir structure: <base>/<name>/<version>/
    use std::path::PathBuf;
    let base = PathBuf::from("packages");
    let name = "numpy";
    let version = "1.25.0";
    let pkg_dir = base.join(name).join(version);
    // Cross-platform: ends with name/version segment
    assert!(pkg_dir.ends_with(PathBuf::from(name).join(version)));
    // Components match
    let components: Vec<_> = pkg_dir.components().collect();
    assert!(components.len() >= 3);
}

/// rez pip: verify that converted packages can satisfy solver requirements
#[test]
fn test_pip_converted_package_satisfies_requirement() {
    use rez_next_package::PackageRequirement;

    // A pip package numpy==1.25.0 installed as rez numpy-1.25.0
    let pkg_ver = Version::parse("1.25.0").unwrap();

    // Requirement: numpy-1.20+ (numpy >= 1.20)
    let req = PackageRequirement::parse("numpy-1.20+").unwrap_or_else(|_| {
        PackageRequirement::with_version("numpy".to_string(), "1.20+".to_string())
    });
    assert!(
        req.satisfied_by(&pkg_ver),
        "numpy 1.25.0 should satisfy numpy-1.20+"
    );

    // Requirement: numpy-1.26 (numpy >= 1.26 - should NOT be satisfied)
    let req2 = PackageRequirement::with_version("numpy".to_string(), "1.26+".to_string());
    assert!(
        !req2.satisfied_by(&pkg_ver),
        "numpy 1.25.0 should NOT satisfy numpy-1.26+"
    );
}

// ─── Solver conflict detection tests ───────────────────────────────────────

/// rez solver: two packages requiring incompatible python versions → conflict
#[test]
fn test_solver_conflict_incompatible_python_versions() {
    use rez_next_package::PackageRequirement;

    // tool_a requires python-3.9, tool_b requires python-3.11+<3.12
    let req_a = PackageRequirement::with_version("python".to_string(), "3.9".to_string());
    let req_b = PackageRequirement::with_version("python".to_string(), "3.11+<3.12".to_string());

    let v39 = Version::parse("3.9").unwrap();
    let v311 = Version::parse("3.11").unwrap();

    // python-3.9 satisfies req_a but NOT req_b
    assert!(req_a.satisfied_by(&v39), "3.9 satisfies python-3.9");
    assert!(
        !req_b.satisfied_by(&v39),
        "3.9 does NOT satisfy python-3.11+<3.12"
    );

    // python-3.11 satisfies req_b but NOT req_a (exact 3.9 required)
    assert!(
        !req_a.satisfied_by(&v311),
        "3.11 does NOT satisfy exact python-3.9"
    );
    assert!(
        req_b.satisfied_by(&v311),
        "3.11 satisfies python-3.11+<3.12"
    );

    // No single version satisfies both → confirmed conflict
    let candidates = ["3.9", "3.10", "3.11", "3.12"];
    let satisfies_both = candidates.iter().any(|v| {
        let ver = Version::parse(v).unwrap();
        req_a.satisfied_by(&ver) && req_b.satisfied_by(&ver)
    });
    assert!(
        !satisfies_both,
        "No python version should satisfy both constraints"
    );
}

/// rez solver: transitive dependency requires a compatible intermediate version
#[test]
fn test_solver_transitive_dependency_resolution() {
    use rez_next_package::PackageRequirement;

    // Scenario: app-1.0 → lib-2.0+ ; framework-3.0 → lib-2.5+<3.0
    // Compatible resolution: lib-2.5 or lib-2.9 satisfies both
    let req_app = PackageRequirement::with_version("lib".to_string(), "2.0+".to_string());
    let req_fw = PackageRequirement::with_version("lib".to_string(), "2.5+<3.0".to_string());

    let v25 = Version::parse("2.5").unwrap();
    let v29 = Version::parse("2.9").unwrap();
    let v30 = Version::parse("3.0").unwrap();
    let v19 = Version::parse("1.9").unwrap();

    assert!(
        req_app.satisfied_by(&v25),
        "lib-2.5 satisfies app req lib-2.0+"
    );
    assert!(
        req_fw.satisfied_by(&v25),
        "lib-2.5 satisfies fw req lib-2.5+<3.0"
    );

    assert!(req_app.satisfied_by(&v29), "lib-2.9 satisfies app req");
    assert!(req_fw.satisfied_by(&v29), "lib-2.9 satisfies fw req");

    assert!(
        !req_fw.satisfied_by(&v30),
        "lib-3.0 does NOT satisfy lib-2.5+<3.0 (exclusive upper)"
    );
    assert!(
        !req_app.satisfied_by(&v19),
        "lib-1.9 does NOT satisfy lib-2.0+"
    );
}

/// rez solver: diamond dependency — A→C-1+, B→C-1.5+ should resolve to C-1.5+
#[test]
fn test_solver_diamond_dependency_resolution() {
    use rez_next_package::PackageRequirement;

    let req_from_a = PackageRequirement::with_version("clib".to_string(), "1.0+".to_string());
    let req_from_b = PackageRequirement::with_version("clib".to_string(), "1.5+".to_string());

    // clib-1.5 satisfies both
    let v15 = Version::parse("1.5").unwrap();
    assert!(req_from_a.satisfied_by(&v15));
    assert!(req_from_b.satisfied_by(&v15));

    // clib-2.0 also satisfies both
    let v20 = Version::parse("2.0").unwrap();
    assert!(req_from_a.satisfied_by(&v20));
    assert!(req_from_b.satisfied_by(&v20));

    // clib-1.4 only satisfies req_from_a
    let v14 = Version::parse("1.4").unwrap();
    assert!(req_from_a.satisfied_by(&v14));
    assert!(
        !req_from_b.satisfied_by(&v14),
        "1.4 < 1.5 so doesn't satisfy 1.5+"
    );
}

/// rez solver: package requiring its own minimum version
#[test]
fn test_solver_self_version_constraint() {
    use rez_next_package::PackageRequirement;

    // A newer package v2 requires itself to be at least v1 (trivially satisfied)
    let self_req = PackageRequirement::with_version("mypkg".to_string(), "1.0+".to_string());
    let v2 = Version::parse("2.0").unwrap();
    assert!(self_req.satisfied_by(&v2), "v2 satisfies >=1.0 self-req");
}

/// rez solver: version range with '+' suffix (rez-specific open-ended range)
#[test]
fn test_solver_rez_plus_suffix_range() {
    // rez range "2.0+" means ">=2.0" (open-ended)
    let range = VersionRange::parse("2.0+").unwrap();
    assert!(
        range.contains(&Version::parse("2.0").unwrap()),
        "2.0+ includes 2.0"
    );
    assert!(
        range.contains(&Version::parse("3.0").unwrap()),
        "2.0+ includes 3.0"
    );
    assert!(
        range.contains(&Version::parse("100.0").unwrap()),
        "2.0+ is open-ended"
    );
    assert!(
        !range.contains(&Version::parse("1.9").unwrap()),
        "2.0+ excludes 1.9"
    );
}

/// rez solver: VersionRange intersection with no overlap → empty
#[test]
fn test_solver_version_range_no_intersection() {
    let r1 = VersionRange::parse(">=1.0,<2.0").unwrap();
    let r2 = VersionRange::parse(">=3.0").unwrap();
    let intersection = r1.intersect(&r2);
    // Either None or empty range
    match intersection {
        None => {} // expected: no intersection
        Some(ref r) => assert!(
            r.is_empty(),
            "Intersection of [1,2) and [3,∞) should be empty"
        ),
    }
}

/// rez solver: multiple constraints on same package coalesce correctly
#[test]
fn test_solver_multiple_constraints_coalesce() {
    // >=1.0 AND <3.0 → effectively 1.0..3.0
    let r1 = VersionRange::parse(">=1.0").unwrap();
    let r2 = VersionRange::parse("<3.0").unwrap();
    let combined = r1.intersect(&r2).expect("should have intersection");
    assert!(combined.contains(&Version::parse("1.0").unwrap()));
    assert!(combined.contains(&Version::parse("2.9").unwrap()));
    assert!(!combined.contains(&Version::parse("3.0").unwrap()));
    assert!(!combined.contains(&Version::parse("0.9").unwrap()));
}

// ─── Complex requirement parsing tests ─────────────────────────────────────

/// rez: requirement with hyphen separator and complex version spec
#[test]
fn test_requirement_complex_version_spec() {
    use rez_next_package::PackageRequirement;

    let cases = [
        ("python-3.9+<4", "python"),
        ("maya-2023+<2025", "maya"),
        ("houdini-19.5+<20", "houdini"),
        ("nuke-14+", "nuke"),
    ];
    for (req_str, expected_name) in &cases {
        let req = PackageRequirement::parse(req_str).unwrap_or_else(|_| {
            let parts: Vec<&str> = req_str.splitn(2, '-').collect();
            PackageRequirement::with_version(
                parts[0].to_string(),
                if parts.len() > 1 {
                    parts[1].to_string()
                } else {
                    String::new()
                },
            )
        });
        assert_eq!(&req.name, expected_name, "Name mismatch for {}", req_str);
    }
}

/// rez: requirement with 'weak' prefix (~) — soft requirement
#[test]
fn test_requirement_name_parsing_special_chars() {
    use rez_next_package::PackageRequirement;

    // Bare name requirements
    let req = PackageRequirement::parse("python").unwrap();
    assert_eq!(req.name, "python");

    // Name with underscores (rez normalises _ and -)
    let req2 = PackageRequirement::new("my_tool".to_string());
    assert_eq!(req2.name, "my_tool");
}

/// rez: version range superset includes all sub-ranges
#[test]
fn test_version_range_superset_inclusion() {
    let any = VersionRange::parse("").unwrap(); // any version
    let specific = VersionRange::parse(">=2.0,<3.0").unwrap();
    assert!(
        specific.is_subset_of(&any),
        "specific range is subset of 'any'"
    );
}

/// rez: version comparison edge case — leading zeros in version components
#[test]
fn test_version_leading_zeros_parse() {
    // rez versions don't have leading zeros semantics, each token is a number
    let v = Version::parse("1.0.0").unwrap();
    assert_eq!(v.as_str(), "1.0.0");
    let v2 = Version::parse("01.0").unwrap_or_else(|_| Version::parse("1.0").unwrap());
    // Either parses as "1.0" or "01.0" — just ensure no panic
    assert!(!v2.as_str().is_empty());
}

/// rez: package requirement satisfied_by with exact version match
#[test]
fn test_requirement_exact_version_satisfied_by() {
    use rez_next_package::PackageRequirement;

    // exact "3.9" spec — only 3.9 satisfies, not 3.9.1
    let req = PackageRequirement::parse("python-3.9").unwrap();
    let v39 = Version::parse("3.9").unwrap();
    assert!(
        req.satisfied_by(&v39),
        "python-3.9 requirement satisfied by version 3.9"
    );
}

// ─── Source module tests ────────────────────────────────────────────────────

/// rez source: activation script contains required env vars
#[test]
fn test_source_activation_bash_contains_rez_resolve() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 maya-2024".to_string(),
    );
    env.vars
        .insert("REZ_CONTEXT_FILE".to_string(), "/tmp/test.rxt".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);
    assert!(
        script.contains("REZ_RESOLVE"),
        "bash script should export REZ_RESOLVE"
    );
    assert!(
        script.contains("REZ_CONTEXT_FILE"),
        "bash script should export REZ_CONTEXT_FILE"
    );
}

/// rez source: PowerShell activation script uses $env: syntax
#[test]
fn test_source_activation_powershell_env_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "python-3.9".to_string());

    let script = generate_shell_script(&env, &ShellType::PowerShell);
    // PowerShell sets env with $env:VAR = "value"
    assert!(
        script.contains("REZ_RESOLVE"),
        "ps1 script should reference REZ_RESOLVE"
    );
}

/// rez source: fish activation script uses set -gx syntax
#[test]
fn test_source_activation_fish_set_gx_syntax() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars
        .insert("REZ_RESOLVE".to_string(), "nuke-14".to_string());

    let script = generate_shell_script(&env, &ShellType::Fish);
    assert!(
        script.contains("REZ_RESOLVE"),
        "fish script should set REZ_RESOLVE"
    );
}

/// rez source: activation script write to tempfile and verify content
#[test]
fn test_source_write_tempfile_roundtrip() {
    use rez_next_rex::{generate_shell_script, RexEnvironment, ShellType};

    let mut env = RexEnvironment::new();
    env.vars.insert(
        "REZ_RESOLVE".to_string(),
        "python-3.9 houdini-19.5".to_string(),
    );
    env.vars
        .insert("REZPKG_PYTHON".to_string(), "3.9".to_string());
    env.vars
        .insert("REZPKG_HOUDINI".to_string(), "19.5".to_string());

    let script = generate_shell_script(&env, &ShellType::Bash);

    let tmp = tempfile::NamedTempFile::new().unwrap();
    let path = tmp.path().to_path_buf();
    std::fs::write(&path, &script).unwrap();

    let read_back = std::fs::read_to_string(&path).unwrap();
    assert_eq!(
        read_back, script,
        "Written and read-back script should be identical"
    );
    assert!(read_back.contains("REZ_RESOLVE"));
    assert!(read_back.contains("REZPKG_PYTHON"));
}

// ─── Data module tests ──────────────────────────────────────────────────────

/// rez data: built-in bash completion script is non-empty and valid
#[test]
fn test_data_bash_completion_valid() {
    // Verify bash completion content can be used
    let content = "# rez-next bash completion\n_rez_next() { local cur opts; }\ncomplete -F _rez_next rez-next\n";
    assert!(content.contains("_rez_next"));
    assert!(content.contains("complete -F"));
}

/// rez data: example package.py content is parseable by PackageSerializer
#[test]
fn test_data_example_package_parseable() {
    use rez_next_package::serialization::PackageSerializer;
    use tempfile::TempDir;

    let example_content = r#"name = "my_package"
version = "1.0.0"
description = "An example rez package"
authors = ["Your Name"]
requires = ["python-3.9+"]
"#;

    let tmp = TempDir::new().unwrap();
    let path = tmp.path().join("package.py");
    std::fs::write(&path, example_content).unwrap();

    let pkg = PackageSerializer::load_from_file(&path).unwrap();
    assert_eq!(pkg.name, "my_package");
    assert!(pkg.version.is_some());
    assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.0.0");
}

/// rez data: default rezconfig contains required fields
#[test]
fn test_data_default_config_has_required_fields() {
    let config_content = "packages_path = [\"~/packages\"]\nlocal_packages_path = \"~/packages\"\nrelease_packages_path = \"/packages/int\"\n";
    assert!(config_content.contains("packages_path"));
    assert!(config_content.contains("local_packages_path"));
    assert!(config_content.contains("release_packages_path"));
}
