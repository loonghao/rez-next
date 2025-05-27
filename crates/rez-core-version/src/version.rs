//! Version implementation

use super::version_token::AlphanumericVersionToken;
use rez_core_common::RezCoreError;
use pyo3::prelude::*;
use pyo3::types::PyTuple;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use regex::Regex;

/// High-performance version representation compatible with rez
#[pyclass]
#[derive(Debug)]
pub struct Version {
    /// Version tokens (rez-compatible)
    tokens: Vec<PyObject>,
    /// Separators between tokens
    separators: Vec<String>,
    /// Cached string representation
    #[pyo3(get)]
    string_repr: String,
    /// Cached hash value
    cached_hash: Option<u64>,
}

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
    pub fn create_inf(_cls: &Bound<'_, pyo3::types::PyType>) -> Self {
        Self::inf()
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

impl Version {
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

    /// Check if this is an empty version
    pub fn is_empty(&self) -> bool {
        self.tokens.is_empty() && self.string_repr.is_empty()
    }

    /// Parse a version string into a Version object using rez-compatible logic
    pub fn parse(s: &str) -> Result<Self, RezCoreError> {
        let s = s.trim();

        // Handle empty version (valid in rez - smallest version)
        if s.is_empty() {
            return Ok(Self {
                tokens: vec![],
                separators: vec![],
                string_repr: "".to_string(),
                cached_hash: None,
            });
        }

        // Handle infinite version
        if s == "inf" {
            return Ok(Self::inf());
        }

        Python::with_gil(|py| {
            // Use regex to find tokens (alphanumeric + underscore)
            let token_regex = Regex::new(r"[a-zA-Z0-9_]+").unwrap();
            let tokens: Vec<&str> = token_regex.find_iter(s).map(|m| m.as_str()).collect();

            if tokens.is_empty() {
                return Err(RezCoreError::VersionParse(format!("Invalid version syntax: '{}'", s)));
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
            }

            // Create rez-compatible tokens
            let mut py_tokens = Vec::new();
            for token_str in tokens {
                // For now, create all tokens as AlphanumericVersionToken
                // TODO: Implement proper NumericToken vs AlphanumericVersionToken distinction
                let alpha_class = py.get_type::<AlphanumericVersionToken>();
                let py_token = alpha_class.call1((token_str,))?.into();
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

    /// Reconstruct string representation from tokens and separators
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

    /// Compare two versions using rez-compatible rules
    pub fn cmp(&self, other: &Self) -> Ordering {
        // Handle infinite versions (inf is largest)
        match (self.is_inf(), other.is_inf()) {
            (true, true) => return Ordering::Equal,
            (true, false) => return Ordering::Greater,
            (false, true) => return Ordering::Less,
            (false, false) => {} // Continue with normal comparison
        }

        // Handle empty versions (empty version is smallest)
        if self.tokens.is_empty() && other.tokens.is_empty() {
            return Ordering::Equal;
        }
        if self.tokens.is_empty() {
            return Ordering::Less;
        }
        if other.tokens.is_empty() {
            return Ordering::Greater;
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
                    (Some(_), None) => return Ordering::Greater, // More tokens = greater
                    (None, Some(_)) => return Ordering::Less,    // Fewer tokens = less
                    (None, None) => break, // Both exhausted
                }
            }

            Ordering::Equal
        })
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
