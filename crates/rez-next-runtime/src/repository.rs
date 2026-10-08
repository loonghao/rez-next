//! Compatibility discovery for serialized descriptors using the core scanner.

use std::collections::HashMap;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;

use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageSerializer};
use rez_next_repository::simple_repository::PackageRepository;
use rez_next_repository::{RepositoryScanner, ScannerConfig};

/// Bridge the core scanner's results into the core solver repository contract.
pub(crate) struct RuntimeRepository {
    root: PathBuf,
    name: String,
    packages: HashMap<String, Vec<Arc<Package>>>,
    errors: HashMap<String, String>,
}

impl RuntimeRepository {
    pub(crate) async fn new(root: &Path, name: String) -> Result<Self, RezCoreError> {
        // A repository is root/family[/version]/package.*. Bound
        // the core scanner so application payloads cannot turn activation into
        // a recursive scan of an entire Blender or FreeCAD installation.
        let scanner = RepositoryScanner::new(ScannerConfig {
            max_depth: 2,
            include_patterns: vec![
                "package.py".to_string(),
                "package.yaml".to_string(),
                "package.yml".to_string(),
            ],
            enable_scan_cache: false,
            ..ScannerConfig::default()
        });
        let scan = scanner.scan_repository(root).await?;
        // The 0.3.9 scanner discovers every configured descriptor, but parses
        // all of them as Python. Feed both successful discoveries and failed
        // parse paths through the core format-aware loading APIs instead.
        let mut descriptors: Vec<_> = scan
            .packages
            .into_iter()
            .map(|discovered| discovered.package_file)
            .chain(scan.errors.into_iter().map(|error| error.path))
            .filter(|path| Self::is_preferred_descriptor(root, path))
            .collect();
        descriptors.sort();
        descriptors.dedup();
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

    fn is_preferred_descriptor(root: &Path, descriptor: &Path) -> bool {
        if !descriptor.file_name().is_some_and(|name| {
            ["package.py", "package.yaml", "package.yml"]
                .iter()
                .any(|candidate| name == *candidate)
        }) {
            return false;
        }
        let Some(directory) = descriptor.parent() else {
            return false;
        };
        if (descriptor
            .file_name()
            .is_some_and(|name| name != "package.py")
            && directory.join("package.py").is_file())
            || (descriptor
                .file_name()
                .is_some_and(|name| name == "package.yml")
                && directory.join("package.yaml").is_file())
        {
            return false;
        }
        let mut ancestor = directory.parent();
        while let Some(directory) = ancestor {
            if directory == root {
                break;
            }
            if ["package.py", "package.yaml", "package.yml"]
                .iter()
                .any(|name| directory.join(name).is_file())
            {
                return false;
            }
            ancestor = directory.parent();
        }
        true
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
