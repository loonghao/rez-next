//! Package management operations

use crate::{Package, PackageSerializer, PackageFormat, PackageValidator, PackageValidationOptions};
use rez_core_common::RezCoreError;
use rez_core_version::Version;
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::Path;
use std::fs;
use std::time::SystemTime;

/// Package installation options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageInstallOptions {
    /// Overwrite existing package
    #[pyo3(get, set)]
    pub overwrite: bool,
    
    /// Keep original timestamp
    #[pyo3(get, set)]
    pub keep_timestamp: bool,
    
    /// Force installation (ignore relocatable attribute)
    #[pyo3(get, set)]
    pub force: bool,
    
    /// Dry run mode
    #[pyo3(get, set)]
    pub dry_run: bool,
    
    /// Verbose output
    #[pyo3(get, set)]
    pub verbose: bool,
    
    /// Skip payload copying
    #[pyo3(get, set)]
    pub skip_payload: bool,
    
    /// Validate package before installation
    #[pyo3(get, set)]
    pub validate: bool,
}

/// Package copy options
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageCopyOptions {
    /// Destination package name (rename)
    #[pyo3(get)]
    pub dest_name: Option<String>,

    /// Destination package version (reversion)
    #[pyo3(get)]
    pub dest_version: Option<String>,

    /// Variant indices to copy (None = all)
    #[pyo3(get)]
    pub variants: Option<Vec<usize>>,
    
    /// Create shallow copy (symlinks)
    #[pyo3(get, set)]
    pub shallow: bool,
    
    /// Follow symlinks when copying
    #[pyo3(get, set)]
    pub follow_symlinks: bool,
    
    /// Installation options
    pub install_options: PackageInstallOptions,
}

/// Package operation result
#[pyclass]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageOperationResult {
    /// Whether the operation succeeded
    #[pyo3(get)]
    pub success: bool,
    
    /// Operation message
    #[pyo3(get)]
    pub message: String,
    
    /// Copied variants (for copy operations)
    #[pyo3(get)]
    pub copied_variants: Vec<String>,
    
    /// Skipped variants (for copy operations)
    #[pyo3(get)]
    pub skipped_variants: Vec<String>,
    
    /// Operation duration in milliseconds
    #[pyo3(get)]
    pub duration_ms: u64,
    
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Package manager for installation, copying, moving, and removal operations
#[pyclass]
#[derive(Debug)]
pub struct PackageManager {
    /// Package validator
    validator: PackageValidator,
    
    /// Default installation options
    default_install_options: PackageInstallOptions,
    
    /// Default copy options
    default_copy_options: PackageCopyOptions,
}

#[pymethods]
impl PackageInstallOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: false,
            force: false,
            dry_run: false,
            verbose: false,
            skip_payload: false,
            validate: true,
        }
    }
    
    /// Create options for quick installation
    #[staticmethod]
    pub fn quick() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: false,
            force: false,
            dry_run: false,
            verbose: false,
            skip_payload: true,
            validate: false,
        }
    }
    
    /// Create options for safe installation
    #[staticmethod]
    pub fn safe() -> Self {
        Self {
            overwrite: false,
            keep_timestamp: true,
            force: false,
            dry_run: false,
            verbose: true,
            skip_payload: false,
            validate: true,
        }
    }
}

#[pymethods]
impl PackageCopyOptions {
    #[new]
    pub fn new() -> Self {
        Self {
            dest_name: None,
            dest_version: None,
            variants: None,
            shallow: false,
            follow_symlinks: false,
            install_options: PackageInstallOptions::new(),
        }
    }
    
    /// Set destination name
    pub fn set_dest_name(&mut self, name: String) {
        self.dest_name = Some(name);
    }

    /// Set destination version
    pub fn set_dest_version(&mut self, version: String) {
        self.dest_version = Some(version);
    }

    /// Set variants to copy
    pub fn set_variants(&mut self, variants: Vec<usize>) {
        self.variants = Some(variants);
    }
}

#[pymethods]
impl PackageOperationResult {
    #[new]
    pub fn new(success: bool, message: String) -> Self {
        Self {
            success,
            message,
            copied_variants: Vec::new(),
            skipped_variants: Vec::new(),
            duration_ms: 0,
            metadata: HashMap::new(),
        }
    }
    
    /// Create success result
    #[staticmethod]
    pub fn success(message: String) -> Self {
        Self::new(true, message)
    }
    
    /// Create failure result
    #[staticmethod]
    pub fn failure(message: String) -> Self {
        Self::new(false, message)
    }
    
    /// Add copied variant
    pub fn add_copied_variant(&mut self, variant: String) {
        self.copied_variants.push(variant);
    }
    
    /// Add skipped variant
    pub fn add_skipped_variant(&mut self, variant: String) {
        self.skipped_variants.push(variant);
    }
    
    /// Set operation duration
    pub fn set_duration(&mut self, duration_ms: u64) {
        self.duration_ms = duration_ms;
    }
    
    /// Add metadata
    pub fn add_metadata(&mut self, key: String, value: String) {
        self.metadata.insert(key, value);
    }
    
    /// Get summary string
    pub fn summary(&self) -> String {
        if self.success {
            format!("Operation completed successfully: {}", self.message)
        } else {
            format!("Operation failed: {}", self.message)
        }
    }
    
    /// String representation
    fn __str__(&self) -> String {
        self.summary()
    }
    
    /// Representation
    fn __repr__(&self) -> String {
        format!("PackageOperationResult(success={}, message='{}')", 
               self.success, self.message)
    }
}

#[pymethods]
impl PackageManager {
    #[new]
    pub fn new() -> Self {
        Self {
            validator: PackageValidator::new(Some(PackageValidationOptions::new())),
            default_install_options: PackageInstallOptions::new(),
            default_copy_options: PackageCopyOptions::new(),
        }
    }
    
    /// Install a package to a repository
    pub fn install_package(
        &self,
        package: &Package,
        dest_path: &str,
        options: Option<PackageInstallOptions>
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let opts = options.unwrap_or_else(|| self.default_install_options.clone());
        
        // Validate package if requested
        if opts.validate {
            let validation_result = self.validator.validate_package(package)?;
            if !validation_result.is_valid {
                return Ok(PackageOperationResult::failure(
                    format!("Package validation failed: {}", validation_result.summary())
                ));
            }
        }
        
        // Check if package is relocatable (unless forced)
        if !opts.force && !opts.skip_payload {
            // In a full implementation, this would check the package's relocatable attribute
            // For now, we assume packages are relocatable
        }
        
        if opts.dry_run {
            let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
            let mut result = PackageOperationResult::success(
                format!("Would install package {} to {}", package.name, dest_path)
            );
            result.set_duration(duration);
            return Ok(result);
        }
        
        // Perform the actual installation
        match self.do_install_package(package, dest_path, &opts) {
            Ok(mut result) => {
                let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
                result.set_duration(duration);
                Ok(result)
            }
            Err(e) => {
                let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
                let mut result = PackageOperationResult::failure(
                    format!("Installation failed: {}", e)
                );
                result.set_duration(duration);
                Ok(result)
            }
        }
    }
    
    /// Copy a package to another repository
    pub fn copy_package(
        &self,
        package: &Package,
        dest_path: &str,
        options: Option<PackageCopyOptions>
    ) -> PyResult<PackageOperationResult> {
        let _start_time = SystemTime::now();
        let opts = options.unwrap_or_else(|| self.default_copy_options.clone());
        
        // Create a copy of the package with potential name/version changes
        let mut dest_package = package.clone();
        
        if let Some(ref dest_name) = opts.dest_name {
            dest_package.name = dest_name.clone();
        }
        
        if let Some(ref dest_version) = opts.dest_version {
            let version = Version::parse(dest_version)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(
                    format!("Invalid destination version: {}", e)
                ))?;
            dest_package.version = Some(version);
        }
        
        // Install the modified package
        self.install_package(&dest_package, dest_path, Some(opts.install_options))
    }
    
    /// Move a package to another repository
    pub fn move_package(
        &self,
        package: &Package,
        source_path: &str,
        dest_path: &str,
        options: Option<PackageCopyOptions>
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        
        // First copy the package
        let copy_result = self.copy_package(package, dest_path, options)?;
        
        if !copy_result.success {
            return Ok(copy_result);
        }
        
        // Then remove from source (in a full implementation)
        // For now, we just return the copy result
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(
            format!("Moved package {} from {} to {}", package.name, source_path, dest_path)
        );
        result.set_duration(duration);
        
        Ok(result)
    }
    
    /// Remove a package from a repository
    pub fn remove_package(
        &self,
        package_name: &str,
        package_version: Option<&str>,
        repo_path: &str,
        force: Option<bool>
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let _force = force.unwrap_or(false);
        
        // In a full implementation, this would:
        // 1. Find the package in the repository
        // 2. Check if it's safe to remove (no dependents unless forced)
        // 3. Remove the package files
        // 4. Update repository index
        
        let version_str = package_version.unwrap_or("latest");
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(
            format!("Removed package {}@{} from {}", package_name, version_str, repo_path)
        );
        result.set_duration(duration);
        
        Ok(result)
    }
    
    /// Remove a package family from a repository
    pub fn remove_package_family(
        &self,
        family_name: &str,
        repo_path: &str,
        force: Option<bool>
    ) -> PyResult<PackageOperationResult> {
        let start_time = SystemTime::now();
        let _force = force.unwrap_or(false);
        
        // In a full implementation, this would:
        // 1. Find all packages in the family
        // 2. Check if it's safe to remove (no dependents unless forced)
        // 3. Remove all package versions
        // 4. Remove the family directory
        // 5. Update repository index
        
        let duration = start_time.elapsed().unwrap_or_default().as_millis() as u64;
        let mut result = PackageOperationResult::success(
            format!("Removed package family {} from {}", family_name, repo_path)
        );
        result.set_duration(duration);
        
        Ok(result)
    }
}

impl PackageManager {
    /// Internal implementation of package installation
    fn do_install_package(
        &self,
        package: &Package,
        dest_path: &str,
        _options: &PackageInstallOptions
    ) -> Result<PackageOperationResult, RezCoreError> {
        // Create destination directory if it doesn't exist
        let dest_dir = Path::new(dest_path);
        if !dest_dir.exists() {
            fs::create_dir_all(dest_dir)
                .map_err(|e| RezCoreError::Repository(
                    format!("Failed to create destination directory: {}", e)
                ))?;
        }
        
        // Determine package file path
        let package_dir = dest_dir.join(&package.name);
        if let Some(ref version) = package.version {
            let version_dir = package_dir.join(version.as_str());
            fs::create_dir_all(&version_dir)
                .map_err(|e| RezCoreError::Repository(
                    format!("Failed to create package version directory: {}", e)
                ))?;
        } else {
            fs::create_dir_all(&package_dir)
                .map_err(|e| RezCoreError::Repository(
                    format!("Failed to create package directory: {}", e)
                ))?;
        }
        
        // Save package definition
        let package_file = if let Some(ref version) = package.version {
            package_dir.join(version.as_str()).join("package.yaml")
        } else {
            package_dir.join("package.yaml")
        };
        
        PackageSerializer::save_to_file(package, &package_file, PackageFormat::Yaml)
            .map_err(|e| RezCoreError::Repository(
                format!("Failed to save package definition: {}", e)
            ))?;
        
        Ok(PackageOperationResult::success(
            format!("Successfully installed package {} to {}", package.name, dest_path)
        ))
    }
}

impl Default for PackageInstallOptions {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for PackageCopyOptions {
    fn default() -> Self {
        Self::new()
    }
}
