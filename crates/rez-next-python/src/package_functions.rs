//! Package query and management functions exposed to Python.
//!
//! Covers: get_latest_package, get_package, resolve_packages, iter_packages,
//! get_package_family_names, walk_packages, copy_package, move_package, remove_package.

use pyo3::prelude::*;
use pyo3::types::PyList;

use crate::context_bindings::PyResolvedContext;
use crate::package_bindings::PyPackage;
use crate::runtime::get_runtime;

/// Expand `~` in path strings.
pub(crate) fn expand_home(p: &str) -> String {
    if p.starts_with("~/") || p == "~" {
        if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
            return p.replacen("~", &home, 1);
        }
    }
    p.to_string()
}

/// Build a `RepositoryManager` from the provided or configured package paths.
fn make_repo_manager(
    paths: Option<Vec<String>>,
) -> rez_next_repository::simple_repository::RepositoryManager {
    use rez_next_common::config::RezCoreConfig;
    use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let mut repo_manager = RepositoryManager::new();

    let pkg_paths: Vec<PathBuf> = paths
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    for (i, path) in pkg_paths.iter().enumerate() {
        if path.exists() {
            repo_manager.add_repository(Box::new(SimpleRepository::new(
                path.clone(),
                format!("repo_{}", i),
            )));
        }
    }

    repo_manager
}

/// Get the latest version of a package from all configured repositories.
/// Equivalent to `rez.packages.get_latest_package(name, range_)`
#[pyfunction]
#[pyo3(signature = (name, range_=None, paths=None))]
pub fn get_latest_package(
    name: &str,
    range_: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Filter by range if specified
    let filtered: Vec<_> = packages
        .into_iter()
        .filter(|pkg| {
            if let Some(range_str) = range_ {
                if let Some(ref version) = pkg.version {
                    if let Ok(range) = rez_next_version::VersionRange::parse(range_str) {
                        return range.contains(version);
                    }
                }
                true
            } else {
                true
            }
        })
        .collect();

    // Return latest (first after sort by version descending)
    let mut sorted = filtered;
    sorted.sort_by(|a, b| {
        b.version
            .as_ref()
            .and_then(|bv| a.version.as_ref().map(|av| av.cmp(bv)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(sorted.into_iter().next().map(|p| PyPackage((*p).clone())))
}

/// Get a specific version of a package.
/// Equivalent to `rez.packages.get_package(name, version)`
#[pyfunction]
#[pyo3(signature = (name, version=None, paths=None))]
pub fn get_package(
    name: &str,
    version: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Option<PyPackage>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let result = packages.into_iter().find(|pkg| {
        if let Some(ver) = version {
            pkg.version.as_ref().is_some_and(|v| v.as_str() == ver)
        } else {
            true
        }
    });

    Ok(result.map(|p| PyPackage((*p).clone())))
}

/// Resolve a list of package requirements into a ResolvedContext.
/// Equivalent to `rez.resolved_context.ResolvedContext(packages)`
#[pyfunction]
#[pyo3(signature = (packages, paths=None))]
pub fn resolve_packages(
    packages: Vec<String>,
    paths: Option<Vec<String>>,
) -> PyResult<PyResolvedContext> {
    PyResolvedContext::new(packages, paths)
}

/// Iterate over all versions of a package.
/// Equivalent to `rez.packages_.iter_packages(name, range_=None, paths=None)`
/// Returns a Python list of Package objects.
#[pyfunction]
#[pyo3(signature = (name, range_=None, paths=None))]
pub fn iter_packages(
    py: Python,
    name: &str,
    range_: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<Py<PyAny>> {
    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    let packages = rt
        .block_on(repo_manager.find_packages(name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Filter by range if specified
    let mut filtered: Vec<PyPackage> = packages
        .into_iter()
        .filter(|pkg| {
            if let Some(range_str) = range_ {
                if let Some(ref version) = pkg.version {
                    if let Ok(range) = rez_next_version::VersionRange::parse(range_str) {
                        return range.contains(version);
                    }
                }
                true
            } else {
                true
            }
        })
        .map(|p| PyPackage((*p).clone()))
        .collect();

    // Sort by version ascending (standard rez iter_packages order)
    filtered.sort_by(|a, b| {
        a.0.version
            .as_ref()
            .and_then(|av| b.0.version.as_ref().map(|bv| av.cmp(bv)))
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    // Return as Python list
    let list = PyList::new(py, filtered)?;
    Ok(list.into_any().unbind())
}

/// Get all package family names from configured repositories.
/// Equivalent to `rez.packages_.get_package_family_names(paths=None)`
#[pyfunction]
#[pyo3(signature = (paths=None))]
pub fn get_package_family_names(paths: Option<Vec<String>>) -> PyResult<Vec<String>> {
    use std::collections::HashSet;

    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    // Search with empty string to list all packages
    let packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let mut names: HashSet<String> = packages.iter().map(|p| p.name.clone()).collect();
    let mut result: Vec<String> = names.drain().collect();
    result.sort();
    Ok(result)
}

/// Walk all packages across all configured repositories.
/// Equivalent to `rez.packages_.walk_packages(paths=None)`
/// Returns a Python list of (family_name, version_list) tuples.
#[pyfunction]
#[pyo3(signature = (paths=None))]
pub fn walk_packages(py: Python, paths: Option<Vec<String>>) -> PyResult<Py<PyAny>> {
    use std::collections::HashMap;

    let rt = get_runtime();

    let repo_manager = make_repo_manager(paths);

    // Find all packages (empty string matches all)
    let packages = rt
        .block_on(repo_manager.find_packages(""))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    // Group by family name
    let mut families: HashMap<String, Vec<String>> = HashMap::new();
    for pkg in &packages {
        let ver = pkg
            .version
            .as_ref()
            .map(|v| v.as_str())
            .unwrap_or("unknown");
        families
            .entry(pkg.name.clone())
            .or_default()
            .push(ver.to_string());
    }

    // Sort versions within each family
    for versions in families.values_mut() {
        versions.sort();
    }

    // Build result list of (name, versions) tuples
    let result_list = pyo3::types::PyList::empty(py);
    let mut sorted_families: Vec<_> = families.into_iter().collect();
    sorted_families.sort_by(|a, b| a.0.cmp(&b.0));

    for (name, versions) in sorted_families {
        let tuple = pyo3::types::PyTuple::new(
            py,
            [
                name.into_pyobject(py)?.into_any().unbind(),
                pyo3::types::PyList::new(py, versions)?.into_any().unbind(),
            ],
        )?;
        result_list.append(tuple)?;
    }

    Ok(result_list.into_any().unbind())
}

/// Copy a package to another location.
/// Equivalent to `rez cp <pkg> <dest>` / `rez.copy_package(pkg, dest_repo_path)`
#[pyfunction]
#[pyo3(signature = (pkg_name, dest_path, version=None, src_paths=None, force=false))]
pub fn copy_package(
    pkg_name: &str,
    dest_path: &str,
    version: Option<&str>,
    src_paths: Option<Vec<String>>,
    force: bool,
) -> PyResult<String> {
    use rez_next_common::config::RezCoreConfig;
    use std::path::PathBuf;

    let config = RezCoreConfig::load();
    let rt = get_runtime();

    let search_paths: Vec<PathBuf> = src_paths
        .clone()
        .map(|p| p.into_iter().map(PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| PathBuf::from(expand_home(p)))
                .collect()
        });

    let repo_manager = make_repo_manager(src_paths);
    let packages = rt
        .block_on(repo_manager.find_packages(pkg_name))
        .map_err(|e| pyo3::exceptions::PyRuntimeError::new_err(e.to_string()))?;

    let pkg = if let Some(ver) = version {
        packages
            .into_iter()
            .find(|p| p.version.as_ref().is_some_and(|v| v.as_str() == ver))
    } else {
        let mut sorted = packages;
        sorted.sort_by(|a, b| {
            b.version
                .as_ref()
                .and_then(|bv| a.version.as_ref().map(|av| av.cmp(bv)))
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        sorted.into_iter().next()
    };

    let pkg = pkg.ok_or_else(|| {
        pyo3::exceptions::PyLookupError::new_err(format!("Package '{}' not found", pkg_name))
    })?;

    let ver_str = pkg
        .version
        .as_ref()
        .map(|v| v.as_str())
        .unwrap_or("unknown");

    // Find source path
    let mut src_root: Option<PathBuf> = None;
    for base in &search_paths {
        let candidate = base.join(&pkg.name).join(ver_str);
        if candidate.exists() {
            src_root = Some(candidate);
            break;
        }
    }

    let src_root = src_root.ok_or_else(|| {
        pyo3::exceptions::PyFileNotFoundError::new_err(format!(
            "Package directory for '{}-{}' not found",
            pkg_name, ver_str
        ))
    })?;

    let dest_root = PathBuf::from(dest_path).join(&pkg.name).join(ver_str);
    if !force && dest_root.exists() {
        return Err(pyo3::exceptions::PyFileExistsError::new_err(format!(
            "Destination already exists: {}. Use force=True to overwrite.",
            dest_root.display()
        )));
    }

    if force && dest_root.exists() {
        std::fs::remove_dir_all(&dest_root)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
    }

    copy_dir_recursive(&src_root, &dest_root)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

    Ok(dest_root.to_string_lossy().to_string())
}

/// Move a package to another location.
/// Equivalent to `rez mv <pkg> <dest>`
#[pyfunction]
#[pyo3(signature = (pkg_name, dest_path, version=None, src_paths=None, force=false, keep_source=false))]
pub fn move_package(
    pkg_name: &str,
    dest_path: &str,
    version: Option<&str>,
    src_paths: Option<Vec<String>>,
    force: bool,
    keep_source: bool,
) -> PyResult<String> {
    // Copy first, then remove source
    let dest = copy_package(pkg_name, dest_path, version, src_paths.clone(), force)?;

    if !keep_source {
        use rez_next_common::config::RezCoreConfig;
        let config = RezCoreConfig::load();
        let search_paths: Vec<std::path::PathBuf> = src_paths
            .map(|p| p.into_iter().map(std::path::PathBuf::from).collect())
            .unwrap_or_else(|| {
                config
                    .packages_path
                    .iter()
                    .map(|p| std::path::PathBuf::from(expand_home(p)))
                    .collect()
            });

        let ver_display = version.unwrap_or("unknown");
        for base in &search_paths {
            let candidate = base.join(pkg_name).join(ver_display);
            if candidate.exists() {
                std::fs::remove_dir_all(&candidate)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                break;
            }
        }
    }

    Ok(dest)
}

/// Remove a package version from repositories.
/// Equivalent to `rez rm <pkg>`
#[pyfunction]
#[pyo3(signature = (pkg_name, version=None, paths=None))]
pub fn remove_package(
    pkg_name: &str,
    version: Option<&str>,
    paths: Option<Vec<String>>,
) -> PyResult<usize> {
    use rez_next_common::config::RezCoreConfig;
    let config = RezCoreConfig::load();

    let search_paths: Vec<std::path::PathBuf> = paths
        .map(|p| p.into_iter().map(std::path::PathBuf::from).collect())
        .unwrap_or_else(|| {
            config
                .packages_path
                .iter()
                .map(|p| std::path::PathBuf::from(expand_home(p)))
                .collect()
        });

    let mut removed = 0usize;
    for base in &search_paths {
        let pkg_base = base.join(pkg_name);
        if !pkg_base.exists() {
            continue;
        }

        if let Some(ver) = version {
            let candidate = pkg_base.join(ver);
            if candidate.exists() {
                std::fs::remove_dir_all(&candidate)
                    .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
                removed += 1;
            }
        } else {
            // Remove entire package family
            std::fs::remove_dir_all(&pkg_base)
                .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;
            removed += 1;
        }
    }

    Ok(removed)
}

/// Recursively copy a directory tree.
pub(crate) fn copy_dir_recursive(
    src: &std::path::Path,
    dest: &std::path::Path,
) -> std::io::Result<()> {
    std::fs::create_dir_all(dest)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    mod test_expand_home {
        use super::*;

        #[test]
        fn test_expand_home_no_tilde() {
            let p = "/absolute/path";
            assert_eq!(expand_home(p), p);
        }

        #[test]
        fn test_expand_home_relative_no_tilde() {
            let p = "relative/path";
            assert_eq!(expand_home(p), p);
        }

        #[test]
        fn test_expand_home_tilde_slash_expands() {
            // If HOME/USERPROFILE is set, ~/foo must be expanded to <home>/foo
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                let expanded = expand_home("~/packages");
                assert!(
                    expanded.starts_with(&home),
                    "expanded '{}' should start with home '{}'",
                    expanded,
                    home
                );
                assert!(
                    expanded.ends_with("packages") || expanded.contains("packages"),
                    "expanded path should retain the suffix"
                );
            }
        }

        #[test]
        fn test_expand_home_bare_tilde_expands() {
            if let Ok(home) = std::env::var("USERPROFILE").or_else(|_| std::env::var("HOME")) {
                let expanded = expand_home("~");
                assert_eq!(expanded, home);
            }
        }

        #[test]
        fn test_expand_home_tilde_in_middle_is_unchanged() {
            // Only leading ~ is handled
            let p = "/some/~/path";
            assert_eq!(expand_home(p), p);
        }
    }

    mod test_remove_package {
        use super::*;

        #[test]
        fn test_remove_package_nonexistent_returns_zero() {
            // Removing a package from an empty/nonexistent path must return 0, not error.
            let tmp = std::env::temp_dir().join("rez_test_rm_nonexistent");
            let _ = fs::remove_dir_all(&tmp);
            fs::create_dir_all(&tmp).unwrap();

            let result = remove_package(
                "nonexistent_pkg_xyz",
                None,
                Some(vec![tmp.to_string_lossy().to_string()]),
            );
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 0, "nothing to remove → count must be 0");

            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_remove_package_specific_version() {
            // Create a fake package directory and remove a specific version
            let tmp = std::env::temp_dir().join("rez_test_rm_version");
            let _ = fs::remove_dir_all(&tmp);

            let pkg_dir = tmp.join("mypkg").join("1.0.0");
            fs::create_dir_all(&pkg_dir).unwrap();
            fs::write(pkg_dir.join("package.py"), b"name = 'mypkg'\nversion = '1.0.0'\n")
                .unwrap();

            let result = remove_package(
                "mypkg",
                Some("1.0.0"),
                Some(vec![tmp.to_string_lossy().to_string()]),
            );
            assert!(result.is_ok(), "remove must succeed: {:?}", result);
            assert_eq!(result.unwrap(), 1, "should have removed 1 version");
            assert!(!pkg_dir.exists(), "version directory must be deleted");

            let _ = fs::remove_dir_all(&tmp);
        }

        #[test]
        fn test_remove_package_entire_family() {
            // Remove the entire package family (no version specified)
            let tmp = std::env::temp_dir().join("rez_test_rm_family");
            let _ = fs::remove_dir_all(&tmp);

            let v1 = tmp.join("myfamily").join("1.0.0");
            let v2 = tmp.join("myfamily").join("2.0.0");
            fs::create_dir_all(&v1).unwrap();
            fs::create_dir_all(&v2).unwrap();

            let result = remove_package(
                "myfamily",
                None,
                Some(vec![tmp.to_string_lossy().to_string()]),
            );
            assert!(result.is_ok());
            assert_eq!(result.unwrap(), 1, "should have removed 1 family dir");
            assert!(!tmp.join("myfamily").exists());

            let _ = fs::remove_dir_all(&tmp);
        }
    }

    mod test_copy_dir_recursive {
        use super::*;

        #[test]
        fn test_copy_flat_directory() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_src_flat");
            let dest = tmp.join("rez_test_copy_dest_flat");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("file1.txt"), b"hello").unwrap();
            fs::write(src.join("file2.txt"), b"world").unwrap();

            copy_dir_recursive(&src, &dest).unwrap();

            assert!(dest.join("file1.txt").exists());
            assert!(dest.join("file2.txt").exists());
            assert_eq!(fs::read(dest.join("file1.txt")).unwrap(), b"hello");
            assert_eq!(fs::read(dest.join("file2.txt")).unwrap(), b"world");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_nested_directory() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_src_nested");
            let dest = tmp.join("rez_test_copy_dest_nested");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            let sub = src.join("subdir");
            fs::create_dir_all(&sub).unwrap();
            fs::write(src.join("root.txt"), b"root").unwrap();
            fs::write(sub.join("child.txt"), b"child").unwrap();

            copy_dir_recursive(&src, &dest).unwrap();

            assert!(dest.join("root.txt").exists());
            assert!(dest.join("subdir").join("child.txt").exists());
            assert_eq!(
                fs::read(dest.join("subdir").join("child.txt")).unwrap(),
                b"child"
            );

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_empty_directory() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_src_empty");
            let dest = tmp.join("rez_test_copy_dest_empty");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            fs::create_dir_all(&src).unwrap();
            copy_dir_recursive(&src, &dest).unwrap();
            assert!(dest.exists());

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_preserves_file_content() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_src_content");
            let dest = tmp.join("rez_test_copy_dest_content");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            fs::create_dir_all(&src).unwrap();
            let content = b"rez-next package.py content\nversion = '1.0.0'\n";
            fs::write(src.join("package.py"), content).unwrap();

            copy_dir_recursive(&src, &dest).unwrap();

            let copied = fs::read(dest.join("package.py")).unwrap();
            assert_eq!(copied, content);

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_over_existing_dest_overwrites() {
            // copy_dir_recursive does NOT check for conflict; it just overwrites.
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_overwrite_src");
            let dest = tmp.join("rez_test_copy_overwrite_dest");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            fs::create_dir_all(&src).unwrap();
            fs::write(src.join("package.py"), b"new content").unwrap();

            // Pre-create dest with old content
            fs::create_dir_all(&dest).unwrap();
            fs::write(dest.join("package.py"), b"old content").unwrap();

            copy_dir_recursive(&src, &dest).unwrap();

            let result = fs::read(dest.join("package.py")).unwrap();
            assert_eq!(result, b"new content", "copy must overwrite old file");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_multiple_files_all_transferred() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_copy_multi_src");
            let dest = tmp.join("rez_test_copy_multi_dest");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            fs::create_dir_all(&src).unwrap();
            for i in 0..5 {
                fs::write(src.join(format!("file{}.txt", i)), format!("content{}", i).as_bytes())
                    .unwrap();
            }

            copy_dir_recursive(&src, &dest).unwrap();

            for i in 0..5 {
                let p = dest.join(format!("file{}.txt", i));
                assert!(p.exists(), "file{}.txt should exist in dest", i);
                let content = fs::read_to_string(&p).unwrap();
                assert_eq!(content, format!("content{}", i));
            }

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }

        #[test]
        fn test_copy_deeply_nested_structure() {
            let tmp = std::env::temp_dir();
            let src = tmp.join("rez_test_deep_src");
            let dest = tmp.join("rez_test_deep_dest");

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);

            let deep = src.join("a").join("b").join("c");
            fs::create_dir_all(&deep).unwrap();
            fs::write(deep.join("leaf.txt"), b"deep file").unwrap();

            copy_dir_recursive(&src, &dest).unwrap();

            assert!(
                dest.join("a").join("b").join("c").join("leaf.txt").exists(),
                "deeply nested file must be copied"
            );

            let _ = fs::remove_dir_all(&src);
            let _ = fs::remove_dir_all(&dest);
        }
    }

    mod test_expand_home_extra {
        use super::*;

        #[test]
        fn test_expand_home_empty_string() {
            // Empty string should return empty (not panic)
            let result = expand_home("");
            assert_eq!(result, "");
        }

        #[test]
        fn test_expand_home_only_slash() {
            let result = expand_home("/");
            assert_eq!(result, "/");
        }

        #[test]
        fn test_expand_home_tilde_not_at_start() {
            let result = expand_home("no/tilde/here");
            assert_eq!(result, "no/tilde/here");
        }

        #[test]
        fn test_expand_home_double_slash_path() {
            let result = expand_home("//some//path");
            assert_eq!(result, "//some//path");
        }

        #[test]
        fn test_expand_home_windows_absolute_path() {
            // Windows paths like C:\... should pass through unchanged
            let result = expand_home(r"C:\Users\foo\packages");
            assert_eq!(result, r"C:\Users\foo\packages");
        }
    }
}

