//! Extraction helpers: convert AST expressions to typed Rust values.

use super::types::PythonValue;
use super::PythonAstParser;
use rez_next_common::RezCoreError;
use rustpython_ast::{Constant, Expr};
use std::collections::HashMap;

impl PythonAstParser {
    /// Extract a string value from an expression
    pub(super) fn extract_string_value(&self, expr: &Expr) -> Result<String, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::String(s)) => Ok(s),
            Ok(PythonValue::Integer(i)) => Ok(i.to_string()),
            Ok(PythonValue::Float(f)) => Ok(f.to_string()),
            Ok(PythonValue::Boolean(b)) => Ok(b.to_string()),
            Ok(PythonValue::Expression(s)) => Ok(s),
            _ => match expr {
                Expr::Constant(constant) => match &constant.value {
                    Constant::Str(s) => Ok(s.clone()),
                    Constant::Int(i) => Ok(i.to_string()),
                    Constant::Float(f) => Ok(f.to_string()),
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected string/number value, got: {:?}",
                        constant.value
                    ))),
                },
                _ => Err(RezCoreError::PackageParse(format!(
                    "Expected constant value, got: {:?}",
                    expr
                ))),
            },
        }
    }

    /// Extract an optional boolean value from an expression
    pub(super) fn extract_bool_value(&self, expr: &Expr) -> Result<Option<bool>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Boolean(b)) => Ok(Some(b)),
            Ok(PythonValue::None) => Ok(None),
            _ => match expr {
                Expr::Constant(constant) => match &constant.value {
                    Constant::Bool(b) => Ok(Some(*b)),
                    Constant::None => Ok(None),
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected boolean value, got: {:?}",
                        constant.value
                    ))),
                },
                _ => Err(RezCoreError::PackageParse(format!(
                    "Expected constant value, got: {:?}",
                    expr
                ))),
            },
        }
    }

    /// Extract an integer value from an expression
    pub(super) fn extract_int_value(&self, expr: &Expr) -> Result<i32, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Integer(i)) => Ok(i as i32),
            _ => match expr {
                Expr::Constant(constant) => match &constant.value {
                    Constant::Int(i) => i.to_string().parse::<i32>().map_err(|e| {
                        RezCoreError::PackageParse(format!("Integer too large for i32: {}", e))
                    }),
                    _ => Err(RezCoreError::PackageParse(format!(
                        "Expected integer value, got: {:?}",
                        constant.value
                    ))),
                },
                _ => Err(RezCoreError::PackageParse(format!(
                    "Expected constant value, got: {:?}",
                    expr
                ))),
            },
        }
    }

    /// Extract a list of strings from an expression
    pub(super) fn extract_string_list(&self, expr: &Expr) -> Result<Vec<String>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::List(list)) => {
                let mut result = Vec::new();
                for item in list {
                    match item {
                        PythonValue::String(s) => result.push(s),
                        PythonValue::Integer(i) => result.push(i.to_string()),
                        PythonValue::Float(f) => result.push(f.to_string()),
                        PythonValue::Boolean(b) => result.push(b.to_string()),
                        _ => {
                            return Err(RezCoreError::PackageParse(
                                "List contains non-string values".to_string(),
                            ))
                        }
                    }
                }
                Ok(result)
            }
            _ => match expr {
                Expr::List(list) => {
                    let mut result = Vec::new();
                    for elt in &list.elts {
                        result.push(self.extract_string_value(elt)?);
                    }
                    Ok(result)
                }
                Expr::Tuple(tuple) => {
                    let mut result = Vec::new();
                    for elt in &tuple.elts {
                        result.push(self.extract_string_value(elt)?);
                    }
                    Ok(result)
                }
                _ => Err(RezCoreError::PackageParse(format!(
                    "Expected list, got: {:?}",
                    expr
                ))),
            },
        }
    }

    /// Extract variants — a list of lists of strings
    pub(super) fn extract_variants(&self, expr: &Expr) -> Result<Vec<Vec<String>>, RezCoreError> {
        match expr {
            Expr::List(list) => {
                let mut result = Vec::new();
                for elt in &list.elts {
                    result.push(self.extract_string_list(elt)?);
                }
                Ok(result)
            }
            _ => Err(RezCoreError::PackageParse(format!(
                "Expected list of lists for variants, got: {:?}",
                expr
            ))),
        }
    }

    /// Extract tests — a dictionary mapping test names to command strings
    pub(super) fn extract_tests(
        &self,
        expr: &Expr,
    ) -> Result<HashMap<String, String>, RezCoreError> {
        match self.evaluate_expression(expr) {
            Ok(PythonValue::Dict(dict)) => {
                let mut result = HashMap::new();
                for (key, value) in dict {
                    let value_str = match value {
                        PythonValue::String(s) => s,
                        PythonValue::Integer(i) => i.to_string(),
                        PythonValue::Float(f) => f.to_string(),
                        PythonValue::Boolean(b) => b.to_string(),
                        _ => format!("{:?}", value),
                    };
                    result.insert(key, value_str);
                }
                Ok(result)
            }
            _ => match expr {
                Expr::Dict(dict) => {
                    let mut result = HashMap::new();
                    for (key, value) in dict.keys.iter().zip(dict.values.iter()) {
                        if let Some(key) = key {
                            let key_str = self.extract_string_value(key)?;
                            let value_str = self.extract_string_value(value)?;
                            result.insert(key_str, value_str);
                        }
                    }
                    Ok(result)
                }
                _ => Err(RezCoreError::PackageParse(format!(
                    "Expected dictionary for tests, got: {:?}",
                    expr
                ))),
            },
        }
    }
}
