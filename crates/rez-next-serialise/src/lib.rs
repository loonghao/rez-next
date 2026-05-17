//! Package serialisation module for rez-next.
//!
//! This module provides functionality to serialise package data to various formats
//! (YAML, JSON, Python, TOML). It is a Rust reimplementation of rez's
//! `package_serialise.py` module.

pub mod package_serialise;

pub use package_serialise::{
    dump_package_data, dump_yaml, as_block_string, dict_to_attributes_code,
    package_key_order, read_package_data, FileFormat, PackageSerialiseError, Result,
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_file_format_variants() {
        let formats = vec![
            FileFormat::Yaml,
            FileFormat::Json,
            FileFormat::Python,
            FileFormat::Toml,
        ];
        for fmt in formats {
            assert!(matches!(fmt, FileFormat::Yaml | FileFormat::Json | FileFormat::Python | FileFormat::Toml));
        }
    }
}
