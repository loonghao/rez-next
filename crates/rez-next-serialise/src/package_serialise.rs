//! Package serialisation functionality.
//!
//! This module provides functions to serialise package data to various formats,
//! including YAML, JSON, Python, and TOML.

use serde::Serialize;
use std::fs::File;
use std::io::Write;
use std::path::Path;
use thiserror::Error;

/// Supported file formats for package serialisation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileFormat {
    /// YAML format (.yaml, .yml)
    Yaml,
    /// JSON format (.json)
    Json,
    /// Python format (.py)
    Python,
    /// TOML format (.toml)
    Toml,
    /// YAML compressed (.yaml.gz)
    YamlCompressed,
    /// JSON compressed (.json.gz)
    JsonCompressed,
}

/// Errors that can occur during package serialisation.
#[derive(Error, Debug)]
pub enum PackageSerialiseError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialisation error: {0}")]
    Serialisation(String),

    #[error("Unsupported format: {0:?}")]
    UnsupportedFormat(FileFormat),

    #[error("Invalid package data: {0}")]
    InvalidData(String),
}

/// Result type for package serialisation operations.
pub type Result<T> = std::result::Result<T, PackageSerialiseError>;

// ── Constants ──────────────────────────────────────────────────────────────────

/// Standard key order for package serialisation.
pub const PACKAGE_KEY_ORDER: &[&str] = &[
    "name",
    "version",
    "description",
    "authors",
    "license",
    "requires",
    "variants",
    "build_requires",
    "private_build_requires",
    "uuid",
    "revision",
    "timestamp",
    "config",
    "help",
    "tests",
    "pre_commands",
    "commands",
    "post_commands",
    "release_commands",
    "pre_build_commands",
    "build_commands",
    "post_build_commands",
    "pre_test_commands",
    "test_commands",
    "post_test_commands",
    "implicit",
    "cachable",
    "universal",
    "hashed_variants",
];

// ── Public API ─────────────────────────────────────────────────────────────────

/// Serialise package data to a file.
///
/// # Arguments
///
/// * `data` - The package data to serialise (as a serialisable type)
/// * `path` - The file path to write to
/// * `format` - The file format to use
///
/// # Errors
///
/// Returns `PackageSerialiseError` if serialisation or file writing fails.
pub fn dump_package_data<T: Serialize>(data: &T, path: &Path, format: FileFormat) -> Result<()> {
    let serialized = match format {
        FileFormat::Yaml | FileFormat::YamlCompressed => serde_yaml::to_string(data)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
        FileFormat::Json | FileFormat::JsonCompressed => serde_json::to_string_pretty(data)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
        FileFormat::Python => dict_to_attributes_code(data)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
        FileFormat::Toml => toml::to_string(data)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
    };

    let mut file = File::create(path)?;
    file.write_all(serialized.as_bytes())?;
    file.flush()?;

    Ok(())
}

/// Deserialise package data from a file.
///
/// # Arguments
///
/// * `path` - The file path to read from
/// * `format` - The file format to use
///
/// # Returns
///
/// The deserialised data.
///
/// # Errors
///
/// Returns `PackageSerialiseError` if deserialisation or file reading fails.
pub fn read_package_data<T: serde::de::DeserializeOwned>(
    path: &Path,
    format: FileFormat,
) -> Result<T> {
    let content = std::fs::read_to_string(path)?;

    let deserialised: T = match format {
        FileFormat::Yaml | FileFormat::YamlCompressed => serde_yaml::from_str(&content)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
        FileFormat::Json | FileFormat::JsonCompressed => serde_json::from_str(&content)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
        FileFormat::Python => {
            // Python format: execute the Python file and get the dict
            // For now, treat it as YAML (package.py is YAML-like)
            serde_yaml::from_str(&content)
                .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?
        }
        FileFormat::Toml => toml::from_str(&content)
            .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?,
    };

    Ok(deserialised)
}

/// Serialise data to a YAML string.
///
/// # Arguments
///
/// * `data` - The data to serialise
///
/// # Returns
///
/// The YAML string representation.
///
/// # Errors
///
/// Returns `PackageSerialiseError` if serialisation fails.
pub fn dump_yaml<T: Serialize>(data: &T) -> Result<String> {
    serde_yaml::to_string(data).map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))
}

/// Format a string as a YAML block string (literal block style).
///
/// This is used to format multi-line strings in YAML format,
/// similar to Python's `|` block style.
///
/// # Arguments
///
/// * `s` - The string to format
/// * `indent` - The indentation level
///
/// # Returns
///
/// A YAML block string representation.
pub fn as_block_string(s: &str, indent: usize) -> String {
    let indent_str = " ".repeat(indent);
    let lines: Vec<&str> = s.lines().collect();

    if lines.is_empty() {
        return "''".to_string();
    }

    if lines.len() == 1 {
        return format!("'{}'", lines[0]);
    }

    let mut result = String::from("|\n");
    for line in lines {
        if line.is_empty() {
            result.push('\n');
        } else {
            result.push_str(&indent_str);
            result.push_str(line);
            result.push('\n');
        }
    }

    result
}

/// Convert a serialisable value to Python attribute code.
///
/// This generates Python code that can be executed to recreate the data
/// as Python objects. This is used for `package.py` files.
///
/// # Arguments
///
/// * `data` - The data to convert
///
/// # Returns
///
/// A string containing Python code.
///
/// # Errors
///
/// Returns `PackageSerialiseError` if conversion fails.
pub fn dict_to_attributes_code<T: Serialize>(data: &T) -> Result<String> {
    // First serialise to JSON for easy traversal
    let json_value = serde_json::to_value(data)
        .map_err(|e| PackageSerialiseError::Serialisation(e.to_string()))?;

    let mut output = String::from("# -*- coding: utf-8 -*-\n");
    output.push_str("# Rez package definition\n\n");

    dict_to_python_code(&json_value, &mut output, 0)?;

    Ok(output)
}

/// Get the standard key order for package serialisation.
///
/// # Returns
///
/// A vector of key names in the standard order.
pub fn package_key_order() -> Vec<&'static str> {
    PACKAGE_KEY_ORDER.to_vec()
}

// ── Internal Helpers ───────────────────────────────────────────────────────────

/// Recursively convert a JSON value to Python code.
fn dict_to_python_code(
    value: &serde_json::Value,
    output: &mut String,
    indent: usize,
) -> Result<()> {
    let indent_str = " ".repeat(indent);

    match value {
        serde_json::Value::Null => {
            output.push_str("None");
        }
        serde_json::Value::Bool(b) => {
            output.push_str(if *b { "True" } else { "False" });
        }
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                output.push_str(&i.to_string());
            } else if let Some(f) = n.as_f64() {
                output.push_str(&f.to_string());
            }
        }
        serde_json::Value::String(s) => {
            output.push_str(&format_python_string(s));
        }
        serde_json::Value::Array(arr) => {
            output.push_str("[\n");
            for (i, item) in arr.iter().enumerate() {
                output.push_str(&indent_str);
                output.push_str("    ");
                dict_to_python_code(item, output, indent + 4)?;
                if i < arr.len() - 1 {
                    output.push(',');
                }
                output.push('\n');
            }
            output.push_str(&indent_str);
            output.push(']');
        }
        serde_json::Value::Object(map) => {
            // Generate Python attribute code (key = value) for package.py format
            // Use IndexMap to preserve key order
            let mut keys: Vec<&String> = map.keys().collect();

            // Sort keys according to PACKAGE_KEY_ORDER if possible
            keys.sort_by(|a, b| {
                let a_idx = PACKAGE_KEY_ORDER.iter().position(|&k| k == a.as_str());
                let b_idx = PACKAGE_KEY_ORDER.iter().position(|&k| k == b.as_str());
                match (a_idx, b_idx) {
                    (Some(a), Some(b)) => a.cmp(&b),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => a.cmp(b),
                }
            });

            for (i, key) in keys.iter().enumerate() {
                if i > 0 {
                    output.push('\n');
                }
                output.push_str(&indent_str);
                output.push_str(key);
                output.push_str(" = ");
                dict_to_python_code(&map[*key], output, indent)?;
                if i < keys.len() - 1 {
                    output.push(',');
                }
            }
        }
    }

    Ok(())
}

/// Format a string as a Python string literal.
fn format_python_string(s: &str) -> String {
    if s.contains('\n') {
        // Multiline string - use triple quotes (align with Rez's package.py format)
        format!("'''{}'''", s.replace("'''", "\\'\\'\\'"))
    } else if s.contains('\'') && s.contains('"') {
        // Use triple quotes
        format!("'''{}'''", s.replace("'''", "\\'\\'\\'"))
    } else if s.contains('\'') {
        format!("\"{}\"", s)
    } else {
        format!("'{}'", s.replace('\'', "\\'"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_dump_yaml() {
        let data = json!({
            "name": "test_package",
            "version": "1.0.0"
        });

        let yaml = dump_yaml(&data).unwrap();
        assert!(yaml.contains("name:"));
        assert!(yaml.contains("test_package"));
    }

    #[test]
    fn test_as_block_string_single_line() {
        let result = as_block_string("hello world", 0);
        assert_eq!(result, "'hello world'");
    }

    #[test]
    fn test_as_block_string_multi_line() {
        let input = "line1\nline2\nline3";
        let result = as_block_string(input, 4);
        assert!(result.starts_with("|"));
        assert!(result.contains("    line1"));
        assert!(result.contains("    line2"));
    }

    #[test]
    fn test_as_block_string_empty() {
        let result = as_block_string("", 0);
        assert_eq!(result, "''");
    }

    #[test]
    fn test_dict_to_attributes_code() {
        let data = json!({
            "name": "test_package",
            "version": "1.0.0",
            "requires": ["python-3.9"]
        });

        let code = dict_to_attributes_code(&data).unwrap();
        assert!(code.contains("test_package"));
        assert!(code.contains("1.0.0"));
    }

    #[test]
    fn test_package_key_order() {
        let order = package_key_order();
        assert_eq!(order[0], "name");
        assert_eq!(order[1], "version");
        assert!(order.contains(&"requires"));
    }

    #[test]
    fn test_format_python_string() {
        assert_eq!(format_python_string("hello"), "'hello'");
        assert_eq!(format_python_string("it's"), "\"it's\"");
    }
}
