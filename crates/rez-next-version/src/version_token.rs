use pyo3::prelude::*;
use pyo3::types::PyType;
use regex::Regex;
use std::cmp::Ordering;

/// Base version token class (abstract in Python, like rez)
/// This class should not be instantiated directly
#[pyclass(subclass)]
#[derive(Clone, Debug)]
pub struct VersionToken {
    // This is abstract in rez - we store the token string for display
    token_str: String,
}

#[pymethods]
impl VersionToken {
    #[new]
    fn new(_token: String) -> PyResult<Self> {
        // In rez, VersionToken is abstract and raises NotImplementedError
        // We'll allow creation but this should typically not be used directly
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "VersionToken is abstract - use NumericToken or AlphanumericVersionToken instead",
        ))
    }

    /// Create a random token string for testing purposes
    #[classmethod]
    fn create_random_token_string(_cls: &Bound<'_, PyType>) -> PyResult<String> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Subclasses must implement create_random_token_string",
        ))
    }

    /// Compare to another VersionToken
    fn less_than(&self, _other: &Self) -> PyResult<bool> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Subclasses must implement less_than",
        ))
    }

    /// Returns the next largest token
    fn next(&self) -> PyResult<Self> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Subclasses must implement next",
        ))
    }

    fn __str__(&self) -> PyResult<String> {
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Subclasses must implement __str__",
        ))
    }

    fn __repr__(&self) -> String {
        format!("{}('{}')", "VersionToken", self.token_str)
    }

    fn __lt__(&self, other: &Self) -> PyResult<bool> {
        self.less_than(other)
    }

    fn __eq__(&self, other: &Self) -> PyResult<bool> {
        let self_lt_other = self.less_than(other)?;
        let other_lt_self = other.less_than(self)?;
        Ok(!self_lt_other && !other_lt_self)
    }

    fn __le__(&self, other: &Self) -> PyResult<bool> {
        let lt = self.less_than(other)?;
        let eq = self.__eq__(other)?;
        Ok(lt || eq)
    }

    fn __gt__(&self, other: &Self) -> PyResult<bool> {
        other.less_than(self)
    }

    fn __ge__(&self, other: &Self) -> PyResult<bool> {
        let gt = self.__gt__(other)?;
        let eq = self.__eq__(other)?;
        Ok(gt || eq)
    }
}

/// Numeric version token (numbers only)
/// Version token supporting numbers only. Padding is ignored.
#[pyclass(extends = VersionToken)]
#[derive(Clone, Debug)]
pub struct NumericToken {
    n: u64,
}

#[pymethods]
impl NumericToken {
    #[new]
    fn new(token: String) -> PyResult<(Self, VersionToken)> {
        // Validate that token contains only digits
        if !token.chars().all(|c| c.is_ascii_digit()) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid version token: '{}'",
                token
            )));
        }

        let n = token.parse::<u64>().map_err(|_| {
            PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid version token: '{}'",
                token
            ))
        })?;

        Ok((Self { n }, VersionToken { token_str: token }))
    }

    /// Create a random token string for testing purposes
    #[classmethod]
    fn create_random_token_string(_cls: &Bound<'_, PyType>) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        (0..8).map(|_| rng.gen_range(0..10).to_string()).collect()
    }

    fn less_than(&self, other: &Self) -> bool {
        self.n < other.n
    }

    fn next(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let new_n = self.n + 1;
            let new_token = Self { n: new_n };
            let base = VersionToken {
                token_str: new_n.to_string(),
            };
            Ok(Py::new(py, (new_token, base))?.into())
        })
    }

    fn __str__(&self) -> String {
        self.n.to_string()
    }

    fn __repr__(&self) -> String {
        format!("NumericToken({})", self.n)
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.n == other.n
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.less_than(other)
    }

    fn __le__(&self, other: &Self) -> bool {
        self.n <= other.n
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.n > other.n
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.n >= other.n
    }
}

/// SubToken used internally by AlphanumericVersionToken
/// Implements rez-compatible comparison rules
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SubToken {
    s: String,
    n: Option<u64>,
}

impl PartialOrd for SubToken {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SubToken {
    fn cmp(&self, other: &Self) -> Ordering {
        match (&self.n, &other.n) {
            // Both are strings - compare alphabetically
            (None, None) => self.s.cmp(&other.s),
            // String vs number - strings come before numbers (alphas < numbers)
            (None, Some(_)) => Ordering::Less,
            (Some(_), None) => Ordering::Greater,
            // Both are numbers - use rez's exact logic: (self.n, self.s) < (other.n, other.s)
            (Some(a), Some(b)) => {
                // This matches rez's behavior exactly
                (a, &self.s).cmp(&(b, &other.s))
            }
        }
    }
}

impl SubToken {
    pub(crate) fn new(s: String) -> Self {
        let n = if s.chars().all(|c| c.is_ascii_digit()) && !s.is_empty() {
            s.parse::<u64>().ok()
        } else {
            None
        };
        Self { s, n }
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.s
    }
}

/// Alphanumeric version token with rez-compatible comparison
///
/// These tokens compare as follows:
/// - each token is split into alpha and numeric groups (subtokens)
/// - the resulting subtoken list is compared
/// - alpha comparison is case-sensitive, numeric comparison is padding-sensitive
///
/// Subtokens compare as follows:
/// - alphas come before numbers
/// - alphas are compared alphabetically (_, then A-Z, then a-z)
/// - numbers are compared numerically. If numbers are equivalent but zero-padded
///   differently, they are then compared alphabetically. Thus "01" < "1".
#[pyclass(extends = VersionToken)]
#[derive(Clone, Debug)]
pub struct AlphanumericVersionToken {
    subtokens: Vec<SubToken>,
}

#[pymethods]
impl AlphanumericVersionToken {
    #[new]
    fn new(token: String) -> PyResult<(Self, VersionToken)> {
        // Handle None case (used internally by rez)
        if token.is_empty() {
            return Ok((
                Self { subtokens: vec![] },
                VersionToken { token_str: token },
            ));
        }

        // Validate token format - only alphanumerics and underscores allowed
        let regex = Regex::new(r"^[a-zA-Z0-9_]+$").unwrap();
        if !regex.is_match(&token) {
            return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                "Invalid version token: '{}'",
                token
            )));
        }

        let subtokens = Self::parse_token(&token);

        Ok((Self { subtokens }, VersionToken { token_str: token }))
    }

    /// Create a random token string for testing purposes
    #[classmethod]
    fn create_random_token_string(_cls: &Bound<'_, PyType>) -> String {
        use rand::Rng;
        let mut rng = rand::thread_rng();
        let chars = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
        (0..8)
            .map(|_| {
                let idx = rng.gen_range(0..chars.len());
                chars.chars().nth(idx).unwrap()
            })
            .collect()
    }

    fn less_than(&self, other: &Self) -> bool {
        self.subtokens < other.subtokens
    }

    fn next(&self) -> PyResult<PyObject> {
        Python::with_gil(|py| {
            let mut new_subtokens = self.subtokens.clone();

            if let Some(last) = new_subtokens.last_mut() {
                if last.n.is_none() {
                    // If last subtoken is alpha, append '_'
                    last.s.push('_');
                } else {
                    // If last subtoken is numeric, append new '_' subtoken
                    new_subtokens.push(SubToken::new("_".to_string()));
                }
            } else {
                // If no subtokens, add '_'
                new_subtokens.push(SubToken::new("_".to_string()));
            }

            let token_str = new_subtokens.iter().map(|t| t.as_str()).collect::<String>();
            let new_token = Self {
                subtokens: new_subtokens,
            };
            let base = VersionToken { token_str };

            Ok(Py::new(py, (new_token, base))?.into())
        })
    }

    fn __str__(&self) -> String {
        self.subtokens
            .iter()
            .map(|t| t.as_str())
            .collect::<String>()
    }

    fn __repr__(&self) -> String {
        format!("AlphanumericVersionToken('{}')", self.__str__())
    }

    fn __eq__(&self, other: &Self) -> bool {
        self.subtokens == other.subtokens
    }

    fn __lt__(&self, other: &Self) -> bool {
        self.less_than(other)
    }

    fn __le__(&self, other: &Self) -> bool {
        self.subtokens <= other.subtokens
    }

    fn __gt__(&self, other: &Self) -> bool {
        self.subtokens > other.subtokens
    }

    fn __ge__(&self, other: &Self) -> bool {
        self.subtokens >= other.subtokens
    }
}

impl AlphanumericVersionToken {
    /// Parse a token string into subtokens using rez's algorithm
    /// This follows the exact logic from rez's _parse method
    pub(crate) fn parse_token(token: &str) -> Vec<SubToken> {
        let mut subtokens = Vec::new();

        // Use regex to split into numeric and alpha parts
        let numeric_regex = Regex::new(r"[0-9]+").unwrap();

        // Split by numeric parts to get alpha parts
        let alphas: Vec<&str> = numeric_regex.split(token).collect();
        // Find all numeric parts
        let numerics: Vec<&str> = numeric_regex.find_iter(token).map(|m| m.as_str()).collect();

        let mut alpha_iter = alphas.iter();
        let mut numeric_iter = numerics.iter();
        let mut is_alpha_turn = true;

        // Alternate between alpha and numeric parts
        while alpha_iter.len() > 0 || numeric_iter.len() > 0 {
            if is_alpha_turn {
                if let Some(alpha) = alpha_iter.next() {
                    if !alpha.is_empty() {
                        subtokens.push(SubToken::new(alpha.to_string()));
                    }
                }
            } else {
                if let Some(numeric) = numeric_iter.next() {
                    subtokens.push(SubToken::new(numeric.to_string()));
                }
            }
            is_alpha_turn = !is_alpha_turn;
        }

        subtokens
    }
}
