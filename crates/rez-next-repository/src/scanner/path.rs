//! Path utilities for the repository scanner.
//!
//! This module handles:
//! - Path normalisation for consistent cache keys.
//! - Exclude-pattern matching (`should_exclude_path`).
//! - Include-pattern matching (`is_package_file`, `matches_pattern`).
//! - Glob-to-regex compilation (`glob_to_regex`).

use super::RepositoryScanner;
use std::path::{Path, PathBuf};

impl RepositoryScanner {
    /// Convert a glob pattern string into a compiled `Regex`.
    ///
    /// Transformation rules:
    /// - `**` → `.*`  (any path segment(s))
    /// - `*`  → `[^/]*`  (any characters except `/`)
    /// - `?`  → `.`  (any single character)
    /// - all other regex metacharacters are escaped
    pub(super) fn glob_to_regex(pattern: &str) -> Option<regex::Regex> {
        const DOUBLE_STAR_PLACEHOLDER: &str = "\x00DS\x00";
        let re_pattern = pattern
            .replace("**", DOUBLE_STAR_PLACEHOLDER)
            .replace('*', "[^/]*")
            .replace(DOUBLE_STAR_PLACEHOLDER, ".*")
            .replace('?', ".");
        regex::Regex::new(&format!("^{}$", re_pattern)).ok()
    }

    /// Check if a file is a package definition file.
    pub(super) fn is_package_file(&self, path: &Path) -> bool {
        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
            // Fast O(1) lookup for exact filenames (e.g. "package.py").
            // Falls back to wildcard matching for any non-exact patterns.
            if self.include_filenames.contains(filename) {
                return true;
            }
            // Wildcard patterns (if any were configured)
            self.config
                .include_patterns
                .iter()
                .filter(|p| p.contains('*') || p.contains('?'))
                .any(|pattern| self.matches_pattern(filename, pattern))
        } else {
            false
        }
    }

    /// Check if a path should be excluded from scanning.
    pub(super) fn should_exclude_path(&self, path: &Path) -> bool {
        // Normalize to forward slashes for cross-platform pattern matching
        let path_str = path.to_string_lossy().replace('\\', "/");

        self.exclude_regexes.iter().any(|re| {
            // Try full-path match first
            if re.is_match(&path_str) {
                return true;
            }
            // Also try matching any path suffix so that patterns like ".git/**"
            // match paths such as "/repo/.git/objects".
            let mut search_start = 0usize;
            while let Some(sep_idx) = path_str[search_start..].find('/') {
                let abs_idx = search_start + sep_idx + 1;
                let suffix = &path_str[abs_idx..];
                if !suffix.is_empty() && re.is_match(suffix) {
                    return true;
                }
                search_start = abs_idx;
            }
            false
        })
    }

    /// Normalize path for consistent cache key generation.
    pub(super) fn normalize_path(&self, path: &Path) -> PathBuf {
        match path.canonicalize() {
            Ok(canonical) => canonical,
            Err(_) => {
                // Fallback to simple normalization if canonicalize fails
                let mut normalized = PathBuf::new();
                for component in path.components() {
                    match component {
                        std::path::Component::ParentDir => {
                            normalized.pop();
                        }
                        std::path::Component::CurDir => {
                            // Skip current directory references
                        }
                        _ => {
                            normalized.push(component);
                        }
                    }
                }
                normalized
            }
        }
    }

    /// Simple pattern matching (supports `*` and `?` wildcards).
    pub(super) fn matches_pattern(&self, text: &str, pattern: &str) -> bool {
        if pattern == "*" {
            return true;
        }

        // Normalize path separators to forward slash for cross-platform matching
        let normalized_text = text.replace('\\', "/");

        // Convert glob pattern to regex.
        // Use a placeholder for ** to prevent the subsequent * replacement from
        // corrupting the already-converted .* token.
        const DOUBLE_STAR_PLACEHOLDER: &str = "\x00DOUBLESTAR\x00";
        let regex_pattern = pattern
            .replace("**", DOUBLE_STAR_PLACEHOLDER)
            .replace('*', "[^/]*")
            .replace(DOUBLE_STAR_PLACEHOLDER, ".*")
            .replace('?', ".");

        if let Ok(regex) = regex::Regex::new(&format!("^{}$", regex_pattern)) {
            regex.is_match(&normalized_text)
        } else {
            // Fallback to exact match
            normalized_text == pattern
        }
    }
}
