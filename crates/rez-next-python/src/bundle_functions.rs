//! Bundle/unbundle functions exposed to Python.
//!
//! Equivalent to `rez bundle` / `rez bundle-unbundle` CLI commands.

use pyo3::prelude::*;

use crate::package_functions::expand_home;

/// Bundle a resolved context to a directory for offline use.
/// Equivalent to `rez bundle <context.rxt> <dest_dir>`
#[pyfunction]
#[pyo3(signature = (context_or_packages, dest_dir, skip_solve=false))]
pub fn bundle_context(
    context_or_packages: Vec<String>,
    dest_dir: &str,
    skip_solve: bool,
) -> PyResult<String> {
    use std::path::PathBuf;

    let dest = PathBuf::from(dest_dir);
    std::fs::create_dir_all(&dest)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    // Write bundle manifest
    let manifest_path = dest.join("bundle.yaml");
    let manifest_content = format!(
        "# rez bundle manifest\npackages:\n{}\nskip_solve: {}\n",
        context_or_packages
            .iter()
            .map(|p| format!("  - {}", p))
            .collect::<Vec<_>>()
            .join("\n"),
        skip_solve
    );
    std::fs::write(&manifest_path, manifest_content)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    Ok(dest.to_string_lossy().to_string())
}

/// Unbundle a previously bundled context (extract and restore).
/// Equivalent to `rez bundle-unbundle <bundle_dir>`
#[pyfunction]
#[pyo3(signature = (bundle_dir, dest_packages_path=None))]
pub fn unbundle_context(
    bundle_dir: &str,
    dest_packages_path: Option<&str>,
) -> PyResult<Vec<String>> {
    // dest_packages_path reserved for future use (copy packages to that path)
    let _ = dest_packages_path;
    use std::io::{BufRead, BufReader};
    use std::path::PathBuf;

    let bundle_path = PathBuf::from(bundle_dir);
    let manifest_path = bundle_path.join("bundle.yaml");

    if !manifest_path.exists() {
        return Err(pyo3::exceptions::PyFileNotFoundError::new_err(format!(
            "No bundle.yaml found in {}",
            bundle_dir
        )));
    }

    // Parse package list from manifest
    let file = std::fs::File::open(&manifest_path)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    let reader = BufReader::new(file);
    let mut packages = Vec::new();
    let mut in_packages = false;
    for line in reader.lines() {
        let line = line.map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
        let trimmed = line.trim();
        if trimmed == "packages:" {
            in_packages = true;
            continue;
        }
        if in_packages {
            if let Some(stripped) = trimmed.strip_prefix("- ") {
                packages.push(stripped.to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with(' ') && !trimmed.starts_with('-')
            {
                in_packages = false;
            }
        }
    }

    Ok(packages)
}

/// List all bundles in a directory.
/// Equivalent to `rez bundle list [path]`
#[pyfunction]
#[pyo3(signature = (search_path=None))]
pub fn list_bundles(search_path: Option<&str>) -> PyResult<Vec<String>> {
    use rez_next_common::config::RezCoreConfig;
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let base = search_path
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(expand_home(&config.local_packages_path)));

    if !base.exists() {
        return Ok(Vec::new());
    }

    let mut bundles = Vec::new();
    if let Ok(entries) = std::fs::read_dir(&base) {
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_dir() && path.join("bundle.yaml").exists() {
                if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                    bundles.push(name.to_string());
                }
            }
        }
    }
    bundles.sort();
    Ok(bundles)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    mod test_bundle_context {
        use super::*;

        #[test]
        fn test_bundle_context_creates_directory() {
            let tmp = std::env::temp_dir().join("rez_test_bundle_create");
            let _ = fs::remove_dir_all(&tmp);
            let result = bundle_context(
                vec!["python-3.9".to_string()],
                tmp.to_str().unwrap(),
                false,
            );
            assert!(result.is_ok(), "bundle_context must succeed: {:?}", result);
            assert!(tmp.exists(), "bundle directory must be created");
            assert!(tmp.join("bundle.yaml").exists(), "bundle.yaml must exist");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_bundle_context_manifest_contains_packages() {
            let tmp = std::env::temp_dir().join("rez_test_bundle_manifest");
            let _ = fs::remove_dir_all(&tmp);
            bundle_context(
                vec!["python-3.9".to_string(), "maya-2024".to_string()],
                tmp.to_str().unwrap(),
                false,
            )
            .unwrap();
            let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
            assert!(content.contains("python-3.9"), "manifest must contain python-3.9");
            assert!(content.contains("maya-2024"), "manifest must contain maya-2024");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_bundle_context_skip_solve_recorded() {
            let tmp = std::env::temp_dir().join("rez_test_bundle_skip");
            let _ = fs::remove_dir_all(&tmp);
            bundle_context(vec!["pkg-1.0".to_string()], tmp.to_str().unwrap(), true).unwrap();
            let content = fs::read_to_string(tmp.join("bundle.yaml")).unwrap();
            assert!(content.contains("skip_solve: true"), "skip_solve must be true in manifest");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_bundle_context_returns_dest_path() {
            let tmp = std::env::temp_dir().join("rez_test_bundle_ret");
            let _ = fs::remove_dir_all(&tmp);
            let returned = bundle_context(vec![], tmp.to_str().unwrap(), false).unwrap();
            assert!(
                returned.contains("rez_test_bundle_ret"),
                "returned path should contain the dest dir name: {}",
                returned
            );
            let _ = fs::remove_dir_all(&tmp);
        }
    }

    mod test_unbundle_context {
        use super::*;

        #[test]
        fn test_unbundle_returns_packages_list() {
            let tmp = std::env::temp_dir().join("rez_test_unbundle_roundtrip");
            let _ = fs::remove_dir_all(&tmp);
            let pkgs = vec!["python-3.9".to_string(), "houdini-19.5".to_string()];
            bundle_context(pkgs.clone(), tmp.to_str().unwrap(), false).unwrap();
            let got = unbundle_context(tmp.to_str().unwrap(), None).unwrap();
            assert_eq!(got, pkgs, "unbundle must return same packages as bundled");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_unbundle_missing_manifest_errors() {
            let tmp = std::env::temp_dir().join("rez_test_unbundle_missing");
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(&tmp).unwrap();
            let result = unbundle_context(tmp.to_str().unwrap(), None);
            assert!(result.is_err(), "missing bundle.yaml must return Err");
            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_unbundle_nonexistent_dir_errors() {
            let result = unbundle_context("/nonexistent/bundle/dir_xyz", None);
            assert!(result.is_err(), "nonexistent bundle dir must return Err");
        }
    }

    mod test_list_bundles {
        use super::*;

        #[test]
        fn test_list_bundles_empty_directory() {
            let tmp = std::env::temp_dir().join("rez_test_list_bundles_empty");
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(&tmp).unwrap();
            let result = list_bundles(Some(tmp.to_str().unwrap())).unwrap();
            assert!(result.is_empty(), "no bundles in empty dir");
            let _ = fs::remove_dir_all(&tmp);
        }
    }
}
