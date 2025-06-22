//! Network source handling for remote package building
//!
//! This module provides support for building packages from various network sources
//! including Git repositories, HTTP/HTTPS URLs, and other remote locations.

use rez_core_common::{RezCoreError, error::RezCoreResult};
use std::path::PathBuf;
use std::collections::HashMap;
use url::Url;
use serde::{Deserialize, Serialize};

/// Supported network source types
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum SourceType {
    /// Git repository (git://, https://.git, ssh://git@)
    Git,
    /// HTTP/HTTPS archive (zip, tar.gz, etc.)
    Http,
    /// SSH-based source
    Ssh,
    /// Local filesystem path
    Local,
}

/// Network source configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkSource {
    /// Source URL or path
    pub url: String,
    /// Source type
    pub source_type: SourceType,
    /// Optional branch/tag/commit for Git sources
    pub reference: Option<String>,
    /// Optional subdirectory within the source
    pub subdirectory: Option<String>,
    /// Authentication credentials
    pub auth: Option<SourceAuth>,
    /// Additional options
    pub options: HashMap<String, String>,
}

/// Authentication configuration for network sources
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SourceAuth {
    /// Username for authentication
    pub username: Option<String>,
    /// Password or token
    pub token: Option<String>,
    /// SSH key path
    pub ssh_key: Option<PathBuf>,
}

/// Source fetcher trait for different source types
#[async_trait::async_trait]
pub trait SourceFetcher {
    /// Fetch source to a local directory
    async fn fetch(&self, source: &NetworkSource, dest_dir: &PathBuf) -> RezCoreResult<PathBuf>;

    /// Check if this fetcher can handle the given source
    fn can_handle(&self, source: &NetworkSource) -> bool;

    /// Get the name of this fetcher
    fn name(&self) -> &'static str;
}

/// Git source fetcher
pub struct GitFetcher;

#[async_trait::async_trait]
impl SourceFetcher for GitFetcher {
    async fn fetch(&self, source: &NetworkSource, dest_dir: &PathBuf) -> RezCoreResult<PathBuf> {
        use tokio::process::Command;
        
        let clone_dir = dest_dir.join("source");
        
        // Ensure destination directory exists
        tokio::fs::create_dir_all(&clone_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create clone directory: {}", e)))?;
        
        // Build git clone command
        let mut cmd = Command::new("git");
        cmd.arg("clone");
        
        // Add depth for shallow clone if no specific reference
        if source.reference.is_none() {
            cmd.args(&["--depth", "1"]);
        }
        
        cmd.arg(&source.url);
        cmd.arg(&clone_dir);
        
        // Execute git clone
        let output = cmd.output().await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to execute git clone: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!("Git clone failed: {}", stderr)));
        }
        
        // Checkout specific reference if provided
        if let Some(ref reference) = source.reference {
            let mut checkout_cmd = Command::new("git");
            checkout_cmd.current_dir(&clone_dir);
            checkout_cmd.args(&["checkout", reference]);
            
            let checkout_output = checkout_cmd.output().await
                .map_err(|e| RezCoreError::BuildError(format!("Failed to execute git checkout: {}", e)))?;
            
            if !checkout_output.status.success() {
                let stderr = String::from_utf8_lossy(&checkout_output.stderr);
                return Err(RezCoreError::BuildError(format!("Git checkout failed: {}", stderr)));
            }
        }
        
        // Return subdirectory if specified
        if let Some(ref subdir) = source.subdirectory {
            Ok(clone_dir.join(subdir))
        } else {
            Ok(clone_dir)
        }
    }
    
    fn can_handle(&self, source: &NetworkSource) -> bool {
        source.source_type == SourceType::Git
    }
    
    fn name(&self) -> &'static str {
        "git"
    }
}

/// HTTP source fetcher for archives
pub struct HttpFetcher;

#[async_trait::async_trait]
impl SourceFetcher for HttpFetcher {
    async fn fetch(&self, source: &NetworkSource, dest_dir: &PathBuf) -> RezCoreResult<PathBuf> {
        use tokio::fs::File;
        use tokio::io::AsyncWriteExt;
        
        let download_dir = dest_dir.join("download");
        tokio::fs::create_dir_all(&download_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create download directory: {}", e)))?;
        
        // Download file
        let response = reqwest::get(&source.url).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to download: {}", e)))?;
        
        if !response.status().is_success() {
            return Err(RezCoreError::BuildError(format!("HTTP error: {}", response.status())));
        }
        
        // Determine filename from URL or Content-Disposition header
        let filename = self.extract_filename(&source.url, &response)?;
        let file_path = download_dir.join(&filename);
        
        // Write downloaded content to file
        let bytes = response.bytes().await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read response: {}", e)))?;
        
        let mut file = File::create(&file_path).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create file: {}", e)))?;
        
        file.write_all(&bytes).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to write file: {}", e)))?;
        
        // Extract archive if it's a known archive format
        let extract_dir = download_dir.join("extracted");
        self.extract_archive(&file_path, &extract_dir).await?;
        
        // Return subdirectory if specified
        if let Some(ref subdir) = source.subdirectory {
            Ok(extract_dir.join(subdir))
        } else {
            Ok(extract_dir)
        }
    }
    
    fn can_handle(&self, source: &NetworkSource) -> bool {
        source.source_type == SourceType::Http
    }
    
    fn name(&self) -> &'static str {
        "http"
    }
}

impl HttpFetcher {
    fn extract_filename(&self, url: &str, response: &reqwest::Response) -> RezCoreResult<String> {
        // Try to get filename from Content-Disposition header
        if let Some(content_disposition) = response.headers().get("content-disposition") {
            if let Ok(header_value) = content_disposition.to_str() {
                if let Some(filename) = self.parse_content_disposition(header_value) {
                    return Ok(filename);
                }
            }
        }
        
        // Fall back to extracting from URL
        if let Ok(parsed_url) = Url::parse(url) {
            if let Some(segments) = parsed_url.path_segments() {
                if let Some(last_segment) = segments.last() {
                    if !last_segment.is_empty() {
                        return Ok(last_segment.to_string());
                    }
                }
            }
        }
        
        // Default filename
        Ok("download".to_string())
    }
    
    fn parse_content_disposition(&self, header: &str) -> Option<String> {
        // Simple parser for Content-Disposition header
        for part in header.split(';') {
            let part = part.trim();
            if part.starts_with("filename=") {
                let filename = &part[9..];
                return Some(filename.trim_matches('"').to_string());
            }
        }
        None
    }
    
    async fn extract_archive(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> RezCoreResult<()> {
        tokio::fs::create_dir_all(extract_dir).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create extract directory: {}", e)))?;
        
        let extension = archive_path.extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("");
        
        match extension {
            "zip" => self.extract_zip(archive_path, extract_dir).await,
            "gz" | "tgz" => self.extract_tar_gz(archive_path, extract_dir).await,
            "tar" => self.extract_tar(archive_path, extract_dir).await,
            _ => {
                // If not a known archive format, just copy the file
                let dest_file = extract_dir.join(archive_path.file_name().unwrap());
                tokio::fs::copy(archive_path, dest_file).await
                    .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
                Ok(())
            }
        }
    }
    
    async fn extract_zip(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> RezCoreResult<()> {
        use tokio::process::Command;
        
        // Use system unzip command for simplicity
        let output = Command::new("unzip")
            .arg("-q")  // quiet
            .arg(archive_path)
            .arg("-d")
            .arg(extract_dir)
            .output()
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to execute unzip: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!("Unzip failed: {}", stderr)));
        }
        
        Ok(())
    }
    
    async fn extract_tar_gz(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> RezCoreResult<()> {
        use tokio::process::Command;
        
        let output = Command::new("tar")
            .arg("-xzf")
            .arg(archive_path)
            .arg("-C")
            .arg(extract_dir)
            .output()
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to execute tar: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!("Tar extraction failed: {}", stderr)));
        }
        
        Ok(())
    }
    
    async fn extract_tar(&self, archive_path: &PathBuf, extract_dir: &PathBuf) -> RezCoreResult<()> {
        use tokio::process::Command;
        
        let output = Command::new("tar")
            .arg("-xf")
            .arg(archive_path)
            .arg("-C")
            .arg(extract_dir)
            .output()
            .await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to execute tar: {}", e)))?;
        
        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(RezCoreError::BuildError(format!("Tar extraction failed: {}", stderr)));
        }
        
        Ok(())
    }
}

/// Local filesystem source fetcher
pub struct LocalFetcher;

#[async_trait::async_trait]
impl SourceFetcher for LocalFetcher {
    async fn fetch(&self, source: &NetworkSource, dest_dir: &PathBuf) -> RezCoreResult<PathBuf> {
        let source_path = PathBuf::from(&source.url);

        if !source_path.exists() {
            return Err(RezCoreError::BuildError(format!("Source path does not exist: {}", source.url)));
        }

        let copy_dir = dest_dir.join("source");

        if source_path.is_dir() {
            // Copy directory recursively
            self.copy_dir_recursive(&source_path, &copy_dir).await?;
        } else {
            // Copy single file
            tokio::fs::create_dir_all(&copy_dir).await
                .map_err(|e| RezCoreError::BuildError(format!("Failed to create directory: {}", e)))?;

            let dest_file = copy_dir.join(source_path.file_name().unwrap());
            tokio::fs::copy(&source_path, dest_file).await
                .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
        }

        // Return subdirectory if specified
        if let Some(ref subdir) = source.subdirectory {
            Ok(copy_dir.join(subdir))
        } else {
            Ok(copy_dir)
        }
    }

    fn can_handle(&self, source: &NetworkSource) -> bool {
        source.source_type == SourceType::Local
    }

    fn name(&self) -> &'static str {
        "local"
    }
}

impl LocalFetcher {
    async fn copy_dir_recursive(&self, src: &PathBuf, dest: &PathBuf) -> RezCoreResult<()> {
        tokio::fs::create_dir_all(dest).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to create directory: {}", e)))?;

        let mut entries = tokio::fs::read_dir(src).await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory: {}", e)))?;

        while let Some(entry) = entries.next_entry().await
            .map_err(|e| RezCoreError::BuildError(format!("Failed to read directory entry: {}", e)))? {

            let src_path = entry.path();
            let dest_path = dest.join(entry.file_name());

            if src_path.is_dir() {
                Box::pin(self.copy_dir_recursive(&src_path, &dest_path)).await?;
            } else {
                tokio::fs::copy(&src_path, &dest_path).await
                    .map_err(|e| RezCoreError::BuildError(format!("Failed to copy file: {}", e)))?;
            }
        }

        Ok(())
    }
}

/// Source manager for handling different source types
pub struct SourceManager {
    fetchers: Vec<Box<dyn SourceFetcher + Send + Sync>>,
}

impl SourceManager {
    /// Create a new source manager with default fetchers
    pub fn new() -> Self {
        let mut manager = Self {
            fetchers: Vec::new(),
        };

        // Register default fetchers
        manager.register_fetcher(Box::new(GitFetcher));
        manager.register_fetcher(Box::new(HttpFetcher));
        manager.register_fetcher(Box::new(LocalFetcher));

        manager
    }

    /// Register a new source fetcher
    pub fn register_fetcher(&mut self, fetcher: Box<dyn SourceFetcher + Send + Sync>) {
        self.fetchers.push(fetcher);
    }

    /// Parse a source URL and determine its type
    pub fn parse_source(&self, source_url: &str) -> RezCoreResult<NetworkSource> {
        let url = source_url.trim();

        // Determine source type based on URL pattern
        let (source_type, cleaned_url, reference) = if url.starts_with("git://") ||
                                                        url.starts_with("git@") ||
                                                        url.ends_with(".git") ||
                                                        url.contains("github.com") ||
                                                        url.contains("gitlab.com") ||
                                                        url.contains("bitbucket.org") {
            self.parse_git_url(url)?
        } else if url.starts_with("http://") || url.starts_with("https://") {
            // Check if it's a Git repository or HTTP archive
            if url.ends_with(".git") || url.contains("/archive/") || url.contains("/releases/download/") {
                if url.ends_with(".git") {
                    (SourceType::Git, url.to_string(), None)
                } else {
                    (SourceType::Http, url.to_string(), None)
                }
            } else {
                (SourceType::Http, url.to_string(), None)
            }
        } else if url.starts_with("ssh://") {
            (SourceType::Git, url.to_string(), None)
        } else {
            // Assume local path
            (SourceType::Local, url.to_string(), None)
        };

        Ok(NetworkSource {
            url: cleaned_url,
            source_type,
            reference,
            subdirectory: None,
            auth: None,
            options: HashMap::new(),
        })
    }

    /// Parse Git URL and extract reference if present
    fn parse_git_url(&self, url: &str) -> RezCoreResult<(SourceType, String, Option<String>)> {
        // Handle GitHub-style URLs with branch/tag specification
        // Examples:
        // - https://github.com/user/repo@branch
        // - https://github.com/user/repo@v1.0.0
        // - git@github.com:user/repo@branch

        if let Some(at_pos) = url.rfind('@') {
            // Check if @ is part of the domain (like git@github.com) or a reference separator
            let before_at = &url[..at_pos];
            let after_at = &url[at_pos + 1..];

            // If there's a colon before the @, it's likely git@host:repo format
            if before_at.contains(':') && !before_at.starts_with("http") {
                // This is git@host:repo@ref format
                if let Some(second_at) = before_at.rfind('@') {
                    let git_url = &url[..second_at];
                    let reference = after_at;
                    return Ok((SourceType::Git, git_url.to_string(), Some(reference.to_string())));
                }
            } else if before_at.starts_with("http") {
                // This is https://host/repo@ref format
                let git_url = before_at;
                let reference = after_at;
                return Ok((SourceType::Git, git_url.to_string(), Some(reference.to_string())));
            }
        }

        // No reference specified
        Ok((SourceType::Git, url.to_string(), None))
    }

    /// Fetch source to a temporary directory
    pub async fn fetch_source(&self, source: &NetworkSource, temp_dir: &PathBuf) -> RezCoreResult<PathBuf> {
        // Find appropriate fetcher
        let fetcher = self.fetchers.iter()
            .find(|f| f.can_handle(source))
            .ok_or_else(|| RezCoreError::BuildError(
                format!("No fetcher available for source type: {:?}", source.source_type)
            ))?;

        // Fetch the source
        fetcher.fetch(source, temp_dir).await
    }
}

impl Default for SourceManager {
    fn default() -> Self {
        Self::new()
    }
}
