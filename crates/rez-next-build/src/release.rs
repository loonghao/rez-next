//! Complete release workflow implementation
//!
//! This module provides the core release workflow logic that orchestrates
//! VCS validation, package building, tag creation, and metadata generation.

use crate::vcs::{ReleaseVCS, VCSMetadata, detect_vcs};
use rez_next_common::{RezCoreConfig, RezCoreError};
use rez_next_package::Package;
use rez_next_package::serialization::PackageSerializer;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

/// Release mode for the package release process
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum ReleaseMode {
    /// Normal release to release_packages_path
    Release,
    /// Local release to local_packages_path
    Local,
    /// Dry run: validate but don't write
    DryRun,
}

impl ReleaseMode {
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        match s {
            "local" => ReleaseMode::Local,
            "dry-run" | "dry_run" => ReleaseMode::DryRun,
            _ => ReleaseMode::Release,
        }
    }
}

/// Result of a release operation
#[derive(Debug, Clone, Default)]
pub struct ReleaseResult {
    pub success: bool,
    pub package_name: String,
    pub version: String,
    pub install_path: String,
    pub vcs_metadata: Option<VCSMetadata>,
    pub changelog: Option<String>,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

/// Release manager that orchestrates the complete release workflow
#[derive(Debug, Clone)]
pub struct ReleaseManager {
    mode: ReleaseMode,
    skip_build: bool,
    skip_tests: bool, // now used in release() method
    skip_vcs_validation: bool,
}

impl ReleaseManager {
    /// Create a new ReleaseManager
    pub fn new(mode: ReleaseMode, skip_build: bool, skip_tests: bool) -> Self {
        Self {
            mode,
            skip_build,
            skip_tests,
            skip_vcs_validation: false,
        }
    }

    /// Set whether to skip VCS validation
    pub fn set_skip_vcs_validation(&mut self, skip: bool) {
        self.skip_vcs_validation = skip;
    }

    /// Execute the complete release workflow
    ///
    /// Steps:
    /// 1. Load and validate package definition
    /// 2. Detect and validate VCS repository state
    /// 3. Build the package (including variants)
    /// 4. Create VCS tag
    /// 5. Generate changelog
    /// 6. Write release metadata
    /// 7. Install the package
    pub fn release(
        &self,
        source_dir: &Path,
        message: Option<&str>,
    ) -> Result<ReleaseResult, RezCoreError> {
        let mut result = ReleaseResult::default();

        // Step 1: Load package definition
        let package = self.load_package(source_dir, &mut result)?;
        if !result.errors.is_empty() {
            return Ok(result);
        }

        // Determine install path
        let install_path = self.get_install_path(&package)?;
        result.install_path = install_path.to_string_lossy().to_string();

        // Step 2: VCS validation (if not skipped)
        let vcs = if !self.skip_vcs_validation {
            self.validate_vcs(source_dir, &mut result)?
        } else {
            None
        };

        // For dry-run mode, add prefix and return early
        if self.mode == ReleaseMode::DryRun {
            result.install_path = format!("[dry-run] {}", result.install_path);
            result.success = true;
            if let Some(msg) = message {
                result.warnings.push(format!("[dry-run] note: {}", msg));
            }
            return Ok(result);
        }

        // Step 3: Build the package (if not skipped)
        if !self.skip_build {
            self.build_package(source_dir, &package, &install_path, &mut result)?;
        }

        // Step 3.5: Run tests (if not skipped)
        if self.skip_tests {
            result
                .warnings
                .push("Tests skipped (skip_tests=true)".to_string());
        } else {
            self.run_tests(source_dir, &package, &install_path, &mut result)?;
        }

        // Step 4: Create VCS tag (if VCS is available)
        if let Some(ref vcs_impl) = vcs {
            self.create_vcs_tag(vcs_impl, &package, message, &mut result)?;
        }

        // Step 5: Generate changelog (if VCS is available)
        if let Some(ref vcs_impl) = vcs {
            self.generate_changelog(vcs_impl, &package, &mut result)?;
        }

        // Step 6: Write release metadata
        self.write_release_metadata(&package, &install_path, &vcs, &mut result)?;

        // Step 7: Install package definition
        self.install_package_definition(source_dir, &install_path, &package, &mut result)?;

        result.success = result.errors.is_empty();
        Ok(result)
    }

    /// Load and validate package definition
    fn load_package(
        &self,
        source_dir: &Path,
        result: &mut ReleaseResult,
    ) -> Result<Package, RezCoreError> {
        let pkg_file = source_dir.join("package.py");
        let pkg_yaml = source_dir.join("package.yaml");

        let pkg_path = if pkg_file.exists() {
            &pkg_file
        } else if pkg_yaml.exists() {
            &pkg_yaml
        } else {
            result
                .errors
                .push("No package.py or package.yaml found".to_string());
            // Return Ok with default package - let release() decide based on result.errors
            return Ok(Package::new("".to_string()));
        };

        match PackageSerializer::load_from_file(pkg_path) {
            Ok(pkg) => {
                result.package_name = pkg.name.clone();
                result.version = pkg
                    .version
                    .as_ref()
                    .map(|v| v.as_str().to_string())
                    .unwrap_or_else(|| "unknown".to_string());

                // Validate package
                if pkg.name.is_empty() {
                    result.errors.push("Package name is empty".to_string());
                }
                if pkg.version.is_none() {
                    result.errors.push("Package version is not set".to_string());
                }

                Ok(pkg)
            }
            Err(e) => {
                let err_msg = format!("Failed to parse package: {}", e);
                result.errors.push(err_msg);
                // Return Ok with default package - let release() decide based on result.errors
                Ok(Package::new("".to_string()))
            }
        }
    }

    /// Get the install path for the package
    fn get_install_path(&self, package: &Package) -> Result<PathBuf, RezCoreError> {
        let config = RezCoreConfig::load();
        let version_str = package
            .version
            .as_ref()
            .map(|v| v.as_str().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let install_base = match self.mode {
            ReleaseMode::Local | ReleaseMode::DryRun => {
                PathBuf::from(expand_home(&config.local_packages_path))
            }
            ReleaseMode::Release => {
                let rp = &config.release_packages_path;
                if !rp.is_empty() && rp != "~/.rez/packages/int" {
                    PathBuf::from(expand_home(rp))
                } else {
                    PathBuf::from(expand_home(&config.local_packages_path))
                }
            }
        };

        Ok(install_base.join(&package.name).join(&version_str))
    }

    /// Validate VCS repository state
    fn validate_vcs(
        &self,
        source_dir: &Path,
        result: &mut ReleaseResult,
    ) -> Result<Option<Box<dyn ReleaseVCS + Send + Sync>>, RezCoreError> {
        match detect_vcs(source_dir) {
            Some(vcs_impl) => {
                // Validate repo state
                if let Err(e) = vcs_impl.validate_repo_state() {
                    result.errors.push(format!("VCS validation failed: {}", e));
                    return Ok(None);
                }

                // Get metadata
                match vcs_impl.get_metadata() {
                    Ok(metadata) => {
                        result.vcs_metadata = Some(metadata);
                    }
                    Err(e) => {
                        result
                            .warnings
                            .push(format!("Failed to get VCS metadata: {}", e));
                    }
                }

                Ok(Some(vcs_impl))
            }
            None => {
                result
                    .warnings
                    .push("No VCS detected in source directory".to_string());
                Ok(None)
            }
        }
    }

    /// Build the package (including variants)
    fn build_package(
        &self,
        source_dir: &Path,
        package: &Package,
        install_path: &Path,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        // Create base install directory
        if let Err(e) = fs::create_dir_all(install_path) {
            result
                .errors
                .push(format!("Failed to create install directory: {}", e));
            return Err(RezCoreError::BuildError(e.to_string()));
        }

        // Check if package has variants
        if !package.variants.is_empty() {
            result.warnings.push(format!(
                "Package has {} variant(s), creating variant directories",
                package.variants.len()
            ));

            // Create a hashed directory for each variant
            for variant in &package.variants {
                // Compute variant hash (SHA256 of variant debug representation)
                let mut hasher = Sha256::new();
                hasher.update(format!("{:?}", variant).as_bytes());
                let hash_bytes = hasher.finalize();
                let hash = hex::encode(hash_bytes)[..8].to_string();

                let variant_path = install_path.join(&hash);

                if let Err(e) = fs::create_dir_all(&variant_path) {
                    result.errors.push(format!(
                        "Failed to create variant directory for hash '{}': {}",
                        hash, e
                    ));
                    continue;
                }

                // Copy package.py to variant directory (basic implementation)
                let pkg_file = source_dir.join("package.py");
                if pkg_file.exists() {
                    let dest_file = variant_path.join("package.py");
                    if let Err(e) = fs::copy(&pkg_file, &dest_file) {
                        result.warnings.push(format!(
                            "Failed to copy package.py to variant '{}': {}",
                            hash, e
                        ));
                    }
                }

                // Write variant metadata file
                let metadata = serde_json::json!({
                    "variant": variant,
                    "hash": hash,
                });
                let metadata_path = variant_path.join("variant.json");
                if let Err(e) = fs::write(
                    &metadata_path,
                    serde_json::to_string_pretty(&metadata).unwrap_or_default(),
                ) {
                    result.warnings.push(format!(
                        "Failed to write variant metadata for hash '{}': {}",
                        hash, e
                    ));
                }

                result.warnings.push(format!(
                    "Created variant directory with hash: {} for variant {:?}",
                    hash, variant
                ));
            }
        } else {
            // No variants, just create the base install directory
            result
                .warnings
                .push("No variants defined, using base install path".to_string());

            // Copy package.py to install directory (basic implementation)
            let pkg_file = source_dir.join("package.py");
            if pkg_file.exists() {
                let dest_file = install_path.join("package.py");
                if let Err(e) = fs::copy(&pkg_file, &dest_file) {
                    result
                        .warnings
                        .push(format!("Failed to copy package.py: {}", e));
                }
            }
        }

        Ok(())
    }

    /// Create VCS tag for the release
    fn create_vcs_tag(
        &self,
        vcs: &dyn ReleaseVCS,
        package: &Package,
        message: Option<&str>,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        let tag_name = format!(
            "{}-{}",
            package.name,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );

        match vcs.tag_exists(&tag_name) {
            Ok(true) => {
                result
                    .warnings
                    .push(format!("Tag '{}' already exists", tag_name));
                return Ok(());
            }
            Ok(false) => {}
            Err(e) => {
                result
                    .warnings
                    .push(format!("Failed to check tag existence: {}", e));
            }
        }

        let default_message = format!(
            "Release {}-{}",
            package.name,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );
        let tag_message = message.unwrap_or(&default_message);

        match vcs.create_tag(&tag_name, tag_message) {
            Ok(_) => {
                result
                    .warnings
                    .push(format!("Created VCS tag: {}", tag_name));
            }
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to create VCS tag: {}", e));
            }
        }

        Ok(())
    }

    /// Generate changelog from VCS
    fn generate_changelog(
        &self,
        vcs: &dyn ReleaseVCS,
        _package: &Package,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        match vcs.get_changelog(None, None) {
            Ok(changelog) => {
                result.changelog = Some(changelog);
            }
            Err(e) => {
                result
                    .warnings
                    .push(format!("Failed to generate changelog: {}", e));
            }
        }
        Ok(())
    }

    /// Write release metadata to the package definition
    fn write_release_metadata(
        &self,
        _package: &Package,
        install_path: &Path,
        vcs: &Option<Box<dyn ReleaseVCS + Send + Sync>>,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        // Get VCS metadata if VCS is available
        if let Some(vcs_impl) = vcs {
            match vcs_impl.get_metadata() {
                Ok(metadata) => {
                    // Write VCS metadata to a separate JSON file
                    let metadata_path = install_path.join("vcs_metadata.json");
                    match serde_json::to_string_pretty(&metadata) {
                        Ok(json_str) => match fs::write(&metadata_path, json_str) {
                            Ok(_) => {
                                result.vcs_metadata = Some(metadata);
                            }
                            Err(e) => {
                                result
                                    .warnings
                                    .push(format!("Failed to write VCS metadata: {}", e));
                            }
                        },
                        Err(e) => {
                            result
                                .warnings
                                .push(format!("Failed to serialize VCS metadata: {}", e));
                        }
                    }
                }
                Err(e) => {
                    result
                        .warnings
                        .push(format!("Failed to get VCS metadata: {}", e));
                }
            }
        } else {
            // No VCS detected, skip metadata writing
            result
                .warnings
                .push("No VCS detected, skipping metadata writing".to_string());
        }
        Ok(())
    }

    /// Install package definition to the install path
    fn install_package_definition(
        &self,
        source_dir: &Path,
        install_path: &Path,
        _package: &Package,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        let pkg_file = source_dir.join("package.py");
        let pkg_yaml = source_dir.join("package.yaml");

        let (src, dest_name) = if pkg_file.exists() {
            (&pkg_file, "package.py")
        } else if pkg_yaml.exists() {
            (&pkg_yaml, "package.yaml")
        } else {
            return Ok(());
        };

        let dest = install_path.join(dest_name);
        match fs::copy(src, &dest) {
            Ok(_) => {}
            Err(e) => {
                result
                    .errors
                    .push(format!("Failed to copy package definition: {}", e));
                return Err(RezCoreError::BuildError(e.to_string()));
            }
        }

        Ok(())
    }

    /// Run package tests
    ///
    /// Executes tests defined in `package.py::tests()` function.
    /// Tests are shell commands that are executed in the install path.
    fn run_tests(
        &self,
        source_dir: &Path,
        _package: &Package,
        install_path: &Path,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        // Try to get test commands from package.py::tests()
        let test_commands = Self::get_test_commands(source_dir)?;

        if test_commands.is_empty() {
            result
                .warnings
                .push("No test commands found in package.py::tests()".to_string());
            return Ok(());
        }

        // Execute each test command
        for (i, cmd) in test_commands.iter().enumerate() {
            match Self::execute_test_command(cmd, install_path) {
                Ok(output) => {
                    if !output.status.success() {
                        let stderr = String::from_utf8_lossy(&output.stderr);
                        result.errors.push(format!(
                            "Test {} failed:\nCommand: {}\nError: {}",
                            i + 1,
                            cmd,
                            stderr
                        ));
                    }
                }
                Err(e) => {
                    result.errors.push(format!(
                        "Failed to execute test {}: {}\nCommand: {}",
                        i + 1,
                        e,
                        cmd
                    ));
                }
            }
        }

        Ok(())
    }

    /// Get test commands from package.py::tests() function
    fn get_test_commands(source_dir: &Path) -> Result<Vec<String>, RezCoreError> {
        let package_py = source_dir.join("package.py");

        if !package_py.exists() {
            return Ok(Vec::new());
        }

        // Python script to call tests() and print result as JSON
        let python_script = format!(
            r#"
import sys
import json
sys.path.insert(0, r"{}")
try:
    from package import tests
    commands = tests()
    print(json.dumps(commands))
except ImportError:
    print("[]")
except Exception as e:
    print("[]")
"#,
            source_dir.display()
        );

        // Execute Python script
        let output = std::process::Command::new("python")
            .arg("-c")
            .arg(&python_script)
            .output();

        match output {
            Ok(out) => {
                if out.status.success() {
                    let stdout = String::from_utf8_lossy(&out.stdout);
                    let trimmed = stdout.trim();
                    if trimmed.starts_with('[') {
                        // Parse JSON array of strings
                        match serde_json::from_str::<Vec<String>>(trimmed) {
                            Ok(commands) => Ok(commands),
                            Err(_) => Ok(Vec::new()),
                        }
                    } else {
                        Ok(Vec::new())
                    }
                } else {
                    Ok(Vec::new())
                }
            }
            Err(_) => Ok(Vec::new()),
        }
    }

    /// Execute a single test command (cross-platform)
    fn execute_test_command(
        cmd: &str,
        install_path: &Path,
    ) -> Result<std::process::Output, std::io::Error> {
        // Detect platform and use appropriate shell
        #[cfg(windows)]
        {
            std::process::Command::new("cmd")
                .arg("/c")
                .arg(cmd)
                .current_dir(install_path)
                .output()
        }
        #[cfg(not(windows))]
        {
            std::process::Command::new("sh")
                .arg("-c")
                .arg(cmd)
                .current_dir(install_path)
                .output()
        }
    }
}

/// Expand `~` in path to the user's home directory
fn expand_home(path: &str) -> String {
    if path.starts_with("~") {
        // Try to get home directory from environment variables
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .unwrap_or_default();
        if !home.is_empty() {
            return path.replacen("~", &home, 1);
        }
    }
    path.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::vcs::{StubVCS, VCSMetadata};
    use std::fs::File;
    use std::io::Write;
    use tempfile::TempDir;

    fn create_test_package(dir: &Path, name: &str, version: &str) -> PathBuf {
        let pkg_file = dir.join("package.py");
        let content = format!(
            r#"name = "{}"
version = "{}"
"#,
            name, version
        );
        let mut file = File::create(&pkg_file).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        pkg_file
    }

    #[test]
    fn test_release_mode_from_str() {
        assert_eq!(ReleaseMode::from_str("release"), ReleaseMode::Release);
        assert_eq!(ReleaseMode::from_str("local"), ReleaseMode::Local);
        assert_eq!(ReleaseMode::from_str("dry-run"), ReleaseMode::DryRun);
        assert_eq!(ReleaseMode::from_str("dry_run"), ReleaseMode::DryRun);
        assert_eq!(ReleaseMode::from_str("unknown"), ReleaseMode::Release);
    }

    #[test]
    fn test_release_manager_new() {
        let _manager = ReleaseManager::new(ReleaseMode::Release, false, false);
        // Just verify it creates without error
    }

    #[test]
    fn test_load_package_success() {
        let temp_dir = TempDir::new().unwrap();
        create_test_package(temp_dir.path(), "test_pkg", "1.0.0");

        let manager = ReleaseManager::new(ReleaseMode::DryRun, true, true);
        let mut result = ReleaseResult::default();

        let pkg = manager.load_package(temp_dir.path(), &mut result);
        assert!(pkg.is_ok());
        assert_eq!(result.package_name, "test_pkg");
        assert_eq!(result.version, "1.0.0");
    }

    #[test]
    fn test_load_package_no_file() {
        let temp_dir = TempDir::new().unwrap();
        let manager = ReleaseManager::new(ReleaseMode::DryRun, true, true);
        let mut result = ReleaseResult::default();

        let pkg = manager.load_package(temp_dir.path(), &mut result);
        // load_package now returns Ok with default Package when file is missing
        assert!(
            pkg.is_ok(),
            "load_package should return Ok with default Package"
        );
        assert!(
            !result.errors.is_empty(),
            "should have errors when package file is missing"
        );
    }

    #[test]
    fn test_write_release_metadata_creates_file() {
        // Create temp dirs
        let source_dir = TempDir::new().unwrap();
        let install_dir = TempDir::new().unwrap();

        // Create package.py in source
        let pkg_file = source_dir.path().join("package.py");
        let content = r#"name = "test_pkg"
version = "1.0.0"
"#;
        std::fs::write(&pkg_file, content).unwrap();

        // Create StubVCS with metadata
        let metadata = VCSMetadata {
            vcs_type: "stub".to_string(),
            repository_url: Some("https://example.com/repo.git".to_string()),
            branch: Some("main".to_string()),
            commit_hash: "abc123".to_string(),
            ..Default::default()
        };
        let vcs = StubVCS::with_metadata(source_dir.path().to_path_buf(), metadata);

        // Create ReleaseManager (dry-run mode)
        let manager = ReleaseManager::new(ReleaseMode::DryRun, true, true);

        // Create a Package for the test
        let pkg = rez_next_package::Package::new("test_pkg".to_string());
        let mut pkg = pkg;
        pkg.version = Some(rez_next_version::Version::new(Some("1.0.0")).unwrap());

        // Manually call write_release_metadata
        let mut result = ReleaseResult::default();
        manager
            .write_release_metadata(
                &pkg,
                install_dir.path(),
                &Some(Box::new(vcs) as Box<dyn ReleaseVCS + Send + Sync>),
                &mut result,
            )
            .unwrap();

        // Verify vcs_metadata.json was created
        let metadata_path = install_dir.path().join("vcs_metadata.json");
        assert!(
            metadata_path.exists(),
            "vcs_metadata.json should be created"
        );

        // Verify JSON content
        let json_content = std::fs::read_to_string(&metadata_path).unwrap();
        assert!(json_content.contains("stub"), "Should contain vcs_type");
        assert!(
            json_content.contains("abc123"),
            "Should contain commit_hash"
        );
        assert!(json_content.contains("main"), "Should contain branch");

        // Verify result has vcs_metadata
        assert!(result.vcs_metadata.is_some());
        let vcs_meta = result.vcs_metadata.unwrap();
        assert_eq!(vcs_meta.vcs_type, "stub");
        assert_eq!(vcs_meta.commit_hash, "abc123");
    }

    #[test]
    fn test_write_release_metadata_no_vcs() {
        let source_dir = TempDir::new().unwrap();
        let install_dir = TempDir::new().unwrap();

        // Create package.py
        let pkg_file = source_dir.path().join("package.py");
        let content = r#"name = "test_pkg"
version = "1.0.0"
"#;
        std::fs::write(&pkg_file, content).unwrap();

        // No VCS
        let vcs: Option<Box<dyn ReleaseVCS + Send + Sync>> = None;

        // Create ReleaseManager
        let manager = ReleaseManager::new(ReleaseMode::DryRun, true, true);

        // Create a Package for the test
        let pkg = rez_next_package::Package::new("test_pkg".to_string());

        // Call write_release_metadata with no VCS
        let mut result = ReleaseResult::default();
        manager
            .write_release_metadata(&pkg, install_dir.path(), &vcs, &mut result)
            .unwrap();

        // Verify vcs_metadata.json was NOT created
        let metadata_path = install_dir.path().join("vcs_metadata.json");
        assert!(
            !metadata_path.exists(),
            "vcs_metadata.json should NOT be created when no VCS"
        );

        // Verify warning was added
        assert!(!result.warnings.is_empty());
        assert!(result.warnings[0].contains("No VCS detected"));
    }

    // ── Tests for run_tests() functionality ─────────────────────────────

    #[test]
    fn test_get_test_commands_with_tests_function() {
        // Create a temp directory with package.py that has tests() function
        let temp_dir = TempDir::new().unwrap();
        let pkg_file = temp_dir.path().join("package.py");
        let content = r#"name = "test_pkg"
version = "1.0.0"

def tests():
    return ["echo 'test1'", "echo 'test2'"]
"#;
        std::fs::write(&pkg_file, content).unwrap();

        // Call get_test_commands
        let commands = ReleaseManager::get_test_commands(temp_dir.path()).unwrap();

        // Should return the two test commands
        assert_eq!(commands.len(), 2);
        assert_eq!(commands[0], "echo 'test1'");
        assert_eq!(commands[1], "echo 'test2'");
    }

    #[test]
    fn test_get_test_commands_without_tests_function() {
        // Create a temp directory with package.py that does NOT have tests() function
        let temp_dir = TempDir::new().unwrap();
        let pkg_file = temp_dir.path().join("package.py");
        let content = r#"name = "test_pkg"
version = "1.0.0"
"#;
        std::fs::write(&pkg_file, content).unwrap();

        // Call get_test_commands
        let commands = ReleaseManager::get_test_commands(temp_dir.path()).unwrap();

        // Should return empty vec (no tests() function)
        assert!(commands.is_empty());
    }

    #[test]
    fn test_get_test_commands_no_package_py() {
        // Create an empty temp directory (no package.py)
        let temp_dir = TempDir::new().unwrap();

        // Call get_test_commands
        let commands = ReleaseManager::get_test_commands(temp_dir.path()).unwrap();

        // Should return empty vec (no package.py)
        assert!(commands.is_empty());
    }

    #[test]
    fn test_execute_test_command_success() {
        // Execute a simple command that succeeds
        #[cfg(windows)]
        let cmd = "echo test_passed";
        #[cfg(not(windows))]
        let cmd = "echo test_passed";

        let result = ReleaseManager::execute_test_command(cmd, std::path::Path::new("."));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(output.status.success());
    }

    #[test]
    fn test_execute_test_command_failure() {
        // Execute a command that fails
        #[cfg(windows)]
        let cmd = "exit 1";
        #[cfg(not(windows))]
        let cmd = "false";

        let result = ReleaseManager::execute_test_command(cmd, std::path::Path::new("."));
        assert!(result.is_ok());
        let output = result.unwrap();
        assert!(!output.status.success());
    }
}
