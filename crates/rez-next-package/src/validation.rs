//! Package validation functionality

use crate::{Package, requirement::Requirement};
use pyo3::prelude::*;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::Path;
use regex::Regex;

/// Package validation result
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageValidationResult {
    /// Whether the package is valid
    #[pyo3(get)]
    pub is_valid: bool,

    /// Validation errors
    #[pyo3(get)]
    pub errors: Vec<String>,

    /// Validation warnings
    #[pyo3(get)]
    pub warnings: Vec<String>,

    /// Package metadata validation
    #[pyo3(get)]
    pub metadata_valid: bool,

    /// Dependencies validation
    #[pyo3(get)]
    pub dependencies_valid: bool,

    /// Variants validation
    #[pyo3(get)]
    pub variants_valid: bool,

    /// File structure validation
    #[pyo3(get)]
    pub structure_valid: bool,

    /// Version compatibility validation
    #[pyo3(get)]
    pub version_compatibility_valid: bool,

    /// Platform compatibility validation
    #[pyo3(get)]
    pub platform_compatibility_valid: bool,

    /// Package integrity validation
    #[pyo3(get)]
    pub integrity_valid: bool,

    /// Security validation
    #[pyo3(get)]
    pub security_valid: bool,

    /// Performance validation
    #[pyo3(get)]
    pub performance_valid: bool,

    /// Validation details for each category
    #[pyo3(get)]
    pub validation_details: HashMap<String, Vec<String>>,
}

/// Package validation options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageValidationOptions {
    /// Check package metadata
    #[pyo3(get, set)]
    pub check_metadata: bool,

    /// Check dependencies
    #[pyo3(get, set)]
    pub check_dependencies: bool,

    /// Check variants
    #[pyo3(get, set)]
    pub check_variants: bool,

    /// Check file structure
    #[pyo3(get, set)]
    pub check_structure: bool,

    /// Check for circular dependencies
    #[pyo3(get, set)]
    pub check_circular_deps: bool,

    /// Strict validation mode
    #[pyo3(get, set)]
    pub strict_mode: bool,

    /// Check version compatibility
    #[pyo3(get, set)]
    pub check_version_compatibility: bool,

    /// Check platform compatibility
    #[pyo3(get, set)]
    pub check_platform_compatibility: bool,

    /// Check package integrity
    #[pyo3(get, set)]
    pub check_package_integrity: bool,

    /// Check for deprecated features
    #[pyo3(get, set)]
    pub check_deprecated_features: bool,

    /// Check for security issues
    #[pyo3(get, set)]
    pub check_security_issues: bool,

    /// Maximum dependency depth to check
    #[pyo3(get, set)]
    pub max_dependency_depth: usize,

    /// Allowed platforms (empty means all platforms allowed)
    #[pyo3(get, set)]
    pub allowed_platforms: Vec<String>,
}

/// Package validator
#[pyclass]
#[derive(Debug)]
pub struct PackageValidator {
    /// Validation options
    options: PackageValidationOptions,

    /// Known packages for dependency validation
    known_packages: HashMap<String, Vec<Version>>,
}

#[pymethods]
impl PackageValidationResult {
    #[new]
    pub fn new() -> Self {
        Self {
            is_valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            metadata_valid: true,
            dependencies_valid: true,
            variants_valid: true,
            structure_valid: true,
            version_compatibility_valid: true,
            platform_compatibility_valid: true,
            integrity_valid: true,
            security_valid: true,
            performance_valid: true,
            validation_details: HashMap::new(),
        }
    }

    /// Add an error
    pub fn add_error(&mut self, error: String) {
        self.errors.push(error);
        self.is_valid = false;
    }

    /// Add a warning
    pub fn add_warning(&mut self, warning: String) {
        self.warnings.push(warning);
    }

    /// Get total issue count
    #[getter]
    pub fn total_issues(&self) -> usize {
        self.errors.len() + self.warnings.len()
    }

    /// Get error count
    #[getter]
    pub fn error_count(&self) -> usize {
        self.errors.len()
    }

    /// Get warning count
    #[getter]
    pub fn warning_count(&self) -> usize {
        self.warnings.len()
    }

    /// Get summary string
    pub fn summary(&self) -> String {
        if self.is_valid {
            format!(
                "Package validation passed with {} warnings",
                self.warnings.len()
            )
        } else {
            format!(
                "Package validation failed with {} errors and {} warnings",
                self.errors.len(),
                self.warnings.len()
            )
        }
    }

    /// String representation
    fn __str__(&self) -> String {
        self.summary()
    }

    /// Representation
    fn __repr__(&self) -> String {
        format!(
            "PackageValidationResult(valid={}, errors={}, warnings={})",
            self.is_valid,
            self.errors.len(),
            self.warnings.len()
        )
    }
}

#[pymethods]
impl PackageValidationOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            check_metadata: true,
            check_dependencies: true,
            check_variants: true,
            check_structure: true,
            check_circular_deps: true,
            strict_mode: false,
            check_version_compatibility: true,
            check_platform_compatibility: true,
            check_package_integrity: true,
            check_deprecated_features: true,
            check_security_issues: true,
            max_dependency_depth: 10,
            allowed_platforms: Vec::new(),
        }
    }

    /// Create options for quick validation
    #[staticmethod]
    pub fn quick() -> Self {
        Self {
            check_metadata: true,
            check_dependencies: false,
            check_variants: false,
            check_structure: false,
            check_circular_deps: false,
            strict_mode: false,
            check_version_compatibility: false,
            check_platform_compatibility: false,
            check_package_integrity: false,
            check_deprecated_features: false,
            check_security_issues: false,
            max_dependency_depth: 3,
            allowed_platforms: Vec::new(),
        }
    }

    /// Create options for full validation
    #[staticmethod]
    pub fn full() -> Self {
        Self {
            check_metadata: true,
            check_dependencies: true,
            check_variants: true,
            check_structure: true,
            check_circular_deps: true,
            strict_mode: true,
            check_version_compatibility: true,
            check_platform_compatibility: true,
            check_package_integrity: true,
            check_deprecated_features: true,
            check_security_issues: true,
            max_dependency_depth: 20,
            allowed_platforms: Vec::new(),
        }
    }

    /// Create options for security-focused validation
    #[staticmethod]
    pub fn security() -> Self {
        Self {
            check_metadata: true,
            check_dependencies: true,
            check_variants: false,
            check_structure: true,
            check_circular_deps: true,
            strict_mode: true,
            check_version_compatibility: true,
            check_platform_compatibility: false,
            check_package_integrity: true,
            check_deprecated_features: true,
            check_security_issues: true,
            max_dependency_depth: 15,
            allowed_platforms: Vec::new(),
        }
    }
}

#[pymethods]
impl PackageValidator {
    #[new]
    pub fn new(options: Option<PackageValidationOptions>) -> Self {
        Self {
            options: options.unwrap_or_else(PackageValidationOptions::new),
            known_packages: HashMap::new(),
        }
    }

    /// Add known packages for dependency validation
    pub fn add_known_packages(&mut self, packages: HashMap<String, Vec<String>>) -> PyResult<()> {
        for (name, versions) in packages {
            let parsed_versions: Result<Vec<Version>, _> =
                versions.into_iter().map(|v| Version::parse(&v)).collect();

            match parsed_versions {
                Ok(versions) => {
                    self.known_packages.insert(name, versions);
                }
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(format!(
                        "Invalid version in known packages: {}",
                        e
                    )));
                }
            }
        }
        Ok(())
    }

    /// Validate a package
    pub fn validate_package(&self, package: &Package) -> PyResult<PackageValidationResult> {
        let mut result = PackageValidationResult::new();

        // Validate metadata
        if self.options.check_metadata {
            self.validate_metadata(package, &mut result);
        }

        // Validate dependencies
        if self.options.check_dependencies {
            self.validate_dependencies(package, &mut result);
        }

        // Validate variants
        if self.options.check_variants {
            self.validate_variants(package, &mut result);
        }

        // Check circular dependencies
        if self.options.check_circular_deps {
            self.check_circular_dependencies(package, &mut result);
        }

        // Check version compatibility
        if self.options.check_version_compatibility {
            self.validate_version_compatibility(package, &mut result);
        }

        // Check platform compatibility
        if self.options.check_platform_compatibility {
            self.validate_platform_compatibility(package, &mut result);
        }

        // Check package integrity
        if self.options.check_package_integrity {
            self.validate_package_integrity(package, &mut result);
        }

        // Check for deprecated features
        if self.options.check_deprecated_features {
            self.check_deprecated_features(package, &mut result);
        }

        // Check for security issues
        if self.options.check_security_issues {
            self.check_security_issues(package, &mut result);
        }

        Ok(result)
    }

    /// Validate package from file
    pub fn validate_package_file(&self, _path: &str) -> PyResult<PackageValidationResult> {
        // This would load the package from file and validate it
        // Implementation depends on the serialization module
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Package file validation not yet implemented",
        ))
    }
}

impl PackageValidator {
    /// Validate package metadata
    fn validate_metadata(&self, package: &Package, result: &mut PackageValidationResult) {
        // Check required fields
        if package.name.is_empty() {
            result.add_error("Package name is required".to_string());
            result.metadata_valid = false;
        }

        // Validate package name format
        if !self.is_valid_package_name(&package.name) {
            result.add_error(format!("Invalid package name format: '{}'", package.name));
            result.metadata_valid = false;
        }

        // Check version if present
        if let Some(ref version) = package.version {
            if version.as_str().is_empty() {
                result.add_error("Package version cannot be empty".to_string());
                result.metadata_valid = false;
            }
        }

        // Validate authors
        if package.authors.is_empty() && self.options.strict_mode {
            result.add_warning("Package has no authors specified".to_string());
        }

        // Validate description
        if package.description.is_none() && self.options.strict_mode {
            result.add_warning("Package has no description".to_string());
        }
    }

    /// Validate package dependencies
    fn validate_dependencies(&self, package: &Package, result: &mut PackageValidationResult) {
        // Validate requires
        for req in &package.requires {
            if let Err(e) = self.validate_requirement_string(req) {
                result.add_error(format!("Invalid requirement '{}': {}", req, e));
                result.dependencies_valid = false;
            }
        }

        // Validate build_requires
        for req in &package.build_requires {
            if let Err(e) = self.validate_requirement_string(req) {
                result.add_error(format!("Invalid build requirement '{}': {}", req, e));
                result.dependencies_valid = false;
            }
        }

        // Validate private_build_requires
        for req in &package.private_build_requires {
            if let Err(e) = self.validate_requirement_string(req) {
                result.add_error(format!(
                    "Invalid private build requirement '{}': {}",
                    req, e
                ));
                result.dependencies_valid = false;
            }
        }
    }

    /// Validate package variants
    fn validate_variants(&self, package: &Package, result: &mut PackageValidationResult) {
        if package.variants.is_empty() {
            return; // No variants to validate
        }

        // Check for duplicate variants
        let mut seen_variants = HashSet::new();
        for (i, variant) in package.variants.iter().enumerate() {
            let variant_key = variant.join(",");
            if seen_variants.contains(&variant_key) {
                result.add_error(format!(
                    "Duplicate variant at index {}: [{}]",
                    i, variant_key
                ));
                result.variants_valid = false;
            }
            seen_variants.insert(variant_key);
        }

        // Validate variant requirements
        for (i, variant) in package.variants.iter().enumerate() {
            for req in variant {
                if let Err(e) = self.validate_requirement_string(req) {
                    result.add_error(format!(
                        "Invalid variant requirement '{}' in variant {}: {}",
                        req, i, e
                    ));
                    result.variants_valid = false;
                }
            }
        }
    }

    /// Check for circular dependencies
    fn check_circular_dependencies(&self, package: &Package, result: &mut PackageValidationResult) {
        // This is a simplified check - a full implementation would need
        // access to all packages in the repository
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if self.has_circular_dependency(&package.name, &package.requires, &mut visited, &mut path) {
            result.add_error(format!(
                "Circular dependency detected: {}",
                path.join(" -> ")
            ));
            result.dependencies_valid = false;
        }
    }

    /// Check if package name is valid
    fn is_valid_package_name(&self, name: &str) -> bool {
        // Package names should be alphanumeric with underscores and hyphens
        !name.is_empty()
            && name
                .chars()
                .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
            && !name.starts_with('-')
            && !name.ends_with('-')
    }

    /// Validate a requirement string
    fn validate_requirement_string(&self, req: &str) -> Result<(), String> {
        if req.is_empty() {
            return Err("Requirement cannot be empty".to_string());
        }

        // Try to parse as a proper requirement
        match req.parse::<Requirement>() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Invalid requirement format: {}", e)),
        }
    }

    /// Check for circular dependencies (simplified)
    fn has_circular_dependency(
        &self,
        package_name: &str,
        _requires: &[String],
        visited: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if visited.contains(package_name) {
            return true;
        }

        visited.insert(package_name.to_string());
        path.push(package_name.to_string());

        // In a full implementation, this would resolve requirements
        // and recursively check dependencies

        path.pop();
        visited.remove(package_name);
        false
    }

    /// Validate version compatibility
    fn validate_version_compatibility(&self, package: &Package, result: &mut PackageValidationResult) {
        let mut details = Vec::new();

        // Check if version follows semantic versioning
        if let Some(ref version) = package.version {
            if !self.is_semantic_version(version.as_str()) {
                result.add_warning(format!(
                    "Package version '{}' does not follow semantic versioning",
                    version.as_str()
                ));
                details.push(format!("Non-semantic version: {}", version.as_str()));
            }
        }

        // Check version compatibility with dependencies
        for req_str in &package.requires {
            if let Ok(req) = req_str.parse::<Requirement>() {
                if let Some(known_versions) = self.known_packages.get(req.package_name()) {
                    let compatible_versions: Vec<_> = known_versions
                        .iter()
                        .filter(|v| req.is_satisfied_by(v))
                        .collect();

                    if compatible_versions.is_empty() {
                        result.add_error(format!(
                            "No compatible versions found for requirement '{}'",
                            req_str
                        ));
                        result.version_compatibility_valid = false;
                        details.push(format!("No compatible versions: {}", req_str));
                    } else if compatible_versions.len() == 1 {
                        result.add_warning(format!(
                            "Only one compatible version found for requirement '{}': {}",
                            req_str,
                            compatible_versions[0].as_str()
                        ));
                        details.push(format!("Single compatible version: {}", req_str));
                    }
                }
            }
        }

        if !details.is_empty() {
            result.validation_details.insert("version_compatibility".to_string(), details);
        }
    }

    /// Validate platform compatibility
    fn validate_platform_compatibility(&self, package: &Package, result: &mut PackageValidationResult) {
        let mut details = Vec::new();

        // Check if package specifies platform requirements
        let has_platform_reqs = package.requires.iter()
            .chain(package.build_requires.iter())
            .chain(package.private_build_requires.iter())
            .any(|req| req.contains("platform") || req.contains("arch"));

        if !has_platform_reqs && self.options.strict_mode {
            result.add_warning("Package does not specify platform requirements".to_string());
            details.push("No platform requirements specified".to_string());
        }

        // Check against allowed platforms
        if !self.options.allowed_platforms.is_empty() {
            for req_str in &package.requires {
                if req_str.contains("platform") {
                    let platform_found = self.options.allowed_platforms.iter()
                        .any(|platform| req_str.contains(platform));

                    if !platform_found {
                        result.add_error(format!(
                            "Platform requirement '{}' not in allowed platforms: {:?}",
                            req_str,
                            self.options.allowed_platforms
                        ));
                        result.platform_compatibility_valid = false;
                        details.push(format!("Disallowed platform: {}", req_str));
                    }
                }
            }
        }

        if !details.is_empty() {
            result.validation_details.insert("platform_compatibility".to_string(), details);
        }
    }

    /// Validate package integrity
    fn validate_package_integrity(&self, package: &Package, result: &mut PackageValidationResult) {
        let mut details = Vec::new();

        // Check for required fields consistency
        if package.name.is_empty() {
            result.add_error("Package name is required for integrity".to_string());
            result.integrity_valid = false;
            details.push("Missing package name".to_string());
        }

        // Check for version consistency
        if package.version.is_none() && !package.variants.is_empty() {
            result.add_warning("Package has variants but no version specified".to_string());
            details.push("Variants without version".to_string());
        }

        // Check for build system consistency
        if !package.build_requires.is_empty() && package.build_command.is_none() {
            result.add_warning("Package has build requirements but no build command".to_string());
            details.push("Build requirements without build command".to_string());
        }

        // Check for tools consistency
        if !package.tools.is_empty() && package.commands_function.is_none() {
            result.add_warning("Package defines tools but has no commands function".to_string());
            details.push("Tools without commands function".to_string());
        }

        // Check for UUID format if present
        if let Some(ref uuid) = package.uuid {
            if !self.is_valid_uuid(uuid) {
                result.add_error(format!("Invalid UUID format: '{}'", uuid));
                result.integrity_valid = false;
                details.push(format!("Invalid UUID: {}", uuid));
            }
        }

        if !details.is_empty() {
            result.validation_details.insert("package_integrity".to_string(), details);
        }
    }

    /// Check for deprecated features
    fn check_deprecated_features(&self, package: &Package, result: &mut PackageValidationResult) {
        let mut details = Vec::new();

        // Check for deprecated fields or patterns
        if package.format_version.is_some() && package.format_version.unwrap() < 2 {
            result.add_warning("Package uses deprecated format version".to_string());
            details.push(format!("Deprecated format version: {}", package.format_version.unwrap()));
        }

        // Check for deprecated requirement patterns
        for req in &package.requires {
            if req.contains("~") && !req.starts_with("~") {
                result.add_warning(format!("Deprecated requirement syntax: '{}'", req));
                details.push(format!("Deprecated syntax: {}", req));
            }
        }

        // Check for deprecated build patterns
        if let Some(ref build_cmd) = package.build_command {
            if build_cmd.contains("python setup.py") {
                result.add_warning("Package uses deprecated 'python setup.py' build command".to_string());
                details.push("Deprecated build command: python setup.py".to_string());
            }
        }

        if !details.is_empty() {
            result.validation_details.insert("deprecated_features".to_string(), details);
        }
    }

    /// Check for security issues
    fn check_security_issues(&self, package: &Package, result: &mut PackageValidationResult) {
        let mut details = Vec::new();

        // Check for potentially unsafe commands
        if let Some(ref commands) = package.commands_function {
            let unsafe_patterns = [
                "rm -rf",
                "sudo",
                "chmod 777",
                "eval",
                "exec",
                "system(",
                "shell=True",
            ];

            for pattern in &unsafe_patterns {
                if commands.contains(pattern) {
                    result.add_warning(format!(
                        "Potentially unsafe command pattern found: '{}'",
                        pattern
                    ));
                    details.push(format!("Unsafe pattern: {}", pattern));
                }
            }
        }

        // Check for insecure URLs in requirements
        for req in &package.requires {
            if req.contains("http://") {
                result.add_warning(format!(
                    "Insecure HTTP URL in requirement: '{}'",
                    req
                ));
                details.push(format!("Insecure URL: {}", req));
            }
        }

        // Check for overly permissive version constraints
        for req in &package.requires {
            if req.contains(">=") && !req.contains("<") && !req.contains("~=") {
                result.add_warning(format!(
                    "Overly permissive version constraint: '{}'",
                    req
                ));
                details.push(format!("Permissive constraint: {}", req));
            }
        }

        if !details.is_empty() {
            result.validation_details.insert("security_issues".to_string(), details);
        }
    }

    /// Check if version follows semantic versioning
    fn is_semantic_version(&self, version: &str) -> bool {
        let semver_regex = Regex::new(r"^\d+\.\d+\.\d+(-[a-zA-Z0-9\-\.]+)?(\+[a-zA-Z0-9\-\.]+)?$").unwrap();
        semver_regex.is_match(version)
    }

    /// Check if UUID is valid
    fn is_valid_uuid(&self, uuid: &str) -> bool {
        let uuid_regex = Regex::new(r"^[0-9a-fA-F]{8}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{4}-[0-9a-fA-F]{12}$").unwrap();
        uuid_regex.is_match(uuid)
    }
}

impl Default for PackageValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PackageValidationOptions {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn create_test_package() -> Package {
        Package {
            name: "test_package".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            description: Some("Test package".to_string()),
            authors: vec!["Test Author".to_string()],
            requires: vec!["python>=3.8".to_string()],
            build_requires: vec!["cmake".to_string()],
            private_build_requires: vec![],
            tools: vec!["test_tool".to_string()],
            variants: vec![
                vec!["python-3.8".to_string()],
                vec!["python-3.9".to_string()],
            ],
            commands_function: Some("env.PATH.append('/usr/local/bin')".to_string()),
            build_command: Some("cmake --build .".to_string()),
            build_system: Some("cmake".to_string()),
            uuid: Some("550e8400-e29b-41d4-a716-446655440000".to_string()),
            relocatable: Some(true),
            cachable: Some(true),
            base: None,
            hashed_variants: Some(false),
            has_plugins: Some(false),
            plugin_for: vec![],
            format_version: Some(2),
            preprocess: None,
            pre_commands: None,
            post_commands: None,
            pre_test_commands: None,
            pre_build_commands: None,
            requires_rez_version: None,
            help: None,
        }
    }

    #[test]
    fn test_basic_validation() {
        let package = create_test_package();
        let validator = PackageValidator::new(None);
        let result = validator.validate_package(&package).unwrap();

        assert!(result.is_valid);
        assert_eq!(result.errors.len(), 0);
    }

    #[test]
    fn test_invalid_package_name() {
        let mut package = create_test_package();
        package.name = "invalid@name".to_string();

        let validator = PackageValidator::new(None);
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Invalid package name format")));
    }

    #[test]
    fn test_empty_package_name() {
        let mut package = create_test_package();
        package.name = String::new();

        let validator = PackageValidator::new(None);
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Package name is required")));
    }

    #[test]
    fn test_invalid_requirement() {
        let mut package = create_test_package();
        package.requires = vec!["".to_string()]; // Empty requirement

        let validator = PackageValidator::new(None);
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Invalid requirement")));
    }

    #[test]
    fn test_duplicate_variants() {
        let mut package = create_test_package();
        package.variants = vec![
            vec!["python-3.8".to_string()],
            vec!["python-3.8".to_string()], // Duplicate
        ];

        let validator = PackageValidator::new(None);
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Duplicate variant")));
    }

    #[test]
    fn test_security_validation() {
        let mut package = create_test_package();
        package.commands_function = Some("rm -rf /important/data".to_string());

        let mut options = PackageValidationOptions::new();
        options.check_security_issues = true;

        let validator = PackageValidator::new(Some(options));
        let result = validator.validate_package(&package).unwrap();

        // Should still be valid but have warnings
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("unsafe command pattern")));
    }

    #[test]
    fn test_version_compatibility() {
        let package = create_test_package();

        let mut options = PackageValidationOptions::new();
        options.check_version_compatibility = true;

        let mut validator = PackageValidator::new(Some(options));

        // Add known packages
        let mut known_packages = HashMap::new();
        known_packages.insert("python".to_string(), vec!["3.7.0".to_string(), "3.8.0".to_string(), "3.9.0".to_string()]);
        validator.add_known_packages(known_packages).unwrap();

        let result = validator.validate_package(&package).unwrap();

        // Should be valid since we have compatible Python versions
        assert!(result.is_valid);
    }

    #[test]
    fn test_validation_options() {
        let quick_options = PackageValidationOptions::quick();
        assert!(quick_options.check_metadata);
        assert!(!quick_options.check_dependencies);
        assert!(!quick_options.strict_mode);

        let full_options = PackageValidationOptions::full();
        assert!(full_options.check_metadata);
        assert!(full_options.check_dependencies);
        assert!(full_options.check_security_issues);
        assert!(full_options.strict_mode);

        let security_options = PackageValidationOptions::security();
        assert!(security_options.check_security_issues);
        assert!(security_options.check_package_integrity);
        assert!(security_options.strict_mode);
    }

    #[test]
    fn test_uuid_validation() {
        let mut package = create_test_package();
        package.uuid = Some("invalid-uuid".to_string());

        let mut options = PackageValidationOptions::new();
        options.check_package_integrity = true;

        let validator = PackageValidator::new(Some(options));
        let result = validator.validate_package(&package).unwrap();

        assert!(!result.is_valid);
        assert!(result.errors.iter().any(|e| e.contains("Invalid UUID format")));
    }

    #[test]
    fn test_semantic_version_validation() {
        let mut package = create_test_package();
        package.version = Some(Version::parse("1.0").unwrap()); // Not semantic

        let mut options = PackageValidationOptions::new();
        options.check_version_compatibility = true;

        let validator = PackageValidator::new(Some(options));
        let result = validator.validate_package(&package).unwrap();

        // Should be valid but have warnings about non-semantic version
        assert!(result.is_valid);
        assert!(result.warnings.iter().any(|w| w.contains("does not follow semantic versioning")));
    }
}
