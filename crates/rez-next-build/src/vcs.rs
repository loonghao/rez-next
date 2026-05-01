//! Version Control System integration for release management
//!
//! This module provides the `ReleaseVCS` trait and implementations for
//! different version control systems (Git, Mercurial, SVN, etc.)

use rez_next_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Trait for VCS integration in release workflow
///
/// This trait defines the interface for interacting with version control
/// systems during the package release process.
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
}

/// VCS metadata for release
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct VCSMetadata {
    /// VCS type (git, hg, svn, etc.)
    pub vcs_type: String,
    /// Repository URL
    pub repository_url: Option<String>,
    /// Current branch name
    pub branch: Option<String>,
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
    pub extra: HashMap<String, String>,
}

/// Stub VCS implementation for testing or when no VCS is available
#[derive(Debug, Default)]
pub struct StubVCS {
    /// Simulated repository root
    repo_root: PathBuf,
    /// Simulated metadata
    metadata: VCSMetadata,
}

impl StubVCS {
    /// Create a new StubVCS
    pub fn new(repo_root: PathBuf) -> Self {
        Self {
            repo_root,
            metadata: VCSMetadata {
                vcs_type: "stub".to_string(),
                commit_hash: "stub-commit-hash".to_string(),
                ..Default::default()
            },
        }
    }

    /// Create a new StubVCS with custom metadata
    pub fn with_metadata(repo_root: PathBuf, metadata: VCSMetadata) -> Self {
        Self {
            repo_root,
            metadata,
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
        // Stub VCS always reports clean
        Ok(true)
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        Ok("main".to_string())
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        Ok(self.metadata.commit_hash.clone())
    }

    fn tag_exists(&self, _tag: &str) -> Result<bool, RezCoreError> {
        // Stub VCS never has tags
        Ok(false)
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        // Stub VCS just logs the tag creation
        tracing::info!("StubVCS: would create tag '{}' with message '{}'", tag, message);
        Ok(())
    }

    fn get_changelog(
        &self,
        _from_rev: Option<&str>,
        _to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        Ok("Stub changelog: no VCS available".to_string())
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        Ok(self.metadata.clone())
    }
}

/// Git VCS implementation (requires `git` feature)
#[cfg(feature = "git")]
#[derive(Debug)]
pub struct GitVCS {
    /// Repository root path
    repo_root: PathBuf,
}

#[cfg(feature = "git")]
impl GitVCS {
    /// Create a new GitVCS
    pub fn new(repo_root: PathBuf) -> Result<Self, RezCoreError> {
        // Verify this is a git repository
        if !repo_root.join(".git").exists() {
            return Err(RezCoreError::BuildError(
                "Not a git repository".to_string(),
            ));
        }

        Ok(Self { repo_root })
    }
}

#[cfg(feature = "git")]
impl ReleaseVCS for GitVCS {
    fn get_type_name(&self) -> &str {
        "git"
    }

    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
        Ok(self.repo_root.clone())
    }

    fn is_clean(&self) -> Result<bool, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true);
        status_opts.include_ignored(false);

        let statuses = repo.statuses(Some(&mut status_opts))
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get git status: {}", e)))?;

        // Repository is clean if there are no status entries
        Ok(statuses.is_empty())
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get HEAD: {}", e)))?;

        // Check if HEAD is a branch reference
        if head.is_branch() {
            if let Some(branch_name) = head.shorthand() {
                return Ok(branch_name.to_string());
            }
        }

        // Detached HEAD state
        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to peel to commit: {}", e)))?;
        Ok(format!("detached-{}", &commit.id().to_string()[..8]))
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get HEAD: {}", e)))?;

        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get commit: {}", e)))?;

        Ok(commit.id().to_string())
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        // Check if tag reference exists
        let tag_ref_name = format!("refs/tags/{}", tag);
        let result = match repo.find_reference(&tag_ref_name) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(e) => Err(RezCoreError::BuildError(format!("Failed to check tag: {}", e))),
        };
        result
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        // Get the current HEAD commit
        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get HEAD: {}", e)))?;
        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get commit: {}", e)))?;

        // Create signature for tag
        let sig = git2::Signature::now("Rez Next Build", "rez-next@build")
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create signature: {}", e)))?;

        // Create an annotated tag (force = false)
        let oid = commit.id();
        let obj = repo.find_object(oid, Some(git2::ObjectType::Commit))
            .map_err(|e| RezCoreError::BuildError(format!("Failed to find commit object: {}", e)))?;

        repo.tag(tag, &obj, &sig, message, false)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create tag: {}", e)))?;

        tracing::info!("GitVCS: created tag '{}' with message '{}'", tag, message);
        Ok(())
    }

    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        let from = from_rev.unwrap_or("HEAD~10");
        let to = to_rev.unwrap_or("HEAD");

        // Resolve revisions to commits
        let to_obj = repo.revparse_single(to)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to parse 'to' revision: {}", e)))?;

        // Walk commits from to to from
        let mut revwalk = repo.revwalk()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create revwalk: {}", e)))?;
        revwalk.push(to_obj.id())
            .map_err(|e| RezCoreError::BuildError(format!("Failed to push to revwalk: {}", e)))?;

        // Try to hide from_rev, but don't fail if it doesn't exist
        if let Ok(from_obj) = repo.revparse_single(from) {
            if let Ok(from_commit) = from_obj.peel_to_commit() {
                let _ = revwalk.hide(from_commit.id());
            }
        }

        let mut changelog = String::new();
        changelog.push_str(&format!("Changelog from {} to {}:\n", from, to));

        for id in revwalk {
            let id = id.map_err(|e| RezCoreError::BuildError(format!("Failed to walk revisions: {}", e)))?;
            let commit = repo.find_commit(id)
                .map_err(|e| RezCoreError::BuildError(format!("Failed to find commit: {}", e)))?;

            let message = commit.message().unwrap_or("(no message)");
            let short_id = &id.to_string()[..8];
            changelog.push_str(&format!("  {} {}\n", short_id, message.lines().next().unwrap_or("")));
        }

        Ok(changelog)
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!("Failed to open git repository: {}", e)))?;

        // Get latest commit
        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get HEAD: {}", e)))?;
        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get commit: {}", e)))?;

        // Get branch name
        let branch = if head.is_branch() {
            head.shorthand().map(|s| s.to_string())
        } else {
            None
        };

        // Get author info
        let author = commit.author();
        let author_name = author.name().map(|s| s.to_string());
        let author_email = author.email().map(|s| s.to_string());

        // Get commit message
        let commit_message = commit.message().map(|s| s.to_string());

        // Get timestamp
        let timestamp = Some(commit.time().seconds());

        // Try to get remote URL
        let repository_url = repo.remotes()
            .ok()
            .and_then(|remotes| {
                if !remotes.is_empty() {
                    let remote_name = remotes.get(0)?;
                    repo.find_remote(remote_name)
                        .ok()
                        .and_then(|remote| remote.url().map(|s| s.to_string()))
                } else {
                    None
                }
            });

        Ok(VCSMetadata {
            vcs_type: "git".to_string(),
            repository_url,
            branch,
            commit_hash: commit.id().to_string(),
            commit_message,
            author_name,
            author_email,
            timestamp,
            extra: HashMap::new(),
        })
    }
}

/// Detect VCS type from repository path
pub fn detect_vcs(repo_path: &Path) -> Option<Box<dyn ReleaseVCS>> {
    // Check for Git
    if repo_path.join(".git").exists() {
        #[cfg(feature = "git")]
        return Some(Box::new(GitVCS::new(repo_path.to_path_buf()).ok()?));
        
        // Fall back to StubVCS if git feature is not enabled
        #[cfg(not(feature = "git"))]
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
    }

    // Check for Mercurial
    if repo_path.join(".hg").exists() {
        // TODO: Implement MercurialVCS
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
    }

    // Check for SVN
    if repo_path.join(".svn").exists() {
        // TODO: Implement SvnVCS
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
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
        repo_config.set_str("user.email", "test@example.com").unwrap();
        
        // Create an initial commit
        let sig = git2::Signature::now("Test User", "test@example.com").unwrap();
        let tree_id = {
            let mut index = repo.index().unwrap();
            index.write_tree().unwrap()
        };
        
        // Create commit in a separate scope so tree is dropped before we return repo
        {
            let tree = repo.find_tree(tree_id).unwrap();
            repo.commit(Some("HEAD"), &sig, &sig, "Initial commit", &tree, &[]).unwrap();
        }
        
        (temp_dir, repo)
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_creation() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        assert_eq!(vcs.get_type_name(), "git");
        assert_eq!(vcs.get_repo_root().unwrap(), repo_root);
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_is_clean() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
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
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
        // Default branch should be "main"
        let branch = vcs.get_current_branch().unwrap();
        assert_eq!(branch, "main");
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_get_latest_commit() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
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
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
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
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
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
        
        let vcs = GitVCS::new(repo_root.clone()).unwrap();
        
        // Get changelog
        let changelog = vcs.get_changelog(None, None).unwrap();
        assert!(changelog.contains("Changelog from"));
        assert!(changelog.contains("Initial commit"));
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_not_a_repo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = GitVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
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
}
