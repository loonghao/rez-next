//! Mercurial VCS implementation
//!
//! This module provides the `MercurialVCS` implementation of the `ReleaseVCS` trait.
//! It uses the `hg` command-line tool.

use rez_next_common::RezCoreError;
use std::collections::HashMap;
use std::path::PathBuf;

/// Parse timestamp from hgdate format: "(<unix_timestamp>, <offset>)"
///
/// # Arguments
/// * `date_line` - A string in hgdate format, e.g., "(1706745600, 0)"
///
/// # Returns
/// * `Some(i64)` - the Unix timestamp
/// * `None` - if parsing fails
fn parse_hg_timestamp(date_line: &str) -> Option<i64> {
    let date_line = date_line.trim();
    // hgdate format: "(<timestamp>, <offset>)"
    if let Some(start) = date_line.find('(') {
        if let Some(end) = date_line.find(',') {
            let ts_str = &date_line[start + 1..end];
            if let Ok(ts) = ts_str.trim().parse::<i64>() {
                return Some(ts);
            }
        }
    }
    None
}

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
            .map_err(|e| {
                RezCoreError::BuildError(format!(
                    "MercurialVCS: failed to run hg command '{:?}' in repository at '{}': {}",
                    args,
                    self.repo_root.display(),
                    e
                ))
            })?;

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

impl super::ReleaseVCS for MercurialVCS {
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
        tracing::info!(
            "MercurialVCS: created tag '{}' with message '{}'",
            tag,
            message
        );
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
        let changelog =
            self.run_hg(&["log", "-r", &revspec, "--template", "{node|short} {desc}\n"])?;

        Ok(format!("Changelog from {} to {}:\n{}", from, to, changelog))
    }

    fn get_metadata(&self) -> Result<super::VCSMetadata, RezCoreError> {
        // Get commit hash
        let commit_hash = self.get_latest_commit()?;

        // Get branch
        let branch = Some(self.get_current_branch()?);

        // Get commit info with timestamp
        // Template: author, desc, date|hgdate (tuple of (unix_timestamp, offset))
        let info = self.run_hg(&[
            "log",
            "-r",
            ".",
            "--template",
            "{author}\n{desc}\n{date|hgdate}",
        ])?;

        let lines: Vec<&str> = info.lines().collect();
        let author_name = lines.first().map(|s| s.to_string());
        let commit_message = lines.get(1).map(|s| s.to_string());

        // Parse timestamp from hgdate format: "(<unix_timestamp>, <offset>)"
        let timestamp = if lines.len() >= 3 {
            parse_hg_timestamp(lines[2])
        } else {
            None
        };

        // Get push URL (default push location)
        let push_url = self.run_hg(&["paths", "default"]).ok();

        Ok(super::VCSMetadata {
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
            timestamp,
            extra: HashMap::new(),
        })
    }

    /// Get the current revision as a VCSRevision object.
    ///
    /// For Mercurial, the revision is identified by the changeset hash (40-char hex).
    /// We also include the revision number (integer) in metadata.
    fn get_current_revision(&self) -> Result<super::VCSRevision, RezCoreError> {
        // Get changeset hash (40-char hex)
        let revision_id = self.run_hg(&["identify", "--id"])?;
        let revision_id = revision_id.trim();

        // Get revision number
        let rev_num = self.run_hg(&["identify", "--num"]).ok();

        // Get branch name
        let branch = self.get_current_branch().ok();

        // Get tags (if any)
        let tags = self.run_hg(&["identify", "--tags"]).ok();
        let tags = tags.and_then(|t| {
            let t = t.trim();
            if t.is_empty() || t == "tip" {
                None
            } else {
                Some(t.to_string())
            }
        });

        // Build metadata
        let mut metadata = HashMap::new();
        if let Some(ref rev) = rev_num {
            metadata.insert("rev".to_string(), rev.trim().to_string());
        }
        if let Some(ref branch_name) = branch {
            metadata.insert("branch".to_string(), branch_name.clone());
        }
        if let Some(ref tag_list) = tags {
            metadata.insert("tags".to_string(), tag_list.clone());
        }

        // Build data JSON
        let data = serde_json::json!({
            "revision_id": revision_id,
            "rev": rev_num.as_deref().map(|s| s.trim()),
            "branch": branch,
            "tags": tags,
        });

        let mut revision = super::VCSRevision::with_data(
            "hg",
            revision_id,
            data,
        );
        revision.metadata = metadata;

        Ok(revision)
    }

    /// Export the repository at the given revision to the given path.
    ///
    /// Uses `hg archive -r <rev> <path>` to export.
    fn export(&self, revision: &super::VCSRevision, path: &std::path::Path) -> Result<(), RezCoreError> {
        use std::fs;

        // Validate target path
        if path.exists() {
            return Err(RezCoreError::BuildError(
                format!("Export path '{}' already exists", path.display())
            ));
        }

        // Create target directory
        fs::create_dir_all(path).map_err(|e| {
            RezCoreError::BuildError(format!(
                "Failed to create export directory '{}': {}",
                path.display(),
                e
            ))
        })?;

        // Use `hg archive` to export
        let revision_spec = &revision.revision_id;
        let output = std::process::Command::new("hg")
            .args(["archive", "-r", revision_spec, &path.to_string_lossy()])
            .current_dir(&self.repo_root)
            .output()
            .map_err(|e| {
                RezCoreError::BuildError(format!(
                    "Failed to run `hg archive` for repository at '{}': {}",
                    self.repo_root.display(),
                    e
                ))
            })?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!(
                "Failed to export Mercurial repository at '{}' to '{}': {}",
                self.repo_root.display(),
                path.display(),
                stderr
            )));
        }

        tracing::info!(
            "MercurialVCS: exported revision '{}' to '{}'",
            revision_spec,
            path.display()
        );

        Ok(())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hg_timestamp_valid() {
        // Standard hgdate format: "(timestamp, offset)"
        let result = parse_hg_timestamp("(1706745600, 0)");
        assert_eq!(result, Some(1706745600));
    }

    #[test]
    fn test_parse_hg_timestamp_with_offset() {
        // hgdate format with offset (e.g., +3600 for UTC+1)
        let result = parse_hg_timestamp("(1706745600, 3600)");
        assert_eq!(result, Some(1706745600));
    }

    #[test]
    fn test_parse_hg_timestamp_negative_offset() {
        // hgdate format with negative offset (UTC-1)
        let result = parse_hg_timestamp("(1706745600, -3600)");
        assert_eq!(result, Some(1706745600));
    }

    #[test]
    fn test_parse_hg_timestamp_invalid_format() {
        // Invalid format: no parenthesis
        let result = parse_hg_timestamp("1706745600");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hg_timestamp_empty_string() {
        // Empty string
        let result = parse_hg_timestamp("");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hg_timestamp_malformed_number() {
        // Malformed number
        let result = parse_hg_timestamp("(abc, 0)");
        assert_eq!(result, None);
    }

    #[test]
    fn test_parse_hg_timestamp_real_world_example() {
        // Real-world example from hg log --template "{date|hgdate}"
        // Output: (1706745600, 0) for 2024-02-01 00:00:00 UTC
        let result = parse_hg_timestamp("(1706745600, 0)");
        assert_eq!(result, Some(1706745600));
    }
}
