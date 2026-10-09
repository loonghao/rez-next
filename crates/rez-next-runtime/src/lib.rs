//! High-level APIs for resolving and activating local Rez repositories.
//!
//! This crate is the supported embedding boundary for runtime managers. It
//! reuses rez-next's repository manager, strict dependency solver, materialized
//! packages and environment manager.

use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::PathBuf;
use std::process::Command;
use std::sync::Arc;

use thiserror::Error;

use rez_next_common::RezCoreError;
use rez_next_context::{ContextConfig, ContextStatus, EnvironmentManager, ResolvedContext};
use rez_next_package::{PackageRequirement, Requirement, VersionConstraint};
use rez_next_repository::RepositoryManager;
use rez_next_solver::{DependencyResolver, SolverConfig};
use rez_next_version::Version;

mod installation_plan;
mod repository;

pub use installation_plan::{InstallationPlan, InstallationPlanError};

use repository::RuntimeRepository;

/// Structured failures returned by the runtime integration boundary.
#[derive(Debug, Error)]
pub enum RezRuntimeError {
    /// No local repository path was supplied.
    #[error("at least one Rez bundle directory is required")]
    NoRepositories,
    /// A local repository path is not a directory.
    #[error("invalid Rez bundle directory: {path}", path = .path.display())]
    InvalidBundle {
        /// Invalid local repository path.
        path: PathBuf,
    },
    /// The requested platform is unsupported.
    #[error("unsupported Rez target platform: {platform}")]
    UnsupportedPlatform {
        /// Unsupported platform value.
        platform: String,
    },
    /// The requested architecture is not a valid Rez version identifier.
    #[error("unsupported Rez target architecture: {architecture}")]
    UnsupportedArchitecture {
        /// Unsupported architecture value.
        architecture: String,
    },
    /// An explicit target request would weaken or contradict the target.
    #[error("Rez requirement {requirement:?} is incompatible with target {target}")]
    IncompatibleTargetRequirement {
        /// Original explicit requirement.
        requirement: String,
        /// Exact target requirement.
        target: String,
    },
    /// A requirement could not be parsed.
    #[error("invalid Rez requirement {requirement:?}: {source}")]
    InvalidRequirement {
        /// Original requirement text.
        requirement: String,
        /// Parser failure.
        #[source]
        source: RezCoreError,
    },
    /// No package family exists for a required request.
    #[error("required Rez package is missing: {requirement}")]
    MissingPackage {
        /// Requirement whose family could not be found.
        requirement: String,
    },
    /// Repository scanning failed.
    #[error("failed to read Rez bundle repositories: {0}")]
    Repository(#[source] RezCoreError),
    /// Strict dependency resolution failed.
    #[error("failed to resolve Rez requirements: {0}")]
    Resolve(#[source] RezCoreError),
    /// Environment generation failed.
    #[error("failed to generate Rez environment: {0}")]
    Environment(#[source] RezCoreError),
}

/// Explicit target used to constrain Rez's platform and architecture packages.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTarget {
    platform: String,
    architecture: String,
}

impl RuntimeTarget {
    fn new(platform: &str, architecture: &str) -> Result<Self, RezRuntimeError> {
        let original_platform = platform.trim();
        let platform = match original_platform.to_ascii_lowercase().as_str() {
            "windows" => "windows",
            "linux" => "linux",
            "osx" | "macos" | "darwin" => "osx",
            _ => {
                return Err(RezRuntimeError::UnsupportedPlatform {
                    platform: original_platform.to_string(),
                });
            }
        };
        let architecture = architecture.trim();
        if architecture.is_empty() || Version::parse(architecture).is_err() {
            return Err(RezRuntimeError::UnsupportedArchitecture {
                architecture: architecture.to_string(),
            });
        }
        Ok(Self {
            platform: platform.to_string(),
            architecture: architecture.to_string(),
        })
    }

    /// Rez platform version selected by this target.
    #[must_use]
    pub fn platform(&self) -> &str {
        &self.platform
    }

    /// Rez architecture version selected by this target.
    #[must_use]
    pub fn architecture(&self) -> &str {
        &self.architecture
    }
}

/// Resolver and environment generator for existing local Rez repositories.
#[derive(Debug, Clone)]
pub struct RezRuntime {
    package_paths: Vec<PathBuf>,
    solver_config: SolverConfig,
    context_config: ContextConfig,
    parent_environment: Option<HashMap<String, String>>,
    target: Option<RuntimeTarget>,
}

impl RezRuntime {
    /// Create a runtime backed by existing local repository directories.
    pub fn new<I, P>(package_paths: I) -> Result<Self, RezRuntimeError>
    where
        I: IntoIterator<Item = P>,
        P: Into<PathBuf>,
    {
        let package_paths: Vec<PathBuf> = package_paths
            .into_iter()
            .map(|path| {
                let path = path.into();
                std::path::absolute(&path).map_err(|_| RezRuntimeError::InvalidBundle { path })
            })
            .collect::<Result<_, _>>()?;
        if package_paths.is_empty() {
            return Err(RezRuntimeError::NoRepositories);
        }
        for path in &package_paths {
            if !path.is_dir() {
                return Err(RezRuntimeError::InvalidBundle { path: path.clone() });
            }
        }
        Ok(Self {
            package_paths,
            solver_config: SolverConfig {
                strict_mode: true,
                ..SolverConfig::default()
            },
            context_config: ContextConfig::default(),
            parent_environment: None,
            target: None,
        })
    }

    /// Resolve for an explicit Rez platform and architecture.
    ///
    /// Platforms are `windows`, `linux` and `osx`; `macos` and `darwin`
    /// normalize to `osx`. Exact target constraints are always enforced.
    pub fn with_target(
        mut self,
        platform: &str,
        architecture: &str,
    ) -> Result<Self, RezRuntimeError> {
        self.target = Some(RuntimeTarget::new(platform, architecture)?);
        Ok(self)
    }

    /// Customize solver preferences while retaining strict resolution.
    ///
    /// `strict_mode` remains true: a runtime environment must satisfy every
    /// required dependency before it can be used to execute a child process.
    #[must_use]
    pub fn with_solver_config(mut self, mut solver_config: SolverConfig) -> Self {
        solver_config.strict_mode = true;
        self.solver_config = solver_config;
        self
    }

    /// Override environment generation behavior.
    #[must_use]
    pub fn with_context_config(mut self, mut context_config: ContextConfig) -> Self {
        if self.parent_environment.is_some() {
            context_config.inherit_parent_env = true;
        }
        self.context_config = context_config;
        self
    }

    /// Inject the complete parent environment before package actions.
    ///
    /// An empty map deliberately isolates the result from the ambient process.
    /// Windows environment keys are folded by the core environment manager.
    #[must_use]
    pub fn with_parent_environment(mut self, parent_environment: HashMap<String, String>) -> Self {
        self.context_config.inherit_parent_env = true;
        self.parent_environment = Some(parent_environment);
        self
    }

    /// Resolve requirements strictly and export their generated environment.
    pub async fn resolve<I, S>(
        &self,
        requirements: I,
    ) -> Result<ResolvedEnvironment, RezRuntimeError>
    where
        I: IntoIterator<Item = S>,
        S: AsRef<str>,
    {
        let requirement_texts: Vec<String> = requirements
            .into_iter()
            .map(|requirement| requirement.as_ref().trim().to_string())
            .collect();
        let requested = requirement_texts
            .iter()
            .map(|requirement| Self::parse_requirement(requirement))
            .collect::<Result<Vec<_>, _>>()?;
        let requested_solver_requirements = requirement_texts
            .iter()
            .map(|requirement| Self::parse_solver_requirement(requirement))
            .collect::<Result<Vec<_>, _>>()?;
        let mut package_requirements = Vec::new();
        let mut solver_requirements = Vec::new();
        if let Some(target) = &self.target {
            // Resolve target families first so the core variant selector sees
            // the selected platform/architecture before it considers payloads.
            for (family, value) in [
                ("platform", &target.platform),
                ("arch", &target.architecture),
            ] {
                let target_text = format!("{family}=={value}");
                let target_version = Version::parse(value).map_err(|source| {
                    RezRuntimeError::InvalidRequirement {
                        requirement: target_text.clone(),
                        source,
                    }
                })?;
                for (text, request) in requirement_texts.iter().zip(&requested_solver_requirements)
                {
                    if request.name == family
                        && (request.weak
                            || request.conflict
                            || !request.is_satisfied_by(&target_version))
                    {
                        return Err(RezRuntimeError::IncompatibleTargetRequirement {
                            requirement: text.clone(),
                            target: target_text,
                        });
                    }
                }
                package_requirements.push(Self::parse_requirement(&target_text)?);
                // Core Exact intentionally compares at prefix depth. A wildcard
                // constraint without '*' additionally fixes the token count.
                solver_requirements.push(Requirement::with_version(
                    family.to_string(),
                    VersionConstraint::Wildcard(value.to_string()),
                ));
            }
        }
        package_requirements.extend(requested);
        solver_requirements.extend(requested_solver_requirements);

        let mut repositories = RepositoryManager::new();
        for (index, path) in self.package_paths.iter().enumerate() {
            repositories.add_repository(Box::new(
                RuntimeRepository::new(path, format!("repository_{index}"))
                    .await
                    .map_err(RezRuntimeError::Repository)?,
            ));
        }
        for requirement in &package_requirements {
            if requirement.conflict || requirement.weak {
                continue;
            }
            if repositories
                .find_packages(&requirement.name)
                .await
                .map_err(RezRuntimeError::Repository)?
                .is_empty()
            {
                return Err(RezRuntimeError::MissingPackage {
                    requirement: requirement.to_string(),
                });
            }
        }
        let resolution =
            DependencyResolver::new(Arc::new(repositories), self.solver_config.clone())
                .resolve(solver_requirements)
                .await
                .map_err(RezRuntimeError::Resolve)?;

        let mut context = ResolvedContext::from_requirements(package_requirements);
        context.config = self.context_config.clone();
        context.resolved_packages = resolution
            .resolved_packages
            .iter()
            .map(|package| package.try_materialized_package())
            .collect::<Result<Vec<_>, _>>()
            .map_err(RezRuntimeError::Resolve)?;
        context.status = ContextStatus::Resolved;
        if let Some(target) = &self.target {
            for (family, expected) in [
                ("platform", &target.platform),
                ("arch", &target.architecture),
            ] {
                if context
                    .get_package(family)
                    .and_then(|package| package.version.as_ref())
                    .map(Version::as_str)
                    != Some(expected.as_str())
                {
                    return Err(RezRuntimeError::Resolve(RezCoreError::Solver(format!(
                        "Resolved {family} does not match exact target {expected}"
                    ))));
                }
            }
            context.set_platform(target.platform.clone());
            context.set_arch(target.architecture.clone());
        }
        let package_paths = std::env::join_paths(&self.package_paths)
            .map_err(|error| {
                RezRuntimeError::Environment(RezCoreError::ConfigError(format!(
                    "Package repository paths cannot be encoded for this platform: {error}"
                )))
            })?
            .to_string_lossy()
            .into_owned();
        context
            .metadata
            .insert("package_paths".to_string(), package_paths.clone());
        let manager = match &self.parent_environment {
            Some(parent) => {
                EnvironmentManager::with_base_environment(context.config.clone(), parent.clone())
            }
            None => EnvironmentManager::new(context.config.clone()),
        };
        let mut environment = manager
            .generate_environment(&context.resolved_packages)
            .await
            .map_err(RezRuntimeError::Environment)?;
        let resolve = context
            .resolved_packages
            .iter()
            .map(|package| match &package.version {
                Some(version) => format!("{}-{}", package.name, version.as_str()),
                None => package.name.clone(),
            })
            .collect::<Vec<_>>()
            .join(" ");
        environment.insert("REZ_USED_REQUEST".to_string(), requirement_texts.join(" "));
        environment.insert("REZ_USED_RESOLVE".to_string(), resolve.clone());
        environment.insert("REZ_USED_PACKAGES_NAMES".to_string(), resolve);
        environment.insert("REZ_USED_PACKAGES_PATH".to_string(), package_paths);
        environment.insert(
            "REZ_USED_VERSION".to_string(),
            env!("CARGO_PKG_VERSION").to_string(),
        );
        environment.insert(
            "REZ_USED_TIMESTAMP".to_string(),
            context.created_at.to_string(),
        );
        context.environment_vars = environment.clone();
        Ok(ResolvedEnvironment {
            context,
            environment,
        })
    }

    fn parse_requirement(requirement: &str) -> Result<PackageRequirement, RezRuntimeError> {
        PackageRequirement::parse(requirement).map_err(|source| {
            RezRuntimeError::InvalidRequirement {
                requirement: requirement.to_string(),
                source,
            }
        })
    }

    fn parse_solver_requirement(requirement: &str) -> Result<Requirement, RezRuntimeError> {
        let parsed: Requirement =
            requirement
                .parse()
                .map_err(|message| RezRuntimeError::InvalidRequirement {
                    requirement: requirement.to_string(),
                    source: RezCoreError::RequirementParse(message),
                })?;
        if parsed.name.is_empty()
            || parsed.name.starts_with('.')
            || !parsed
                .name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'.'))
        {
            return Err(RezRuntimeError::InvalidRequirement {
                requirement: requirement.to_string(),
                source: RezCoreError::RequirementParse("invalid package family name".to_string()),
            });
        }
        Ok(parsed)
    }
}

/// A resolved context together with its exported environment.
#[derive(Debug, Clone)]
pub struct ResolvedEnvironment {
    context: ResolvedContext,
    environment: HashMap<String, String>,
}

impl ResolvedEnvironment {
    /// The resolved context, including materialized package roots.
    #[must_use]
    pub fn context(&self) -> &ResolvedContext {
        &self.context
    }

    /// Environment variables generated by the resolved packages.
    #[must_use]
    pub fn environment(&self) -> &HashMap<String, String> {
        &self.environment
    }

    /// Create a direct child process with exactly the resolved environment.
    ///
    /// Add arguments and a working directory through [`Command`]. No shell is
    /// involved. Executing a different target requires a compatible host.
    #[must_use]
    pub fn command<S: AsRef<OsStr>>(&self, program: S) -> Command {
        let mut command = Command::new(program);
        command.env_clear().envs(&self.environment);
        command
    }

    /// Consume the result into its context and environment components.
    #[must_use]
    pub fn into_parts(self) -> (ResolvedContext, HashMap<String, String>) {
        (self.context, self.environment)
    }
}
