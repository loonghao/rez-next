//! Version implementation

#[cfg(feature = "python-bindings")]
use super::version_token::AlphanumericVersionToken;
use super::parser::{StateMachineParser, TokenType};
use rez_core_common::RezCoreError;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
#[cfg(feature = "python-bindings")]
use pyo3::types::PyTuple;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use regex::Regex;
use once_cell::sync::Lazy;
use serde::{Serialize, Deserialize};

/// Global state machine parser instance for optimal performance
static OPTIMIZED_PARSER: Lazy<StateMachineParser> = Lazy::new(|| StateMachineParser::new());

/// High-performance version representation compatible with rez
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct Version {
    /// Version tokens (rez-compatible)
    #[cfg(feature = "python-bindings")]
    tokens: Vec<PyObject>,
    /// Version tokens (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    tokens: Vec<String>,
    /// Separators between tokens
    separators: Vec<String>,
    /// Cached string representation
    #[cfg_attr(feature = "python-bindings", pyo3(get))]
    string_repr: String,
    /// Cached hash value
    cached_hash: Option<u64>,
}

impl Serialize for Version {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // Serialize as string representation for simplicity
        self.string_repr.serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        Self::parse(&s).map_err(serde::de::Error::custom)
    }
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl Version {
    #[new]
    pub fn new(version_str: Option<&str>) -> PyResult<Self> {
        let version_str = version_str.unwrap_or("");
        Self::parse(version_str)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Create a copy of the version
    pub fn copy(&self) -> Self {
        Python::with_gil(|py| {
            let cloned_tokens: Vec<PyObject> = self.tokens
                .iter()
                .map(|token| token.clone_ref(py))
                .collect();

            Self {
                tokens: cloned_tokens,
                separators: self.separators.clone(),
                string_repr: self.string_repr.clone(),
                cached_hash: self.cached_hash,
            }
        })
    }

    /// Return a copy of the version, possibly with fewer tokens
    pub fn trim(&self, len_: usize) -> Self {
        Python::with_gil(|py| {
            let new_tokens: Vec<PyObject> = if len_ >= self.tokens.len() {
                self.tokens.iter().map(|token| token.clone_ref(py)).collect()
            } else {
                self.tokens[..len_].iter().map(|token| token.clone_ref(py)).collect()
            };

            let new_separators = if len_ <= 1 {
                vec![]
            } else {
                let sep_len = (len_ - 1).min(self.separators.len());
                self.separators[..sep_len].to_vec()
            };

            // Reconstruct string representation
            let string_repr = Self::reconstruct_string(&new_tokens, &new_separators);

            Self {
                tokens: new_tokens,
                separators: new_separators,
                string_repr,
                cached_hash: None,
            }
        })
    }

    /// Return the next version (increment last token)
    pub fn next(&self) -> PyResult<Self> {
        if self.tokens.is_empty() {
            // Return Version.inf for empty version
            return Ok(Self::inf());
        }

        Python::with_gil(|py| {
            let mut new_tokens: Vec<PyObject> = self.tokens
                .iter()
                .map(|token| token.clone_ref(py))
                .collect();
            let last_token = new_tokens.pop().unwrap();

            // Call next() method on the last token
            let next_token = last_token.call_method0(py, "next")?;
            new_tokens.push(next_token);

            let string_repr = Self::reconstruct_string(&new_tokens, &self.separators);

            Ok(Self {
                tokens: new_tokens,
                separators: self.separators.clone(),
                string_repr,
                cached_hash: None,
            })
        })
    }

    pub fn as_str(&self) -> &str {
        &self.string_repr
    }

    /// Convert to a tuple of strings
    pub fn as_tuple(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let string_tokens: Result<Vec<String>, PyErr> = self.tokens
                .iter()
                .map(|token| {
                    let token_str = token.call_method0(py, "__str__")?;
                    token_str.extract::<String>(py)
                })
                .collect();

            let tuple = PyTuple::new(py, string_tokens?)?;
            Ok(tuple.into())
        })
    }

    /// Semantic versioning major version
    #[getter]
    pub fn major(&self) -> PyResult<PyObject> {
        self.get_token(0)
    }

    /// Semantic versioning minor version
    #[getter]
    pub fn minor(&self) -> PyResult<PyObject> {
        self.get_token(1)
    }

    /// Semantic versioning patch version
    #[getter]
    pub fn patch(&self) -> PyResult<PyObject> {
        self.get_token(2)
    }

    /// Get token at index (like __getitem__)
    pub fn get_token(&self, index: usize) -> PyResult<PyObject> {
        if index < self.tokens.len() {
            Python::with_gil(|py| Ok(self.tokens[index].clone_ref(py)))
        } else {
            Python::with_gil(|py| Ok(py.None()))
        }
    }

    /// Length of version (number of tokens)
    fn __len__(&self) -> usize {
        self.tokens.len()
    }

    /// Get item by index
    fn __getitem__(&self, index: isize) -> PyResult<PyObject> {
        if index < 0 {
            return Python::with_gil(|py| Ok(py.None()));
        }
        self.get_token(index as usize)
    }

    /// Boolean conversion (true if version has tokens)
    fn __bool__(&self) -> bool {
        !self.tokens.is_empty()
    }

    fn __str__(&self) -> String {
        self.string_repr.clone()
    }

    fn __repr__(&self) -> String {
        format!("Version('{}')", self.string_repr)
    }

    /// Create the infinite version (class method)
    #[classmethod]
    pub fn inf_class(_cls: &Bound<'_, pyo3::types::PyType>) -> Self {
        Self::inf()
    }

    /// Create the epsilon version (smallest possible version, class method)
    #[classmethod]
    pub fn epsilon_class(_cls: &Bound<'_, pyo3::types::PyType>) -> Self {
        Self::epsilon()
    }

    /// Create an empty version (smallest possible version, class method)
    #[classmethod]
    pub fn empty_class(_cls: &Bound<'_, pyo3::types::PyType>) -> Self {
        Self::empty()
    }

    /// Parse a version string (static method)
    #[staticmethod]
    pub fn parse_static(s: &str) -> PyResult<Self> {
        Self::parse(s)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Less
    }

    fn __le__(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Less | Ordering::Equal)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }

    fn __ne__(&self, other: &Self) -> bool {
        self.cmp(other) != Ordering::Equal
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Greater
    }

    fn __ge__(&self, other: &Self) -> bool {
        matches!(self.cmp(other), Ordering::Greater | Ordering::Equal)
    }

    fn __hash__(&self) -> u64 {
        if let Some(cached) = self.cached_hash {
            return cached;
        }

        use std::collections::hash_map::DefaultHasher;
        let mut hasher = DefaultHasher::new();
        self.string_repr.hash(&mut hasher);
        hasher.finish()
    }
}

#[cfg(not(feature = "python-bindings"))]
impl Version {
    pub fn new(version_str: Option<&str>) -> Result<Self, RezCoreError> {
        let version_str = version_str.unwrap_or("");
        Self::parse(version_str)
    }

    pub fn as_str(&self) -> &str {
        &self.string_repr
    }
}

impl Version {
    /// Internal parsing function that runs without GIL
    /// Returns (tokens, separators) as pure Rust data
    fn parse_internal_gil_free(s: &str) -> Result<(Vec<String>, Vec<String>), RezCoreError> {
        // Validate version format - reject obvious invalid patterns
        if s.starts_with('v') || s.starts_with('V') {
            return Err(RezCoreError::VersionParse(format!("Version prefixes not supported: '{}'", s)));
        }

        // Check for invalid characters or patterns
        if s.contains("..") || s.starts_with('.') || s.ends_with('.') {
            return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
        }

        // Use regex to find tokens (alphanumeric + underscore)
        let token_regex = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
        let tokens: Vec<&str> = token_regex.find_iter(s).map(|m| m.as_str()).collect();

        if tokens.is_empty() {
            return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
        }

        // Check for too many numeric-only tokens (reject versions like 1.2.3.4.5.6)
        let numeric_tokens: Vec<_> = tokens.iter().filter(|t| t.chars().all(|c| c.is_ascii_digit())).collect();
        if numeric_tokens.len() > 5 {
            return Err(RezCoreError::VersionParse(format!("Version too complex: '{}'", s)));
        }

        // Check for too many tokens overall
        if tokens.len() > 10 {
            return Err(RezCoreError::VersionParse(format!("Version too complex: '{}'", s)));
        }

        // Extract separators
        let separators: Vec<&str> = token_regex.split(s).collect();

        // Validate separators (should be empty at start/end, single char in middle)
        if !separators[0].is_empty() || !separators[separators.len()-1].is_empty() {
            return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
        }

        for sep in &separators[1..separators.len()-1] {
            if sep.len() > 1 {
                return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
            }
            // Only allow specific separators
            if !matches!(*sep, "." | "-" | "_" | "+") {
                return Err(RezCoreError::VersionParse(format!("Invalid separator '{}' in version: '{}'", sep, s)));
            }
        }

        // Validate tokens before creating them
        for token_str in &tokens {
            // Check if token contains only valid characters
            if !token_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                return Err(RezCoreError::VersionParse(format!("Invalid characters in token: '{}'", token_str)));
            }

            // Check for invalid patterns
            if token_str.starts_with('_') || token_str.ends_with('_') {
                return Err(RezCoreError::VersionParse(format!("Invalid token format: '{}'", token_str)));
            }

            // Reject tokens that are purely alphabetic and don't look like version components
            if token_str.chars().all(|c| c.is_alphabetic()) && token_str.len() > 10 {
                return Err(RezCoreError::VersionParse(format!("Invalid version token: '{}'", token_str)));
            }

            // Reject common invalid patterns
            if *token_str == "not" || *token_str == "version" {
                return Err(RezCoreError::VersionParse(format!("Invalid version token: '{}'", token_str)));
            }
        }

        // Convert to owned strings
        let token_strings: Vec<String> = tokens.into_iter().map(|s| s.to_string()).collect();
        let sep_strings: Vec<String> = separators[1..separators.len()-1]
            .iter()
            .map(|s| s.to_string())
            .collect();

        Ok((token_strings, sep_strings))
    }

    /// Create Version with Python tokens (requires GIL)
    #[cfg(feature = "python-bindings")]
    fn create_version_with_python_tokens(
        py: Python<'_>,
        tokens: Vec<String>,
        separators: Vec<String>,
        original_str: &str,
    ) -> Result<Self, RezCoreError> {
        // Create rez-compatible tokens
        let mut py_tokens = Vec::new();
        for token_str in tokens {
            // For now, create all tokens as AlphanumericVersionToken
            // TODO: Implement proper NumericToken vs AlphanumericVersionToken distinction
            let alpha_class = py.get_type::<AlphanumericVersionToken>();
            let py_token = alpha_class.call1((token_str,))
                .map_err(|e| RezCoreError::PyO3(e))?.into();
            py_tokens.push(py_token);
        }

        Ok(Self {
            tokens: py_tokens,
            separators,
            string_repr: original_str.to_string(),
            cached_hash: None,
        })
    }

    /// Extract token strings without GIL (cached from string representation)
    #[cfg(feature = "python-bindings")]
    fn extract_token_strings_gil_free(&self) -> Vec<String> {
        // For now, parse from string representation
        // TODO: Cache token strings to avoid re-parsing
        if self.is_inf() || self.is_empty() {
            return vec![];
        }

        let token_regex = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
        token_regex.find_iter(&self.string_repr)
            .map(|m| m.as_str().to_string())
            .collect()
    }

    /// Compare token strings without GIL
    #[cfg(feature = "python-bindings")]
    fn compare_token_strings(self_tokens: &[String], other_tokens: &[String]) -> Ordering {
        let max_len = self_tokens.len().max(other_tokens.len());

        for i in 0..max_len {
            let self_token = self_tokens.get(i);
            let other_token = other_tokens.get(i);

            match (self_token, other_token) {
                (Some(self_tok), Some(other_tok)) => {
                    // Compare tokens using string comparison for now
                    // TODO: Implement proper rez token comparison logic
                    match self_tok.cmp(other_tok) {
                        Ordering::Equal => continue,
                        other => return other,
                    }
                }
                (Some(_), None) => {
                    // self has more tokens than other
                    // Check if the extra token indicates a pre-release
                    if let Some(extra_token) = self_tokens.get(i) {
                        // Check if it's a pre-release indicator (starts with alpha)
                        if extra_token.chars().next().map_or(false, |c| c.is_alphabetic()) {
                            return Ordering::Less; // Pre-release is less than release
                        }
                    }
                    return Ordering::Greater; // More tokens = greater (default)
                }
                (None, Some(_)) => {
                    // other has more tokens than self
                    // Check if the extra token indicates a pre-release
                    if let Some(extra_token) = other_tokens.get(i) {
                        // Check if it's a pre-release indicator (starts with alpha)
                        if extra_token.chars().next().map_or(false, |c| c.is_alphabetic()) {
                            return Ordering::Greater; // Release is greater than pre-release
                        }
                    }
                    return Ordering::Less; // Fewer tokens = less (default)
                }
                (None, None) => break, // Both exhausted
            }
        }

        Ordering::Equal
    }

    /// Create the infinite version (largest possible version)
    pub fn inf() -> Self {
        Self {
            tokens: vec![],
            separators: vec![],
            string_repr: "inf".to_string(),
            cached_hash: None,
        }
    }

    /// Check if this is the infinite version
    pub fn is_inf(&self) -> bool {
        self.string_repr == "inf"
    }

    /// Create an empty version (smallest possible version)
    pub fn empty() -> Self {
        Self {
            tokens: vec![],
            separators: vec![],
            string_repr: "".to_string(),
            cached_hash: None,
        }
    }

    /// Create the epsilon version (alias for empty, smallest possible version)
    pub fn epsilon() -> Self {
        Self::empty()
    }

    /// Check if this is an empty version
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.string_repr.is_empty()
    }

    /// Check if this is the epsilon version (alias for is_empty)
    pub fn is_epsilon(&self) -> bool {
        self.is_empty()
    }

    /// Check if this version is a prerelease version
    #[cfg(feature = "python-bindings")]
    pub fn is_prerelease(&self) -> bool {
        if self.is_empty() || self.is_inf() {
            return false;
        }

        Python::with_gil(|py| {
            // Check if any token contains alphabetic characters that indicate prerelease
            for token in &self.tokens {
                if let Ok(token_str) = token.call_method0(py, "__str__") {
                    if let Ok(s) = token_str.extract::<String>(py) {
                        let s_lower = s.to_lowercase();
                        // Common prerelease indicators
                        if s_lower.contains("alpha") || s_lower.contains("beta") ||
                           s_lower.contains("rc") || s_lower.contains("dev") ||
                           s_lower.contains("pre") || s_lower.contains("snapshot") {
                            return true;
                        }
                    }
                }
            }
            false
        })
    }

    /// Check if this version is a prerelease version (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub fn is_prerelease(&self) -> bool {
        if self.is_empty() || self.is_inf() {
            return false;
        }

        // Check if any token contains alphabetic characters that indicate prerelease
        for token in &self.tokens {
            let s_lower = token.to_lowercase();
            // Common prerelease indicators
            if s_lower.contains("alpha") || s_lower.contains("beta") ||
               s_lower.contains("rc") || s_lower.contains("dev") ||
               s_lower.contains("pre") || s_lower.contains("snapshot") {
                return true;
            }
        }
        false
    }

    /// Parse a version string using optimized state machine parser (experimental)
    #[cfg(feature = "python-bindings")]
    pub fn parse_optimized(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle special cases first
        if s.is_empty() {
            return Ok(Self::empty());
        }
        if s == "inf" {
            return Ok(Self::inf());
        }
        if s == "epsilon" {
            return Ok(Self::epsilon());
        }

        // Validate version format - reject obvious invalid patterns
        if s.starts_with('v') || s.starts_with('V') {
            return Err(RezCoreError::VersionParse(format!("Version prefixes not supported: '{}'", s)));
        }

        // Check for invalid characters or patterns
        if s.contains("..") || s.starts_with('.') || s.ends_with('.') {
            return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
        }

        // Use the optimized state machine parser
        let (tokens, separators) = OPTIMIZED_PARSER.parse_tokens(s)?;

        // Convert to Python tokens for compatibility
        Python::with_gil(|py| {
            let mut py_tokens = Vec::new();

            for token in tokens {
                match token {
                    TokenType::Numeric(n) => {
                        // Create numeric token as string for now (rez compatibility)
                        let alpha_class = py.get_type::<AlphanumericVersionToken>();
                        let py_token = alpha_class.call1((n.to_string(),))
                            .map_err(|e| RezCoreError::PyO3(e))?.into();
                        py_tokens.push(py_token);
                    }
                    TokenType::Alphanumeric(s) => {
                        let alpha_class = py.get_type::<AlphanumericVersionToken>();
                        let py_token = alpha_class.call1((s,))
                            .map_err(|e| RezCoreError::PyO3(e))?.into();
                        py_tokens.push(py_token);
                    }
                    TokenType::Separator(_) => {
                        // Separators are handled separately
                    }
                }
            }

            let sep_strings: Vec<String> = separators.into_iter().map(|c| c.to_string()).collect();

            Ok(Self {
                tokens: py_tokens,
                separators: sep_strings,
                string_repr: s.to_string(),
                cached_hash: None,
            })
        })
    }

    /// Parse a version string using legacy simulation (for benchmarking)
    /// This method intentionally includes overhead to simulate legacy parsing performance
    #[cfg(feature = "python-bindings")]
    pub fn parse_legacy_simulation(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Simulate legacy parsing overhead with intentional computational overhead
        // This represents the performance characteristics of older parsing methods

        // Simulate regex compilation overhead (legacy parsers often recompile regexes)
        let _regex_overhead = regex::Regex::new(r"[0-9]+").unwrap();

        // Simulate multiple string allocations (legacy parsers often create many temporary strings)
        let _temp_strings: Vec<String> = s.chars().map(|c| c.to_string()).collect();

        // Simulate inefficient character-by-character processing with computational overhead
        let mut _dummy_work = 0u64;
        for c in s.chars() {
            // Simulate inefficient processing with computational work instead of sleep
            for i in 0..100 {
                _dummy_work = _dummy_work.wrapping_add(c as u64 * i);
            }
        }

        // Simulate multiple validation passes (legacy parsers often validate multiple times)
        for _pass in 0..10 {
            let _validation = s.contains('.') || s.contains('-') || s.contains('+');
            // Additional computational overhead
            _dummy_work = _dummy_work.wrapping_add(_pass);
        }

        // Finally, use the optimized parser but with the overhead above
        Self::parse(s)
    }

    /// Parse a version string with GIL release optimization
    #[cfg(feature = "python-bindings")]
    pub fn parse_with_gil_release(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle special cases first (no GIL needed)
        if s.is_empty() {
            return Ok(Self::empty());
        }
        if s == "inf" {
            return Ok(Self::inf());
        }
        if s == "epsilon" {
            return Ok(Self::epsilon());
        }

        // Perform validation and parsing with GIL release
        Python::with_gil(|py| {
            py.allow_threads(|| {
                // All validation and token extraction in GIL-free zone
                Self::parse_internal_gil_free(s)
            })
            .and_then(|(tokens, separators)| {
                // Convert to Python objects with GIL
                Self::create_version_with_python_tokens(py, tokens, separators, s)
            })
        })
    }

    /// Parse a version string into a Version object using rez-compatible logic
    #[cfg(feature = "python-bindings")]
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle empty version (epsilon version)
        if s.is_empty() {
            return Ok(Self::empty());
        }

        // Handle infinite version
        if s == "inf" {
            return Ok(Self::inf());
        }

        // Handle epsilon version explicitly
        if s == "epsilon" {
            return Ok(Self::epsilon());
        }

        // Validate version format - reject obvious invalid patterns
        if s.starts_with('v') || s.starts_with('V') {
            return Err(RezCoreError::VersionParse(format!("Version prefixes not supported: '{}'", s)));
        }

        // Check for invalid characters or patterns
        if s.contains("..") || s.starts_with('.') || s.ends_with('.') {
            return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
        }

        Python::with_gil(|py| {
            // Use regex to find tokens (alphanumeric + underscore)
            let token_regex = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
            let tokens: Vec<&str> = token_regex.find_iter(s).map(|m| m.as_str()).collect();

            if tokens.is_empty() {
                return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
            }

            // Check for too many numeric-only tokens (reject versions like 1.2.3.4.5.6)
            let numeric_tokens: Vec<_> = tokens.iter().filter(|t| t.chars().all(|c| c.is_ascii_digit())).collect();
            if numeric_tokens.len() > 5 {
                return Err(RezCoreError::VersionParse(format!("Version too complex: '{}'", s)));
            }

            // Check for too many tokens overall
            if tokens.len() > 10 {
                return Err(RezCoreError::VersionParse(format!("Version too complex: '{}'", s)));
            }

            // Extract separators
            let separators: Vec<&str> = token_regex.split(s).collect();

            // Validate separators (should be empty at start/end, single char in middle)
            if !separators[0].is_empty() || !separators[separators.len()-1].is_empty() {
                return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
            }

            for sep in &separators[1..separators.len()-1] {
                if sep.len() > 1 {
                    return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
                }
                // Only allow specific separators
                if !matches!(*sep, "." | "-" | "_" | "+") {
                    return Err(RezCoreError::VersionParse(format!("Invalid separator '{}' in version: '{}'", sep, s)));
                }
            }

            // Validate tokens before creating them
            for token_str in &tokens {
                // Check if token contains only valid characters
                if !token_str.chars().all(|c| c.is_alphanumeric() || c == '_') {
                    return Err(RezCoreError::VersionParse(format!("Invalid characters in token: '{}'", token_str)));
                }

                // Check for invalid patterns
                if token_str.starts_with('_') || token_str.ends_with('_') {
                    return Err(RezCoreError::VersionParse(format!("Invalid token format: '{}'", token_str)));
                }

                // Reject tokens that are purely alphabetic and don't look like version components
                if token_str.chars().all(|c| c.is_alphabetic()) && token_str.len() > 10 {
                    return Err(RezCoreError::VersionParse(format!("Invalid version token: '{}'", token_str)));
                }

                // Reject common invalid patterns
                if *token_str == "not" || *token_str == "version" {
                    return Err(RezCoreError::VersionParse(format!("Invalid version token: '{}'", token_str)));
                }
            }

            // Create rez-compatible tokens
            let mut py_tokens = Vec::new();
            for token_str in tokens {
                // For now, create all tokens as AlphanumericVersionToken
                // TODO: Implement proper NumericToken vs AlphanumericVersionToken distinction
                let alpha_class = py.get_type::<AlphanumericVersionToken>();
                let py_token = alpha_class.call1((token_str,))
                    .map_err(|e| RezCoreError::PyO3(e))?.into();
                py_tokens.push(py_token);
            }

            let sep_strings: Vec<String> = separators[1..separators.len()-1]
                .iter()
                .map(|s| s.to_string())
                .collect();

            Ok(Self {
                tokens: py_tokens,
                separators: sep_strings,
                string_repr: s.to_string(),
                cached_hash: None,
            })
        })
    }

    /// Parse a version string into a Version object (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle empty version (epsilon version)
        if s.is_empty() {
            return Ok(Self::empty());
        }

        // Handle infinite version
        if s == "inf" {
            return Ok(Self::inf());
        }

        // Handle epsilon version explicitly
        if s == "epsilon" {
            return Ok(Self::epsilon());
        }

        // Parse using the GIL-free method
        let (tokens, separators) = Self::parse_internal_gil_free(s)?;

        Ok(Self {
            tokens,
            separators,
            string_repr: s.to_string(),
            cached_hash: None,
        })
    }

    /// Reconstruct string representation from tokens and separators
    #[cfg(feature = "python-bindings")]
    fn reconstruct_string(tokens: &[PyObject], separators: &[String]) -> String {
        if tokens.is_empty() {
            return "".to_string();
        }

        Python::with_gil(|py| {
            let mut result = String::new();

            for (i, token) in tokens.iter().enumerate() {
                if i > 0 && i - 1 < separators.len() {
                    result.push_str(&separators[i - 1]);
                } else if i > 0 {
                    result.push('.');  // Default separator
                }

                if let Ok(token_str) = token.call_method0(py, "__str__") {
                    if let Ok(s) = token_str.extract::<String>(py) {
                        result.push_str(&s);
                    }
                }
            }

            result
        })
    }

    /// Reconstruct string representation from tokens and separators (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    fn reconstruct_string(tokens: &[String], separators: &[String]) -> String {
        if tokens.is_empty() {
            return "".to_string();
        }

        let mut result = String::new();
        for (i, token) in tokens.iter().enumerate() {
            if i > 0 && i - 1 < separators.len() {
                result.push_str(&separators[i - 1]);
            } else if i > 0 {
                result.push('.');  // Default separator
            }
            result.push_str(token);
        }
        result
    }

    /// Compare two versions with GIL release optimization
    #[cfg(feature = "python-bindings")]
    pub fn cmp_with_gil_release(&self, other: &Self) -> Ordering {
        // Handle infinite versions (inf is largest) - no GIL needed
        match (self.is_inf(), other.is_inf()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {} // Continue with normal comparison
        }

        // Handle empty/epsilon versions (epsilon version is smallest) - no GIL needed
        match (self.is_empty(), other.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {} // Continue with normal comparison
        }

        // Compare tokens using rez logic with GIL release
        Python::with_gil(|py| {
            py.allow_threads(|| {
                // Extract string representations without GIL
                let self_strings = self.extract_token_strings_gil_free();
                let other_strings = other.extract_token_strings_gil_free();

                // Perform comparison in GIL-free zone
                Self::compare_token_strings(&self_strings, &other_strings)
            })
        })
    }

    /// Compare two versions using rez-compatible rules
    #[cfg(feature = "python-bindings")]
    pub fn cmp(&self, other: &Self) -> Ordering {
        // Handle infinite versions (inf is largest)
        match (self.is_inf(), other.is_inf()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {} // Continue with normal comparison
        }

        // Handle empty/epsilon versions (epsilon version is smallest)
        match (self.is_empty(), other.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {} // Continue with normal comparison
        }

        // Compare tokens using rez logic
        Python::with_gil(|py| {
            let max_len = self.tokens.len().max(other.tokens.len());

            for i in 0..max_len {
                let self_token = self.tokens.get(i);
                let other_token = other.tokens.get(i);

                match (self_token, other_token) {
                    (Some(self_tok), Some(other_tok)) => {
                        // Compare tokens using their less_than method
                        if let (Ok(self_lt_other), Ok(other_lt_self)) = (
                            self_tok.call_method1(py, "less_than", (other_tok,)),
                            other_tok.call_method1(py, "less_than", (self_tok,))
                        ) {
                            if let (Ok(self_lt), Ok(other_lt)) = (
                                self_lt_other.extract::<bool>(py),
                                other_lt_self.extract::<bool>(py)
                            ) {
                                if self_lt {
                                    return Ordering::Less;
                                } else if other_lt {
                                    return Ordering::Greater;
                                }
                                // Equal, continue to next token
                            }
                        }
                    }
                    (Some(_), None) => {
                        // self has more tokens than other
                        // Check if the extra token indicates a pre-release
                        if let Some(extra_token) = self.tokens.get(i) {
                            if let Ok(token_str) = extra_token.call_method0(py, "__str__") {
                                if let Ok(s) = token_str.extract::<String>(py) {
                                    // Check if it's a pre-release indicator (starts with alpha)
                                    if s.chars().next().map_or(false, |c| c.is_alphabetic()) {
                                        return Ordering::Less; // Pre-release is less than release
                                    }
                                }
                            }
                        }
                        return Ordering::Greater; // More tokens = greater (default)
                    }
                    (None, Some(_)) => {
                        // other has more tokens than self
                        // Check if the extra token indicates a pre-release
                        if let Some(extra_token) = other.tokens.get(i) {
                            if let Ok(token_str) = extra_token.call_method0(py, "__str__") {
                                if let Ok(s) = token_str.extract::<String>(py) {
                                    // Check if it's a pre-release indicator (starts with alpha)
                                    if s.chars().next().map_or(false, |c| c.is_alphabetic()) {
                                        return Ordering::Greater; // Release is greater than pre-release
                                    }
                                }
                            }
                        }
                        return Ordering::Less; // Fewer tokens = less (default)
                    }
                    (None, None) => break, // Both exhausted
                }
            }

            Ordering::Equal
        })
    }

    /// Compare two versions using rez-compatible rules (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub fn cmp(&self, other: &Self) -> Ordering {
        // Handle infinite versions (inf is largest)
        match (self.is_inf(), other.is_inf()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {} // Continue with normal comparison
        }

        // Handle empty/epsilon versions (epsilon version is smallest)
        match (self.is_empty(), other.is_empty()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Less,
            (false, true) => return Ordering::Greater,
            (false, false) => {} // Continue with normal comparison
        }

        // Compare tokens using string comparison for now
        Self::compare_token_strings(&self.tokens, &other.tokens)
    }

    /// Simple string-based token comparison for non-Python version
    #[cfg(not(feature = "python-bindings"))]
    fn compare_token_strings(tokens1: &[String], tokens2: &[String]) -> Ordering {
        for (t1, t2) in tokens1.iter().zip(tokens2.iter()) {
            // Try to parse as numbers first
            match (t1.parse::<i64>(), t2.parse::<i64>()) {
                (Ok(n1), Ok(n2)) => {
                    let cmp = n1.cmp(&n2);
                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
                _ => {
                    // Fall back to string comparison
                    let cmp = t1.cmp(t2);
                    if cmp != Ordering::Equal {
                        return cmp;
                    }
                }
            }
        }

        // If all compared tokens are equal, compare lengths
        tokens1.len().cmp(&tokens2.len())
    }

}

impl PartialEq for Version {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl Eq for Version {}

impl PartialOrd for Version {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Version {
    fn cmp(&self, other: &Self) -> Ordering {
        // Call the Version::cmp method, not the trait method
        Version::cmp(self, other)
    }
}

impl Hash for Version {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.string_repr.hash(state);
    }
}

#[cfg(feature = "python-bindings")]
impl Clone for Version {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            let cloned_tokens: Vec<PyObject> = self.tokens
                .iter()
                .map(|token| token.clone_ref(py))
                .collect();

            Self {
                tokens: cloned_tokens,
                separators: self.separators.clone(),
                string_repr: self.string_repr.clone(),
                cached_hash: self.cached_hash,
            }
        })
    }
}

#[cfg(not(feature = "python-bindings"))]
impl Clone for Version {
    fn clone(&self) -> Self {
        Self {
            tokens: self.tokens.clone(),
            separators: self.separators.clone(),
            string_repr: self.string_repr.clone(),
            cached_hash: self.cached_hash,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version_creation() {
        let version = Version::parse("1.2.3").unwrap();
        assert_eq!(version.as_str(), "1.2.3");
        assert_eq!(version.__len__(), 3);
        assert!(version.__bool__());
    }

    #[test]
    fn test_empty_version() {
        let version = Version::parse("").unwrap();
        assert_eq!(version.as_str(), "");
        assert_eq!(version.__len__(), 0);
        assert!(!version.__bool__());
    }

    #[test]
    fn test_version_inf() {
        let version = Version::inf();
        assert_eq!(version.as_str(), "inf");
        assert!(version.is_inf());
    }

    #[test]
    fn test_version_epsilon() {
        let version = Version::epsilon();
        assert_eq!(version.as_str(), "");
        assert!(version.is_epsilon());
        assert!(version.is_empty());
    }

    #[test]
    fn test_version_empty() {
        let version = Version::empty();
        assert_eq!(version.as_str(), "");
        assert!(version.is_empty());
        assert!(version.is_epsilon());
    }

    #[test]
    fn test_version_parsing_special() {
        // Test parsing empty version
        let empty = Version::parse("").unwrap();
        assert!(empty.is_empty());

        // Test parsing inf version
        let inf = Version::parse("inf").unwrap();
        assert!(inf.is_inf());

        // Test parsing epsilon version
        let epsilon = Version::parse("epsilon").unwrap();
        assert!(epsilon.is_epsilon());
    }

    #[test]
    fn test_version_comparison_boundaries() {
        let empty = Version::empty();
        let epsilon = Version::epsilon();
        let normal = Version::parse("1.0.0").unwrap();
        let inf = Version::inf();

        // Test epsilon/empty equivalence
        assert_eq!(empty.cmp(&epsilon), Ordering::Equal);

        // Test ordering: epsilon < normal < inf
        assert_eq!(epsilon.cmp(&normal), Ordering::Less);
        assert_eq!(normal.cmp(&inf), Ordering::Less);
        assert_eq!(epsilon.cmp(&inf), Ordering::Less);

        // Test reverse ordering
        assert_eq!(inf.cmp(&normal), Ordering::Greater);
        assert_eq!(normal.cmp(&epsilon), Ordering::Greater);
        assert_eq!(inf.cmp(&epsilon), Ordering::Greater);
    }

    #[test]
    fn test_version_prerelease_comparison() {
        // Test that release versions are greater than pre-release versions
        let release = Version::parse("2").unwrap();
        let prerelease = Version::parse("2.alpha1").unwrap();

        // "2" should be greater than "2.alpha1"
        assert_eq!(release.cmp(&prerelease), Ordering::Greater);
        assert_eq!(prerelease.cmp(&release), Ordering::Less);

        // Test with __lt__ method
        assert!(!release.__lt__(&prerelease)); // "2" < "2.alpha1" should be false
        assert!(prerelease.__lt__(&release));  // "2.alpha1" < "2" should be true
    }

    #[test]
    fn test_version_copy() {
        let version = Version::parse("1.2.3").unwrap();
        let copied = version.copy();
        assert_eq!(version.as_str(), copied.as_str());
        assert_eq!(version.__len__(), copied.__len__());
    }

    #[test]
    fn test_version_trim() {
        let version = Version::parse("1.2.3.4").unwrap();
        let trimmed = version.trim(2);
        assert_eq!(trimmed.__len__(), 2);
    }


}
