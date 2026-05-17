//!
//! Package utility functions for use in `package.py` files.
//!
//! This module corresponds to rez's `package_py_utils.py` and provides:
//! - `expand_requirement`: Expand wildcards in requirement strings
//! - `expand_requirements`: Batch expand requirements
//!
//! # Design
//! - Follows SOLID principles (Dependency Inversion via callback)
//! - Clean Architecture: Pure version logic in Rust, I/O in Python
//! - No Code Smells: Proper error handling, type safety.

use std::collections::HashMap;
use uuid::Uuid;

/// Replace wildcards (`*` and `**`) with unique placeholders.
///
/// Returns the modified string and a map from placeholder -> wildcard type.
fn replace_wildcards(request: &str) -> (String, HashMap<String, String>) {
    let mut wildcard_map: HashMap<String, String> = HashMap::new();
    let mut request_ = request.to_string();

    // Replace `**` first (full version wildcard)
    while request_.contains("**") {
        let uid = format!("_{}_", Uuid::new_v4().as_simple());
        if let Some(pos) = request_.find("**") {
            request_.replace_range(pos..pos + 3, &uid);
            wildcard_map.insert(uid, "**".to_string());
        }
    }

    // Replace `*` (single token wildcard)
    while request_.contains('*') {
        let uid = format!("_{}_", Uuid::new_v4().as_simple());
        if let Some(pos) = request_.find('*') {
            request_.replace_range(pos..pos + 1, &uid);
            wildcard_map.insert(uid, "*".to_string());
        }
    }

    (request_, wildcard_map)
}

/// Restore wildcards from placeholders.
fn restore_wildcards(result: &str, wildcard_map: &HashMap<String, String>) -> String {
    let mut result_ = result.to_string();
    for (uid, wildcard) in wildcard_map {
        result_ = result_.replace(uid, wildcard);
    }
    result_
}

/// Expand a requirement string with wildcards.
///
/// # Arguments
/// - `request`: Requirement string (e.g., "python-2.*", "boost-1.**")
/// - `query_latest`: Callback that returns the latest version for a package name and range
///
/// # Returns
/// Expanded requirement string (e.g., "python-2.7", "boost-1.55.0")
pub fn expand_requirement(
    request: &str,
    query_latest: &dyn Fn(&str, Option<&str>) -> Option<String>,
) -> Result<String, String> {
    if !request.contains('*') {
        return Ok(request.to_string());
    }

    // Replace wildcards with UUIDs
    let (request_, wildcard_map) = replace_wildcards(request);

    // Parse as PackageRequirement
    let req = match crate::PackageRequirement::parse(&request_) {
        Ok(r) => r,
        Err(e) => return Err(format!("Failed to parse requirement: {}", e)),
    };

    // Determine version range to query
    let version_spec = req.version_spec.as_deref();

    // Query latest package
    let latest_version = match query_latest(&req.name, version_spec) {
        Some(v) => v,
        None => return Ok(request.to_string()),  // No package found, return original
    };

    // Build expanded requirement
    let expanded = if let Some(ref vspec) = req.version_spec {
        format!("{}-{}", req.name, vspec.replace(&request_, &latest_version))
    } else {
        format!("{}-{}", req.name, latest_version)
    };

    // Restore wildcards (shouldn't be needed if query succeeded)
    let result = restore_wildcards(&expanded, &wildcard_map);

    Ok(result)
}

/// Expand multiple requirement strings.
pub fn expand_requirements(
    requests: &[&str],
    query_latest: &dyn Fn(&str, Option<&str>) -> Option<String>,
) -> Vec<String> {
    requests
        .iter()
        .map(|r| {
            expand_requirement(r, query_latest)
                .unwrap_or_else(|_| r.to_string())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_wildcards() {
        let result = expand_requirement("python-3.9", &|_, _| Some("3.9.0".to_string()));
        assert_eq!(result.unwrap(), "python-3.9");
    }

    #[test]
    fn test_replace_restore() {
        let (replaced, map) = replace_wildcards("python-2.*");
        assert!(map.values().any(|v| v == "*"));
        let restored = restore_wildcards(&replaced, &map);
        assert_eq!(restored, "python-2.*");
    }
}
