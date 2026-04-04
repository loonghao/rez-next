//! Search filter definitions

/// How the name pattern should be matched
#[derive(Debug, Clone, PartialEq, Default)]
pub enum FilterMode {
    /// Exact name match (case-insensitive)
    Exact,
    /// Prefix match: names starting with the pattern
    #[default]
    Prefix,
    /// Substring match
    Contains,
    /// Regex pattern match
    Regex,
}

/// A composite filter for package search
#[derive(Debug, Clone, Default)]
pub struct SearchFilter {
    /// Name pattern (empty = match all)
    pub name_pattern: String,
    /// Matching mode for name
    pub mode: FilterMode,
    /// Only return packages whose latest version satisfies this range
    pub version_range: Option<String>,
    /// Maximum number of results (0 = unlimited)
    pub limit: usize,
    /// Include packages from all repos or stop at first match
    pub all_repos: bool,
}

impl SearchFilter {
    pub fn new(pattern: impl Into<String>) -> Self {
        Self {
            name_pattern: pattern.into(),
            mode: FilterMode::Prefix,
            version_range: None,
            limit: 0,
            all_repos: true,
        }
    }

    pub fn with_mode(mut self, mode: FilterMode) -> Self {
        self.mode = mode;
        self
    }

    pub fn with_version_range(mut self, range: impl Into<String>) -> Self {
        self.version_range = Some(range.into());
        self
    }

    pub fn with_limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    /// Check whether a package name matches this filter
    pub fn matches_name(&self, name: &str) -> bool {
        if self.name_pattern.is_empty() {
            return true;
        }
        let pattern = self.name_pattern.to_lowercase();
        let target = name.to_lowercase();
        match self.mode {
            FilterMode::Exact => target == pattern,
            FilterMode::Prefix => target.starts_with(&pattern),
            FilterMode::Contains => target.contains(&pattern),
            FilterMode::Regex => regex::Regex::new(&self.name_pattern)
                .map(|re| re.is_match(name))
                .unwrap_or(false),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── empty pattern (match-all) ─────────────────────────────────────────────

    #[test]
    fn test_empty_pattern_matches_anything() {
        let f = SearchFilter::default();
        assert!(f.matches_name("python"));
        assert!(f.matches_name(""));
        assert!(f.matches_name("some-very-long-package-name-123"));
    }

    // ── Prefix mode (default) ─────────────────────────────────────────────────

    #[test]
    fn test_prefix_exact_prefix_matches() {
        let f = SearchFilter::new("py");
        assert!(f.matches_name("python"));
        assert!(f.matches_name("pytest"));
        assert!(f.matches_name("py"));
    }

    #[test]
    fn test_prefix_non_prefix_no_match() {
        let f = SearchFilter::new("py");
        assert!(!f.matches_name("numpy"));
        assert!(!f.matches_name("maya"));
    }

    #[test]
    fn test_prefix_case_insensitive() {
        let f = SearchFilter::new("PY");
        assert!(f.matches_name("python"));
        assert!(f.matches_name("PyTest"));
    }

    #[test]
    fn test_prefix_full_name_matches() {
        let f = SearchFilter::new("python");
        assert!(f.matches_name("python"));
    }

    #[test]
    fn test_prefix_longer_pattern_no_match() {
        let f = SearchFilter::new("python3");
        assert!(!f.matches_name("python"));
    }

    // ── Exact mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_exact_matches_identical() {
        let f = SearchFilter::new("python").with_mode(FilterMode::Exact);
        assert!(f.matches_name("python"));
        assert!(f.matches_name("PYTHON")); // case-insensitive
    }

    #[test]
    fn test_exact_no_partial_match() {
        let f = SearchFilter::new("py").with_mode(FilterMode::Exact);
        assert!(!f.matches_name("python"));
        assert!(!f.matches_name("numpy"));
    }

    // ── Contains mode ────────────────────────────────────────────────────────

    #[test]
    fn test_contains_middle_substring() {
        let f = SearchFilter::new("umi").with_mode(FilterMode::Contains);
        assert!(f.matches_name("luminance"));
        assert!(f.matches_name("illumination"));
    }

    #[test]
    fn test_contains_prefix_also_matches() {
        let f = SearchFilter::new("py").with_mode(FilterMode::Contains);
        assert!(f.matches_name("python"));
        assert!(f.matches_name("numpy"));
    }

    #[test]
    fn test_contains_no_match() {
        let f = SearchFilter::new("xyz").with_mode(FilterMode::Contains);
        assert!(!f.matches_name("python"));
    }

    #[test]
    fn test_contains_case_insensitive() {
        let f = SearchFilter::new("NUMPY").with_mode(FilterMode::Contains);
        assert!(f.matches_name("numpy"));
    }

    // ── Regex mode ───────────────────────────────────────────────────────────

    #[test]
    fn test_regex_anchored_match() {
        let f = SearchFilter::new("^py").with_mode(FilterMode::Regex);
        assert!(f.matches_name("python"));
        assert!(f.matches_name("pytest"));
        assert!(!f.matches_name("numpy"));
    }

    #[test]
    fn test_regex_wildcard_match() {
        let f = SearchFilter::new("py.*on").with_mode(FilterMode::Regex);
        assert!(f.matches_name("python"));
        assert!(!f.matches_name("pytest"));
    }

    #[test]
    fn test_regex_invalid_pattern_no_match() {
        // Invalid regex should not panic — just return false
        let f = SearchFilter::new("[invalid(").with_mode(FilterMode::Regex);
        assert!(!f.matches_name("python"));
    }

    // ── Builder methods ───────────────────────────────────────────────────────

    #[test]
    fn test_with_version_range_sets_field() {
        let f = SearchFilter::new("py").with_version_range(">=3.9");
        assert_eq!(f.version_range.as_deref(), Some(">=3.9"));
    }

    #[test]
    fn test_with_limit_sets_field() {
        let f = SearchFilter::new("py").with_limit(10);
        assert_eq!(f.limit, 10);
    }

    #[test]
    fn test_default_mode_is_prefix() {
        let f = SearchFilter::new("py");
        assert_eq!(f.mode, FilterMode::Prefix);
    }

    #[test]
    fn test_default_limit_is_zero() {
        let f = SearchFilter::new("py");
        assert_eq!(f.limit, 0);
    }
}
