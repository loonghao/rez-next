//! Canonical filesystem repository paths shared by builds and runtime consumers.

use std::path::{Path, PathBuf};

use rez_next_common::RezCoreError;
use sha1::{Digest, Sha1};

use crate::requirement::RequirementParser;
use crate::{Package, VersionConstraint};

/// A validated package base and selected variant payload within a repository.
///
/// Package definitions live at the package base; payloads live at the variant
/// root. Nonhashed variants use ordered Rez requirement strings as directory
/// components. Hashed variants use the full SHA1 of the Python list
/// representation of those strings, as Rez's `VariantResourceHelper._subpath`
/// does. Optional repository shortlinks are not canonical installation paths.
///
/// See <https://github.com/AcademySoftwareFoundation/rez/blob/main/src/rez/package_resources.py>.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PackageInstallLayout {
    package_relative_path: PathBuf,
    variant_subpath: PathBuf,
    variant_relative_path: PathBuf,
}

impl PackageInstallLayout {
    /// Plan a package's selected variant without reading or writing files.
    ///
    /// Variant packages require an explicit, in-range index. Packages without
    /// variants require `None`. Unsafe portable paths and requirement syntax
    /// whose Rez normalization is not supported are rejected.
    pub fn for_variant(package: &Package, index: Option<usize>) -> Result<Self, RezCoreError> {
        let package_relative_path = Self::package_base_relative_path(package)?;
        let requirements = match (package.variants.is_empty(), index) {
            (true, None) => None,
            (true, Some(_)) => {
                return Err(layout_error("Nonvariant package cannot select a variant"));
            }
            (false, None) => {
                return Err(layout_error(
                    "Package requires an explicit variant selection",
                ));
            }
            (false, Some(index)) => Some(package.variants.get(index).ok_or_else(|| {
                layout_error(&format!(
                    "Variant index {index} is out of range for package '{}'",
                    package.name
                ))
            })?),
        };

        let mut variant_subpath = PathBuf::new();
        if let Some(requirements) = requirements {
            if requirements.is_empty() && package.hashed_variants != Some(true) {
                return Err(layout_error(
                    "An empty declared variant has no supported Rez filesystem subpath",
                ));
            }
            let canonical = requirements
                .iter()
                .map(|requirement| canonical_requirement(requirement))
                .collect::<Result<Vec<_>, _>>()?;
            if package.hashed_variants == Some(true) {
                // Canonical requirements contain only ASCII version/name syntax,
                // so Python repr uses single quotes without any escapes.
                let python_list = format!(
                    "[{}]",
                    canonical
                        .iter()
                        .map(|value| format!("'{value}'"))
                        .collect::<Vec<_>>()
                        .join(", ")
                );
                let digest = Sha1::digest(python_list.as_bytes());
                let hash = digest
                    .iter()
                    .map(|byte| format!("{byte:02x}"))
                    .collect::<String>();
                variant_subpath.push(hash);
            } else {
                for requirement in canonical {
                    validate_component(&requirement, "variant requirement")?;
                    variant_subpath.push(requirement);
                }
            }
        }
        let variant_relative_path = package_relative_path.join(&variant_subpath);
        Ok(Self {
            package_relative_path,
            variant_subpath,
            variant_relative_path,
        })
    }

    /// Return the validated family/version directory for a package definition.
    /// Unversioned packages use the family directory itself.
    pub fn package_base_relative_path(package: &Package) -> Result<PathBuf, RezCoreError> {
        package.validate()?;
        validate_component(&package.name, "package name")?;
        let mut path = PathBuf::from(&package.name);
        if let Some(version) = &package.version
            && !version.as_str().is_empty()
        {
            validate_component(version.as_str(), "package version")?;
            path.push(version.as_str());
        }
        Ok(path)
    }

    /// Package definition directory relative to the repository root.
    pub fn package_relative_path(&self) -> &Path {
        &self.package_relative_path
    }

    /// Payload directory relative to the package definition directory.
    pub fn variant_subpath(&self) -> &Path {
        &self.variant_subpath
    }

    /// Payload directory relative to the repository root.
    pub fn variant_relative_path(&self) -> &Path {
        &self.variant_relative_path
    }
}

fn layout_error(message: &str) -> RezCoreError {
    RezCoreError::PackageParse(format!("Invalid package installation layout: {message}"))
}

fn validate_component(value: &str, label: &str) -> Result<(), RezCoreError> {
    let reserved = value
        .split('.')
        .next()
        .unwrap_or_default()
        .to_ascii_uppercase();
    let device = matches!(reserved.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || reserved
            .strip_prefix("COM")
            .or_else(|| reserved.strip_prefix("LPT"))
            .is_some_and(|suffix| {
                matches!(
                    suffix,
                    "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9" | "¹" | "²" | "³"
                )
            });
    if value.is_empty()
        || value == "."
        || value == ".."
        || value.ends_with(['.', ' '])
        || value.chars().any(|ch| {
            ch.is_control() || matches!(ch, '/' | '\\' | ':' | '<' | '>' | '"' | '|' | '?' | '*')
        })
        || device
    {
        return Err(layout_error(&format!(
            "Unsafe {label} path component {value:?}"
        )));
    }
    Ok(())
}

fn canonical_requirement(value: &str) -> Result<String, RezCoreError> {
    if value.is_empty() || !value.is_ascii() || value.chars().any(char::is_whitespace) {
        return Err(layout_error(&format!(
            "Unsupported variant requirement {value:?}"
        )));
    }
    let requirement = RequirementParser::new()
        .parse(value)
        .map_err(|error| layout_error(&error))?;
    if requirement.namespace.is_some()
        || !requirement.platform_conditions.is_empty()
        || !requirement.env_conditions.is_empty()
        || !requirement
            .name
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_')
        || requirement.name.is_empty()
    {
        return Err(layout_error(&format!(
            "Unsupported Rez variant requirement {value:?}"
        )));
    }
    let suffix = match &requirement.version_constraint {
        None | Some(VersionConstraint::Any) => String::new(),
        Some(VersionConstraint::Prefix(version)) => format!("-{version}"),
        Some(VersionConstraint::Exact(version)) => format!("=={version}"),
        Some(VersionConstraint::GreaterThanOrEqual(version)) => format!("-{version}+"),
        Some(VersionConstraint::LessThan(version)) => format!("<{version}"),
        Some(VersionConstraint::Range(min, max)) => format!("-{min}+<{max}"),
        Some(VersionConstraint::Multiple(constraints)) => match constraints.as_slice() {
            [
                VersionConstraint::GreaterThanOrEqual(min),
                VersionConstraint::LessThan(max),
            ] => format!("-{min}+<{max}"),
            _ => {
                return Err(layout_error(&format!(
                    "Unsupported Rez variant requirement normalization {value:?}"
                )));
            }
        },
        _ => {
            return Err(layout_error(&format!(
                "Unsupported Rez variant requirement normalization {value:?}"
            )));
        }
    };
    let prefix = if requirement.conflict {
        "!"
    } else if requirement.weak {
        "~"
    } else {
        ""
    };
    let canonical = format!("{prefix}{}{suffix}", requirement.name);
    if canonical != value
        || canonical
            .chars()
            .any(|ch| !ch.is_ascii_alphanumeric() && !"_-.!=+<~".contains(ch))
    {
        return Err(layout_error(&format!(
            "Variant requirement {value:?} must use supported canonical Rez syntax ({canonical:?})"
        )));
    }
    Ok(canonical)
}
