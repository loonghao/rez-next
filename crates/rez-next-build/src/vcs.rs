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
    /// - No pending changes or conflicts
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
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to open repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

        let mut status_opts = git2::StatusOptions::new();
        status_opts.include_untracked(true);
        status_opts.include_ignored(false);

        let statuses = repo.statuses(Some(&mut status_opts))
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to get status for repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

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
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to peel to commit for repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;
        Ok(format!("detached-{}", &commit.id().to_string()[..8]))
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to open repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get HEAD: {}", e)))?;

        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to get commit: {}", e)))?;

        Ok(commit.id().to_string())
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to open repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

        // Check if tag reference exists
        let tag_ref_name = format!("refs/tags/{}", tag);
        let result = match repo.find_reference(&tag_ref_name) {
            Ok(_) => Ok(true),
            Err(e) if e.code() == git2::ErrorCode::NotFound => Ok(false),
            Err(e) => Err(RezCoreError::BuildError(format!(
                "GitVCS: failed to check tag '{}' in repository at '{}': {}",
                tag,
                self.repo_root.display(),
                e
            ))),
        };
        result
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to open repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

        // Get the current HEAD commit
        let head = repo.head()
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to get HEAD for tag '{}' in repository at '{}': {}",
                tag,
                self.repo_root.display(),
                e
            )))?;
        let commit = head.peel_to_commit()
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to peel to commit for tag '{}' in repository at '{}': {}",
                tag,
                self.repo_root.display(),
                e
            )))?;

        // Create signature for tag
        let sig = git2::Signature::now("Rez Next Build", "rez-next@build")
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to create signature for tag '{}': {}",
                tag,
                e
            )))?;

        // Create an annotated tag (force = false)
        let oid = commit.id();
        let obj = repo.find_object(oid, Some(git2::ObjectType::Commit))
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to find commit object for tag '{}' in repository at '{}': {}",
                tag,
                self.repo_root.display(),
                e
            )))?;

        repo.tag(tag, &obj, &sig, message, false)
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to create tag '{}' in repository at '{}': {}",
                tag,
                self.repo_root.display(),
                e
            )))?;

        tracing::info!("GitVCS: created tag '{}' with message '{}'", tag, message);
        Ok(())
    }

    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        let repo = git2::Repository::open(&self.repo_root)
            .map_err(|e| RezCoreError::BuildError(format!(
                "GitVCS: failed to open repository at '{}': {}",
                self.repo_root.display(),
                e
            )))?;

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

        // Try to get remote URL (simplified - just get the first remote's URL)
                        let repository_url = {
                            if let Ok(remotes) = repo.remotes() {
                                if !remotes.is_empty() {
                                    let remote_name = remotes.get(0).unwrap_or("origin");
                                    if let Ok(remote) = repo.find_remote(remote_name) {
                                        remote.url().map(|s| s.to_string())
                                    } else {
                                        None
                                    }
                                } else {
                                    None
                                }
                            } else {
                                None
                            }
                        };

        // Clone repository_url before moving it into the struct
        let repository_url_clone = repository_url.clone();

        Ok(VCSMetadata {
            vcs_type: "git".to_string(),
            repository_url,
            branch,
            tracking_branch: None,  // TODO: implement properly
            fetch_url: repository_url_clone,
            push_url: None, // TODO: get pushurl properly
            commit_hash: commit.id().to_string(),
            commit_message,
            author_name,
            author_email,
            timestamp,
            extra: HashMap::new(),
        })
    }

    fn is_releasable_branch(&self) -> Result<Option<bool>, RezCoreError> {
        let branch = self.get_current_branch()?;
        // In Git, "main" and "master" are the main releasable branches
        // Also allow branches starting with "release/"
        Ok(Some(branch == "main" || branch == "master" || branch.starts_with("release/")))
    }
}

/// Mercurial VCS implementation (uses `hg` command-line)
#[derive(Debug)]
pub struct MercurialVCS {
    /// Repository root path
    repo_root: PathBuf,
}

impl MercurialVCS {
    /// Create a new MercurialVCS
    pub fn new(repo_root: PathBuf) -> Result<Self, RezCoreError> {
        // Verify this is a mercurial repository
        if !repo_root.join(".hg").exists() {
            return Err(RezCoreError::BuildError(
                "Not a mercurial repository".to_string(),
            ));
        }

        Ok(Self { repo_root })
    }

    /// Run an hg command and return stdout
    fn run_hg(&self, args: &[&str]) -> Result<String, RezCoreError> {
        let output = std::process::Command::new("hg")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RezCoreError::BuildError(format!(
                "MercurialVCS: failed to run hg command '{:?}' in repository at '{}': {}",
                args,
                self.repo_root.display(),
                e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!(
                "MercurialVCS: hg command '{:?}' failed in repository at '{}': {}",
                args,
                self.repo_root.display(),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }
}

impl ReleaseVCS for MercurialVCS {
    fn get_type_name(&self) -> &str {
        "hg"
    }

    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
        // `hg root` returns the repository root
        let root = self.run_hg(&["root"])?;
        Ok(PathBuf::from(root))
    }

    fn is_clean(&self) -> Result<bool, RezCoreError> {
        // `hg status --quiet` returns empty if clean
        let status = self.run_hg(&["status"])?;
        Ok(status.is_empty())
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        // `hg branch` returns current branch name
        self.run_hg(&["branch"])
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        // `hg log -r . -T "{node}"` returns current commit hash
        self.run_hg(&["log", "-r", ".", "-T", "{node}"])
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> {
        // `hg tags` lists all tags
        let tags = self.run_hg(&["tags"])?;
        Ok(tags.lines().any(|line| line.contains(tag)))
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        // `hg tag -m <message> <tag>`
        self.run_hg(&["tag", "-m", message, tag])?;
        tracing::info!("MercurialVCS: created tag '{}' with message '{}'", tag, message);
        Ok(())
    }

    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        let from = from_rev.unwrap_or(".");
        let to = to_rev.unwrap_or("tip");

        // `hg log -r <from>::<to> --template "{node|short} {desc}\n"`
        let revspec = format!("{}::{}", from, to);
        let changelog = self.run_hg(&["log", "-r", &revspec, "--template", "{node|short} {desc}\n"])?;

        Ok(format!("Changelog from {} to {}:\n{}", from, to, changelog))
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        // Get commit hash
        let commit_hash = self.get_latest_commit()?;

        // Get branch
        let branch = Some(self.get_current_branch()?);

        // Get commit info
        let info = self.run_hg(&["log", "-r", ".", "--template", "{author}\n{desc}"])?;
        let lines: Vec<&str> = info.lines().collect();
        let author_name = lines.first().map(|s| s.to_string());
        let commit_message = lines.get(1).map(|s| s.to_string());

        // Get push URL (default push location)
        let push_url = self.run_hg(&["paths", "default"]).ok();

        Ok(VCSMetadata {
            vcs_type: "hg".to_string(),
            repository_url: push_url.clone(),
            branch,
            tracking_branch: None,
            fetch_url: push_url.clone(),
            push_url,
            commit_hash,
            commit_message,
            author_name,
            author_email: None, // hg doesn't expose email separately by default
            timestamp: None,     // TODO: parse timestamp from hg log
            extra: HashMap::new(),
        })
    }

    fn validate_repo_state(&self) -> Result<(), RezCoreError> {
        // Check if repo is clean
        if !self.is_clean()? {
            return Err(RezCoreError::BuildError(
                "Mercurial repository is not clean".to_string(),
            ));
        }

        // Check for mq (Mercurial Queues) patches
        let patches = self.run_hg(&["qseries"]).ok();
        if let Some(ref series) = patches {
            if !series.is_empty() {
                return Err(RezCoreError::BuildError(
                    "Mercurial repository has active mq patches".to_string(),
                ));
            }
        }

        Ok(())
    }

    fn is_releasable_branch(&self) -> Result<Option<bool>, RezCoreError> {
        let branch = self.get_current_branch()?;
        // In Mercurial, "default" is the main branch
        // Releases typically happen from "default" or release-named branches
        Ok(Some(branch == "default" || branch.starts_with("release")))
    }
}

/// SVN VCS implementation (uses `svn` command-line)
#[derive(Debug)]
pub struct SvnVCS {
    /// Repository root path
    repo_root: PathBuf,
}

impl SvnVCS {
    /// Create a new SvnVCS
    pub fn new(repo_root: PathBuf) -> Result<Self, RezCoreError> {
        // Verify this is an SVN working copy
        if !repo_root.join(".svn").exists() {
            return Err(RezCoreError::BuildError(
                "Not an SVN working copy".to_string(),
            ));
        }

        Ok(Self { repo_root })
    }

    /// Run an svn command and return stdout
    fn run_svn(&self, args: &[&str]) -> Result<String, RezCoreError> {
        let output = std::process::Command::new("svn")
            .args(args)
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| RezCoreError::BuildError(format!(
                "SvnVCS: failed to run svn command '{:?}' in repository at '{}': {}",
                args,
                self.repo_root.display(),
                e
            )))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!(
                "SvnVCS: svn command '{:?}' failed in repository at '{}': {}",
                args,
                self.repo_root.display(),
                stderr
            )));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Get svn info as a HashMap
    fn get_svn_info(&self) -> Result<HashMap<String, String>, RezCoreError> {
        let info_str = self.run_svn(&["info"])?;
        let mut info = HashMap::new();

        for line in info_str.lines() {
            if let Some((key, value)) = line.split_once(": ") {
                info.insert(key.to_string(), value.to_string());
            }
        }

        Ok(info)
    }
}

impl ReleaseVCS for SvnVCS {
    fn get_type_name(&self) -> &str {
        "svn"
    }

    fn get_repo_root(&self) -> Result<PathBuf, RezCoreError> {
        // `svn info --show-item wc-root`
        let root = self.run_svn(&["info", "--show-item", "wc-root"])?;
        Ok(PathBuf::from(root))
    }

    fn is_clean(&self) -> Result<bool, RezCoreError> {
        // `svn status` returns empty if clean
        let status = self.run_svn(&["status"])?;
        Ok(status.is_empty())
    }

    fn get_current_branch(&self) -> Result<String, RezCoreError> {
        // SVN uses directories for branches, not branches in the same working copy
        // Return the relative path in the repository
        let info = self.get_svn_info()?;
        Ok(info.get("Relative URL").cloned().unwrap_or_else(|| "unknown".to_string()))
    }

    fn get_latest_commit(&self) -> Result<String, RezCoreError> {
        // `svn info --show-item last-changed-revision`
        self.run_svn(&["info", "--show-item", "last-changed-revision"])
    }

    fn tag_exists(&self, tag: &str) -> Result<bool, RezCoreError> {
        // Check if tags/<tag> exists in the repository
        let info = self.get_svn_info()?;
        let repo_url = info.get("Repository Root").ok_or_else(|| {
            RezCoreError::BuildError("Could not get repository root URL".to_string())
        })?;

        let tags_url = format!("{}/tags/{}", repo_url, tag);
        let result = self.run_svn(&["info", &tags_url]);

        Ok(result.is_ok())
    }

    fn create_tag(&self, tag: &str, message: &str) -> Result<(), RezCoreError> {
        let info = self.get_svn_info()?;
        let repo_url = info.get("Repository Root").ok_or_else(|| {
            RezCoreError::BuildError("Could not get repository root URL".to_string())
        })?;

        let trunk_url = format!("{}/trunk", repo_url);
        let tags_url = format!("{}/tags/{}", repo_url, tag);

        // `svn copy <trunk> <tags/tag> -m <message>`
        let output = std::process::Command::new("svn")
            .args(["copy", &trunk_url, &tags_url, "-m", message])
            .output()
            .map_err(|e| RezCoreError::BuildError(format!("Failed to run svn copy: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!("svn copy failed: {}", stderr)));
        }

        tracing::info!("SvnVCS: created tag '{}' with message '{}'", tag, message);
        Ok(())
    }

    fn get_changelog(
        &self,
        from_rev: Option<&str>,
        to_rev: Option<&str>,
    ) -> Result<String, RezCoreError> {
        let info = self.get_svn_info()?;
        // Validate that URL exists (but don't store it)
        let _repo_url = info.get("URL").ok_or_else(|| {
            RezCoreError::BuildError("Could not get repository URL".to_string())
        })?;

        let from = from_rev.unwrap_or("BASE");
        let to = to_rev.unwrap_or("HEAD");

        // `svn log -r <from>:<to>`
        let changelog = self.run_svn(&["log", "-r", &format!("{}:{}", from, to)])?;

        Ok(format!("Changelog from {} to {}:\n{}\n", from, to, changelog))
    }

    fn get_metadata(&self) -> Result<VCSMetadata, RezCoreError> {
        let info = self.get_svn_info()?;

        let commit_hash = info.get("Last Changed Rev")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let branch = info.get("Relative URL").cloned();

        let repository_url = info.get("Repository Root").cloned();

        let commit_message = info.get("Last Changed Author").cloned();

        Ok(VCSMetadata {
            vcs_type: "svn".to_string(),
            repository_url,
            branch,
            tracking_branch: None,
            fetch_url: None,
            push_url: None,
            commit_hash,
            commit_message,
            author_name: info.get("Last Changed Author").cloned(),
            author_email: None,
            timestamp: None,
            extra: HashMap::new(),
        })
    }

    fn validate_repo_state(&self) -> Result<(), RezCoreError> {
        // Check if working copy is clean
        if !self.is_clean()? {
            return Err(RezCoreError::BuildError(
                "SVN working copy is not clean".to_string(),
            ));
        }

        Ok(())
    }

    fn is_releasable_branch(&self) -> Result<Option<bool>, RezCoreError> {
        // In SVN, releases are typically from trunk or a release branch
        let relative_url = self.get_current_branch()?;
        Ok(Some(relative_url.contains("/trunk") || relative_url.contains("/branches/release")))
    }
}

/// Implement ReleaseVCS for Box<dyn ReleaseVCS + Send + Sync>
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
        return Some(Box::new(GitVCS::new(repo_path.to_path_buf()).ok()?));
        
        // Fall back to StubVCS if git feature is not enabled
        #[cfg(not(feature = "git"))]
        return Some(Box::new(StubVCS::new(repo_path.to_path_buf())));
    }

    // Check for Mercurial
    if repo_path.join(".hg").exists() {
        return Some(Box::new(MercurialVCS::new(repo_path.to_path_buf()).ok()?));
    }

    // Check for SVN
    if repo_path.join(".svn").exists() {
        return Some(Box::new(SvnVCS::new(repo_path.to_path_buf()).ok()?));
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
    use git2;

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

    // --- MercurialVCS tests ---

    #[test]
    fn test_mercurial_vcs_not_a_repo() {
        let temp_dir = tempfile::TempDir::new().unwrap();
        let result = MercurialVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_mercurial_vcs_creation_placeholder() {
        // Create a temp dir with .hg directory (simulates hg repo)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".hg")).unwrap();

        let result = MercurialVCS::new(temp_dir.path().to_path_buf());
        // Should succeed (creation only checks for .hg directory)
        assert!(result.is_ok());
    }

    #[test]
    fn test_mercurial_vcs_type_name() {
        let vcs = MercurialVCS { repo_root: PathBuf::from("/tmp/hg-repo") };
        assert_eq!(vcs.get_type_name(), "hg");
    }

    #[test]
    fn test_mercurial_vcs_validate_repo_state() {
        let vcs = MercurialVCS { repo_root: PathBuf::from("/tmp/hg-repo") };
        // validate_repo_state will fail because /tmp/hg-repo is not a real repo
        // This test just ensures the method exists and returns Result
        let _result: Result<(), RezCoreError> = vcs.validate_repo_state();
    }

    #[test]
    fn test_mercurial_vcs_is_releasable_branch() {
        let vcs = MercurialVCS { repo_root: PathBuf::from("/tmp/hg-repo") };
        // is_releasable_branch will fail because /tmp/hg-repo is not a real repo
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
        let result = SvnVCS::new(temp_dir.path().to_path_buf());
        assert!(result.is_err());
    }

    #[test]
    fn test_svn_vcs_creation_placeholder() {
        // Create a temp dir with .svn directory (simulates svn working copy)
        let temp_dir = tempfile::TempDir::new().unwrap();
        std::fs::create_dir(temp_dir.path().join(".svn")).unwrap();

        let result = SvnVCS::new(temp_dir.path().to_path_buf());
        // Should succeed (creation only checks for .svn directory)
        assert!(result.is_ok());
    }

    #[test]
    fn test_svn_vcs_type_name() {
        let vcs = SvnVCS { repo_root: PathBuf::from("/tmp/svn-repo") };
        assert_eq!(vcs.get_type_name(), "svn");
    }

    #[test]
    fn test_svn_vcs_validate_repo_state() {
        let vcs = SvnVCS { repo_root: PathBuf::from("/tmp/svn-repo") };
        // validate_repo_state will fail because /tmp/svn-repo is not a real repo
        // This test just ensures the method exists and returns Result
        let _result: Result<(), RezCoreError> = vcs.validate_repo_state();
    }

    #[test]
    fn test_svn_vcs_is_releasable_branch() {
        let vcs = SvnVCS { repo_root: PathBuf::from("/tmp/svn-repo") };
        // is_releasable_branch will fail because /tmp/svn-repo is not a real repo
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
        let vcs = GitVCS::new(repo_root).unwrap();

        // Fresh repo should pass validation
        assert!(vcs.validate_repo_state().is_ok());
    }

    #[cfg(feature = "git")]
    #[test]
    fn test_git_vcs_is_releasable_branch() {
        let (_temp_dir, repo) = create_temp_git_repo();
        let repo_root = repo.workdir().unwrap().to_path_buf();
        let vcs = GitVCS::new(repo_root).unwrap();

        // Default branch should be releasable
        let result = vcs.is_releasable_branch().unwrap();
        assert_eq!(result, Some(true));
    }
}
