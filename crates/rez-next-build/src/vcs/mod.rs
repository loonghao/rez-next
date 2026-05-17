//! Version Control System integration for release management
//!
//! This module provides the `ReleaseVCS` trait and implementations for
//! different version control systems (Git, Mercurial, SVN, etc.)
//!
//! The implementations are split into separate files:
//! - `git.rs` — `GitVCS` implementation (requires feature "git")
//! - `hg.rs` — `MercurialVCS` implementation
//! - `svn.rs` — `SvnVCS` implementation

#[cfg(feature = "git")]
mod git;

mod hg;
mod svn;

// Re-export VCS implementations for Python bindings
#[cfg(feature = "git")]
pub use git::GitVCS;
pub use hg::MercurialVCS;
pub use svn::SvnVCS;

use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

// ============================================================================
// VCS Revision
// ============================================================================

/// A type-erased VCS revision that can be serialized/deserialized.
///
/// This aligns with Rez's `get_current_revision()` which can return
/// "any type (str, dict etc.)". We use a structured representation
/// that preserves the revision data in a JSON-compatible format.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VCSRevision {
    /// VCS type (git, hg, svn, etc.)
    pub revision_type: String,
    /// The revision identifier (commit hash, changeset hash, revision number, etc.)
    pub revision_id: String,
    /// Full revision data (can be a string, dict, etc.)
    pub data: JsonValue,
    /// Additional metadata (branch, tags, etc.)
    #[serde(default)]
    pub metadata: HashMap<String, String>,
}

impl VCSRevision {
    /// Create a new VCSRevision
    pub fn new(revision_type: &str, revision_id: &str) -> Self {
        Self {
            revision_type: revision_type.to_string(),
            revision_id: revision_id.to_string(),
            data: JsonValue::String(revision_id.to_string()),
            metadata: HashMap::new(),
        }
    }

    /// Create a VCSRevision with custom data
    pub fn with_data(revision_type: &str, revision_id: &str, data: JsonValue) -> Self {
        Self {
            revision_type: revision_type.to_string(),
            revision_id: revision_id.to_string(),
            data,
            metadata: HashMap::new(),
        }
    }
}

/// Trait for VCS integration in release workflow
///
/// This trait defines the interface for interacting with version control
/// systems during the package release process.
/// Compatible with original rez ReleaseVCS interface.
pub trait ReleaseVCS: Send + Sync {
    /// Get the VCS type name
    fn get_type_name(&self) -> &str;

    /// Get the repository root path
    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError>;

    /// Check if the repository is clean (no uncommitted changes)
    fn is_clean(&self) -> Result<bool, RezCoreError>;

    /// Get the current branch name
    fn get_current_branch(&self) -> Result<String, RezCoreError>;

    /// Get the latest commit hash
    fn get_latest_commit(&self) -> Result<String, RezCoreError>;

    /// Check if a tag exists
    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError>;

    /// Create a new tag
    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError>;

    /// Get changelog between two revisions
    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError>;

    /// Get VCS metadata for release
    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError>;

    /// Validate repository state for release
    ///
    /// Checks that the repository is in a valid state for releasing:
    /// - Working directory is clean
    /// - On a valid release branch (if applicable)
    /// - No pending changes or conflicts.
    fn validate_repo_state(&self) -> Result<(), RezCoreError> {
        // Default implementation: check if repo is clean
        if !self.is_clean()? {
            return Err(RezCoreError::BuildError(
                "Repository is not clean".to_string(),
            ));
        }
        Ok(())
    }

    /// Check if the current branch is a releasable branch
    ///
    /// For Git, this might check if we're on `main`, `master`, or a release branch.
    /// For Mercurial, this might check if we're on `default` or a named branch.
    /// Returns `None` if the concept doesn't apply to this VCS.
    fn is_releasable_branch(&self) -> Result<Option<bool>, RezCoreError> {
        // Default: not applicable (return None)
        Ok(None)
    }

    /// Get the current revision object.
    ///
    /// This aligns with Rez's `ReleaseVCS.get_current_revision()` which can return
    /// "any type (str, dict etc.)". We return a `VCSRevision` struct that
    /// encapsulates the revision data in a type-safe way.
    ///
    /// The returned `VCSRevision` contains:
    /// - `revision_type`: VCS type (git, hg, svn, etc.)
    /// - `revision_id`: The revision identifier (commit hash, changset hash, etc.)
    /// - `data`: Full revision data (JSON-compatible for flexibility)
    /// - `metadata`: Additional metadata (branch, tags, etc.)
    fn get_current_revision(&self) -> Result<VCSRevision, RezCoreError> {
        // Default: return the latest commit hash as the revision
        let commit_hash = self.get_latest_commit()?;
        Ok(VCSRevision::new(self.get_type_name(), &commit_hash))
    }

    /// Export the repository at the given revision to the given path.
    ///
    /// This aligns with Rez's `ReleaseVCS.export()` classmethod.
    /// Exports the repository to the given path at the given revision.
    ///
    /// # Arguments
    ///
    /// * `revision` - The revision to export (as `VCSRevision`)
    /// * `path` - The path to export to (must not exist, parent must exist)
    ///
    /// # Returns
    ///
    /// `Ok(())` on success, or `RezCoreError` on failure.
    fn export(&self, _revision: &VCSRevision, _path: &Path) -> Result<(), RezCoreError> {
        Err(RezCoreError::BuildError(
            "export() not implemented for this VCS".to_string(),
        ))
    }
}

/// VCS metadata for release
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VCSMetadata {
    /// VCS type (git, hg, svn, etc.)
    pub vcs_type: String,
    /// Repository URL (fetch URL)
    pub repository_url: Option<String>,
    /// Current branch name
    pub branch: Option<String>,
    /// Tracking branch name (e.g., "origin/main")
    pub tracking_branch: Option<String>,
    /// Fetch URL (where the repo is fetched from)
    pub fetch_url: Option<String>,
    /// Push URL (where the repo is pushed to)
    pub push_url: Option<String>,
    /// Latest commit hash
    pub commit_hash: String,
    /// Commit message
    pub commit_message: Option<String>,
    /// Author name
    pub author_name: Option<String>,
    /// Author email
    pub author_email: Option<String>,
    /// Timestamp of the commit
    pub timestamp: Option<i64>,
    /// Additional metadata
    #[serde(default)]
    pub extra: HashMap<String, String>,
}

/// Stub VCS implementation for testing
///
/// This is a no-op implementation that returns dummy data.
/// Used when VCS is not available or for testing.
#[derive(Debug)]
pub struct StubVCS {
    /// Repository root path
    repo_root: PathBuf,
    /// Optional metadata override for testing
    metadata: Option<VCSMetadata>,
}

impl StubVCS {
    /// Create a new StubVCS
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            metadata: None,
        }
    }

    /// Create a new StubVCS with custom metadata
    pub fn with_metadata(repo_root: PathBuf, metadata: VCSMetadata) -> Self {
        Self {
            repo_root,
            metadata: Some(metadata),
        }
    }
}

impl ReleaseVCS for StubVCS {
    fn get_type_name(&self) -> &str {
        "stub"
    }

    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
        Ok(self.repo_root.clone())
    }

    fn is_clean(&self) -> Result<bool, RezCoreError> {
        Ok(true) // Stub is always clean
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        Ok("main".to_string())
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        Ok("stub-commit-hash".to_string())
    }

    fn tag_exists(&self, _tag: &str) -> Result<bool, RezCoreError> {
        Ok(false) // Stub never has tags
    }

    fn create_tag(&self, _tag: &str, _message: &str) -> Result<(), RezCoreError> {
        Ok(()) // Stub always succeeds
    }

    fn get_changelog(
        &self,
        _from_rev: Option<&str>,
        _to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        Ok("Stub changelog".to_string())
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        if let Some(ref metadata) = self.metadata {
            return Ok(metadata.clone());
        }

        Ok(VCSMetadata {
            vcs_type: "stub".to_string(),
            commit_hash: "stub-commit-hash".to_string(),
            branch: Some("main".to_string()),
            ..Default::default()
        })
    }
}

/// Implement ReleaseVCS for Box<dyn ReleaseVCS + Send + Sync>
///
/// This allows using Box<dyn ReleaseVCS + Send + Sync> as a ReleaseVCS trait object.
impl ReleaseVCS for Box<dyn ReleaseVCS + Send + Sync> {
    fn get_type_name(&self) -> &str {
        (**self).get_type_name()
    }

    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
        (**self).get_repo_root()
    }

    fn is_clean(&self) -> Result<bool, RezCoreError> {
        (**self).is_clean()
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        (**self).get_current_branch()
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        (**self).get_latest_commit()
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> {
        (**self).tag_exists(tag)
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        (**self).create_tag(tag, message)
    }

    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        (**self).get_changelog(from_rev, to_rev)
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        (**self).get_metadata()
    }

    fn validate_repo_state(&self) -> Result<(), RezCoreError> {
        (**self).validate_repo_state()
    }

    fn is_releasable_branch(&self) -> Result<Option<bool>, RezCoreError> {
        (**self).is_releasable_branch()
    }
}

/// Detect VCS type from repository path
pub fn detect_vcs(repo_path: &Path) -> Option<Box<dyn ReleaseVCS + Send + Sync>> {
    // Check for Stub VCS (used for testing)
    if repo_path.join(".stub").exists() {
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
    }

    // Check for Git
    if repo_path.join(".git").exists() {
        #[cfg(feature = "git")]
        return Some(Box::new(git::GitVCS::new(repo_path.to_path_buf()).ok()?));

        // Fall back to StubVCS if git feature is not enabled
        #[cfg(not(feature = "git"))]
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
    }

    // Check for Mercurial
    if repo_path.join(".hg").exists() {
        return Some(Box::new(
            hg::MercurialVCS::new(repo_path.to_path_buf()).ok()?,
        ));
    }

    // Check for SVN
    if repo_path.join(".svn").exists() {
        return Some(Box::new(svn::SvnVCS::new(repo_path.to_path_buf()).ok()?));
    }

    None
}

/// Get VCS metadata for a package repository
pub fn get_vcs_metadata(repo_path: &Path) -> Result<Option<VCSMetadata>, RezCoreError> {
    if let Some(vcs) = detect_vcs(repo_path) {
        let metadata = vcs.get_metadata()?;
        Ok(Some(metadata))
    } else {
        Ok(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(feature = "git")]
    use git2;
    use std::fs;

    #[test]
    fn test_stub_vcs_creation() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));
        assert_eq!(vcs.get_type_name(), "stub");
        assert!(vcs.is_clean().unwrap());
        assert_eq!(vcs.get_current_branch().unwrap(), "main");
    }

    #[test]
    fn test_stub_vcs_metadata() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));
        let metadata = vcs.get_metadata().unwrap();
        assert_eq!(metadata.vcs_type, "stub");
        assert_eq!(metadata.commit_hash, "stub-commit-hash");
    }

    #[test]
    fn test_vcs_metadata_default() {
        let metadata = VCSMetadata::default();
        assert_eq!(metadata.vcs_type, "");
        assert_eq!(metadata.commit_hash, "");
        assert!(metadata.extra.is_empty());
    }

    #[test]
    fn test_stub_vcs_tag_operations() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));

        // Tag should not exist in stub
        assert!(!vcs.tag_exists("v1.0.0").unwrap());

        // Creating tag should succeed
        assert!(vcs.create_tag("v1.0.0", "Release 1.0.0").is_ok());
    }

    #[test]
    fn test_stub_vcs_changelog() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));
        let changelog = vcs.get_changelog(None, None).unwrap();
        assert!(changelog.contains("Stub changelog"));
    }

    #[test]
    fn test_detect_vcs_no_vcs() {
        let path = PathBuf::from("/tmp/non-repo");
        let result = detect_vcs(&path);
        assert!(result.is_none());
    }

    #[test]
    fn test_stub_vcs_with_metadata() {
        let metadata = VCSMetadata {
            vcs_type: "stub".to_string(),
            commit_hash: "custom-hash".to_string(),
            branch: Some("develop".to_string()),
            ..Default::default()
        };
        let vcs = StubVCS::with_metadata(PathBuf::from("/tmp/repo"), metadata);
        let retrieved = vcs.get_metadata().unwrap();
        assert_eq!(retrieved.branch, Some("develop".to_string()));
        assert_eq!(retrieved.commit_hash, "custom-hash");
    }

    /// Helper function to create a temporary git repository for testing
    #[cfg(feature = "git")]
    fn create_temp_git_repo() -> (tempfile::TempDir, git2::Repository) {
        let temp_dir = tempfile::TempDir::new().unwrap();

        // Configure git to use "main" as default branch
        let mut config = git2::Config::open_default().unwrap();
        config.set_str("init.defaultBranch", "main").unwrap();

        let repo = git2::Repository::init(temp_dir.path()).unwrap();

        // Configure git user for commits
        let mut repo_config = repo.config().unwrap();
        repo_config.set_str("user.name", "Test User").unwrap();
        repo_config
            .set_str("user.email", "test@example.com")
            .unwrap();

        // Create an initial commit
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };

        // Create commit in a separate scope so tree is dropped before we return repo
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[])
                .unwrap();
        }

        (temp_dir, repo)
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_creation() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();
        assert_eq!(vcs.get_type_name(), "git");
        assert_eq!(vcs.get_repo_root().unwrap(), repo_root);
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_is_clean() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        // Fresh repo should be clean
        assert!(vcs.is_clean().unwrap());

        // Create a file - should make repo dirty
        fs::write(repo_root.join("test.txt"), "test content").unwrap();
        assert!(!vcs.is_clean().unwrap());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_current_branch() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        // Default branch should be "main"
        let branch = vcs.get_current_branch().unwrap();
        assert_eq!(branch, "main");
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_latest_commit() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        let commit_hash = vcs.get_latest_commit().unwrap();
        // Should be a valid 40-char hex string
        assert_eq!(commit_hash.len(), 40);
        assert!(commit_hash.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_tag_operations() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        // Tag should not exist initially
        assert!(!vcs.tag_exists("v1.0.0").unwrap());

        // Create tag
        vcs.create_tag("v1.0.0", "Release 1.0.0").unwrap();

        // Tag should exist now
        assert!(vcs.tag_exists("v1.0.0").unwrap());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_metadata() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        let metadata = vcs.get_metadata().unwrap();
        assert_eq!(metadata.vcs_type, "git");
        assert_eq!(metadata.branch, Some("main".to_string()));
        assert_eq!(metadata.commit_hash.len(), 40);
        assert_eq!(metadata.author_name, Some("Test User".to_string()));
        assert_eq!(metadata.author_email, Some("test@example.com".to_string()));
        assert!(metadata.timestamp.is_some());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_changelog() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs = git::GitVCS::new(repo_root.clone()).unwrap();

        // Get changelog
        let changelog = vcs.get_changelog(None, None).unwrap();
        assert!(changelog.contains("Changelog from"));
        assert!(changelog.contains("Initial commit"));
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_not_a_repo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = git::GitVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_tracking_branch_none() {
        // Create a repo with no upstream branch
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        // No upstream set, should return None
        let result = vcs.get_tracking_branch(&repo, Some("main"));
        assert!(result.is_none());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_tracking_branch_no_branch() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        // No branch name provided, should return None
        let result = vcs.get_tracking_branch(&repo, None);
        assert!(result.is_none());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_push_url_no_remote() {
        // Create a repo with no remote
        let temp_dir = tempfile::TempDir::new().unwrap();
        let repo = git2::Repository::init(temp_dir.path()).unwrap();

        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        // No remote set, should return None
        let result = vcs.get_push_url(&repo, "origin");
        assert!(result.is_none());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_metadata_has_new_fields() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        let metadata = vcs.get_metadata().unwrap();
        // The new fields should be present (may be None if no remote/upstream configured)
        // Just verify the structure is correct and doesn't panic
        assert_eq!(metadata.vcs_type, "git");
        assert!(!metadata.commit_hash.is_empty());
        // tracking_branch and push_url may be None (no upstream/remote configured)
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_detect_vcs_git() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();

        let vcs_opt = detect_vcs(&repo_root);
        assert!(vcs_opt.is_some());
        assert_eq!(vcs_opt.unwrap().get_type_name(), "git");
    }

    // --- MercurialVCS tests ---

    #[test]
    fn test_mercurial_vcs_not_a_repo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = hg::MercurialVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_mercurial_vcs_creation_placeholder() {
        // Create a temp dir with .hg directory (simulates hg repo)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let result = hg::MercurialVCS::new(temp_dir.path().to_path_buf());
        // Should succeed (creation only checks for .hg directory)
        assert!(result.is_ok());
    }

    #[test]
    fn test_mercurial_vcs_type_name() {
        // Create a temp dir with .hg directory (simulates hg repo)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let vcs = hg::MercurialVCS::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(vcs.get_type_name(), "hg");
    }

    #[test]
    fn test_mercurial_vcs_validate_repo_state() {
        // Create a temp dir with .hg directory (simulates hg repo)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let vcs = hg::MercurialVCS::new(temp_dir.path().to_path_buf()).unwrap();
        // validate_repo_state will check if repo is clean
        // This test just ensures the method exists and returns Result
        let _result: Result<(), RezCoreError> = vcs.validate_repo_state();
    }

    #[test]
    fn test_mercurial_vcs_is_releasable_branch() {
        // Create a temp dir with .hg directory (simulates hg repo)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let vcs = hg::MercurialVCS::new(temp_dir.path().to_path_buf()).unwrap();
        // This test just ensures the method exists and returns Result
        let _result: Result<Option<bool>, RezCoreError> = vcs.is_releasable_branch();
    }

    #[test]
    fn test_detect_vcs_mercurial() {
        // Create a temp dir with .hg directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let vcs_opt = detect_vcs(temp_dir.path());
        assert!(vcs_opt.is_some());
        assert_eq!(vcs_opt.unwrap().get_type_name(), "hg");
    }

    // --- SvnVCS tests ---

    #[test]
    fn test_svn_vcs_not_a_repo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = svn::SvnVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_svn_vcs_creation_placeholder() {
        // Create a temp dir with .svn directory (simulates svn working copy)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let result = svn::SvnVCS::new(temp_dir.path().to_path_buf());
        // Should succeed (creation only checks for .svn directory)
        assert!(result.is_ok());
    }

    #[test]
    fn test_svn_vcs_type_name() {
        // Create a temp dir with .svn directory (simulates svn working copy)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let vcs = svn::SvnVCS::new(temp_dir.path().to_path_buf()).unwrap();
        assert_eq!(vcs.get_type_name(), "svn");
    }

    #[test]
    fn test_svn_vcs_validate_repo_state() {
        // Create a temp dir with .svn directory (simulates svn working copy)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let vcs = svn::SvnVCS::new(temp_dir.path().to_path_buf()).unwrap();
        // validate_repo_state will check if working copy is clean
        // This test just ensures the method exists and returns Result
        let _result: Result<(), RezCoreError> = vcs.validate_repo_state();
    }

    #[test]
    fn test_svn_vcs_is_releasable_branch() {
        // Create a temp dir with .svn directory (simulates svn working copy)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let vcs = svn::SvnVCS::new(temp_dir.path().to_path_buf()).unwrap();
        // This test just ensures the method exists and returns Result
        let _result: Result<Option<bool>, RezCoreError> = vcs.is_releasable_branch();
    }

    #[test]
    fn test_detect_vcs_svn() {
        // Create a temp dir with .svn directory
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let vcs_opt = detect_vcs(temp_dir.path());
        assert!(vcs_opt.is_some());
        assert_eq!(vcs_opt.unwrap().get_type_name(), "svn");
    }

    // --- Validate repo state and is_releasable_branch tests for all VCS ---

    #[test]
    fn test_stub_vcs_validate_repo_state() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));
        // StubVCS should always pass validation (default implementation checks is_clean, which returns true)
        assert!(vcs.validate_repo_state().is_ok());
    }

    #[test]
    fn test_stub_vcs_is_releasable_branch() {
        let vcs = StubVCS::new(PathBuf::from("/tmp/repo"));
        // Default implementation returns Ok(None)
        let result = vcs.is_releasable_branch().unwrap();
        assert_eq!(result, None);
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_validate_repo_state() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        // Fresh repo should pass validation
        assert!(vcs.validate_repo_state().is_ok());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_is_releasable_branch() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = git::GitVCS::new(repo_root).unwrap();

        // Default branch should be releasable
        let result = vcs.is_releasable_branch().unwrap();
        assert_eq!(result, Some(true));
    }
}
