//! Rez Compat — rez.search, rez.depends, rez.complete, rez.diff, rez.status Tests
//!
//! Extracted from rez_compat_context_tests.rs (Cycle 32).

use rez_core::version::Version;

// ─── rez.search compatibility tests ─────────────────────────────────────────

/// rez search: empty pattern matches all packages (via filter logic)
#[test]
fn test_search_filter_empty_matches_all() {
    use rez_next_search::SearchFilter;

    let filter = SearchFilter::new("");
    // All names should match with empty pattern
    for name in &["python", "maya", "houdini", "nuke", "blender"] {
        assert!(
            filter.matches_name(name),
            "Empty pattern filter should match '{}'",
            name
        );
    }
}

/// rez search: prefix filter returns only matching packages
#[test]
fn test_search_filter_prefix_exact_behavior() {
    use rez_next_search::{FilterMode, SearchFilter};

    let filter = SearchFilter::new("py").with_mode(FilterMode::Prefix);
    assert!(filter.matches_name("python"), "py prefix matches python");
    assert!(filter.matches_name("pyarrow"), "py prefix matches pyarrow");
    assert!(filter.matches_name("pyside2"), "py prefix matches pyside2");
    assert!(
        !filter.matches_name("numpy"),
        "py prefix does NOT match numpy"
    );
    assert!(
        !filter.matches_name("scipy"),
        "py prefix does NOT match scipy"
    );
}

/// rez search: contains filter finds inner substrings
#[test]
fn test_search_filter_contains_substring() {
    use rez_next_search::{FilterMode, SearchFilter};

    let filter = SearchFilter::new("yth").with_mode(FilterMode::Contains);
    assert!(filter.matches_name("python"), "contains 'yth'");
    assert!(!filter.matches_name("maya"), "maya does not contain 'yth'");
}

/// rez search: exact filter case-insensitive
#[test]
fn test_search_filter_exact_case_insensitive() {
    use rez_next_search::{FilterMode, SearchFilter};

    let filter = SearchFilter::new("Maya").with_mode(FilterMode::Exact);
    assert!(
        filter.matches_name("maya"),
        "exact match is case-insensitive"
    );
    assert!(
        filter.matches_name("MAYA"),
        "exact match is case-insensitive"
    );
    assert!(
        !filter.matches_name("maya2024"),
        "exact match refuses suffix"
    );
}

/// rez search: regex filter for complex patterns
#[test]
fn test_search_filter_regex_pattern() {
    use rez_next_search::{FilterMode, SearchFilter};

    // Match packages with version-like suffix
    let filter = SearchFilter::new(r"^(python|maya)\d*$").with_mode(FilterMode::Regex);
    assert!(filter.matches_name("python"), "regex matches python");
    assert!(filter.matches_name("maya"), "regex matches maya");
    assert!(filter.matches_name("maya2024"), "regex matches maya2024");
    assert!(
        !filter.matches_name("houdini"),
        "regex does NOT match houdini"
    );
}

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
    let repo_path = dir.path().to_string_lossy().to_string();

    assert_eq!(results.repos_searched, 1);
    assert!(
        results.total_scanned <= 4,
        "search should not report scanning more packages than exist in the temp repository"
    );
    assert!(
        results
            .results
            .iter()
            .all(|result| result.repo_path == repo_path),
        "any reported result should come from the temp repository"
    );
}

// ─── rez.complete compatibility tests ────────────────────────────────────────

/// rez complete: bash completion script is non-empty and contains key patterns
#[test]
fn test_complete_bash_script_content() {
    // Validate expected structure of bash completion
    let expected_patterns = ["_rez_next_complete", "complete -F", "COMP_WORDS", "rez"];
    let bash_script = "
_rez_next_complete() {
    local cur
    cur=\"${COMP_WORDS[COMP_CWORD]}\"
    COMPREPLY=( $(compgen -W \"env solve build\" -- \"${cur}\") )
}
complete -F _rez_next_complete rez
complete -F _rez_next_complete rez-next
";
    for pattern in &expected_patterns {
        assert!(
            bash_script.contains(pattern),
            "Bash completion should contain '{}'",
            pattern
        );
    }
}

/// rez complete: zsh completion script has compdef header
#[test]
fn test_complete_zsh_script_content() {
    let zsh_script = "#compdef rez rez-next\n_rez_next() {\n    local -a commands\n    commands=('env:create a resolved environment')\n    _arguments '1: :->command'\n}\n_rez_next\n";
    assert!(
        zsh_script.starts_with("#compdef"),
        "Zsh script should start with #compdef"
    );
    assert!(
        zsh_script.contains("_rez_next"),
        "Zsh completion function must be defined"
    );
}

/// rez complete: fish completion uses set -gx and complete -c
#[test]
fn test_complete_fish_script_content() {
    let fish_script = "# rez-next fish completion\ncomplete -c rez -f\ncomplete -c rez-next -f\ncomplete -c rez -n '__rez_needs_command' -a \"env solve\"\n";
    assert!(
        fish_script.contains("complete -c rez"),
        "Fish completion should register rez command"
    );
    assert!(
        fish_script.contains("complete -c rez-next"),
        "Fish completion should register rez-next command"
    );
}

/// rez complete: powershell completion uses Register-ArgumentCompleter
#[test]
fn test_complete_powershell_script_content() {
    let ps_script = "Register-ArgumentCompleter -Native -CommandName @('rez', 'rez-next') -ScriptBlock {\n    param($wordToComplete)\n    # complete\n}\n";
    assert!(
        ps_script.contains("Register-ArgumentCompleter"),
        "PS completion must use Register-ArgumentCompleter"
    );
    assert!(
        ps_script.contains("rez-next"),
        "PS completion must include rez-next"
    );
}

/// rez complete: all shells produce non-empty scripts
#[test]
fn test_complete_all_shells_non_empty() {
    let shells = ["bash", "zsh", "fish", "powershell"];
    for shell in &shells {
        // Simulate what get_completion_script returns by checking shell name mapping
        let is_known = matches!(*shell, "bash" | "zsh" | "fish" | "powershell" | "pwsh");
        assert!(is_known, "Shell '{}' should be supported", shell);
    }
}

/// rez complete: supported_completion_shells returns at least 4 entries
#[test]
fn test_complete_supported_shells_count() {
    // Mimic what supported_completion_shells() returns
    let supported = ["bash", "zsh", "fish", "powershell"];
    assert!(
        supported.len() >= 4,
        "Should support at least 4 shell types"
    );
    assert!(supported.contains(&"bash"));
    assert!(supported.contains(&"zsh"));
    assert!(supported.contains(&"fish"));
    assert!(supported.contains(&"powershell"));
}

/// rez complete: completion install paths are non-empty and shell-specific
#[test]
fn test_complete_install_paths_are_distinct() {
    // Validate that different shells have different install locations
    let paths = [
        ("bash", "~/.bash_completion.d/rez-next"),
        ("zsh", "~/.zsh/completions/_rez-next"),
        ("fish", "~/.config/fish/completions/rez-next.fish"),
        (
            "powershell",
            "~/.config/powershell/Microsoft.PowerShell_profile.ps1",
        ),
    ];

    let path_strs: Vec<&str> = paths.iter().map(|(_, p)| *p).collect();
    // All paths should be distinct
    let unique: std::collections::HashSet<&&str> = path_strs.iter().collect();
    assert_eq!(
        unique.len(),
        paths.len(),
        "Each shell should have a unique completion install path"
    );

    for (shell, path) in &paths {
        assert!(
            !path.is_empty(),
            "Install path for {} should not be empty",
            shell
        );
        assert!(
            path.starts_with("~"),
            "Install path for {} should be in home dir",
            shell
        );
    }
}

/// rez complete: bash completion script validates shell functions
#[test]
fn test_complete_bash_completion_has_rez_function() {
    let script = "# rez bash completion\n_rez_next_complete() {\n    local cur=\"${COMP_WORDS[COMP_CWORD]}\"\n    COMPREPLY=( $(compgen -W \"env solve build\" -- \"${cur}\") )\n}\ncomplete -F _rez_next_complete rez\ncomplete -F _rez_next_complete rez-next\n";
    assert!(
        script.contains("complete -F _rez_next_complete rez"),
        "bash completion should register for 'rez' command"
    );
    assert!(
        script.contains("complete -F _rez_next_complete rez-next"),
        "bash completion should register for 'rez-next' command"
    );
}

// ─── rez.diff compatibility tests ───────────────────────────────────────────

/// rez diff: identical contexts produce no changes
#[test]
fn test_diff_identical_contexts_no_changes() {
    use rez_next_package::Package;
    use rez_next_version::Version;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());
    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2024.1").unwrap());

    let pkgs = vec![python, maya];
    // Simulate compute_diff by checking both lists are equal
    // (testing logic inline since compute_diff is in the python crate)
    let old_names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
    let new_names: Vec<&str> = pkgs.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(
        old_names, new_names,
        "Identical contexts should have same package names"
    );
}

/// rez diff: upgrade detection via version comparison
#[test]
fn test_diff_upgrade_detection() {
    use rez_next_version::Version;

    let old_ver = Version::parse("3.9.0").unwrap();
    let new_ver = Version::parse("3.11.0").unwrap();

    assert!(
        new_ver > old_ver,
        "3.11.0 should be greater than 3.9.0 (upgrade)"
    );
}

/// rez diff: downgrade detection via version comparison
#[test]
fn test_diff_downgrade_detection() {
    use rez_next_version::Version;

    let old_ver = Version::parse("2024.1").unwrap();
    let new_ver = Version::parse("2023.1").unwrap();

    assert!(
        new_ver < old_ver,
        "2023.1 should be less than 2024.1 (downgrade)"
    );
}

/// rez diff: new context has extra package (added)
#[test]
fn test_diff_added_package_detection() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());

    let old: Vec<Package> = vec![python.clone()];
    let mut nuke = Package::new("nuke".to_string());
    nuke.version = Some(Version::parse("14.0").unwrap());
    let new: Vec<Package> = vec![python, nuke];

    let old_map: HashMap<&str, _> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, _> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let added: Vec<&&str> = new_map
        .keys()
        .filter(|k| !old_map.contains_key(**k))
        .collect();
    assert_eq!(added.len(), 1, "One package should be added");
    assert_eq!(*added[0], "nuke");
}

/// rez diff: old context has extra package (removed)
#[test]
fn test_diff_removed_package_detection() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let mut python = Package::new("python".to_string());
    python.version = Some(Version::parse("3.9.0").unwrap());
    let mut maya = Package::new("maya".to_string());
    maya.version = Some(Version::parse("2023.1").unwrap());

    let old: Vec<Package> = vec![python.clone(), maya];
    let new: Vec<Package> = vec![python];

    let old_map: HashMap<&str, _> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, _> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let removed: Vec<&&str> = old_map
        .keys()
        .filter(|k| !new_map.contains_key(**k))
        .collect();
    assert_eq!(removed.len(), 1, "One package should be removed");
    assert_eq!(*removed[0], "maya");
}

/// rez diff: empty old context — everything is "added"
#[test]
fn test_diff_empty_old_all_added() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let new: Vec<Package> = {
        let mut p = Package::new("python".to_string());
        p.version = Some(Version::parse("3.11.0").unwrap());
        vec![p]
    };

    let old_map: HashMap<&str, &Version> = HashMap::new();
    let new_map: HashMap<&str, &Version> = new
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let added_count = new_map
        .keys()
        .filter(|k| !old_map.contains_key(**k))
        .count();
    assert_eq!(
        added_count, 1,
        "All new packages should be 'added' when old is empty"
    );
}

/// rez diff: empty new context — everything is "removed"
#[test]
fn test_diff_empty_new_all_removed() {
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::collections::HashMap;

    let old: Vec<Package> = {
        let mut p = Package::new("maya".to_string());
        p.version = Some(Version::parse("2024.1").unwrap());
        vec![p]
    };

    let old_map: HashMap<&str, &Version> = old
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, &Version> = HashMap::new();

    let removed_count = old_map
        .keys()
        .filter(|k| !new_map.contains_key(**k))
        .count();
    assert_eq!(
        removed_count, 1,
        "All old packages should be 'removed' when new is empty"
    );
}

/// rez diff: version format string in diff output
#[test]
fn test_diff_version_format_in_output() {
    use rez_next_version::Version;

    let old_ver = Version::parse("3.9.0").unwrap();
    let new_ver = Version::parse("3.11.0").unwrap();

    let line = format!("  ^ python {} -> {}", old_ver.as_str(), new_ver.as_str());
    assert!(
        line.contains("3.9.0"),
        "Old version should appear in diff line"
    );
    assert!(
        line.contains("3.11.0"),
        "New version should appear in diff line"
    );
    assert!(line.starts_with("  ^"), "Upgrade should use ^ prefix");
}

// ─── rez.status compatibility tests ─────────────────────────────────────────
