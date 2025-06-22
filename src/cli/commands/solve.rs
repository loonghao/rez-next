//! Solve command implementation - dependency resolution

use clap::Args;
use rez_next_common::{error::RezCoreResult, RezCoreError};
use rez_next_package::Requirement;
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::dependency_resolver::ResolutionResult;
use rez_next_solver::{ConflictStrategy, DependencyResolver, SolverConfig, SolverStats};
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Args, Clone)]
pub struct SolveArgs {
    /// Package requirements to resolve
    #[arg(value_name = "REQUIREMENTS")]
    pub requirements: Vec<String>,

    /// Repository paths to search for packages
    #[arg(short, long)]
    pub repository: Vec<PathBuf>,

    /// Show detailed resolution information
    #[arg(short, long)]
    pub verbose: bool,

    /// Maximum resolution time in seconds
    #[arg(long, default_value = "30")]
    pub timeout: u64,

    /// Show resolution statistics
    #[arg(long)]
    pub stats: bool,

    /// Output format (summary, detailed, json)
    #[arg(short, long, default_value = "summary")]
    pub format: String,
}

/// Execute the solve command
pub fn execute(args: SolveArgs) -> RezCoreResult<()> {
    // Use tokio runtime for async operations
    let rt = tokio::runtime::Runtime::new()?;
    rt.block_on(async { solve_async(args).await })
}

/// Async solve implementation
async fn solve_async(args: SolveArgs) -> RezCoreResult<()> {
    println!(
        "Resolving dependencies for: {}",
        args.requirements.join(", ")
    );

    // Parse requirements
    let requirements = parse_requirements(&args.requirements)?;

    // Set up repository manager
    let mut repo_manager = RepositoryManager::new();

    // Add repositories
    if args.repository.is_empty() {
        // Use default test repositories
        add_default_repositories(&mut repo_manager).await?;
    } else {
        for repo_path in &args.repository {
            let repo = SimpleRepository::new(repo_path, format!("repo_{}", repo_path.display()));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    println!("Using {} repositories", repo_manager.repository_count());

    // Create solver configuration
    let config = SolverConfig {
        max_attempts: 1000,
        max_time_seconds: args.timeout,
        enable_parallel: true,
        max_workers: 4,
        enable_caching: true,
        cache_ttl_seconds: 3600,
        prefer_latest: true,
        allow_prerelease: false,
        conflict_strategy: ConflictStrategy::LatestWins,
    };

    // Create dependency resolver
    let mut resolver = DependencyResolver::new(Arc::new(repo_manager), config);

    // Perform resolution
    println!("Starting dependency resolution...");
    let result = resolver.resolve(requirements).await?;

    // Display results
    display_resolution_result(&result, &args)?;

    Ok(())
}

/// Parse requirement strings into Requirement objects
fn parse_requirements(requirement_strs: &[String]) -> RezCoreResult<Vec<Requirement>> {
    let mut requirements = Vec::new();

    for req_str in requirement_strs {
        let requirement: Requirement = req_str.parse().map_err(|e| {
            RezCoreError::RequirementParse(format!("Failed to parse '{}': {}", req_str, e))
        })?;
        requirements.push(requirement);
    }

    Ok(requirements)
}

/// Add default test repositories
async fn add_default_repositories(repo_manager: &mut RepositoryManager) -> RezCoreResult<()> {
    // Add our test packages
    let test_repos = vec![
        "C:/temp/simple_test",
        "C:/temp/test-build-command",
        "C:/temp/test-commands",
        "C:/temp/test-variants",
        "C:/temp/test-complete",
        "C:/temp/perf-test",
    ];

    for (i, repo_path) in test_repos.iter().enumerate() {
        let path = PathBuf::from(repo_path);
        if path.exists() {
            let repo = SimpleRepository::new(&path, format!("test_repo_{}", i));
            repo_manager.add_repository(Box::new(repo));
        }
    }

    Ok(())
}

/// Display resolution result
fn display_resolution_result(result: &ResolutionResult, args: &SolveArgs) -> RezCoreResult<()> {
    match args.format.as_str() {
        "summary" => display_summary(result, args),
        "detailed" => display_detailed(result, args),
        "json" => display_json(result),
        _ => {
            eprintln!(
                "Unknown format: {}. Available formats: summary, detailed, json",
                args.format
            );
            Ok(())
        }
    }
}

/// Display summary format
fn display_summary(result: &ResolutionResult, args: &SolveArgs) -> RezCoreResult<()> {
    println!("\n=== Resolution Summary ===");

    if result.resolved_packages.is_empty() && result.failed_requirements.is_empty() {
        println!("No packages to resolve.");
        return Ok(());
    }

    if !result.resolved_packages.is_empty() {
        println!("✅ Resolved packages ({}):", result.resolved_packages.len());
        for resolved_pkg in &result.resolved_packages {
            let version_str = resolved_pkg
                .package
                .version
                .as_ref()
                .map(|v| v.as_str())
                .unwrap_or("unknown");

            let status = if resolved_pkg.requested {
                "requested"
            } else {
                "dependency"
            };
            println!(
                "  {} {} ({})",
                resolved_pkg.package.name, version_str, status
            );

            if args.verbose && !resolved_pkg.required_by.is_empty() {
                println!("    Required by: {}", resolved_pkg.required_by.join(", "));
            }
        }
    }

    if !result.failed_requirements.is_empty() {
        println!(
            "\n❌ Failed requirements ({}):",
            result.failed_requirements.len()
        );
        for failed_req in &result.failed_requirements {
            println!("  {}", failed_req);
        }
    }

    if !result.conflicts.is_empty() {
        println!("\n⚠️  Conflicts encountered ({}):", result.conflicts.len());
        for conflict in &result.conflicts {
            println!("  Package: {}", conflict.package_name);
            println!(
                "    Conflicting requirements: {}",
                conflict
                    .conflicting_requirements
                    .iter()
                    .map(|r| r.to_string())
                    .collect::<Vec<_>>()
                    .join(", ")
            );
            if !conflict.source_packages.is_empty() {
                println!(
                    "    Source packages: {}",
                    conflict.source_packages.join(", ")
                );
            }
        }
    }

    if args.stats {
        println!("\n=== Resolution Statistics ===");
        println!("Packages considered: {}", result.stats.packages_considered);
        println!("Variants evaluated: {}", result.stats.variants_evaluated);
        println!("Resolution time: {}ms", result.stats.resolution_time_ms);
        println!(
            "Conflicts encountered: {}",
            result.stats.conflicts_encountered
        );
        println!("Backtrack steps: {}", result.stats.backtrack_steps);
    }

    Ok(())
}

/// Display detailed format
fn display_detailed(result: &ResolutionResult, args: &SolveArgs) -> RezCoreResult<()> {
    display_summary(result, args)?;

    if !result.resolved_packages.is_empty() {
        println!("\n=== Package Details ===");
        for resolved_pkg in &result.resolved_packages {
            println!("\n--- {} ---", resolved_pkg.package.name);

            if let Some(ref version) = resolved_pkg.package.version {
                println!("Version: {:?}", version);
            }

            if let Some(ref desc) = resolved_pkg.package.description {
                println!("Description: {}", desc);
            }

            if !resolved_pkg.package.requires.is_empty() {
                println!("Requires: {}", resolved_pkg.package.requires.join(", "));
            }

            if !resolved_pkg.package.tools.is_empty() {
                println!("Tools: {}", resolved_pkg.package.tools.join(", "));
            }

            if let Some(ref satisfying_req) = resolved_pkg.satisfying_requirement {
                println!("Satisfies: {}", satisfying_req);
            }
        }
    }

    Ok(())
}

/// Display JSON format
fn display_json(result: &ResolutionResult) -> RezCoreResult<()> {
    // Create a simplified JSON representation
    let simplified = serde_json::json!({
        "resolved_packages": result.resolved_packages.iter().map(|pkg| {
            serde_json::json!({
                "name": pkg.package.name,
                "version": pkg.package.version.as_ref().map(|v| format!("{:?}", v)),
                "requested": pkg.requested,
                "required_by": pkg.required_by
            })
        }).collect::<Vec<_>>(),
        "failed_requirements": result.failed_requirements.iter().map(|req| req.to_string()).collect::<Vec<_>>(),
        "conflicts": result.conflicts.iter().map(|conflict| {
            serde_json::json!({
                "package_name": conflict.package_name,
                "conflicting_requirements": conflict.conflicting_requirements.iter().map(|r| r.to_string()).collect::<Vec<_>>(),
                "source_packages": conflict.source_packages
            })
        }).collect::<Vec<_>>(),
        "stats": {
            "packages_considered": result.stats.packages_considered,
            "variants_evaluated": result.stats.variants_evaluated,
            "resolution_time_ms": result.stats.resolution_time_ms,
            "conflicts_encountered": result.stats.conflicts_encountered,
            "backtrack_steps": result.stats.backtrack_steps
        }
    });

    let json = serde_json::to_string_pretty(&simplified)?;
    println!("{}", json);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_requirements() {
        let req_strs = vec!["python".to_string(), "numpy-1.20+".to_string()];

        let requirements = parse_requirements(&req_strs).unwrap();
        assert_eq!(requirements.len(), 2);
        assert_eq!(requirements[0].name, "python");
        assert_eq!(requirements[1].name, "numpy");
    }
}
