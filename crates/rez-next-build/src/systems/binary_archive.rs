//! Built-in binary archive build plugin.
//!
//! This covers the common Rez pattern: obtain a prebuilt archive, verify it,
//! extract it, optionally strip a payload prefix, and copy the payload into the
//! package install root.

use crate::{BuildEnvironment, BuildRequest, BuildStep, BuildStepResult};
use flate2::read::GzDecoder;
use rez_next_common::RezCoreError;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tar::Archive;
use tokio::process::Child;
use tokio::sync::Mutex;

const ARTIFACT_ENV: &str = "REZ_BINARY_ARCHIVE_ARTIFACT";
const URL_ENV: &str = "REZ_BINARY_ARCHIVE_URL";
const SHA256_ENV: &str = "REZ_BINARY_ARCHIVE_SHA256";
const PAYLOAD_PREFIX_ENV: &str = "REZ_BINARY_ARCHIVE_PAYLOAD_PREFIX";

/// Native build system for prebuilt binary archives.
#[derive(Debug, Clone)]
pub struct BinaryArchiveBuildSystem;

impl BinaryArchiveBuildSystem {
    pub fn new() -> Self {
        Self
    }

    pub async fn configure(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(
            BuildStep::Configuring,
            "binary_archive plugin configured",
        ))
    }

    pub async fn compile(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(
            BuildStep::Compiling,
            "binary_archive uses prebuilt artifacts",
        ))
    }

    pub async fn test(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
        _child_process: Arc<Mutex<Option<Child>>>,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(BuildStep::Testing, "binary_archive tests skipped"))
    }

    pub async fn package(
        &self,
        _request: &BuildRequest,
        _environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        Ok(step_ok(
            BuildStep::Packaging,
            "binary_archive package step completed",
        ))
    }

    pub async fn install(
        &self,
        _request: &BuildRequest,
        environment: &BuildEnvironment,
    ) -> Result<BuildStepResult, RezCoreError> {
        match self.install_archive(environment).await {
            Ok(output) => Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: true,
                output,
                errors: String::new(),
                duration_ms: 0,
            }),
            Err(err) => Ok(BuildStepResult {
                step: BuildStep::Installing,
                success: false,
                output: String::new(),
                errors: err.to_string(),
                duration_ms: 0,
            }),
        }
    }

    async fn install_archive(
        &self,
        environment: &BuildEnvironment,
    ) -> Result<String, RezCoreError> {
        let env = environment.get_env_vars();
        let work_dir = environment.get_temp_dir().join("binary_archive");
        let extract_dir = work_dir.join("extract");
        let install_dir = environment.get_install_dir();

        tokio::fs::create_dir_all(&work_dir).await.map_err(|err| {
            RezCoreError::BuildError(format!("Failed to create archive work dir: {err}"))
        })?;
        tokio::fs::create_dir_all(&extract_dir)
            .await
            .map_err(|err| {
                RezCoreError::BuildError(format!("Failed to create archive extract dir: {err}"))
            })?;
        tokio::fs::create_dir_all(install_dir)
            .await
            .map_err(|err| {
                RezCoreError::BuildError(format!("Failed to create install dir: {err}"))
            })?;

        let artifact = self.resolve_artifact(env, &work_dir).await?;
        if let Some(expected_sha256) = env.get(SHA256_ENV).filter(|value| !value.is_empty()) {
            verify_sha256(&artifact, expected_sha256)?;
        }

        extract_archive(&artifact, &extract_dir)?;
        let payload = env
            .get(PAYLOAD_PREFIX_ENV)
            .filter(|value| !value.is_empty())
            .map_or(extract_dir.clone(), |prefix| extract_dir.join(prefix));

        if !payload.is_dir() {
            return Err(RezCoreError::BuildError(format!(
                "binary archive payload directory does not exist: {}",
                payload.display()
            )));
        }

        copy_tree_contents(&payload, install_dir)?;

        Ok(format!(
            "Installed binary archive {} to {}",
            artifact.display(),
            install_dir.display()
        ))
    }

    async fn resolve_artifact(
        &self,
        env: &std::collections::HashMap<String, String>,
        work_dir: &Path,
    ) -> Result<PathBuf, RezCoreError> {
        if let Some(path) = env.get(ARTIFACT_ENV).filter(|value| !value.is_empty()) {
            let artifact = PathBuf::from(path);
            if artifact.is_file() {
                return Ok(artifact);
            }
            return Err(RezCoreError::BuildError(format!(
                "binary archive artifact does not exist: {}",
                artifact.display()
            )));
        }

        let url = env
            .get(URL_ENV)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| {
                RezCoreError::BuildError(format!(
                    "binary_archive requires {ARTIFACT_ENV} or {URL_ENV}"
                ))
            })?;
        let filename = url
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or("artifact.bin");
        let destination = work_dir.join(filename);

        let response = reqwest::get(url).await.map_err(|err| {
            RezCoreError::BuildError(format!("Failed to download binary archive {url}: {err}"))
        })?;
        let bytes = response.bytes().await.map_err(|err| {
            RezCoreError::BuildError(format!("Failed to read binary archive response: {err}"))
        })?;
        tokio::fs::write(&destination, bytes).await.map_err(|err| {
            RezCoreError::BuildError(format!("Failed to write downloaded archive: {err}"))
        })?;

        Ok(destination)
    }
}

impl Default for BinaryArchiveBuildSystem {
    fn default() -> Self {
        Self::new()
    }
}

fn step_ok(step: BuildStep, output: &str) -> BuildStepResult {
    BuildStepResult {
        step,
        success: true,
        output: output.to_string(),
        errors: String::new(),
        duration_ms: 0,
    }
}

fn verify_sha256(path: &Path, expected_sha256: &str) -> Result<(), RezCoreError> {
    let mut file = fs::File::open(path).map_err(|err| {
        RezCoreError::BuildError(format!("Failed to open archive for sha256: {err}"))
    })?;
    let mut hasher = Sha256::new();
    let mut buffer = [0_u8; 1024 * 1024];
    loop {
        let bytes_read = file.read(&mut buffer).map_err(|err| {
            RezCoreError::BuildError(format!("Failed to read archive for sha256: {err}"))
        })?;
        if bytes_read == 0 {
            break;
        }
        hasher.update(&buffer[..bytes_read]);
    }
    let actual = hex::encode(hasher.finalize());
    if actual != expected_sha256 {
        return Err(RezCoreError::BuildError(format!(
            "binary archive sha256 mismatch: expected {expected_sha256}, got {actual}"
        )));
    }
    Ok(())
}

fn extract_archive(artifact: &Path, destination: &Path) -> Result<(), RezCoreError> {
    let name = artifact
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();

    if name.ends_with(".zip") {
        extract_zip(artifact, destination)
    } else if name.ends_with(".tar.gz") || name.ends_with(".tgz") {
        extract_tar_gz(artifact, destination)
    } else {
        Err(RezCoreError::BuildError(format!(
            "unsupported binary archive format: {}",
            artifact.display()
        )))
    }
}

fn extract_zip(artifact: &Path, destination: &Path) -> Result<(), RezCoreError> {
    let file = fs::File::open(artifact)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to open zip archive: {err}")))?;
    let mut archive = zip::ZipArchive::new(file)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to read zip archive: {err}")))?;

    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(|err| {
            RezCoreError::BuildError(format!("Failed to read zip entry {index}: {err}"))
        })?;
        let Some(enclosed) = entry.enclosed_name() else {
            continue;
        };
        let target = destination.join(enclosed);
        if entry.is_dir() {
            fs::create_dir_all(&target).map_err(|err| {
                RezCoreError::BuildError(format!("Failed to create zip dir: {err}"))
            })?;
        } else {
            if let Some(parent) = target.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    RezCoreError::BuildError(format!("Failed to create zip parent dir: {err}"))
                })?;
            }
            let mut out = fs::File::create(&target).map_err(|err| {
                RezCoreError::BuildError(format!("Failed to create zip output file: {err}"))
            })?;
            std::io::copy(&mut entry, &mut out).map_err(|err| {
                RezCoreError::BuildError(format!("Failed to extract zip entry: {err}"))
            })?;
        }
    }

    Ok(())
}

fn extract_tar_gz(artifact: &Path, destination: &Path) -> Result<(), RezCoreError> {
    let file = fs::File::open(artifact)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to open tar.gz archive: {err}")))?;
    let decoder = GzDecoder::new(file);
    let mut archive = Archive::new(decoder);
    archive
        .unpack(destination)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to unpack tar.gz archive: {err}")))
}

fn copy_tree_contents(source: &Path, destination: &Path) -> Result<(), RezCoreError> {
    fs::create_dir_all(destination)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to create directory: {err}")))?;
    for entry in fs::read_dir(source)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to read directory: {err}")))?
    {
        let entry = entry
            .map_err(|err| RezCoreError::BuildError(format!("Failed to read entry: {err}")))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            if destination_path.exists() {
                fs::remove_dir_all(&destination_path).map_err(|err| {
                    RezCoreError::BuildError(format!("Failed to remove directory: {err}"))
                })?;
            }
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            if let Some(parent) = destination_path.parent() {
                fs::create_dir_all(parent).map_err(|err| {
                    RezCoreError::BuildError(format!("Failed to create parent directory: {err}"))
                })?;
            }
            fs::copy(&source_path, &destination_path).map_err(|err| {
                RezCoreError::BuildError(format!("Failed to copy archive file: {err}"))
            })?;
        }
    }
    Ok(())
}

fn copy_dir_recursive(source: &Path, destination: &Path) -> Result<(), RezCoreError> {
    fs::create_dir_all(destination)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to create directory: {err}")))?;
    for entry in fs::read_dir(source)
        .map_err(|err| RezCoreError::BuildError(format!("Failed to read directory: {err}")))?
    {
        let entry = entry
            .map_err(|err| RezCoreError::BuildError(format!("Failed to read entry: {err}")))?;
        let source_path = entry.path();
        let destination_path = destination.join(entry.file_name());
        if source_path.is_dir() {
            copy_dir_recursive(&source_path, &destination_path)?;
        } else {
            fs::copy(&source_path, &destination_path).map_err(|err| {
                RezCoreError::BuildError(format!("Failed to copy archive file: {err}"))
            })?;
        }
    }
    Ok(())
}
