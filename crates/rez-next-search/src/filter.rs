//! Search filter definitions

/// How the name pattern should be matched
#[derive(Debug, Clone, PartialEq)]
#[derive(Default)]
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
            FilterMode::Regex => {
                regex::Regex::new(&self.name_pattern)
                    .map(|re| re.is_match(name))
                    .unwrap_or(false)
            }
        }
    }
}
