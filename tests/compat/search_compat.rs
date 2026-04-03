use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

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

