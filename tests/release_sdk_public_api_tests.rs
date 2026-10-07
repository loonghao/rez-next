//! External-consumer tests for the release SDK.
//!
//! These exercise the release workflow the way a downstream crate would, using
//! only `rez_next_build`'s public API: a hand-written [`ReleaseVCS`]
//! implementation is injected through
//! [`ReleaseManager::release_with_vcs`]. Nothing here touches a `pub(crate)`
//! item or a `#[doc(hidden)]` entry point — if the SDK surface regresses, this
//! file stops compiling, which is the point.
//!
//! The crate-internal unit tests in `crates/rez-next-build/src/release.rs`
//! cover step ordering and failure injection; this file covers the contract a
//! caller actually depends on: what it must call, and what it can read back.

use rez_next_build::{ReleaseManager, ReleaseMode, ReleaseResult, ReleaseVCS, VCSMetadata};
use rez_next_common::RezCoreError;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, PoisonError};
use tempfile::TempDir;

/// Serialises the tests that redirect the install path.
///
/// `ReleaseManager` reads the install root from `RezCoreConfig::load()`, which
/// honours the `REZ_LOCAL_PACKAGES_PATH` environment variable. Redirecting it
/// is process-global, so only one test may do it at a time.
fn paths_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// A VCS written from scratch for these tests, exercising the trait the way a
/// downstream provider would.
///
/// It records what the release flow asked of it so a caller can assert on the
/// interaction, and it fails `create_tag` for an existing tag — the behaviour of
/// the real providers, which tag with `force = false` and do not pre-check for
/// existence.
struct DownstreamVCS {
    tag_exists: bool,
    created_tags: Mutex<Vec<String>>,
}

impl DownstreamVCS {
    fn new(tag_exists: bool) -> Self {
        Self {
            tag_exists,
            created_tags: Mutex::new(Vec::new()),
        }
    }

    fn created_tags(&self) -> Vec<String> {
        self.created_tags
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl ReleaseVCS for DownstreamVCS {
    fn get_type_name(&self) -> &str {
        "downstream-vcs"
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
        Ok("downstream-commit".to_string())
    }

    fn tag_exists(&self, _tag: &str) -> Result<bool, RezCoreError> {
        Ok(self.tag_exists)
    }

    fn create_tag(&self, tag: &str, _message: &str) -> Result<(), RezCoreError> {
        if self.tag_exists {
            return Err(RezCoreError::BuildError(format!(
                "tag '{}' already exists",
                tag
            )));
        }
        self.created_tags
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(tag.to_string());
        Ok(())
    }

    fn get_changelog(
        &self,
        _from_rev: Option<&str>,
        _to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        Ok("downstream changelog".to_string())
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        Ok(VCSMetadata {
            vcs_type: "downstream-vcs".to_string(),
            commit_hash: "downstream-commit".to_string(),
            branch: Some("main".to_string()),
            ..Default::default()
        })
    }
}

/// Write a minimal `package.py` a downstream caller would release.
fn write_package(dir: &Path, name: &str, version: &str) {
    fs::write(
        dir.join("package.py"),
        format!("name = \"{}\"\nversion = \"{}\"\n", name, version),
    )
    .expect("write package.py");
}

/// Build a manager the way an external caller would.
///
/// Building is enabled and tests are skipped: the build step is what creates
/// the install directory, so disabling it would leave nothing to install into,
/// and running tests would shell out to `python` and make the run depend on the
/// host.
fn manager(mode: ReleaseMode, ignore_existing_tag: Option<bool>) -> ReleaseManager {
    let mut manager = ReleaseManager::new(mode, false, true);
    manager.set_skip_vcs_validation(true);
    manager.set_ignore_existing_tag(ignore_existing_tag);
    manager
}

/// Redirect `REZ_LOCAL_PACKAGES_PATH` at `dir` for the life of the returned
/// guard.
///
/// `ReleaseManager` resolves the install root from the ambient config, whose
/// `~/packages` default would install into the user's real home directory and
/// would additionally go through `~` expansion. An absolute temporary path
/// keeps the release hermetic and skips expansion entirely.
struct InstallRootGuard<'a> {
    dir: &'a Path,
}

impl<'a> InstallRootGuard<'a> {
    fn new(dir: &'a Path) -> Self {
        // SAFETY: the caller holds `paths_lock`, so no other test observes the
        // variable, and `drop` restores it before the lock is released.
        unsafe { std::env::set_var("REZ_LOCAL_PACKAGES_PATH", dir) };
        Self { dir }
    }
}

impl Drop for InstallRootGuard<'_> {
    fn drop(&mut self) {
        let _ = self.dir;
        // SAFETY: see `new`.
        unsafe { std::env::remove_var("REZ_LOCAL_PACKAGES_PATH") };
    }
}

/// Run a release with the install root redirected into a scratch directory.
///
/// Returns the result, the VCS the release was driven with, and the guards
/// keeping the source and install directories alive while the caller asserts.
fn release_into_scratch(
    name: &str,
    version: &str,
    mode: ReleaseMode,
    ignore_existing_tag: Option<bool>,
    tag_exists: bool,
    message: Option<&str>,
) -> (ReleaseResult, Arc<DownstreamVCS>, TempDir, TempDir) {
    let _lock = paths_lock().lock().unwrap_or_else(PoisonError::into_inner);

    let install_root = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write_package(source.path(), name, version);

    let _guard = InstallRootGuard::new(install_root.path());
    let vcs = Arc::new(DownstreamVCS::new(tag_exists));
    let result = manager(mode, ignore_existing_tag)
        .release_with_vcs(source.path(), message, Some(vcs.clone()))
        .expect("release must not propagate an error");
    drop(_guard);

    (result, vcs, source, install_root)
}

/// A caller can drive a full release with nothing but public API: its own VCS
/// in, a successful `ReleaseResult` out, and the release tag created once.
#[test]
fn downstream_can_release_with_its_own_vcs() {
    let (result, vcs, _source, _install_root) = release_into_scratch(
        "sdk_pkg",
        "1.0.0",
        ReleaseMode::Local,
        Some(false),
        false,
        Some("sdk release"),
    );

    assert!(
        result.errors.is_empty(),
        "a clean release must succeed, got {:?}",
        result.errors
    );
    assert!(result.success, "success must be true");
    assert_eq!(result.package_name, "sdk_pkg");
    assert_eq!(result.version, "1.0.0");

    assert_eq!(
        vcs.created_tags(),
        vec!["sdk_pkg-1.0.0".to_string()],
        "a successful release must create its tag exactly once"
    );
    assert_eq!(
        result.changelog.as_deref(),
        Some("downstream changelog"),
        "the changelog comes from the injected VCS"
    );
    assert_eq!(
        result.vcs_metadata.as_ref().map(|m| m.vcs_type.as_str()),
        Some("downstream-vcs"),
        "the metadata comes from the injected VCS"
    );
}

/// The result a caller reads back is enough to find the installed package:
/// `install_path` points at the copy, and the VCS metadata is persisted next
/// to it under the name the release flow documents.
#[test]
fn downstream_can_read_the_installed_result() {
    let (result, _vcs, _source, _install_root) = release_into_scratch(
        "sdk_installed",
        "2.1.0",
        ReleaseMode::Local,
        Some(false),
        false,
        None,
    );

    assert!(
        result.success,
        "release must succeed, got {:?}",
        result.errors
    );

    let install_path = PathBuf::from(&result.install_path);
    assert!(
        install_path.join("package.py").exists(),
        "the package definition must be installed at {}",
        result.install_path
    );

    let metadata_path = install_path.join("vcs_metadata.json");
    assert!(
        metadata_path.exists(),
        "vcs_metadata.json must be written next to the installed package"
    );
    let metadata: VCSMetadata =
        serde_json::from_str(&fs::read_to_string(&metadata_path).unwrap()).unwrap();
    assert_eq!(metadata.vcs_type, "downstream-vcs");
    assert_eq!(metadata.commit_hash, "downstream-commit");
}

/// A rejected release is `Ok`, not `Err`: the caller must check `success`.
///
/// This is the contract most likely to be got wrong downstream, so it is
/// asserted explicitly — strict tag policy over an existing tag fails the
/// release while still returning `Ok`.
#[test]
fn downstream_rejection_is_ok_with_errors_not_err() {
    let (result, vcs, _source, _install_root) = release_into_scratch(
        "sdk_rejected",
        "1.0.0",
        ReleaseMode::Local,
        Some(false),
        true,
        None,
    );

    assert!(!result.success, "success must be false when rejected");
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("already exists") && e.contains("sdk_rejected-1.0.0")),
        "the error must name the tag, got {:?}",
        result.errors
    );
    assert!(
        vcs.created_tags().is_empty(),
        "a rejected release must not create a tag"
    );
}

/// `Some(true)` over an existing tag: install, report success, and keep the
/// existing tag untouched — the caller's VCS refuses to re-tag, exactly as a
/// real provider would.
#[test]
fn downstream_can_opt_into_releasing_over_an_existing_tag() {
    let (result, vcs, _source, _install_root) = release_into_scratch(
        "sdk_reissue",
        "1.0.0",
        ReleaseMode::Local,
        Some(true),
        true,
        None,
    );

    assert!(
        result.errors.is_empty(),
        "an explicitly ignored existing tag must not fail, got {:?}",
        result.errors
    );
    assert!(result.success, "success must be true");
    assert!(
        vcs.created_tags().is_empty(),
        "an existing tag must never be re-created or moved"
    );
    assert!(
        result.warnings.iter().any(|w| w.contains("already exists")),
        "the existing-tag warning must be kept, got {:?}",
        result.warnings
    );
    assert!(
        PathBuf::from(&result.install_path)
            .join("package.py")
            .exists(),
        "the package must still be installed"
    );
}

/// A missing package definition is a rejection, not a workflow error: the
/// caller gets `Ok` with an error it can report.
#[test]
fn downstream_missing_package_definition_is_a_rejection() {
    let _lock = paths_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let install_root = TempDir::new().unwrap();
    let source = TempDir::new().unwrap(); // no package.py

    let vcs = Arc::new(DownstreamVCS::new(false));
    let result = manager(ReleaseMode::Local, Some(false))
        .release_with_vcs(source.path(), None, Some(vcs.clone()))
        .expect("a rejected release must still return Ok");

    assert!(!result.success, "success must be false");
    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("No package.py or package.yaml found")),
        "the error must name the missing definition, got {:?}",
        result.errors
    );
    assert!(
        vcs.created_tags().is_empty(),
        "a release with no package definition must not tag"
    );
    drop(install_root);
}

/// Dry run: validate only. The caller gets the resolved install path prefixed
/// with `[dry-run] `, and neither the filesystem nor the VCS is touched.
#[test]
fn downstream_dry_run_writes_nothing() {
    let (result, vcs, _source, _install_root) = release_into_scratch(
        "sdk_dry",
        "3.0.0",
        ReleaseMode::DryRun,
        Some(false),
        false,
        Some("would release"),
    );

    assert!(result.success, "a dry run must report success");
    assert!(
        result.install_path.starts_with("[dry-run] "),
        "the install path must be marked as a dry run, got {:?}",
        result.install_path
    );
    let real_path = PathBuf::from(result.install_path.trim_start_matches("[dry-run] "));
    assert!(
        !real_path.join("package.py").exists(),
        "a dry run must not install anything"
    );
    assert!(
        vcs.created_tags().is_empty(),
        "a dry run must not create a tag"
    );
    assert!(
        result.warnings.iter().any(|w| w.contains("would release")),
        "the release message is echoed as a warning, got {:?}",
        result.warnings
    );
}

/// `vcs: None` with VCS validation **enabled** really does fall back to
/// `detect_vcs`, which finds nothing in a plain temporary directory.
///
/// This is the path the earlier `skip_vcs_validation(true)` variant of this
/// test failed to reach: with validation disabled the `vcs: None` branch
/// short-circuits to `None` without ever calling `detect_vcs`, so the fallback
/// itself was untested. Enabling validation is what puts `detect_vcs` on the
/// path taken.
#[test]
fn downstream_none_vcs_falls_back_to_detection() {
    let _lock = paths_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let install_root = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write_package(source.path(), "sdk_novcs", "1.0.0");

    // Validation enabled: `vcs: None` must reach `detect_vcs`.
    let mut manager = ReleaseManager::new(ReleaseMode::Local, false, true);
    manager.set_skip_vcs_validation(false);
    manager.set_ignore_existing_tag(Some(false));

    let _guard = InstallRootGuard::new(install_root.path());
    let result = manager
        .release_with_vcs(source.path(), None, None)
        .expect("release must not propagate an error");
    drop(_guard);

    assert!(
        result.errors.is_empty(),
        "a release without a VCS must still succeed, got {:?}",
        result.errors
    );
    assert!(result.success, "success must be true");
    assert!(
        result.vcs_metadata.is_none(),
        "no VCS means no VCS metadata"
    );
    // Step 2's own wording. Step 7 emits a *different* message
    // ("No VCS detected, skipping metadata writing") whenever there is no VCS
    // at all, whichever branch produced that, so it cannot distinguish
    // "detection ran and found nothing" from "detection was skipped".
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("No VCS detected in source directory")),
        "detect_vcs must have run and found nothing, got warnings {:?}",
        result.warnings
    );
    assert!(
        PathBuf::from(&result.install_path)
            .join("package.py")
            .exists(),
        "the package must still be installed"
    );
}

/// `vcs: None` with VCS validation **disabled** skips detection entirely.
///
/// The counterpart to the test above, and the branch that used to be the only
/// one covered: with validation off, `detect_vcs` is never called, so a
/// directory that *would* be recognised as a repository is not consulted.
#[test]
fn downstream_none_vcs_skips_detection_when_validation_disabled() {
    let _lock = paths_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let install_root = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write_package(source.path(), "sdk_skipdetect", "1.0.0");
    // A marker `detect_vcs` would recognise if it ran.
    fs::create_dir_all(source.path().join(".git")).unwrap();

    let mut manager = ReleaseManager::new(ReleaseMode::Local, false, true);
    manager.set_skip_vcs_validation(true);
    manager.set_ignore_existing_tag(Some(false));

    let _guard = InstallRootGuard::new(install_root.path());
    let result = manager
        .release_with_vcs(source.path(), None, None)
        .expect("release must not propagate an error");
    drop(_guard);

    // Detection never ran, so no "No VCS detected" warning was recorded and no
    // repository was validated or opened.
    assert!(
        result.errors.is_empty(),
        "skipping detection must not produce errors, got {:?}",
        result.errors
    );
    // Step 2's wording is absent because detect_vcs never ran. Note that step 7
    // still says "No VCS detected, skipping metadata writing" here — which is
    // why that looser string is not evidence that detection happened.
    assert!(
        !result
            .warnings
            .iter()
            .any(|w| w.contains("No VCS detected in source directory")),
        "detect_vcs must not have run, got warnings {:?}",
        result.warnings
    );
    assert!(result.vcs_metadata.is_none());
}

/// Dry run over a repository whose state validation fails.
///
/// This pins the documented exception to the `success == errors.is_empty()`
/// equivalence: on the detection path a failed `validate_repo_state()` is
/// recorded but does not stop the run, and the dry-run block then sets
/// `success = true` regardless. Callers must read `errors` too — see the
/// rustdoc on `release_with_vcs`.
#[test]
fn downstream_dry_run_reports_validation_failure_in_errors() {
    let _lock = paths_lock().lock().unwrap_or_else(PoisonError::into_inner);
    let install_root = TempDir::new().unwrap();
    let source = TempDir::new().unwrap();
    write_package(source.path(), "sdk_dirty", "1.0.0");
    fs::create_dir_all(source.path().join(".git")).unwrap();

    // Validation enabled, `vcs: None`: `detect_vcs` finds the marker and its
    // `validate_repo_state()` fails on the fake repository.
    let mut manager = ReleaseManager::new(ReleaseMode::DryRun, false, true);
    manager.set_skip_vcs_validation(false);
    manager.set_ignore_existing_tag(Some(false));

    let _guard = InstallRootGuard::new(install_root.path());
    let result = manager
        .release_with_vcs(source.path(), None, None)
        .expect("release must not propagate an error");
    drop(_guard);

    assert!(
        result
            .errors
            .iter()
            .any(|e| e.contains("VCS validation failed")),
        "the validation failure must be reported in errors, got {:?}",
        result.errors
    );
    // The documented deviation: success is true even though errors is not empty.
    assert!(
        result.success,
        "a dry run reports success unconditionally even with errors"
    );
    assert_ne!(
        result.success,
        result.errors.is_empty(),
        "this is precisely the documented dry-run exception to the invariant"
    );
}
