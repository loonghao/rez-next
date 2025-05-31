//! Package validation functionality

use crate::Package;
use rez_core_version::Version;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
            format!("Package validation passed with {} warnings", self.warnings.len())
        } else {
            format!("Package validation failed with {} errors and {} warnings", 
                   self.errors.len(), self.warnings.len())
        }
    }
    
    /// String representation
    fn __str__(&self) -> String {
        self.summary()
    }
    
    /// Representation
    fn __repr__(&self) -> String {
        format!("PackageValidationResult(valid={}, errors={}, warnings={})", 
               self.is_valid, self.errors.len(), self.warnings.len())
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
            let parsed_versions: Result<Vec<Version>, _> = versions
                .into_iter()
                .map(|v| Version::parse(&v))
                .collect();
                
            match parsed_versions {
                Ok(versions) => {
                    self.known_packages.insert(name, versions);
                }
                Err(e) => {
                    return Err(PyErr::new::<pyo3::exceptions::PyValueError, _>(
                        format!("Invalid version in known packages: {}", e)
                    ));
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
        
        Ok(result)
    }
    
    /// Validate package from file
    pub fn validate_package_file(&self, _path: &str) -> PyResult<PackageValidationResult> {
        // This would load the package from file and validate it
        // Implementation depends on the serialization module
        Err(PyErr::new::<pyo3::exceptions::PyNotImplementedError, _>(
            "Package file validation not yet implemented"
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
                result.add_error(format!("Invalid private build requirement '{}': {}", req, e));
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
                result.add_error(format!("Duplicate variant at index {}: [{}]", i, variant_key));
                result.variants_valid = false;
            }
            seen_variants.insert(variant_key);
        }
        
        // Validate variant requirements
        for (i, variant) in package.variants.iter().enumerate() {
            for req in variant {
                if let Err(e) = self.validate_requirement_string(req) {
                    result.add_error(format!("Invalid variant requirement '{}' in variant {}: {}", req, i, e));
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
            result.add_error(format!("Circular dependency detected: {}", path.join(" -> ")));
            result.dependencies_valid = false;
        }
    }
    
    /// Check if package name is valid
    fn is_valid_package_name(&self, name: &str) -> bool {
        // Package names should be alphanumeric with underscores and hyphens
        !name.is_empty() && 
        name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') &&
        !name.starts_with('-') && 
        !name.ends_with('-')
    }
    
    /// Validate a requirement string
    fn validate_requirement_string(&self, req: &str) -> Result<(), String> {
        if req.is_empty() {
            return Err("Requirement cannot be empty".to_string());
        }
        
        // Basic validation - a full implementation would parse the requirement
        if req.contains("  ") {
            return Err("Requirement contains multiple spaces".to_string());
        }
        
        Ok(())
    }
    
    /// Check for circular dependencies (simplified)
    fn has_circular_dependency(
        &self, 
        package_name: &str, 
        _requires: &[String],
        visited: &mut HashSet<String>, 
        path: &mut Vec<String>
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
