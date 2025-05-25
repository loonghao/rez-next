//! Version token types

use std::cmp::Ordering;
use serde::{Deserialize, Serialize};
use pyo3::prelude::*;

/// Version token representation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum VersionToken {
    /// Numeric token (e.g., "123")
    Numeric(u64),
    /// Alphanumeric token (e.g., "alpha", "beta", "rc1")
    Alphanumeric(String),
}

/// Python wrapper for VersionToken
#[pyclass(name = "VersionToken")]
#[derive(Debug, Clone)]
pub struct PyVersionToken {
    inner: VersionToken,
}

#[pymethods]
impl PyVersionToken {
    /// Create a new token from a string
    #[staticmethod]
    pub fn from_str(s: &str) -> Self {
        let inner = if let Ok(num) = s.parse::<u64>() {
            VersionToken::Numeric(num)
        } else {
            VersionToken::Alphanumeric(s.to_string())
        };
        PyVersionToken { inner }
    }

    /// Get the string representation of the token
    pub fn as_str(&self) -> String {
        match &self.inner {
            VersionToken::Numeric(n) => n.to_string(),
            VersionToken::Alphanumeric(s) => s.clone(),
        }
    }

    fn __str__(&self) -> String {
        self.as_str()
    }

    fn __repr__(&self) -> String {
        match &self.inner {
            VersionToken::Numeric(n) => format!("VersionToken::Numeric({})", n),
            VersionToken::Alphanumeric(s) => format!("VersionToken::Alphanumeric('{}')", s),
        }
    }
}

impl VersionToken {
    /// Create a new token from a string
    pub fn from_str(s: &str) -> Self {
        if let Ok(num) = s.parse::<u64>() {
            Self::Numeric(num)
        } else {
            Self::Alphanumeric(s.to_string())
        }
    }

    /// Get the string representation of the token
    pub fn as_str(&self) -> String {
        match self {
            Self::Numeric(n) => n.to_string(),
            Self::Alphanumeric(s) => s.clone(),
        }
    }
}

impl PartialOrd for VersionToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for VersionToken {
    fn cmp(&self, other: &Self) -> Ordering {
        match (self, other) {
            // Numeric tokens are compared numerically
            (Self::Numeric(a), Self::Numeric(b)) => a.cmp(b),

            // Alphanumeric tokens are compared lexicographically
            (Self::Alphanumeric(a), Self::Alphanumeric(b)) => a.cmp(b),

            // Numeric tokens come before alphanumeric tokens
            (Self::Numeric(_), Self::Alphanumeric(_)) => Ordering::Less,
            (Self::Alphanumeric(_), Self::Numeric(_)) => Ordering::Greater,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_creation() {
        assert_eq!(VersionToken::from_str("123"), VersionToken::Numeric(123));
        assert_eq!(VersionToken::from_str("alpha"), VersionToken::Alphanumeric("alpha".to_string()));
    }

    #[test]
    fn test_token_comparison() {
        let num1 = VersionToken::Numeric(1);
        let num2 = VersionToken::Numeric(2);
        let alpha = VersionToken::Alphanumeric("alpha".to_string());
        let beta = VersionToken::Alphanumeric("beta".to_string());

        assert!(num1 < num2);
        assert!(alpha < beta);
        assert!(num1 < alpha);
        assert!(num2 < alpha);
    }
}
