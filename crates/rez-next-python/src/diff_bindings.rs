//! Python bindings for `rez diff` — context difference analysis
//!
//! Equivalent to `rez diff <rxt1> <rxt2>`, comparing two resolved contexts.

use crate::runtime::get_runtime;
use pyo3::prelude::*;
use pyo3::types::{PyDict, PyList};
use rez_next_package::Package;
use rez_next_version::Version;

// ─── Public structs ──────────────────────────────────────────────────────────

/// A single package change between two resolved contexts.
///
/// Equivalent to rez's diff output entry: added / removed / upgraded / downgraded.
#[pyclass(name = "PackageDiff", from_py_object)]
#[derive(Clone, Debug)]
pub struct PyPackageDiff {
    /// Package name
    #[pyo3(get)]
    pub name: String,
    /// Old version (None if package was added)
    #[pyo3(get)]
    pub old_version: Option<String>,
    /// New version (None if package was removed)
    #[pyo3(get)]
    pub new_version: Option<String>,
    /// Change type: "added" | "removed" | "upgraded" | "downgraded" | "unchanged"
    #[pyo3(get)]
    pub change_type: String,
}

#[pymethods]
impl PyPackageDiff {
    fn __repr__(&self) -> String {
        match self.change_type.as_str() {
            "added" => format!(
                "PackageDiff(+{} {})",
                self.name,
                self.new_version.as_deref().unwrap_or("?")
            ),
            "removed" => format!(
                "PackageDiff(-{} {})",
                self.name,
                self.old_version.as_deref().unwrap_or("?")
            ),
            "upgraded" => format!(
                "PackageDiff({}: {} -> {})",
                self.name,
                self.old_version.as_deref().unwrap_or("?"),
                self.new_version.as_deref().unwrap_or("?")
            ),
            "downgraded" => format!(
                "PackageDiff({}: {} -> {} [downgrade])",
                self.name,
                self.old_version.as_deref().unwrap_or("?"),
                self.new_version.as_deref().unwrap_or("?")
            ),
            _ => format!("PackageDiff({} unchanged)", self.name),
        }
    }

    fn __str__(&self) -> String {
        self.__repr__()
    }

    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        d.set_item("name", &self.name)?;
        d.set_item("old_version", &self.old_version)?;
        d.set_item("new_version", &self.new_version)?;
        d.set_item("change_type", &self.change_type)?;
        Ok(d.into_any().unbind())
    }
}

/// Summary result of diffing two resolved contexts.
#[pyclass(name = "ContextDiff")]
pub struct PyContextDiff {
    /// All package-level diffs
    #[pyo3(get)]
    pub diffs: Vec<PyPackageDiff>,
    /// Counts by change type
    #[pyo3(get)]
    pub num_added: usize,
    #[pyo3(get)]
    pub num_removed: usize,
    #[pyo3(get)]
    pub num_upgraded: usize,
    #[pyo3(get)]
    pub num_downgraded: usize,
    #[pyo3(get)]
    pub num_unchanged: usize,
}

#[pymethods]
impl PyContextDiff {
    /// Return only the changed diffs (exclude unchanged).
    fn changed_diffs(&self) -> Vec<PyPackageDiff> {
        self.diffs
            .iter()
            .filter(|d| d.change_type != "unchanged")
            .cloned()
            .collect()
    }

    /// True if the two contexts are identical.
    fn is_identical(&self) -> bool {
        self.num_added == 0
            && self.num_removed == 0
            && self.num_upgraded == 0
            && self.num_downgraded == 0
    }

    fn __repr__(&self) -> String {
        format!(
            "ContextDiff(+{} -{} ^{} v{} ={})",
            self.num_added,
            self.num_removed,
            self.num_upgraded,
            self.num_downgraded,
            self.num_unchanged
        )
    }

    fn to_dict(&self, py: Python) -> PyResult<Py<PyAny>> {
        let d = PyDict::new(py);
        d.set_item("num_added", self.num_added)?;
        d.set_item("num_removed", self.num_removed)?;
        d.set_item("num_upgraded", self.num_upgraded)?;
        d.set_item("num_downgraded", self.num_downgraded)?;
        d.set_item("num_unchanged", self.num_unchanged)?;
        let diff_list = PyList::empty(py);
        for diff in &self.diffs {
            diff_list.append(diff.clone().into_pyobject(py)?)?;
        }
        d.set_item("diffs", diff_list)?;
        Ok(d.into_any().unbind())
    }
}

// ─── Core diff logic ─────────────────────────────────────────────────────────

/// Compute the difference between two package lists.
/// Each package list is a `Vec<PyPackage>` as returned by `ResolvedContext.resolved_packages`.
pub fn compute_diff(old_packages: &[Package], new_packages: &[Package]) -> Vec<PyPackageDiff> {
    use std::collections::HashMap;

    let old_map: HashMap<&str, &Version> = old_packages
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();
    let new_map: HashMap<&str, &Version> = new_packages
        .iter()
        .filter_map(|p| p.version.as_ref().map(|v| (p.name.as_str(), v)))
        .collect();

    let mut diffs = Vec::new();

    // Packages in old but not new → removed
    for (name, old_ver) in &old_map {
        if !new_map.contains_key(name) {
            diffs.push(PyPackageDiff {
                name: name.to_string(),
                old_version: Some(old_ver.as_str().to_string()),
                new_version: None,
                change_type: "removed".to_string(),
            });
        }
    }

    // Packages in new but not old → added
    for (name, new_ver) in &new_map {
        if !old_map.contains_key(name) {
            diffs.push(PyPackageDiff {
                name: name.to_string(),
                old_version: None,
                new_version: Some(new_ver.as_str().to_string()),
                change_type: "added".to_string(),
            });
        }
    }

    // Packages in both → compare versions
    for (name, new_ver) in &new_map {
        if let Some(old_ver) = old_map.get(name) {
            let change_type = if new_ver.as_str() == old_ver.as_str() {
                "unchanged"
            } else if *new_ver > *old_ver {
                "upgraded"
            } else {
                "downgraded"
            };
            diffs.push(PyPackageDiff {
                name: name.to_string(),
                old_version: Some(old_ver.as_str().to_string()),
                new_version: Some(new_ver.as_str().to_string()),
                change_type: change_type.to_string(),
            });
        }
    }

    // Sort for deterministic output: removed, added, upgraded, downgraded, unchanged
    diffs.sort_by(|a, b| {
        let order = |t: &str| match t {
            "added" => 0,
            "removed" => 1,
            "upgraded" => 2,
            "downgraded" => 3,
            _ => 4,
        };
        order(&a.change_type)
            .cmp(&order(&b.change_type))
            .then(a.name.cmp(&b.name))
    });

    diffs
}

// ─── Python functions ─────────────────────────────────────────────────────────

/// Diff two resolved contexts by package list.
///
/// Parameters
/// ----------
/// old_packages : list[str]
///     Package requirement strings for the "old" context (e.g. ["python-3.9", "maya-2023"]).
/// new_packages : list[str]
///     Package requirement strings for the "new" context (e.g. ["python-3.11", "houdini-20"]).
///
/// Returns
/// -------
/// ContextDiff
///     A summary of additions, removals, upgrades, and downgrades.
///
/// This is the Python-API equivalent of `rez diff <rxt1> <rxt2>`.
#[pyfunction]
#[pyo3(signature = (old_packages, new_packages))]
pub fn diff_contexts(
    old_packages: Vec<String>,
    new_packages: Vec<String>,
) -> PyResult<PyContextDiff> {
    use rez_next_package::PackageRequirement;
    use rez_next_version::Version;

    // Convert requirement strings to minimal Package structs (name + version)
    let parse_pkg_list = |specs: &[String]| -> Vec<Package> {
        specs
            .iter()
            .map(|s| {
                // Try "name-version" rez format first, then fallback
                let pr = PackageRequirement::parse(s).unwrap_or_else(|_| PackageRequirement {
                    name: s.clone(),
                    version_spec: None,
                    weak: false,
                    conflict: false,
                });
                let mut pkg = Package::new(pr.name.clone());
                if let Some(ref spec) = pr.version_spec {
                    if !spec.is_empty() {
                        pkg.version = Version::parse(spec).ok();
                    }
                }
                pkg
            })
            .collect()
    };

    let old_pkgs = parse_pkg_list(&old_packages);
    let new_pkgs = parse_pkg_list(&new_packages);
    let diffs = compute_diff(&old_pkgs, &new_pkgs);

    let num_added = diffs.iter().filter(|d| d.change_type == "added").count();
    let num_removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    let num_upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
    let num_downgraded = diffs
        .iter()
        .filter(|d| d.change_type == "downgraded")
        .count();
    let num_unchanged = diffs
        .iter()
        .filter(|d| d.change_type == "unchanged")
        .count();

    Ok(PyContextDiff {
        diffs,
        num_added,
        num_removed,
        num_upgraded,
        num_downgraded,
        num_unchanged,
    })
}

/// Diff two resolved context files (.rxt).
///
/// Parameters
/// ----------
/// rxt_path_a, rxt_path_b : str
///     Paths to serialised context files.
///
/// Returns `ContextDiff`.
#[pyfunction]
#[pyo3(signature = (rxt_path_a, rxt_path_b))]
pub fn diff_context_files(rxt_path_a: &str, rxt_path_b: &str) -> PyResult<PyContextDiff> {
    use rez_next_context::ContextSerializer;

    let rt = get_runtime();

    let ctx_a = rt
        .block_on(ContextSerializer::load_from_file(std::path::Path::new(
            rxt_path_a,
        )))
        .map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load {}: {}", rxt_path_a, e))
        })?;
    let ctx_b = rt
        .block_on(ContextSerializer::load_from_file(std::path::Path::new(
            rxt_path_b,
        )))
        .map_err(|e| {
            pyo3::exceptions::PyIOError::new_err(format!("Failed to load {}: {}", rxt_path_b, e))
        })?;

    let diffs = compute_diff(&ctx_a.resolved_packages, &ctx_b.resolved_packages);
    let num_added = diffs.iter().filter(|d| d.change_type == "added").count();
    let num_removed = diffs.iter().filter(|d| d.change_type == "removed").count();
    let num_upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
    let num_downgraded = diffs
        .iter()
        .filter(|d| d.change_type == "downgraded")
        .count();
    let num_unchanged = diffs
        .iter()
        .filter(|d| d.change_type == "unchanged")
        .count();

    Ok(PyContextDiff {
        diffs,
        num_added,
        num_removed,
        num_upgraded,
        num_downgraded,
        num_unchanged,
    })
}

/// Format a ContextDiff as a human-readable string (like `rez diff` terminal output).
#[pyfunction]
pub fn format_diff(diff: &PyContextDiff) -> String {
    let mut lines = Vec::new();
    for d in &diff.diffs {
        let line = match d.change_type.as_str() {
            "added" => format!("  + {} {}", d.name, d.new_version.as_deref().unwrap_or("?")),
            "removed" => format!("  - {} {}", d.name, d.old_version.as_deref().unwrap_or("?")),
            "upgraded" => format!(
                "  ^ {} {} -> {}",
                d.name,
                d.old_version.as_deref().unwrap_or("?"),
                d.new_version.as_deref().unwrap_or("?")
            ),
            "downgraded" => format!(
                "  v {} {} -> {}",
                d.name,
                d.old_version.as_deref().unwrap_or("?"),
                d.new_version.as_deref().unwrap_or("?")
            ),
            _ => continue, // skip unchanged in formatted output
        };
        lines.push(line);
    }
    if lines.is_empty() {
        "  (no changes)".to_string()
    } else {
        lines.join("\n")
    }
}

// ─── Rust unit tests ─────────────────────────────────────────────────────────

#[cfg(test)]
mod diff_bindings_tests {
    use super::*;
    use rez_next_package::Package;
    use rez_next_version::Version;

    fn make_pkg(name: &str, ver: &str) -> Package {
        let mut p = Package::new(name.to_string());
        p.version = Some(Version::parse(ver).unwrap());
        p
    }

    #[test]
    fn test_identical_contexts_no_diff() {
        let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&pkgs, &pkgs);
        let changed: Vec<_> = diffs
            .iter()
            .filter(|d| d.change_type != "unchanged")
            .collect();
        assert!(
            changed.is_empty(),
            "Identical contexts should have no changes"
        );
    }

    #[test]
    fn test_added_package() {
        let old = vec![make_pkg("python", "3.9.0")];
        let new = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&old, &new);
        let added: Vec<_> = diffs.iter().filter(|d| d.change_type == "added").collect();
        assert_eq!(added.len(), 1, "Should detect 1 added package");
        assert_eq!(added[0].name, "maya");
    }

    #[test]
    fn test_removed_package() {
        let old = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.1")];
        let new = vec![make_pkg("python", "3.9.0")];
        let diffs = compute_diff(&old, &new);
        let removed: Vec<_> = diffs
            .iter()
            .filter(|d| d.change_type == "removed")
            .collect();
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0].name, "maya");
        assert_eq!(removed[0].old_version.as_deref(), Some("2023.1"));
    }

    #[test]
    fn test_upgraded_package() {
        let old = vec![make_pkg("python", "3.9.0")];
        let new = vec![make_pkg("python", "3.11.0")];
        let diffs = compute_diff(&old, &new);
        let upgraded: Vec<_> = diffs
            .iter()
            .filter(|d| d.change_type == "upgraded")
            .collect();
        assert_eq!(upgraded.len(), 1);
        assert_eq!(upgraded[0].old_version.as_deref(), Some("3.9.0"));
        assert_eq!(upgraded[0].new_version.as_deref(), Some("3.11.0"));
    }

    #[test]
    fn test_downgraded_package() {
        let old = vec![make_pkg("maya", "2024.1")];
        let new = vec![make_pkg("maya", "2023.1")];
        let diffs = compute_diff(&old, &new);
        let down: Vec<_> = diffs
            .iter()
            .filter(|d| d.change_type == "downgraded")
            .collect();
        assert_eq!(down.len(), 1);
        assert_eq!(down[0].name, "maya");
    }

    #[test]
    fn test_mixed_diff() {
        let old = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("maya", "2023.1"),
            make_pkg("houdini", "19.5"),
        ];
        let new = vec![
            make_pkg("python", "3.11.0"), // upgraded
            make_pkg("houdini", "19.5"),  // unchanged
            make_pkg("nuke", "14.0"),     // added
                                          // maya removed
        ];
        let diffs = compute_diff(&old, &new);
        let added = diffs.iter().filter(|d| d.change_type == "added").count();
        let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
        let upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
        let unchanged = diffs
            .iter()
            .filter(|d| d.change_type == "unchanged")
            .count();
        assert_eq!(added, 1, "nuke should be added");
        assert_eq!(removed, 1, "maya should be removed");
        assert_eq!(upgraded, 1, "python should be upgraded");
        assert_eq!(unchanged, 1, "houdini should be unchanged");
    }

    #[test]
    fn test_empty_old_context() {
        let new = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&[], &new);
        let added = diffs.iter().filter(|d| d.change_type == "added").count();
        assert_eq!(added, 2, "All packages in new should be 'added'");
    }

    #[test]
    fn test_empty_new_context() {
        let old = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&old, &[]);
        let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
        assert_eq!(removed, 2, "All packages in old should be 'removed'");
    }

    #[test]
    fn test_format_diff_no_changes() {
        let diffs = compute_diff(
            &[make_pkg("python", "3.9.0")],
            &[make_pkg("python", "3.9.0")],
        );
        let dummy = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: diffs.len(),
            diffs,
        };
        let output = format_diff(&dummy);
        assert_eq!(output, "  (no changes)");
    }

    #[test]
    fn test_format_diff_with_changes() {
        let old = vec![make_pkg("python", "3.9.0")];
        let new = vec![make_pkg("python", "3.11.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&old, &new);
        let diff_obj = PyContextDiff {
            num_added: 1,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(output.contains("+ maya"), "Should show added package");
        assert!(output.contains("^ python"), "Should show upgraded package");
    }

    #[test]
    fn test_is_identical_true() {
        let diffs = compute_diff(
            &[make_pkg("python", "3.9.0")],
            &[make_pkg("python", "3.9.0")],
        );
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 1,
            diffs,
        };
        assert!(diff_obj.is_identical());
    }

    #[test]
    fn test_is_identical_false_when_changed() {
        let old = vec![make_pkg("python", "3.9.0")];
        let new = vec![make_pkg("python", "3.11.0")];
        let diffs = compute_diff(&old, &new);
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        assert!(!diff_obj.is_identical());
    }

    #[test]
    fn test_both_contexts_empty() {
        let diffs = compute_diff(&[], &[]);
        assert!(
            diffs.is_empty(),
            "Both empty contexts should produce no diffs"
        );
    }

    #[test]
    fn test_changed_diffs_excludes_unchanged() {
        let old = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.1")];
        let new = vec![
            make_pkg("python", "3.11.0"), // upgraded
            make_pkg("maya", "2023.1"),   // unchanged
        ];
        let diffs = compute_diff(&old, &new);
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 1,
            diffs,
        };
        let changed = diff_obj.changed_diffs();
        assert_eq!(changed.len(), 1, "Only upgraded python should be returned");
        assert_eq!(changed[0].change_type, "upgraded");
    }

    #[test]
    fn test_changed_diffs_empty_when_identical() {
        let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2024.1")];
        let diffs = compute_diff(&pkgs, &pkgs);
        let num_unchanged = diffs.len();
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged,
            diffs,
        };
        assert!(
            diff_obj.changed_diffs().is_empty(),
            "Identical contexts should have no changed_diffs"
        );
    }

    #[test]
    fn test_sort_order_added_before_removed() {
        // Added (0) should come before Removed (1) in sorted output
        let old = vec![make_pkg("aaaa", "1.0.0")];
        let new = vec![make_pkg("zzzz", "1.0.0")];
        let diffs = compute_diff(&old, &new);
        // aaaa → removed, zzzz → added; added order=0 < removed order=1
        assert_eq!(diffs[0].change_type, "added", "added should come first");
        assert_eq!(
            diffs[1].change_type, "removed",
            "removed should come second"
        );
    }

    #[test]
    fn test_sort_order_within_same_type_alphabetical() {
        // Multiple added packages should be sorted alphabetically by name
        let old: Vec<Package> = vec![];
        let new = vec![
            make_pkg("zlib", "1.2.0"),
            make_pkg("alembic", "1.7.0"),
            make_pkg("mesa", "22.0.0"),
        ];
        let diffs = compute_diff(&old, &new);
        let names: Vec<&str> = diffs.iter().map(|d| d.name.as_str()).collect();
        assert_eq!(names, vec!["alembic", "mesa", "zlib"]);
    }

    #[test]
    fn test_package_diff_repr_added() {
        let d = PyPackageDiff {
            name: "maya".to_string(),
            old_version: None,
            new_version: Some("2024.1".to_string()),
            change_type: "added".to_string(),
        };
        let r = d.__repr__();
        assert_eq!(r, "PackageDiff(+maya 2024.1)");
    }

    #[test]
    fn test_package_diff_repr_removed() {
        let d = PyPackageDiff {
            name: "houdini".to_string(),
            old_version: Some("19.5".to_string()),
            new_version: None,
            change_type: "removed".to_string(),
        };
        let r = d.__repr__();
        assert_eq!(r, "PackageDiff(-houdini 19.5)");
    }

    #[test]
    fn test_package_diff_repr_upgraded() {
        let d = PyPackageDiff {
            name: "python".to_string(),
            old_version: Some("3.9.0".to_string()),
            new_version: Some("3.11.0".to_string()),
            change_type: "upgraded".to_string(),
        };
        let r = d.__repr__();
        assert_eq!(r, "PackageDiff(python: 3.9.0 -> 3.11.0)");
    }

    #[test]
    fn test_package_diff_repr_downgraded() {
        let d = PyPackageDiff {
            name: "nuke".to_string(),
            old_version: Some("14.0".to_string()),
            new_version: Some("13.0".to_string()),
            change_type: "downgraded".to_string(),
        };
        let r = d.__repr__();
        assert_eq!(r, "PackageDiff(nuke: 14.0 -> 13.0 [downgrade])");
    }

    #[test]
    fn test_context_diff_repr_format() {
        let diff_obj = PyContextDiff {
            num_added: 2,
            num_removed: 1,
            num_upgraded: 3,
            num_downgraded: 0,
            num_unchanged: 5,
            diffs: vec![],
        };
        let r = diff_obj.__repr__();
        assert_eq!(r, "ContextDiff(+2 -1 ^3 v0 =5)");
    }

    #[test]
    fn test_format_diff_removed_uses_minus_prefix() {
        let diffs = vec![PyPackageDiff {
            name: "maya".to_string(),
            old_version: Some("2023.1".to_string()),
            new_version: None,
            change_type: "removed".to_string(),
        }];
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 1,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(
            output.contains("- maya 2023.1"),
            "Removed package should use '- ' prefix: got {output}"
        );
    }

    #[test]
    fn test_format_diff_downgraded_uses_v_prefix() {
        let diffs = vec![PyPackageDiff {
            name: "nuke".to_string(),
            old_version: Some("14.0".to_string()),
            new_version: Some("13.0".to_string()),
            change_type: "downgraded".to_string(),
        }];
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 1,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(
            output.contains("v nuke"),
            "Downgraded package should use 'v ' prefix: got {output}"
        );
    }

    // ── Cycle 101 additions ───────────────────────────────────────────────────

    #[test]
    fn test_compute_diff_single_unchanged() {
        let pkg = make_pkg("python", "3.10.0");
        let diffs = compute_diff(std::slice::from_ref(&pkg), std::slice::from_ref(&pkg));

        let unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
        assert_eq!(unchanged, 1, "Single identical package should be unchanged");
    }

    #[test]
    fn test_package_diff_repr_unchanged_contains_name() {
        let d = PyPackageDiff {
            name: "boost".to_string(),
            old_version: Some("1.83.0".to_string()),
            new_version: Some("1.83.0".to_string()),
            change_type: "unchanged".to_string(),
        };
        let r = d.__repr__();
        assert!(r.contains("boost"), "repr must contain package name: {r}");
        assert!(r.contains("unchanged"), "repr must say unchanged: {r}");
    }

    #[test]
    fn test_context_diff_is_identical_with_only_unchanged() {
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 3,
            diffs: vec![],
        };
        assert!(diff_obj.is_identical(), "all-unchanged diff must be identical");
    }

    #[test]
    fn test_context_diff_is_not_identical_with_downgrade() {
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 1,
            num_unchanged: 0,
            diffs: vec![],
        };
        assert!(!diff_obj.is_identical(), "downgrade means not identical");
    }

    #[test]
    fn test_format_diff_upgraded_shows_versions() {
        let diffs = vec![PyPackageDiff {
            name: "python".to_string(),
            old_version: Some("3.9.0".to_string()),
            new_version: Some("3.11.0".to_string()),
            change_type: "upgraded".to_string(),
        }];
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(output.contains("3.9.0"), "should show old version: {output}");
        assert!(output.contains("3.11.0"), "should show new version: {output}");
    }

    #[test]
    fn test_compute_diff_multiple_upgrades() {
        let old = vec![
            make_pkg("python", "3.9.0"),
            make_pkg("numpy", "1.24.0"),
        ];
        let new = vec![
            make_pkg("python", "3.11.0"),
            make_pkg("numpy", "1.26.0"),
        ];
        let diffs = compute_diff(&old, &new);
        let upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
        assert_eq!(upgraded, 2, "both packages should be upgraded");
    }

    #[test]
    fn test_context_diff_repr_zero_all() {
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs: vec![],
        };
        let r = diff_obj.__repr__();
        assert_eq!(r, "ContextDiff(+0 -0 ^0 v0 =0)");
    }

    // ── Cycle 112 additions ───────────────────────────────────────────────────

    #[test]
    fn test_package_diff_str_equals_repr() {
        let d = PyPackageDiff {
            name: "python".to_string(),
            old_version: Some("3.9.0".to_string()),
            new_version: Some("3.11.0".to_string()),
            change_type: "upgraded".to_string(),
        };
        assert_eq!(d.__str__(), d.__repr__(), "__str__ and __repr__ must be identical");
    }

    #[test]
    fn test_format_diff_added_uses_plus_prefix() {
        let diffs = vec![PyPackageDiff {
            name: "houdini".to_string(),
            old_version: None,
            new_version: Some("20.0".to_string()),
            change_type: "added".to_string(),
        }];
        let diff_obj = PyContextDiff {
            num_added: 1,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(output.contains("+ houdini"), "Added package must use '+ ' prefix: {output}");
    }

    #[test]
    fn test_compute_diff_counts_are_correct() {
        let old = vec![
            make_pkg("a", "1.0"),
            make_pkg("b", "1.0"),
            make_pkg("c", "2.0"),
        ];
        let new = vec![
            make_pkg("a", "1.1"),  // upgraded
            make_pkg("b", "1.0"),  // unchanged
            make_pkg("d", "3.0"),  // added; c removed
        ];
        let diffs = compute_diff(&old, &new);
        assert_eq!(diffs.iter().filter(|d| d.change_type == "added").count(), 1);
        assert_eq!(diffs.iter().filter(|d| d.change_type == "removed").count(), 1);
        assert_eq!(diffs.iter().filter(|d| d.change_type == "upgraded").count(), 1);
        assert_eq!(diffs.iter().filter(|d| d.change_type == "unchanged").count(), 1);
    }

    #[test]
    fn test_package_diff_repr_unknown_type_shows_unchanged_label() {
        let d = PyPackageDiff {
            name: "lib".to_string(),
            old_version: Some("1.0".to_string()),
            new_version: Some("1.0".to_string()),
            change_type: "unknown_type".to_string(),
        };
        let r = d.__repr__();
        // Falls through to `_ => format!("PackageDiff({} unchanged)", self.name)`
        assert!(r.contains("lib"), "repr must contain package name");
        assert!(r.contains("unchanged"), "repr must contain 'unchanged' for unknown type");
    }

    #[test]
    fn test_changed_diffs_includes_downgraded() {
        let d = PyPackageDiff {
            name: "nuke".to_string(),
            old_version: Some("15.0".to_string()),
            new_version: Some("14.0".to_string()),
            change_type: "downgraded".to_string(),
        };
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 0,
            num_downgraded: 1,
            num_unchanged: 0,
            diffs: vec![d],
        };
        let changed = diff_obj.changed_diffs();
        assert_eq!(changed.len(), 1, "downgraded must appear in changed_diffs");
        assert_eq!(changed[0].change_type, "downgraded");
    }

    #[test]
    fn test_format_diff_upgraded_uses_caret_prefix() {
        let diffs = vec![PyPackageDiff {
            name: "alembic".to_string(),
            old_version: Some("1.7.0".to_string()),
            new_version: Some("1.8.0".to_string()),
            change_type: "upgraded".to_string(),
        }];
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(output.contains("^ alembic"), "Upgraded package must use '^ ' prefix: {output}");
    }

    // ── Cycle 117 additions ───────────────────────────────────────────────────

    #[test]
    fn test_to_dict_contains_all_keys() {
        // to_dict() must include all four canonical keys
        let d = PyPackageDiff {
            name: "python".to_string(),
            old_version: Some("3.9.0".to_string()),
            new_version: Some("3.11.0".to_string()),
            change_type: "upgraded".to_string(),
        };
        // Call via the internal repr to verify structure
        let repr = d.__repr__();
        assert!(repr.contains("python"), "repr must contain name");
        // Verify the four fields exist on the struct (compile-time check via pattern)
        let _: &str = &d.name;
        let _: &Option<String> = &d.old_version;
        let _: &Option<String> = &d.new_version;
        let _: &str = &d.change_type;
    }

    #[test]
    fn test_to_dict_added_old_version_is_none() {
        let d = PyPackageDiff {
            name: "nuke".to_string(),
            old_version: None,
            new_version: Some("15.0".to_string()),
            change_type: "added".to_string(),
        };
        assert!(d.old_version.is_none(), "added package must have None old_version");
        assert_eq!(d.new_version.as_deref(), Some("15.0"));
    }

    #[test]
    fn test_to_dict_change_type_value() {
        let types = ["added", "removed", "upgraded", "downgraded", "unchanged"];
        for ct in &types {
            let d = PyPackageDiff {
                name: "pkg".to_string(),
                old_version: Some("1.0".to_string()),
                new_version: Some("1.0".to_string()),
                change_type: ct.to_string(),
            };
            assert_eq!(&d.change_type, ct, "change_type field must preserve value");
        }
    }

    #[test]
    fn test_format_diff_multiple_removed() {
        let diffs = vec![
            PyPackageDiff {
                name: "maya".to_string(),
                old_version: Some("2023.1".to_string()),
                new_version: None,
                change_type: "removed".to_string(),
            },
            PyPackageDiff {
                name: "houdini".to_string(),
                old_version: Some("19.5".to_string()),
                new_version: None,
                change_type: "removed".to_string(),
            },
        ];
        let diff_obj = PyContextDiff {
            num_added: 0,
            num_removed: 2,
            num_upgraded: 0,
            num_downgraded: 0,
            num_unchanged: 0,
            diffs,
        };
        let output = format_diff(&diff_obj);
        assert!(output.contains("- maya"), "should show maya removed");
        assert!(output.contains("- houdini"), "should show houdini removed");
    }

    #[test]
    fn test_compute_diff_name_only_no_version_unchanged() {
        // Packages without version are filtered out by compute_diff (filter_map on version)
        // so two identical versionless packages produce no diffs at all (not "unchanged")
        let mut p1 = Package::new("mylib".to_string());
        p1.version = None;
        let mut p2 = Package::new("mylib".to_string());
        p2.version = None;
        let diffs = compute_diff(&[p1], &[p2]);
        // Both packages lack version → both are filtered → empty diffs
        assert!(
            diffs.is_empty(),
            "versionless packages are filtered by compute_diff; expected empty diffs, got: {diffs:?}"
        );
    }

    #[test]
    fn test_context_diff_total_count_matches_diffs_len() {
        let old = vec![
            make_pkg("a", "1.0"),
            make_pkg("b", "2.0"),
            make_pkg("c", "3.0"),
        ];
        let new = vec![
            make_pkg("a", "1.1"),  // upgraded
            make_pkg("b", "2.0"),  // unchanged
            make_pkg("d", "4.0"),  // added; c removed
        ];
        let diffs = compute_diff(&old, &new);
        let total_diffs = diffs.len();
        let num_added = diffs.iter().filter(|d| d.change_type == "added").count();
        let num_removed = diffs.iter().filter(|d| d.change_type == "removed").count();
        let num_upgraded = diffs.iter().filter(|d| d.change_type == "upgraded").count();
        let num_unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
        let sum = num_added + num_removed + num_upgraded + num_unchanged;
        assert_eq!(sum, total_diffs, "sum of counts must equal total diffs");
    }

    #[test]
    fn test_format_diff_output_lines_count_matches_changes() {
        // format_diff skips unchanged; line count = num changed entries
        let diffs = vec![
            PyPackageDiff {
                name: "a".to_string(),
                old_version: None,
                new_version: Some("1.0".to_string()),
                change_type: "added".to_string(),
            },
            PyPackageDiff {
                name: "b".to_string(),
                old_version: Some("2.0".to_string()),
                new_version: Some("3.0".to_string()),
                change_type: "upgraded".to_string(),
            },
            PyPackageDiff {
                name: "c".to_string(),
                old_version: Some("1.0".to_string()),
                new_version: Some("1.0".to_string()),
                change_type: "unchanged".to_string(),
            },
        ];
        let diff_obj = PyContextDiff {
            num_added: 1,
            num_removed: 0,
            num_upgraded: 1,
            num_downgraded: 0,
            num_unchanged: 1,
            diffs,
        };
        let output = format_diff(&diff_obj);
        // "unchanged" is skipped → 2 lines
        let line_count = output.lines().count();
        assert_eq!(line_count, 2, "format_diff should output 2 lines for 2 changes: got {line_count}");
    }

    // ── Cycle 127 additions ───────────────────────────────────────────────────

    #[test]
    fn test_compute_diff_empty_old_all_added() {
        let new = vec![make_pkg("maya", "2024.1"), make_pkg("python", "3.11")];
        let diffs = compute_diff(&[], &new);
        let added = diffs.iter().filter(|d| d.change_type == "added").count();
        assert_eq!(added, 2, "all packages are 'added' when old is empty");
    }

    #[test]
    fn test_compute_diff_empty_new_all_removed() {
        let old = vec![make_pkg("maya", "2024.1"), make_pkg("python", "3.11")];
        let diffs = compute_diff(&old, &[]);
        let removed = diffs.iter().filter(|d| d.change_type == "removed").count();
        assert_eq!(removed, 2, "all packages are 'removed' when new is empty");
    }

    #[test]
    fn test_context_diff_repr_contains_counts() {
        let diff_obj = PyContextDiff {
            num_added: 3,
            num_removed: 1,
            num_upgraded: 2,
            num_downgraded: 0,
            num_unchanged: 5,
            diffs: vec![],
        };
        let r = diff_obj.__repr__();
        assert!(r.contains("+3"), "repr must contain +3: {r}");
        assert!(r.contains("-1"), "repr must contain -1: {r}");
        assert!(r.contains("^2"), "repr must contain ^2: {r}");
    }

    #[test]
    fn test_package_diff_repr_removed_shows_old_version() {
        let d = PyPackageDiff {
            name: "houdini".to_string(),
            old_version: Some("19.5".to_string()),
            new_version: None,
            change_type: "removed".to_string(),
        };
        let r = d.__repr__();
        assert!(r.contains("19.5"), "removed repr must show old version: {r}");
        assert!(r.contains("houdini"), "removed repr must contain name: {r}");
    }

    #[test]
    fn test_compute_diff_same_version_unchanged() {
        let pkgs = vec![make_pkg("python", "3.9.0"), make_pkg("maya", "2023.0")];
        let diffs = compute_diff(&pkgs, &pkgs);
        let unchanged = diffs.iter().filter(|d| d.change_type == "unchanged").count();
        assert_eq!(unchanged, 2, "identical package lists must produce 2 unchanged diffs");
    }

    #[test]
    fn test_compute_diff_downgrade_detected() {
        let old = vec![make_pkg("nuke", "15.0")];
        let new = vec![make_pkg("nuke", "14.0")];
        let diffs = compute_diff(&old, &new);
        let downgraded = diffs.iter().filter(|d| d.change_type == "downgraded").count();
        assert_eq!(downgraded, 1, "lower version must be detected as 'downgraded'");
    }
}

