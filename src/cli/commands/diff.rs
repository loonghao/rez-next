//! Diff command implementation
//!
//! Implements the `rez diff` command for comparing packages.

use clap::Args;
use rez_core_common::{error::RezCoreResult, RezCoreError};
use rez_core_package::Package;
use rez_core_repository::simple_repository::{RepositoryManager, SimpleRepository};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

/// Arguments for the diff command
#[derive(Args, Clone, Debug)]
pub struct DiffArgs {
    /// First package to diff
    #[arg(value_name = "PKG1")]
    pub pkg1: String,

    /// Second package to diff against (optional)
    #[arg(value_name = "PKG2")]
    pub pkg2: Option<String>,

    /// Repository paths to search
    #[arg(long = "paths", value_name = "PATH")]
    pub paths: Vec<PathBuf>,

    /// Output format
    #[arg(short = 'f', long = "format", value_enum, default_value = "text")]
    pub format: OutputFormat,

    /// Show only metadata differences
    #[arg(long = "metadata-only")]
    pub metadata_only: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Output format options
#[derive(clap::ValueEnum, Clone, Debug)]
pub enum OutputFormat {
    Text,
    Json,
}

/// Package difference information
#[derive(Debug, Clone)]
pub struct PackageDiff {
    pub pkg1_name: String,
    pub pkg1_version: Option<String>,
    pub pkg2_name: String,
    pub pkg2_version: Option<String>,
    pub differences: DiffDetails,
}

/// Detailed difference information
#[derive(Debug, Clone)]
pub struct DiffDetails {
    pub metadata_changes: MetadataChanges,
    pub requirements_changes: RequirementsChanges,
    pub variants_changes: VariantsChanges,
}

/// Metadata changes
#[derive(Debug, Clone)]
pub struct MetadataChanges {
    pub description: Option<(Option<String>, Option<String>)>,
    pub authors: Option<(Vec<String>, Vec<String>)>,
    pub tools: Option<(Vec<String>, Vec<String>)>,
}

/// Requirements changes
#[derive(Debug, Clone)]
pub struct RequirementsChanges {
    pub added: Vec<String>,
    pub removed: Vec<String>,
    pub modified: Vec<(String, String)>,
}

/// Variants changes
#[derive(Debug, Clone)]
pub struct VariantsChanges {
    pub count_change: (usize, usize),
    pub added: Vec<String>,
    pub removed: Vec<String>,
}

/// Execute the diff command
pub fn execute(args: DiffArgs) -> RezCoreResult<()> {
    if args.verbose {
        println!("🔍 Rez Diff - Comparing packages...");
        println!("Package 1: {}", args.pkg1);
        if let Some(ref pkg2) = args.pkg2 {
            println!("Package 2: {}", pkg2);
        } else {
            println!("Package 2: (auto-detect previous version)");
        }
    }

    // Create async runtime
    let runtime = tokio::runtime::Runtime::new().map_err(|e| RezCoreError::Io(e.into()))?;

    runtime.block_on(async { execute_diff_async(&args).await })
}

/// Execute diff operation asynchronously
async fn execute_diff_async(args: &DiffArgs) -> RezCoreResult<()> {
    // Setup repositories
    let repo_manager = setup_repositories(args).await?;

    // Find packages
    let (pkg1, pkg2) = find_packages_to_diff(&repo_manager, args).await?;

    if args.verbose {
        println!("Found packages:");
        println!(
            "  {}-{}",
            pkg1.name,
            pkg1.version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );
        println!(
            "  {}-{}",
            pkg2.name,
            pkg2.version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown")
        );
        println!();
    }

    // Compare packages
    let diff = compare_packages(&pkg1, &pkg2)?;

    // Output results
    match args.format {
        OutputFormat::Text => output_text_diff(&diff, args.verbose),
        OutputFormat::Json => output_json_diff(&diff)?,
    }

    Ok(())
}

/// Setup repository manager
async fn setup_repositories(args: &DiffArgs) -> RezCoreResult<RepositoryManager> {
    let mut repo_manager = RepositoryManager::new();
    let paths = if args.paths.is_empty() {
        vec![PathBuf::from("./local_packages")]
    } else {
        args.paths.clone()
    };

    for (i, path) in paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    Ok(repo_manager)
}

/// Find packages to diff
async fn find_packages_to_diff(
    repo_manager: &RepositoryManager,
    args: &DiffArgs,
) -> RezCoreResult<(Package, Package)> {
    // Parse package 1
    let (pkg1_name, pkg1_version) = parse_package_spec(&args.pkg1)?;
    let pkg1 = find_package(repo_manager, &pkg1_name, pkg1_version.as_deref()).await?;

    // Parse package 2 or find previous version
    let pkg2 = if let Some(ref pkg2_spec) = args.pkg2 {
        let (pkg2_name, pkg2_version) = parse_package_spec(pkg2_spec)?;
        find_package(repo_manager, &pkg2_name, pkg2_version.as_deref()).await?
    } else {
        // Find previous version of pkg1
        find_previous_version(repo_manager, &pkg1).await?
    };

    Ok((pkg1, pkg2))
}

/// Parse package specification
fn parse_package_spec(spec: &str) -> RezCoreResult<(String, Option<String>)> {
    if let Some(dash_pos) = spec.rfind('-') {
        let name = spec[..dash_pos].to_string();
        let version = spec[dash_pos + 1..].to_string();

        // Check if version part looks like a version
        if version.chars().next().map_or(false, |c| c.is_ascii_digit()) {
            return Ok((name, Some(version)));
        }
    }

    Ok((spec.to_string(), None))
}

/// Find a specific package
async fn find_package(
    repo_manager: &RepositoryManager,
    package_name: &str,
    version_spec: Option<&str>,
) -> RezCoreResult<Package> {
    let packages = repo_manager.find_packages(package_name).await?;

    if packages.is_empty() {
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}' not found",
            package_name
        )));
    }

    // If version specified, find matching version
    if let Some(version) = version_spec {
        for package in packages {
            if let Some(ref pkg_version) = package.version {
                if pkg_version.as_str() == version {
                    return Ok((*package).clone());
                }
            }
        }
        return Err(RezCoreError::RequirementParse(format!(
            "Package '{}-{}' not found",
            package_name, version
        )));
    }

    // Return latest version (first in list)
    Ok((*packages.into_iter().next().unwrap()).clone())
}

/// Find previous version of a package
async fn find_previous_version(
    repo_manager: &RepositoryManager,
    pkg: &Package,
) -> RezCoreResult<Package> {
    let packages = repo_manager.find_packages(&pkg.name).await?;

    if packages.len() < 2 {
        return Err(RezCoreError::RequirementParse(format!(
            "No previous version found for package '{}'",
            pkg.name
        )));
    }

    // Find the package with the version just before the current one
    // This is a simplified implementation - in reality we'd need proper version comparison
    for package in packages {
        if let (Some(ref pkg_version), Some(ref current_version)) = (&package.version, &pkg.version)
        {
            if pkg_version.as_str() != current_version.as_str() {
                return Ok((*package).clone());
            }
        }
    }

    Err(RezCoreError::RequirementParse(format!(
        "No suitable previous version found for package '{}'",
        pkg.name
    )))
}

/// Compare two packages and generate diff
fn compare_packages(pkg1: &Package, pkg2: &Package) -> RezCoreResult<PackageDiff> {
    let metadata_changes = compare_metadata(pkg1, pkg2);
    let requirements_changes = compare_requirements(pkg1, pkg2);
    let variants_changes = compare_variants(pkg1, pkg2);

    Ok(PackageDiff {
        pkg1_name: pkg1.name.clone(),
        pkg1_version: pkg1.version.as_ref().map(|v| v.as_str().to_string()),
        pkg2_name: pkg2.name.clone(),
        pkg2_version: pkg2.version.as_ref().map(|v| v.as_str().to_string()),
        differences: DiffDetails {
            metadata_changes,
            requirements_changes,
            variants_changes,
        },
    })
}

/// Compare package metadata
fn compare_metadata(pkg1: &Package, pkg2: &Package) -> MetadataChanges {
    let description = if pkg1.description != pkg2.description {
        Some((pkg1.description.clone(), pkg2.description.clone()))
    } else {
        None
    };

    let authors = if pkg1.authors != pkg2.authors {
        Some((pkg1.authors.clone(), pkg2.authors.clone()))
    } else {
        None
    };

    let tools = if pkg1.tools != pkg2.tools {
        Some((pkg1.tools.clone(), pkg2.tools.clone()))
    } else {
        None
    };

    MetadataChanges {
        description,
        authors,
        tools,
    }
}

/// Compare package requirements
fn compare_requirements(pkg1: &Package, pkg2: &Package) -> RequirementsChanges {
    let pkg1_reqs: std::collections::HashSet<_> = pkg1.requires.iter().collect();
    let pkg2_reqs: std::collections::HashSet<_> = pkg2.requires.iter().collect();

    let added: Vec<String> = pkg2_reqs
        .difference(&pkg1_reqs)
        .map(|s| s.to_string())
        .collect();
    let removed: Vec<String> = pkg1_reqs
        .difference(&pkg2_reqs)
        .map(|s| s.to_string())
        .collect();

    RequirementsChanges {
        added,
        removed,
        modified: vec![], // Simplified - would need more complex comparison for modifications
    }
}

/// Compare package variants
fn compare_variants(pkg1: &Package, pkg2: &Package) -> VariantsChanges {
    let pkg1_variants: std::collections::HashSet<_> = pkg1.variants.iter().collect();
    let pkg2_variants: std::collections::HashSet<_> = pkg2.variants.iter().collect();

    let added: Vec<String> = pkg2_variants
        .difference(&pkg1_variants)
        .map(|v| format!("{:?}", v))
        .collect();
    let removed: Vec<String> = pkg1_variants
        .difference(&pkg2_variants)
        .map(|v| format!("{:?}", v))
        .collect();

    VariantsChanges {
        count_change: (pkg1.variants.len(), pkg2.variants.len()),
        added,
        removed,
    }
}

/// Output diff in text format
fn output_text_diff(diff: &PackageDiff, verbose: bool) {
    println!(
        "Package Diff: {} vs {}",
        format!(
            "{}-{}",
            diff.pkg1_name,
            diff.pkg1_version.as_deref().unwrap_or("unknown")
        ),
        format!(
            "{}-{}",
            diff.pkg2_name,
            diff.pkg2_version.as_deref().unwrap_or("unknown")
        )
    );
    println!("=====================================");
    println!();

    // Metadata changes
    let meta = &diff.differences.metadata_changes;
    if meta.description.is_some() || meta.authors.is_some() || meta.tools.is_some() {
        println!("📝 Metadata Changes:");

        if let Some((old, new)) = &meta.description {
            println!("  Description:");
            println!("    - {}", old.as_deref().unwrap_or("(none)"));
            println!("    + {}", new.as_deref().unwrap_or("(none)"));
        }

        if let Some((old, new)) = &meta.authors {
            println!("  Authors:");
            println!("    - {:?}", old);
            println!("    + {:?}", new);
        }

        if let Some((old, new)) = &meta.tools {
            println!("  Tools:");
            println!("    - {:?}", old);
            println!("    + {:?}", new);
        }
        println!();
    }

    // Requirements changes
    let reqs = &diff.differences.requirements_changes;
    if !reqs.added.is_empty() || !reqs.removed.is_empty() || !reqs.modified.is_empty() {
        println!("📦 Requirements Changes:");

        for req in &reqs.added {
            println!("  + {}", req);
        }

        for req in &reqs.removed {
            println!("  - {}", req);
        }

        for (old, new) in &reqs.modified {
            println!("  ~ {} -> {}", old, new);
        }
        println!();
    }

    // Variants changes
    let variants = &diff.differences.variants_changes;
    if variants.count_change.0 != variants.count_change.1
        || !variants.added.is_empty()
        || !variants.removed.is_empty()
    {
        println!("🔧 Variants Changes:");
        println!(
            "  Count: {} -> {}",
            variants.count_change.0, variants.count_change.1
        );

        for variant in &variants.added {
            println!("  + {}", variant);
        }

        for variant in &variants.removed {
            println!("  - {}", variant);
        }
        println!();
    }

    // Summary
    let has_changes = meta.description.is_some()
        || meta.authors.is_some()
        || meta.tools.is_some()
        || !reqs.added.is_empty()
        || !reqs.removed.is_empty()
        || !reqs.modified.is_empty()
        || variants.count_change.0 != variants.count_change.1
        || !variants.added.is_empty()
        || !variants.removed.is_empty();

    if !has_changes {
        println!("✅ No differences found between packages");
    } else if verbose {
        println!("✅ Diff completed");
    }
}

/// Output diff in JSON format
fn output_json_diff(diff: &PackageDiff) -> RezCoreResult<()> {
    // Simple JSON-like output without serde dependency
    println!("{{");
    println!("  \"pkg1_name\": \"{}\",", diff.pkg1_name);
    println!(
        "  \"pkg1_version\": \"{}\",",
        diff.pkg1_version.as_deref().unwrap_or("unknown")
    );
    println!("  \"pkg2_name\": \"{}\",", diff.pkg2_name);
    println!(
        "  \"pkg2_version\": \"{}\",",
        diff.pkg2_version.as_deref().unwrap_or("unknown")
    );
    println!("  \"differences\": {{");

    // Metadata changes
    println!("    \"metadata_changes\": {{");
    if let Some((old, new)) = &diff.differences.metadata_changes.description {
        println!(
            "      \"description\": [\"{}\", \"{}\"],",
            old.as_deref().unwrap_or("null"),
            new.as_deref().unwrap_or("null")
        );
    }
    println!("    }},");

    // Requirements changes
    println!("    \"requirements_changes\": {{");
    println!(
        "      \"added\": {:?},",
        diff.differences.requirements_changes.added
    );
    println!(
        "      \"removed\": {:?},",
        diff.differences.requirements_changes.removed
    );
    println!(
        "      \"modified\": {:?}",
        diff.differences.requirements_changes.modified
    );
    println!("    }},");

    // Variants changes
    println!("    \"variants_changes\": {{");
    println!(
        "      \"count_change\": [{}, {}],",
        diff.differences.variants_changes.count_change.0,
        diff.differences.variants_changes.count_change.1
    );
    println!(
        "      \"added\": {:?},",
        diff.differences.variants_changes.added
    );
    println!(
        "      \"removed\": {:?}",
        diff.differences.variants_changes.removed
    );
    println!("    }}");

    println!("  }}");
    println!("}}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_package_spec() {
        assert_eq!(
            parse_package_spec("python").unwrap(),
            ("python".to_string(), None)
        );

        assert_eq!(
            parse_package_spec("python-3.9").unwrap(),
            ("python".to_string(), Some("3.9".to_string()))
        );

        assert_eq!(
            parse_package_spec("my-package-name").unwrap(),
            ("my-package-name".to_string(), None)
        );
    }

    #[test]
    fn test_diff_args_defaults() {
        let args = DiffArgs {
            pkg1: "test".to_string(),
            pkg2: None,
            paths: vec![],
            format: OutputFormat::Text,
            metadata_only: false,
            verbose: false,
        };

        assert_eq!(args.pkg1, "test");
        assert!(args.pkg2.is_none());
        assert!(!args.metadata_only);
    }
}
