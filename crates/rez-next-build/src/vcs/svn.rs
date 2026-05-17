//! SVN VCS implementation
//!
//! This module provides the `SvnVCS` implementation of the `ReleaseVCS` trait.
//! It uses the `svn` command-line tool.

use rez_next_common::RezCoreError;
use std::collections::HashMap;
use std::path::PathBuf;

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
            .map_err(|e| {
                RezCoreError::BuildError(format!(
                    "SvnVCS: failed to run svn command '{:?}' in repository at '{}': {}",
                    args,
                    self.repo_root.display(),
                    e
                ))
            })?;

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

impl super::ReleaseVCS for SvnVCS {
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
        Ok(info
            .get("Relative URL")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string()))
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
            return Err(RezCoreError::BuildError(format!(
                "svn copy failed: {}",
                stderr
            )));
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
        let _repo_url = info
            .get("URL")
            .ok_or_else(|| RezCoreError::BuildError("Could not get repository URL".to_string()))?;

        let from = from_rev.unwrap_or("BASE");
        let to = to_rev.unwrap_or("HEAD");

        // `svn log -r <from>:<to>`
        let changelog = self.run_svn(&["log", "-r", &format!("{}:{}", from, to)])?;

        Ok(format!(
            "Changelog from {} to {}:\n{}\n",
            from, to, changelog
        ))
    }

    fn get_metadata(&self) -> Result<super::VCSMetadata, RezCoreError> {
        let info = self.get_svn_info()?;

        let commit_hash = info
            .get("Last Changed Rev")
            .cloned()
            .unwrap_or_else(|| "unknown".to_string());

        let branch = info.get("Relative URL").cloned();

        let repository_url = info.get("Repository Root").cloned();

        let commit_message = info.get("Last Changed Author").cloned();

        Ok(super::VCSMetadata {
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

    /// Get the current revision as a VCSRevision object.
    ///
    /// For SVN, the revision is identified by the revision number (integer).
    fn get_current_revision(&self) -> Result<super::VCSRevision, RezCoreError> {
        // Get last changed revision
        let revision_id = self.run_svn(&["info", "--show-item", "last-changed-revision"])?;
        let revision_id = revision_id.trim();

        // Get URL (repository URL)
        let url = self.run_svn(&["info", "--show-item", "url"]).ok();

        // Get relative URL (branch/tag path)
        let relative_url = self.get_current_branch().ok();

        // Build metadata
        let mut metadata = HashMap::new();
        if let Some(ref url) = url {
            metadata.insert("url".to_string(), url.clone());
        }
        if let Some(ref rel_url) = relative_url {
            metadata.insert("relative_url".to_string(), rel_url.clone());
        }

        // Get last changed author
        let author = self
            .run_svn(&["info", "--show-item", "last-changed-author"])
            .ok();
        if let Some(ref a) = author {
            metadata.insert("author".to_string(), a.clone());
        }

        // Build data JSON
        let data = serde_json::json!({
            "revision_id": revision_id,
            "url": url,
            "relative_url": relative_url,
            "author": author,
        });

        let mut revision = super::VCSRevision::with_data("svn", revision_id, data);
        revision.metadata = metadata;

        Ok(revision)
    }

    /// Export the repository at the given revision to the given path.
    ///
    /// Uses `svn export -r <rev> <url> <path>`.
    fn export(
        &self,
        revision: &super::VCSRevision,
        path: &std::path::Path,
    ) -> Result<(), RezCoreError> {
        // Validate target path
        if path.exists() {
            return Err(RezCoreError::BuildError(format!(
                "Export path '{}' already exists",
                path.display()
            )));
        }

        // Ensure parent directory exists (required by rez interface)
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                return Err(RezCoreError::BuildError(format!(
                    "Parent directory '{}' does not exist",
                    parent.display()
                )));
            }
        }

        // Get the repository URL
        let info = self.get_svn_info()?;
        let repo_url = info.get("Repository Root").ok_or_else(|| {
            RezCoreError::BuildError("Could not get repository root URL".to_string())
        })?;

        // Use `svn export -r <rev> <url> <path>`
        let revision_spec = format!("-r{}", revision.revision_id);
        let output = std::process::Command::new("svn")
            .args(["export", &revision_spec, repo_url, &path.to_string_lossy()])
            .output()
            .map_err(|e| {
                RezCoreError::BuildError(format!(
                    "Failed to run `svn export` for repository at '{}': {}",
                    self.repo_root.display(),
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!(
                "Failed to export SVN repository at '{}' to '{}': {}",
                self.repo_root.display(),
                path.display(),
                stderr
            )));
        }

        tracing::info!(
            "SvnVCS: exported revision '{}' to '{}'",
            revision.revision_id,
            path.display()
        );

        Ok(())
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
        Ok(Some(
            relative_url.contains("/trunk") || relative_url.contains("/branches/release"),
        ))
    }
}
