//! Canonical repository discovery feeding the core package and solver APIs.

use std::collections::HashMap;
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageSerializer};
use rez_next_repository::simple_repository::PackageRepository;

/// Bridge canonical package directories into the core solver repository contract.
pub(crate) struct RuntimeRepository {
    root: PathBuf,
    name: String,
    packages: HashMap<String, Vec<Arc<Package>>>,
    errors: HashMap<String, String>,
}

impl RuntimeRepository {
    pub(crate) async fn new(root: &Path, name: String) -> Result<Self, RezCoreError> {
        // Never recurse into an application installation. A family descriptor
        // ends discovery there; otherwise inspect only its version directories.
        let mut descriptors = Vec::new();
        for family in Self::child_directories(root)? {
            if let Some(descriptor) = Self::preferred_descriptor(&family)? {
                descriptors.push(descriptor);
                continue;
            }
            for version in Self::child_directories(&family)? {
                if let Some(descriptor) = Self::preferred_descriptor(&version)? {
                    descriptors.push(descriptor);
                }
            }
        }
        let mut packages: HashMap<String, Vec<Arc<Package>>> = HashMap::new();
        let mut errors = HashMap::new();
        for descriptor in descriptors {
            match Self::load_descriptor(&descriptor) {
                Ok(mut package) => {
                    package.filepath = Some(descriptor.to_string_lossy().into_owned());
                    packages
                        .entry(package.name.clone())
                        .or_default()
                        .push(Arc::new(package));
                }
                Err(error) => {
                    if let Some(family) = descriptor
                        .strip_prefix(root)
                        .ok()
                        .and_then(|relative| relative.components().next())
                    {
                        errors.insert(
                            family.as_os_str().to_string_lossy().into_owned(),
                            error.to_string(),
                        );
                    }
                }
            }
        }
        Ok(Self {
            root: root.to_path_buf(),
            name,
            packages,
            errors,
        })
    }

    fn load_descriptor(path: &Path) -> Result<Package, RezCoreError> {
        if path
            .extension()
            .is_some_and(|extension| extension == "yaml" || extension == "yml")
        {
            // The legacy 0.3.9 YAML loader drops commands. The public Package
            // deserializer preserves the complete core model, including Rex.
            let file = File::open(path).map_err(|error| {
                RezCoreError::PackageParse(format!("Failed to read {}: {error}", path.display()))
            })?;
            let package: Package = serde_yaml::from_reader(file).map_err(|error| {
                RezCoreError::PackageParse(format!("Failed to parse {}: {error}", path.display()))
            })?;
            package.validate()?;
            Ok(package)
        } else {
            PackageSerializer::load_from_file(path)
        }
    }

    fn child_directories(directory: &Path) -> Result<Vec<PathBuf>, RezCoreError> {
        let io_error = |error| {
            RezCoreError::Repository(format!(
                "Failed to read directory {}: {error}",
                directory.display()
            ))
        };
        let mut directories = Vec::new();
        for entry in fs::read_dir(directory).map_err(io_error)? {
            let entry = entry.map_err(io_error)?;
            if entry.file_type().map_err(io_error)?.is_dir() {
                directories.push(entry.path());
            }
        }
        directories.sort();
        Ok(directories)
    }

    fn preferred_descriptor(directory: &Path) -> Result<Option<PathBuf>, RezCoreError> {
        for filename in ["package.py", "package.yaml", "package.yml"] {
            let path = directory.join(filename);
            match fs::metadata(&path) {
                Ok(metadata) if metadata.is_file() => return Ok(Some(path)),
                Ok(_) => {}
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
                Err(error) => {
                    return Err(RezCoreError::Repository(format!(
                        "Failed to inspect descriptor {}: {error}",
                        path.display()
                    )));
                }
            }
        }
        Ok(None)
    }
}

#[async_trait]
impl PackageRepository for RuntimeRepository {
    async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        if let Some(error) = self.errors.get(name) {
            return Err(RezCoreError::Repository(error.clone()));
        }
        Ok(self.packages.get(name).cloned().unwrap_or_default())
    }

    async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError> {
        let mut packages = self.find_packages(name).await?;
        packages.sort_by(|left, right| right.version.cmp(&left.version));
        Ok(packages.into_iter().find(|package| {
            version.is_none_or(|version| {
                package
                    .version
                    .as_ref()
                    .is_some_and(|candidate| candidate.as_str() == version)
            })
        }))
    }

    async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
        let mut names: Vec<_> = self.packages.keys().cloned().collect();
        names.sort();
        names.dedup();
        Ok(names)
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn root_path(&self) -> &Path {
        &self.root
    }
}
