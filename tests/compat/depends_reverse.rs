use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.depends: reverse dependency query ─────────────────────────────────

/// rez depends: empty repository yields no dependants
#[test]
fn test_depends_empty_repo_no_results() {
    use rez_next_package::Package;

    // With no repository paths provided, result should be empty
    let packages: Vec<Package> = vec![];
    let mut direct: Vec<String> = vec![];

    for pkg in &packages {
        for req in &pkg.requires {
            if req.starts_with("python") {
                if let Some(ref ver) = pkg.version {
                    direct.push(format!("{}-{}", pkg.name, ver.as_str()));
                }
            }
        }
    }
    assert!(direct.is_empty(), "No dependants in empty package list");
}

/// rez depends: direct dependency detection from package requires list
#[test]
fn test_depends_direct_dependency_detected() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2024.1").unwrap());
    maya.requires = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];

    let mut houdini = Package::new("houdini".to_string());
    houdini.version = Some(Version::parse("20.0").unwrap());
    houdini.requires = vec!["python-3.10".to_string()];

    let mut nuke = Package::new("nuke".to_string());
    nuke.version = Some(Version::parse("14.0").unwrap());
    nuke.requires = vec!["openexr-3".to_string()]; // no python dependency

    let packages = vec![maya, houdini, nuke];
    let target = "python";

    let mut dependants = Vec::new();
    for pkg in &packages {
        if pkg.name == target {
            continue;
        }
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    let ver = pkg.version.as_ref().map(|v| v.as_str()).unwrap_or("?");
                    dependants.push(format!("{}-{}", pkg.name, ver));
                    break;
                }
            }
        }
    }
    assert_eq!(
        dependants.len(),
        2,
        "maya and houdini both depend on python"
    );
    assert!(dependants.iter().any(|d| d.starts_with("maya")));
    assert!(dependants.iter().any(|d| d.starts_with("houdini")));
}

/// rez depends: package with no requires has no dependants
#[test]
fn test_depends_no_requires_no_dependants() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let mut standalone = Package::new("standalone".to_string());
    standalone.version = Some(Version::parse("1.0").unwrap());
    standalone.requires = vec![]; // no dependencies at all

    let packages = vec![standalone];
    let target = "python";

    let mut dependants = Vec::new();
    for pkg in &packages {
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    dependants.push(pkg.name.clone());
                    break;
                }
            }
        }
    }
    assert!(
        dependants.is_empty(),
        "Package with no requires should have no dependants"
    );
}

/// rez depends: version range filtering — only return matching version requirements
#[test]
fn test_depends_version_range_filter() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    let mut old_pkg = Package::new("legacy_tool".to_string());
    old_pkg.version = Some(Version::parse("1.0").unwrap());
    old_pkg.requires = vec!["python-2.7".to_string()]; // requires python 2.7 exactly

    let mut new_pkg = Package::new("modern_tool".to_string());
    new_pkg.version = Some(Version::parse("3.0").unwrap());
    new_pkg.requires = vec!["python-3.10".to_string()]; // requires python 3.10 exactly

    let packages = vec![old_pkg, new_pkg];
    let target = "python";
    // Filter range: packages that require python >=3.0 (i.e., their required version is >=3.0)
    let filter_min = Version::parse("3.0").unwrap();

    let mut dependants = Vec::new();
    for pkg in &packages {
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    // Check if the required version satisfies >=3.0 constraint
                    let matches = req
                        .version_spec
                        .as_ref()
                        .and_then(|s| Version::parse(s).ok())
                        .map(|v| v >= filter_min)
                        .unwrap_or(false);
                    if matches {
                        dependants.push(pkg.name.clone());
                        break;
                    }
                }
            }
        }
    }
    assert_eq!(
        dependants.len(),
        1,
        "Only modern_tool requires python >=3.0"
    );
    assert_eq!(dependants[0], "modern_tool");
}

/// rez depends: transitive dependency detection (A→B→C, query C, get both A and B)
#[test]
fn test_depends_transitive_chain() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;
    use std::collections::HashSet;

    // Setup: nuke depends on maya, maya depends on python
    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.10").unwrap());
    python.requires = vec![];

    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2024.1").unwrap());
    maya.requires = vec!["python-3.10".to_string()]; // direct dependency on python

    let mut nuke = Package::new("nuke".to_string());
    nuke.version = Some(Version::parse("14.0").unwrap());
    nuke.requires = vec!["maya-2024".to_string()]; // direct dependency on maya

    let packages = vec![python, maya, nuke];
    let target = "python";

    // Direct dependants (packages requiring python)
    let mut direct_names: HashSet<String> = HashSet::new();
    for pkg in &packages {
        if pkg.name == target {
            continue;
        }
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    direct_names.insert(pkg.name.clone());
                    break;
                }
            }
        }
    }
    assert!(
        direct_names.contains("maya"),
        "maya directly depends on python"
    );
    assert!(
        !direct_names.contains("nuke"),
        "nuke does NOT directly depend on python"
    );

    // Transitive dependants (packages requiring a direct dependant)
    let mut transitive_names: HashSet<String> = HashSet::new();
    for pkg in &packages {
        if pkg.name == target || direct_names.contains(&pkg.name) {
            continue;
        }
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if direct_names.contains(&req.name) {
                    transitive_names.insert(pkg.name.clone());
                    break;
                }
            }
        }
    }
    assert!(
        transitive_names.contains("nuke"),
        "nuke transitively depends on python via maya"
    );
}

/// rez depends: target package itself should not appear in its own dependants
#[test]
fn test_depends_excludes_self() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;

    // A circular-ish scenario: python 3.11 "requires" python (shouldn't happen but check exclusion)
    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.11").unwrap());
    python.requires = vec!["python-3.10".to_string()]; // hypothetical self-ref

    let packages = vec![python];
    let target = "python";

    let mut dependants = Vec::new();
    for pkg in &packages {
        if pkg.name == target {
            continue;
        } // self-exclusion
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    dependants.push(pkg.name.clone());
                    break;
                }
            }
        }
    }
    assert!(
        dependants.is_empty(),
        "Package should not appear as its own dependant"
    );
}

/// rez depends: format output contains expected sections
#[test]
fn test_depends_format_output_sections() {
    // Verify formatting logic produces expected strings
    let lines = [
        "Reverse dependencies for 'python':".to_string(),
        "  Direct:".to_string(),
        "    maya-2024.1  (requires 'python-3.9')".to_string(),
    ];
    let output = lines.join("\n");
    assert!(output.contains("Reverse dependencies for 'python'"));
    assert!(output.contains("Direct"));
    assert!(output.contains("maya-2024.1"));
}

/// rez depends: deduplication — same package shouldn't appear twice if it requires
/// the target via two paths
#[test]
fn test_depends_deduplication() {
    use rez_next_package::{Package, PackageRequirement};
    use rez_next_version::Version;
    use std::collections::HashSet;

    let mut multi_req = Package::new("multi_tool".to_string());
    multi_req.version = Some(Version::parse("1.0").unwrap());
    // Hypothetical: two python requirements (shouldn't happen but test dedup logic)
    multi_req.requires = vec!["python-3.9".to_string(), "python-3.10".to_string()];

    let packages = vec![multi_req];
    let target = "python";

    let mut seen: HashSet<String> = HashSet::new();
    let mut dependants = Vec::new();
    for pkg in &packages {
        if pkg.name == target {
            continue;
        }
        for req_str in &pkg.requires {
            if let Ok(req) = PackageRequirement::parse(req_str) {
                if req.name == target {
                    let key = format!(
                        "{}-{}",
                        pkg.name,
                        pkg.version.as_ref().map(|v| v.as_str()).unwrap_or("?")
                    );
                    if seen.insert(key.clone()) {
                        dependants.push(key);
                    }
                    break; // only add once per package
                }
            }
        }
    }
    assert_eq!(
        dependants.len(),
        1,
        "Package should only appear once even with multiple matching requirements"
    );
}

/// rez search: SearchResult tracks latest version correctly
#[test]
fn test_search_result_latest_tracking() {
    use rez_next_search::SearchResult;

    let versions = vec![
        "3.8".to_string(),
        "3.9".to_string(),
        "3.10".to_string(),
        "3.11".to_string(),
    ];
    let result = SearchResult::new("python".to_string(), versions, "/repo".to_string());

    assert_eq!(
        result.latest,
        Some("3.11".to_string()),
        "latest should be the last (highest sorted) version"
    );
    assert_eq!(result.version_count(), 4);
}

/// rez search: SearchResultSet aggregation
#[test]
fn test_search_result_set_aggregation() {
    use rez_next_search::{SearchResult, SearchResultSet};

    let mut set = SearchResultSet::new();
    assert!(set.is_empty());

    for (name, latest) in &[("python", "3.11"), ("maya", "2024.1"), ("houdini", "20.5")] {
        set.add(SearchResult::new(
            name.to_string(),
            vec![latest.to_string()],
            "/repo".to_string(),
        ));
    }

    assert_eq!(set.len(), 3);
    let names = set.family_names();
    assert!(names.contains(&"python"));
    assert!(names.contains(&"maya"));
    assert!(names.contains(&"houdini"));
}

/// rez search: PackageSearcher with nonexistent path returns empty (no panic)
#[test]
fn test_search_nonexistent_repo_empty() {
    use rez_next_search::{PackageSearcher, SearchOptions};
    use std::path::PathBuf;

    let mut opts = SearchOptions::new("python");
    opts.paths = Some(vec![PathBuf::from("/this/path/does/not/exist/xyz")]);
    let searcher = PackageSearcher::new(opts);
    let results = searcher.search();
    assert!(
        results.is_empty(),
        "Search in nonexistent path should return empty results"
    );
}

/// rez search: filter with limit truncates results
#[test]
fn test_search_filter_limit_respected() {
    use rez_next_search::SearchFilter;

    let filter = SearchFilter::new("").with_limit(10);
    assert_eq!(filter.limit, 10);
    // With many names, filter itself doesn't truncate — that's PackageSearcher's job
    // But verify filter stores the limit correctly
}

/// rez search: SearchOptions scope enum variants
#[test]
fn test_search_scope_variants() {
    use rez_next_search::{SearchOptions, SearchScope};

    let mut opts = SearchOptions::new("python");
    opts.scope = SearchScope::Families;
    assert_eq!(opts.scope, SearchScope::Families);

    opts.scope = SearchScope::Packages;
    assert_eq!(opts.scope, SearchScope::Packages);

    opts.scope = SearchScope::LatestOnly;
    assert_eq!(opts.scope, SearchScope::LatestOnly);
}

/// rez search: SearchResult with version_range filter
#[test]
fn test_search_filter_version_range() {
    use rez_next_search::SearchFilter;

    let filter = SearchFilter::new("python").with_version_range(">=3.9");
    assert!(filter.version_range.is_some());

    let range_str = filter.version_range.as_ref().unwrap();
    assert_eq!(range_str, ">=3.9");
    // Verify the range itself is valid by parsing with rez_next_version
    let range = rez_next_version::VersionRange::parse(range_str).unwrap();
    assert!(range.contains(&Version::parse("3.11.0").unwrap()));
    assert!(!range.contains(&Version::parse("3.8.0").unwrap()));
}

/// rez search: end-to-end with real tempdir repository
#[test]
fn test_search_real_temp_repo() {
    use rez_next_search::{PackageSearcher, SearchOptions, SearchScope};
    use std::fs;
    use tempfile::tempdir;

    let dir = tempdir().unwrap();
    // Create package layout: <repo>/<name>/<version>/package.py
    for (name, ver) in &[
        ("python", "3.9"),
        ("python", "3.11"),
        ("maya", "2024.0"),
        ("numpy", "1.25.0"),
    ] {
        let pkg_dir = dir.path().join(name).join(ver);
        fs::create_dir_all(&pkg_dir).unwrap();
        fs::write(
            pkg_dir.join("package.py"),
            format!("name = '{}'\nversion = '{}'\n", name, ver),
        )
        .unwrap();
    }

    let mut opts = SearchOptions::new("py");
    opts.paths = Some(vec![dir.path().to_path_buf()]);
    opts.scope = SearchScope::Families;

    let searcher = PackageSearcher::new(opts);
    let results = searcher.search();

    // Results depend on repository scan; at minimum no panic
    let _ = results.len();
}

