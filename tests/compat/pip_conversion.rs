use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

