//! Read-only installation planning through the core parser, solver and layout.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use async_trait::async_trait;
use serde::Serialize;
use thiserror::Error;

use rez_next_common::RezCoreError;
use rez_next_package::{
    Package, PackageInstallLayout, PackageSerializer, Requirement, VersionConstraint,
};
use rez_next_repository::{RepositoryManager, simple_repository::PackageRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;

use crate::{RezRuntime, RezRuntimeError, RuntimeTarget, repository::RuntimeRepository};

/// Failures while deriving an installation plan without executing a package.
#[derive(Debug, Error)]
pub enum InstallationPlanError {
    /// The explicit target or repository configuration is invalid.
    #[error(transparent)]
    Configuration(#[from] RezRuntimeError),
    /// Core could not read or validate the definition.
    #[error("invalid package definition: {0}")]
    Definition(#[source] RezCoreError),
    /// The core strict solver could not satisfy the definition and its target.
    #[error("failed to resolve installation target: {0}")]
    Resolve(#[source] RezCoreError),
    /// The selected package or target did not match the supplied definition.
    #[error("invalid installation selection: {0}")]
    Selection(String),
    /// Core rejected the selected package's canonical installation paths.
    #[error("invalid package installation layout: {0}")]
    Layout(#[source] RezCoreError),
}

/// Deterministic installation metadata derived from an unchanged definition.
///
/// Paths are relative to the repository and always use `/` separators. The
/// definition keeps its filename within `package_relative_path`; its selected
/// payload belongs beneath `variant_relative_path`. No package environment or
/// build commands are executed while producing this plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct InstallationPlan {
    /// Version of this installation-plan JSON contract.
    pub schema_version: u32,
    /// The explicit Rez platform verified by the strict solver.
    pub platform: String,
    /// The explicit Rez architecture verified by the strict solver.
    pub arch: String,
    /// The exact package family parsed by Core.
    pub name: String,
    /// The exact version text, or `None` for a versionless package.
    pub version: Option<String>,
    /// The variant chosen by Core's strict dependency solver.
    pub variant_index: Option<usize>,
    /// The selected variant's requirements, preserving declaration order.
    pub variant_requirements: Vec<String>,
    /// Canonical package directory relative to the repository.
    pub package_relative_path: String,
    /// Canonical selected payload directory relative to the repository.
    pub variant_relative_path: String,
}

impl InstallationPlan {
    /// Parse a definition and resolve its installation layout for a target.
    ///
    /// Only the supplied definition and explicit `platform`/`arch` bindings
    /// are available. Missing runtime dependencies fail strictly. Use
    /// [`Self::from_definition_with_repositories`] to supply real dependencies.
    pub async fn from_definition(
        definition: impl AsRef<Path>,
        platform: &str,
        architecture: &str,
    ) -> Result<Self, InstallationPlanError> {
        Self::from_definition_with_repositories(definition, platform, architecture, &[]).await
    }

    /// Resolve a definition with additional canonical local repositories.
    ///
    /// Additional repositories provide actual runtime dependencies; they never
    /// replace the requested definition or infer missing dependency versions.
    pub async fn from_definition_with_repositories(
        definition: impl AsRef<Path>,
        platform: &str,
        architecture: &str,
        package_paths: &[PathBuf],
    ) -> Result<Self, InstallationPlanError> {
        let target = RuntimeTarget::new(platform, architecture)?;
        let definition = definition.as_ref();
        let package = PackageSerializer::load_from_file(definition)
            .map_err(InstallationPlanError::Definition)?;
        package
            .validate()
            .map_err(InstallationPlanError::Definition)?;
        let package = Arc::new(package);
        let mut repositories = RepositoryManager::new();
        repositories.add_repository(Box::new(DefinitionRepository::new(
            definition,
            Arc::clone(&package),
            &target,
        )?));
        if !package_paths.is_empty() {
            let runtime = RezRuntime::new(package_paths.iter().cloned())?;
            for (index, path) in runtime.package_paths.iter().enumerate() {
                repositories.add_repository(Box::new(
                    RuntimeRepository::new(path, format!("dependency_repository_{index}"))
                        .await
                        .map_err(RezRuntimeError::Repository)?,
                ));
            }
        }
        // Target families precede the payload so Core selects a variant using
        // the actual resolved platform and architecture. Wildcard without '*'
        // additionally enforces exact token depth in the core version model.
        let mut requirements = vec![
            exact_requirement("platform", &target.platform),
            exact_requirement("arch", &target.architecture),
        ];
        requirements.push(match &package.version {
            Some(version) => exact_requirement(&package.name, version.as_str()),
            None => Requirement::new(package.name.clone()),
        });
        let resolution = DependencyResolver::new(
            Arc::new(repositories),
            SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
        )
        .resolve(requirements)
        .await
        .map_err(InstallationPlanError::Resolve)?;
        for (family, expected) in [
            ("platform", target.platform.as_str()),
            ("arch", target.architecture.as_str()),
        ] {
            if resolution
                .resolved_packages
                .iter()
                .find(|resolved| resolved.package.name == family)
                .and_then(|resolved| resolved.package.version.as_ref())
                .map(Version::as_str)
                != Some(expected)
            {
                return Err(InstallationPlanError::Selection(format!(
                    "resolved {family} does not match exact target {expected}"
                )));
            }
        }
        let selected = resolution
            .resolved_packages
            .iter()
            .find(|resolved| Arc::ptr_eq(&resolved.package, &package))
            .ok_or_else(|| {
                InstallationPlanError::Selection(
                    "solver did not select the supplied package definition".to_string(),
                )
            })?;
        let layout = PackageInstallLayout::for_variant(&package, selected.variant_index)
            .map_err(InstallationPlanError::Layout)?;
        let variant_requirements = selected
            .variant_index
            .map(|index| package.variants[index].clone())
            .unwrap_or_default();
        Ok(Self {
            schema_version: 1,
            platform: target.platform,
            arch: target.architecture,
            name: package.name.clone(),
            version: package
                .version
                .as_ref()
                .map(|version| version.as_str().to_string()),
            variant_index: selected.variant_index,
            variant_requirements,
            package_relative_path: portable_path(layout.package_relative_path())?,
            variant_relative_path: portable_path(layout.variant_relative_path())?,
        })
    }
}

fn exact_requirement(name: &str, version: &str) -> Requirement {
    Requirement::with_version(
        name.to_string(),
        VersionConstraint::Wildcard(version.to_string()),
    )
}

fn portable_path(path: &Path) -> Result<String, InstallationPlanError> {
    path.iter()
        .map(|component| {
            component.to_str().map(str::to_string).ok_or_else(|| {
                InstallationPlanError::Selection(
                    "Core returned a non-UTF-8 installation path".to_string(),
                )
            })
        })
        .collect::<Result<Vec<_>, _>>()
        .map(|components| components.join("/"))
}

/// Ephemeral solver inputs, never written into the package or repository.
struct DefinitionRepository {
    root: PathBuf,
    packages: HashMap<String, Arc<Package>>,
}

impl DefinitionRepository {
    fn new(
        definition: &Path,
        package: Arc<Package>,
        target: &RuntimeTarget,
    ) -> Result<Self, InstallationPlanError> {
        let package_name = package.name.clone();
        let mut packages = HashMap::from([(package_name.clone(), package)]);
        for (family, value) in [
            ("platform", &target.platform),
            ("arch", &target.architecture),
        ] {
            // A real binding definition must itself satisfy the explicit
            // target; do not shadow it with a manufactured replacement.
            if family == package_name {
                continue;
            }
            let mut binding = Package::new(family.to_string());
            binding.version =
                Some(Version::parse(value).map_err(InstallationPlanError::Definition)?);
            packages.insert(family.to_string(), Arc::new(binding));
        }
        Ok(Self {
            root: definition
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .to_path_buf(),
            packages,
        })
    }
}

#[async_trait]
impl PackageRepository for DefinitionRepository {
    async fn find_packages(&self, name: &str) -> Result<Vec<Arc<Package>>, RezCoreError> {
        Ok(self.packages.get(name).cloned().into_iter().collect())
    }

    async fn get_package(
        &self,
        name: &str,
        version: Option<&str>,
    ) -> Result<Option<Arc<Package>>, RezCoreError> {
        Ok(self
            .packages
            .get(name)
            .filter(|package| {
                version.is_none_or(|version| {
                    package
                        .version
                        .as_ref()
                        .is_some_and(|candidate| candidate.as_str() == version)
                })
            })
            .cloned())
    }

    async fn list_packages(&self) -> Result<Vec<String>, RezCoreError> {
        let mut names: Vec<_> = self.packages.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    fn name(&self) -> &str {
        "installation_definition"
    }

    fn root_path(&self) -> &Path {
        &self.root
    }
}
