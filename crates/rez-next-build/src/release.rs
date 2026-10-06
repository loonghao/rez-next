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
    /// How to treat an already-existing release tag.
    ///
    /// - `Some(false)`: hard-fail (the CLI default).
    /// - `Some(true)`: proceed, equivalent to `rez release --ignore-existing-tag`.
    /// - `None`: legacy behaviour — warn and continue.
    ignore_existing_tag: Option<bool>,
}

impl ReleaseManager {
    /// Create a new ReleaseManager
    pub fn new(mode: ReleaseMode, skip_build: bool, skip_tests: bool) -> Self {
        Self {
            mode,
            skip_build,
            skip_tests,
            skip_vcs_validation: false,
            ignore_existing_tag: None,
        }
    }

    /// Set whether to skip VCS validation
    pub fn set_skip_vcs_validation(&mut self, skip: bool) {
        self.skip_vcs_validation = skip;
    }

    /// Set how an existing release tag is treated.
    ///
    /// `None` keeps the legacy warn-and-continue behaviour; `Some(false)` makes
    /// an existing tag a hard error; `Some(true)` allows re-releasing over it.
    pub fn set_ignore_existing_tag(&mut self, ignore_existing_tag: Option<bool>) {
        self.ignore_existing_tag = ignore_existing_tag;
    }

    /// Execute the complete release workflow
    ///
    /// Steps:
    /// 1. Load and validate package definition
    /// 2. Detect and validate VCS repository state
    /// 3. Reject an already-released version (tag policy check, fails fast)
    /// 4. Build the package (including variants)
    /// 5. Run the package tests
    /// 6. Generate changelog
    /// 7. Write release metadata
    /// 8. Install the package
    /// 9. Create the VCS tag
    ///
    /// The tag is created last on purpose, so the invariant
    /// **tag exists ⟺ release succeeded** holds. Creating it earlier would
    /// leave a dangling tag behind whenever a build or test step fails, and
    /// that tag would then block any retry of the same version.
    pub fn release(
        &self,
        source_dir: &Path,
        message: Option<&str>,
    ) -> Result<ReleaseResult, RezCoreError> {
        self.release_with_vcs(source_dir, message, None)
    }

    /// Run the release flow with a caller-supplied VCS instead of the one
    /// `detect_vcs` would find.
    ///
    /// This is the single implementation of the workflow; `release()` is a thin
    /// wrapper passing `None` so the VCS is detected. Keeping one copy means a
    /// test that drives this method really is exercising the same step order
    /// as production.
    ///
    /// `#[doc(hidden)]` because it exists to let tests drive the release flow
    /// without a real repository.
    #[doc(hidden)]
    pub fn release_with_vcs(
        &self,
        source_dir: &Path,
        message: Option<&str>,
        vcs: Option<std::sync::Arc<dyn ReleaseVCS + Send + Sync>>,
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

        // Step 2: VCS detection and validation. A caller-supplied VCS is still
        // validated, so injecting one cannot bypass the repository-state check.
        let vcs = match vcs {
            Some(vcs_impl) => {
                if !self.skip_vcs_validation
                    && let Err(e) = vcs_impl.validate_repo_state()
                {
                    result.errors.push(format!("VCS validation failed: {}", e));
                    return Ok(result);
                }
                match vcs_impl.get_metadata() {
                    Ok(metadata) => result.vcs_metadata = Some(metadata),
                    Err(e) => result
                        .warnings
                        .push(format!("Failed to get VCS metadata: {}", e)),
                }
                Some(vcs_impl)
            }
            None => {
                if !self.skip_vcs_validation {
                    self.validate_vcs(source_dir, &mut result)?
                } else {
                    None
                }
            }
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

        // Step 3: Reject an already-released version before anything is built
        // or installed. The tag check must run first: the build step copies
        // `package.py` into the install path, so checking afterwards would
        // overwrite an already-released version even when the release is
        // ultimately rejected.
        //
        // Invariant: tag exists ⟺ release succeeded. This step therefore only
        // checks the policy; the tag itself is created in step 9, after the
        // package has been installed.
        if let Some(ref vcs_impl) = vcs {
            self.check_existing_tag(vcs_impl.as_ref(), &package, &mut result)?;
        }

        if !result.errors.is_empty() {
            result.success = false;
            return Ok(result);
        }

        // Step 4: Build the package (if not skipped)
        if !self.skip_build {
            self.build_package(source_dir, &package, &install_path, &mut result)?;
        }

        // Step 5: Run tests (if not skipped)
        if self.skip_tests {
            result
                .warnings
                .push("Tests skipped (skip_tests=true)".to_string());
        } else {
            self.run_tests(source_dir, &package, &install_path, &mut result)?;
        }

        // Step 6: Generate changelog (if VCS is available)
        if let Some(ref vcs_impl) = vcs {
            self.generate_changelog(vcs_impl.as_ref(), &package, &mut result)?;
        }

        // Step 7: Write release metadata
        self.write_release_metadata(&package, &install_path, &vcs, &mut result)?;

        // Step 8: Install package definition
        self.install_package_definition(source_dir, &install_path, &package, &mut result)?;

        // Step 9: Create the VCS tag, and only now. Every mutating step has
        // happened above, so `errors.is_empty()` means the package really was
        // installed — a tag therefore never outlives a failed release. A tag
        // creation failure here is recoverable by re-running the release:
        // nothing has been overwritten, so the version is not wedged.
        if let Some(ref vcs_impl) = vcs
            && result.errors.is_empty()
        {
            self.create_vcs_tag(vcs_impl.as_ref(), &package, message, &mut result)?;
        }

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
    ) -> Result<Option<std::sync::Arc<dyn ReleaseVCS + Send + Sync>>, RezCoreError> {
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

                Ok(Some(vcs_impl.into()))
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

    /// Check whether an already-released version is being released again.
    ///
    /// This is the policy gate that runs *before* anything is built or
    /// installed. It only inspects state, never mutates it: the tag itself is
    /// created by [`Self::create_vcs_tag`] once the release has succeeded. That
    /// split keeps the invariant **tag exists ⟺ release succeeded** — a tag
    /// created before the build would survive a build/test failure and leave a
    /// version permanently wedged.
    fn check_existing_tag(
        &self,
        vcs: &dyn ReleaseVCS,
        package: &Package,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        let tag_name = Self::release_tag_name(package);

        match vcs.tag_exists(&tag_name) {
            Ok(true) => match self.ignore_existing_tag {
                // Explicit opt-in: re-release over the existing tag.
                Some(true) => {
                    result
                        .warnings
                        .push(format!("Tag '{}' already exists", tag_name));
                }
                // Strict: an existing tag means this version was already
                // released. Overwriting the install while the tag stays on the
                // old commit would leave the tag and the installed
                // `vcs_metadata.json` disagreeing about the source commit.
                Some(false) => {
                    result.errors.push(format!(
                        "Release tag '{}' already exists. Use --ignore-existing-tag to override.",
                        tag_name
                    ));
                }
                // Legacy: warn and continue (preserves pre-existing behaviour).
                None => {
                    result
                        .warnings
                        .push(format!("Tag '{}' already exists", tag_name));
                }
            },
            Ok(false) => {}
            Err(e) => {
                result
                    .warnings
                    .push(format!("Failed to check tag existence: {}", e));
            }
        }

        Ok(())
    }

    /// Build the VCS tag name for a package version.
    fn release_tag_name(package: &Package) -> String {
        format!(
            "{}-{}",
            package.name,
            package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        )
    }

    /// Create the VCS tag for a successfully released version.
    ///
    /// Called only after the package has been built, tested and installed, so
    /// a failure in any of those steps leaves no tag behind.
    fn create_vcs_tag(
        &self,
        vcs: &dyn ReleaseVCS,
        package: &Package,
        message: Option<&str>,
        result: &mut ReleaseResult,
    ) -> Result<(), RezCoreError> {
        let tag_name = Self::release_tag_name(package);
        let default_message = format!("Release {}", tag_name);
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
        vcs: &Option<std::sync::Arc<dyn ReleaseVCS + Send + Sync>>,
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

    /// The directory `Local` mode installs a package version into.
    fn install_dir_for(name: &str, version: &str) -> PathBuf {
        let config = RezCoreConfig::load();
        PathBuf::from(expand_home(&config.local_packages_path))
            .join(name)
            .join(version)
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
        let vcs: std::sync::Arc<dyn ReleaseVCS + Send + Sync> = std::sync::Arc::new(vcs);
        manager
            .write_release_metadata(&pkg, install_dir.path(), &Some(vcs), &mut result)
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
        let vcs: Option<std::sync::Arc<dyn ReleaseVCS + Send + Sync>> = None;

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

    /// A VCS whose tags can be forced to exist, so `create_vcs_tag` can be
    /// exercised for the already-released case.
    struct ExistingTagVCS {
        tag_exists: bool,
        created: std::sync::Mutex<Vec<String>>,
    }

    impl crate::vcs::ReleaseVCS for ExistingTagVCS {
        fn get_type_name(&self) -> &str {
            "existing-tag-stub"
        }
        fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
            Ok(PathBuf::from("."))
        }
        fn is_clean(&self) -> Result<bool, RezCoreError> {
            Ok(true)
        }
        fn get_current_branch(&self) -> Result<String, RezCoreError> {
            Ok("main".to_string())
        }
        fn get_latest_commit(&self) -> Result<String, RezCoreError> {
            Ok("stub-commit".to_string())
        }
        fn tag_exists(&self, _tag: &str) -> Result<bool, RezCoreError> {
            Ok(self.tag_exists)
        }
        fn create_tag(&self, tag: &str, _message: &str) -> Result<(), RezCoreError> {
            self.created.lock().unwrap().push(tag.to_string());
            Ok(())
        }
        fn get_changelog(
            &self,
            _from: Option<&str>,
            _to: Option<&str>,
        ) -> Result<String, RezCoreError> {
            Ok(String::new())
        }
        fn get_metadata(&self) -> Result<crate::vcs::VCSMetadata, RezCoreError> {
            Ok(crate::vcs::VCSMetadata::default())
        }
    }

    fn manager_with_tag_policy(
        ignore_existing_tag: Option<bool>,
    ) -> (ReleaseManager, std::sync::Arc<ExistingTagVCS>) {
        let mut manager = ReleaseManager::new(ReleaseMode::Release, true, true);
        manager.set_ignore_existing_tag(ignore_existing_tag);
        (
            manager,
            std::sync::Arc::new(ExistingTagVCS {
                tag_exists: true,
                created: std::sync::Mutex::new(Vec::new()),
            }),
        )
    }

    /// Drive `ReleaseManager::release()` end to end with a stub VCS, so the tag
    /// policy and its position in the step order are both exercised.
    ///
    /// `build_and_test` controls whether the build and test steps run; turning
    /// it off keeps the run hermetic (no `python` lookup, no shelling out).
    fn run_release_with_policy(
        source_dir: &Path,
        name: &str,
        version: &str,
        ignore_existing_tag: Option<bool>,
        tag_exists: bool,
        build_and_test: bool,
    ) -> (ReleaseResult, std::sync::Arc<CountingTagVCS>) {
        init_git_repo(source_dir);
        create_test_package(source_dir, name, version);

        let mut manager = ReleaseManager::new(ReleaseMode::Local, !build_and_test, !build_and_test);
        manager.set_skip_vcs_validation(true);
        manager.set_ignore_existing_tag(ignore_existing_tag);

        let vcs = std::sync::Arc::new(CountingTagVCS {
            tag_exists,
            created: std::sync::Mutex::new(Vec::new()),
        });
        let vcs_for_release: std::sync::Arc<dyn crate::vcs::ReleaseVCS + Send + Sync> = vcs.clone();

        let result = manager
            .release_with_vcs(source_dir, None, Some(vcs_for_release))
            .expect("release() must not propagate an error");

        (result, vcs)
    }

    /// Create a directory that `detect_vcs` recognises as a git repository.
    fn init_git_repo(dir: &Path) {
        fs::create_dir_all(dir.join(".git")).expect("create .git dir");
    }

    /// A VCS stub that records every tag it is asked to create.
    struct CountingTagVCS {
        tag_exists: bool,
        created: std::sync::Mutex<Vec<String>>,
    }

    impl CountingTagVCS {
        fn created_tags(&self) -> Vec<String> {
            self.created.lock().unwrap().clone()
        }
    }

    impl crate::vcs::ReleaseVCS for CountingTagVCS {
        fn get_type_name(&self) -> &str {
            "counting-stub"
        }
        fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
            Ok(PathBuf::from("."))
        }
        fn is_clean(&self) -> Result<bool, RezCoreError> {
            Ok(true)
        }
        fn get_current_branch(&self) -> Result<String, RezCoreError> {
            Ok("main".to_string())
        }
        fn get_latest_commit(&self) -> Result<String, RezCoreError> {
            Ok("stub-commit".to_string())
        }
        fn tag_exists(&self, _tag: &str) -> Result<bool, RezCoreError> {
            Ok(self.tag_exists)
        }
        fn create_tag(&self, tag: &str, _message: &str) -> Result<(), RezCoreError> {
            self.created.lock().unwrap().push(tag.to_string());
            Ok(())
        }
        fn get_changelog(
            &self,
            _from: Option<&str>,
            _to: Option<&str>,
        ) -> Result<String, RezCoreError> {
            Ok(String::new())
        }
        fn get_metadata(&self) -> Result<crate::vcs::VCSMetadata, RezCoreError> {
            Ok(crate::vcs::VCSMetadata::default())
        }
    }

    /// `release()` must reject an existing tag *before* the build step, so an
    /// already-installed copy of the package is left byte-for-byte untouched.
    ///
    /// Regression guard: adding the tag policy without moving the check ahead
    /// of the build returned the right exit code but still overwrote the
    /// installed package.
    #[test]
    fn test_release_rejects_existing_tag_without_touching_installed_copy() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();

        let installed = install_dir_for("released_pkg", "1.0.0");
        fs::create_dir_all(&installed).unwrap();
        let installed_copy = installed.join("package.py");
        let sentinel = b"# the already-released package.py\n";
        fs::write(&installed_copy, sentinel).unwrap();

        let (result, vcs) =
            run_release_with_policy(&source, "released_pkg", "1.0.0", Some(false), true, false);

        assert!(
            result.errors.iter().any(|e| e.contains("already exists")),
            "strict mode must fail the release, got errors {:?}",
            result.errors
        );
        assert!(!result.success, "success must be false when rejected");
        assert_eq!(
            fs::read(&installed_copy).unwrap(),
            sentinel.to_vec(),
            "the installed package.py must not be overwritten"
        );
        assert!(
            vcs.created_tags().is_empty(),
            "a rejected release must not create a tag, got {:?}",
            vcs.created_tags()
        );
    }

    /// A release whose build fails must not leave a tag behind: the invariant
    /// is **tag exists ⟺ release succeeded**, so a dangling tag would wedge
    /// the version on every retry.
    ///
    /// The failure is induced inside the build step — after the tag policy
    /// check has passed, which is exactly the window in which a tag created up
    /// front survives a release that never ships. A variant whose hashed
    /// subdirectory cannot be created makes `build_package` record an error and
    /// continue, so the release ends with `success == false` instead of
    /// aborting early.
    #[test]
    fn test_release_does_not_create_tag_when_build_fails() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();
        init_git_repo(&source);

        let variant: Vec<String> = vec!["python-3.9".to_string()];
        fs::write(
            source.join("package.py"),
            "name = \"wedged_pkg\"\nversion = \"1.0.0\"\nvariants = [[\"python-3.9\"]]\n",
        )
        .unwrap();

        let install_dir = install_dir_for("wedged_pkg", "1.0.0");
        fs::create_dir_all(&install_dir).unwrap();

        // Recreate the variant hash `build_package` computes, then occupy that
        // path with a file so `create_dir_all` fails for the variant only.
        let mut hasher = Sha256::new();
        hasher.update(format!("{:?}", variant).as_bytes());
        let hash = hex::encode(hasher.finalize())[..8].to_string();
        fs::write(install_dir.join(&hash), "not a directory").unwrap();

        let mut manager = ReleaseManager::new(ReleaseMode::Local, false, true);
        manager.set_skip_vcs_validation(true);
        manager.set_ignore_existing_tag(Some(false));

        let vcs = std::sync::Arc::new(CountingTagVCS {
            tag_exists: false,
            created: std::sync::Mutex::new(Vec::new()),
        });
        let vcs_for_release: std::sync::Arc<dyn crate::vcs::ReleaseVCS + Send + Sync> = vcs.clone();

        let result = manager
            .release_with_vcs(&source, None, Some(vcs_for_release))
            .expect("release() must not propagate an error");

        assert!(
            result
                .errors
                .iter()
                .any(|e| e.contains("Failed to create variant directory")),
            "an uncreatable variant directory must fail the release, got {:?}",
            result.errors
        );
        assert!(
            !result.success,
            "success must be false after a failed build"
        );
        assert!(
            vcs.created_tags().is_empty(),
            "a failed build must not create a tag, got {:?}",
            vcs.created_tags()
        );
    }

    /// The complement of the two tests above: a successful release *does* tag,
    /// and tags exactly once.
    #[test]
    fn test_release_creates_tag_on_success() {
        let temp_dir = TempDir::new().unwrap();
        let source = temp_dir.path().join("src");
        fs::create_dir_all(&source).unwrap();

        let installed = install_dir_for("fresh_pkg", "2.0.0");
        fs::create_dir_all(&installed).unwrap();

        let (result, vcs) =
            run_release_with_policy(&source, "fresh_pkg", "2.0.0", Some(false), false, false);

        assert!(
            result.errors.is_empty(),
            "a clean release must succeed, got {:?}",
            result.errors
        );
        assert!(result.success, "success must be true");
        assert_eq!(
            vcs.created_tags(),
            vec!["fresh_pkg-2.0.0".to_string()],
            "a successful release must create its tag exactly once"
        );
    }

    #[test]
    fn test_check_existing_tag_strict_rejects() {
        let (manager, vcs) = manager_with_tag_policy(Some(false));
        let mut result = ReleaseResult::default();
        let mut package = rez_next_package::Package::new("tool".to_string());
        package.version = Some(rez_next_version::Version::new(Some("1.0.0")).unwrap());

        manager
            .check_existing_tag(&*vcs, &package, &mut result)
            .unwrap();

        assert!(
            result.errors.iter().any(|e| e.contains("already exists")),
            "strict mode must record an error, got {:?}",
            result.errors
        );
        assert!(
            result
                .warnings
                .iter()
                .all(|w| !w.contains("already exists")),
            "strict mode must not downgrade the rejection to a warning"
        );
        assert!(
            vcs.created.lock().unwrap().is_empty(),
            "the policy check must never create a tag"
        );
    }

    #[test]
    fn test_check_existing_tag_ignore_allows() {
        let (manager, vcs) = manager_with_tag_policy(Some(true));
        let mut result = ReleaseResult::default();
        let mut package = rez_next_package::Package::new("tool".to_string());
        package.version = Some(rez_next_version::Version::new(Some("1.0.0")).unwrap());

        manager
            .check_existing_tag(&*vcs, &package, &mut result)
            .unwrap();

        assert!(
            result.errors.is_empty(),
            "explicit opt-in must proceed, got {:?}",
            result.errors
        );
        assert!(result.warnings.iter().any(|w| w.contains("already exists")));
    }

    #[test]
    fn test_check_existing_tag_default_warns_and_continues() {
        let (manager, vcs) = manager_with_tag_policy(None);
        let mut result = ReleaseResult::default();
        let mut package = rez_next_package::Package::new("tool".to_string());
        package.version = Some(rez_next_version::Version::new(Some("1.0.0")).unwrap());

        manager
            .check_existing_tag(&*vcs, &package, &mut result)
            .unwrap();

        assert!(
            result.errors.is_empty(),
            "the legacy default must not start failing, got {:?}",
            result.errors
        );
        assert!(
            result.warnings.iter().any(|w| w.contains("already exists")),
            "the legacy default still warns"
        );
    }

    /// `create_vcs_tag` is now purely the tag-creation step; it must create the
    /// tag and report it, leaving policy to `check_existing_tag`.
    #[test]
    fn test_create_vcs_tag_creates_tag() {
        let mut manager = ReleaseManager::new(ReleaseMode::Release, true, true);
        manager.set_ignore_existing_tag(None);
        let vcs = std::sync::Arc::new(ExistingTagVCS {
            tag_exists: false,
            created: std::sync::Mutex::new(Vec::new()),
        });
        let mut result = ReleaseResult::default();
        let mut package = rez_next_package::Package::new("tool".to_string());
        package.version = Some(rez_next_version::Version::new(Some("1.0.0")).unwrap());

        manager
            .create_vcs_tag(&*vcs, &package, None, &mut result)
            .unwrap();

        assert!(
            result.errors.is_empty(),
            "tag creation must succeed, got {:?}",
            result.errors
        );
        assert_eq!(
            vcs.created.lock().unwrap().as_slice(),
            ["tool-1.0.0".to_string()],
            "the release tag must be created once"
        );
        assert!(
            result
                .warnings
                .iter()
                .any(|w| w.contains("Created VCS tag"))
        );
    }
}
