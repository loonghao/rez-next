//! Rez depends command implementation

use clap::Args;
use rez_next_common::RezCoreError;
use rez_next_package::{Package, PackageRequirement};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::DependencyGraph;
use rez_next_version::Version;
use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

/// Arguments for the depends command
#[derive(Args, Clone, Debug)]
pub struct DependsArgs {
    /// Package that other packages depend on
    #[arg(value_name = "PKG")]
    pub package: String,

    /// Dependency tree depth limit
    #[arg(short = 'd', long = "depth")]
    pub depth: Option<usize>,

    /// Set package search path
    #[arg(long = "paths")]
    pub paths: Option<String>,

    /// Include build requirements
    #[arg(short = 'b', long = "build-requires")]
    pub build_requires: bool,

    /// Include private build requirements of PKG, if any
    #[arg(short = 'p', long = "private-build-requires")]
    pub private_build_requires: bool,

    /// Display the dependency tree as an image
    #[arg(short = 'g', long = "graph")]
    pub graph: bool,

    /// Print the dependency tree as a string
    #[arg(long = "pg", long = "print-graph")]
    pub print_graph: bool,

    /// Write the dependency tree to FILE
    #[arg(long = "wg", long = "write-graph")]
    pub write_graph: Option<PathBuf>,

    /// Don't print progress bar or depth indicators
    #[arg(short = 'q', long = "quiet")]
    pub quiet: bool,

    /// Verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,
}

/// Dependency tree node for reverse lookup
#[derive(Debug, Clone)]
pub struct DependencyTreeNode {
    pub package_name: String,
    pub version: Option<Version>,
    pub dependents: Vec<String>,
    pub depth: usize,
}

/// Reverse dependency tree structure
#[derive(Debug, Clone)]
pub struct ReverseDependencyTree {
    pub root_package: String,
    pub nodes: HashMap<String, DependencyTreeNode>,
    pub depth_levels: Vec<Vec<String>>,
    pub max_depth: Option<usize>,
}

impl ReverseDependencyTree {
    /// Create a new reverse dependency tree
    pub fn new(root_package: String, max_depth: Option<usize>) -> Self {
        Self {
            root_package,
            nodes: HashMap::new(),
            depth_levels: Vec::new(),
            max_depth,
        }
    }

    /// Add a dependency relationship
    pub fn add_dependency(&mut self, dependent: String, dependency: String, depth: usize) {
        // Ensure we have enough depth levels
        while self.depth_levels.len() <= depth {
            self.depth_levels.push(Vec::new());
        }

        // Add to depth level if not already present
        if !self.depth_levels[depth].contains(&dependent) {
            self.depth_levels[depth].push(dependent.clone());
        }

        // Update or create node
        let dependency_key = dependency.clone();
        let node = self
            .nodes
            .entry(dependency_key)
            .or_insert_with(|| DependencyTreeNode {
                package_name: dependency,
                version: None,
                dependents: Vec::new(),
                depth,
            });

        if !node.dependents.contains(&dependent) {
            node.dependents.push(dependent);
        }
    }

    /// Get packages at a specific depth level
    pub fn get_packages_at_depth(&self, depth: usize) -> Vec<String> {
        self.depth_levels.get(depth).cloned().unwrap_or_default()
    }

    /// Get total number of depth levels
    pub fn get_max_depth(&self) -> usize {
        self.depth_levels.len()
    }
}

/// Execute the depends command
pub async fn execute_depends(args: DependsArgs) -> Result<(), RezCoreError> {
    if args.verbose {
        println!("🔍 Rez Depends - Analyzing package dependencies...");
        println!("📦 Analyzing dependencies for package: {}", args.package);
    }

    // Parse package paths
    let package_paths = if let Some(paths) = &args.paths {
        paths.split(':').map(|p| PathBuf::from(p.trim())).collect()
    } else {
        vec![PathBuf::from("./local_packages")]
    };

    if args.verbose {
        println!("🔍 Searching in paths: {:?}", package_paths);
    }

    // Create repository manager
    let mut repo_manager = RepositoryManager::new();

    // Add filesystem repositories
    for (i, path) in package_paths.iter().enumerate() {
        let repo_name = format!("repo_{}", i);
        let simple_repo = SimpleRepository::new(path.clone(), repo_name);
        repo_manager.add_repository(Box::new(simple_repo));
    }

    // Build reverse dependency tree
    let tree = build_reverse_dependency_tree(
        &repo_manager,
        &args.package,
        args.depth,
        args.build_requires,
        args.private_build_requires,
        args.verbose,
    )
    .await?;

    // Handle graph output options
    if args.graph || args.print_graph || args.write_graph.is_some() {
        let dot_graph = generate_dot_graph(&tree)?;

        if args.print_graph {
            println!("{}", dot_graph);
        } else if let Some(output_file) = &args.write_graph {
            std::fs::write(output_file, &dot_graph).map_err(|e| RezCoreError::Io(e.into()))?;
            if args.verbose {
                println!("✅ Graph written to: {}", output_file.display());
            }
        } else if args.graph {
            // For now, just print the graph since we don't have image viewer integration
            println!("Graph visualization (DOT format):");
            println!("{}", dot_graph);
        }
        return Ok(());
    }

    // Print dependency tree
    print_dependency_tree(&tree, args.quiet)?;

    if args.verbose {
        println!("✅ Dependency analysis completed.");
    }

    Ok(())
}

/// Build reverse dependency tree
async fn build_reverse_dependency_tree(
    repo_manager: &RepositoryManager,
    target_package: &str,
    max_depth: Option<usize>,
    include_build_requires: bool,
    include_private_build_requires: bool,
    verbose: bool,
) -> Result<ReverseDependencyTree, RezCoreError> {
    let mut tree = ReverseDependencyTree::new(target_package.to_string(), max_depth);
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();

    // Start with the target package at depth 0
    queue.push_back((target_package.to_string(), 0));
    visited.insert(target_package.to_string());

    if verbose {
        println!("🔍 Building reverse dependency tree...");
    }

    while let Some((current_package, depth)) = queue.pop_front() {
        // Check depth limit
        if let Some(max_depth) = max_depth {
            if depth >= max_depth {
                continue;
            }
        }

        if verbose {
            println!("  Analyzing depth {}: {}", depth, current_package);
        }

        // Find all packages that depend on current_package
        let dependents = find_package_dependents(
            repo_manager,
            &current_package,
            include_build_requires,
            include_private_build_requires,
        )
        .await?;

        for dependent in dependents {
            if !visited.contains(&dependent) {
                visited.insert(dependent.clone());
                queue.push_back((dependent.clone(), depth + 1));
                tree.add_dependency(dependent, current_package.clone(), depth + 1);
            }
        }
    }

    Ok(tree)
}

/// Find packages that depend on the target package
async fn find_package_dependents(
    repo_manager: &RepositoryManager,
    target_package: &str,
    include_build_requires: bool,
    include_private_build_requires: bool,
) -> Result<Vec<String>, RezCoreError> {
    let mut dependents = Vec::new();

    // Get all packages by searching with empty string (gets all)
    let all_packages = repo_manager.find_packages("").await?;

    for package in all_packages {
        let mut has_dependency = false;

        // Check regular requires
        for req_name in &package.requires {
            if req_name == target_package {
                has_dependency = true;
                break;
            }
        }

        // Check build requires if requested
        if !has_dependency && include_build_requires {
            for req_name in &package.build_requires {
                if req_name == target_package {
                    has_dependency = true;
                    break;
                }
            }
        }

        // Check private build requires if requested
        if !has_dependency && include_private_build_requires {
            for req_name in &package.private_build_requires {
                if req_name == target_package {
                    has_dependency = true;
                    break;
                }
            }
        }

        if has_dependency {
            dependents.push(package.name.clone());
        }
    }

    Ok(dependents)
}

/// Generate DOT graph representation of the dependency tree
fn generate_dot_graph(tree: &ReverseDependencyTree) -> Result<String, RezCoreError> {
    let mut dot = String::new();
    dot.push_str("digraph dependencies {\n");
    dot.push_str("  rankdir=BT;\n");
    dot.push_str("  node [shape=box, style=filled, fillcolor=\"#F6F6F6\", fontsize=10];\n");
    dot.push_str("  edge [color=\"#666666\"];\n\n");

    // Add root node with special styling
    dot.push_str(&format!(
        "  \"{}\" [fillcolor=\"#AAFFAA\", fontweight=bold];\n",
        tree.root_package
    ));

    // Add all nodes and edges
    for (package_name, node) in &tree.nodes {
        // Add node
        dot.push_str(&format!("  \"{}\";\n", package_name));

        // Add edges to dependents
        for dependent in &node.dependents {
            dot.push_str(&format!("  \"{}\" -> \"{}\";\n", package_name, dependent));
        }
    }

    dot.push_str("}\n");
    Ok(dot)
}

/// Print the dependency tree in text format
fn print_dependency_tree(tree: &ReverseDependencyTree, quiet: bool) -> Result<(), RezCoreError> {
    if tree.depth_levels.is_empty() {
        println!("No packages depend on '{}'", tree.root_package);
        return Ok(());
    }

    for (depth, packages) in tree.depth_levels.iter().enumerate() {
        if packages.is_empty() {
            continue;
        }

        let mut sorted_packages = packages.clone();
        sorted_packages.sort();

        if quiet {
            println!("{}", sorted_packages.join(" "));
        } else {
            println!("#{}: {}", depth, sorted_packages.join(" "));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reverse_dependency_tree_creation() {
        let tree = ReverseDependencyTree::new("test_package".to_string(), Some(3));
        assert_eq!(tree.root_package, "test_package");
        assert_eq!(tree.max_depth, Some(3));
        assert!(tree.nodes.is_empty());
        assert!(tree.depth_levels.is_empty());
    }

    #[test]
    fn test_add_dependency() {
        let mut tree = ReverseDependencyTree::new("root".to_string(), None);
        tree.add_dependency("dependent1".to_string(), "root".to_string(), 1);
        tree.add_dependency("dependent2".to_string(), "root".to_string(), 1);

        assert_eq!(tree.depth_levels.len(), 2);
        assert_eq!(tree.depth_levels[1].len(), 2);
        assert!(tree.depth_levels[1].contains(&"dependent1".to_string()));
        assert!(tree.depth_levels[1].contains(&"dependent2".to_string()));

        let root_node = tree.nodes.get("root").unwrap();
        assert_eq!(root_node.dependents.len(), 2);
    }

    #[test]
    fn test_get_packages_at_depth() {
        let mut tree = ReverseDependencyTree::new("root".to_string(), None);
        tree.add_dependency("dep1".to_string(), "root".to_string(), 1);
        tree.add_dependency("dep2".to_string(), "dep1".to_string(), 2);

        let depth_1_packages = tree.get_packages_at_depth(1);
        assert_eq!(depth_1_packages.len(), 1);
        assert!(depth_1_packages.contains(&"dep1".to_string()));

        let depth_2_packages = tree.get_packages_at_depth(2);
        assert_eq!(depth_2_packages.len(), 1);
        assert!(depth_2_packages.contains(&"dep2".to_string()));
    }

    #[test]
    fn test_generate_dot_graph() {
        let mut tree = ReverseDependencyTree::new("root".to_string(), None);
        tree.add_dependency("dependent".to_string(), "root".to_string(), 1);

        let dot_graph = generate_dot_graph(&tree).unwrap();
        assert!(dot_graph.contains("digraph dependencies"));
        assert!(dot_graph.contains("\"root\""));
        assert!(dot_graph.contains("\"dependent\""));
        assert!(dot_graph.contains("\"root\" -> \"dependent\""));
    }

    #[test]
    fn test_depends_args_defaults() {
        let args = DependsArgs {
            package: "test".to_string(),
            depth: None,
            paths: None,
            build_requires: false,
            private_build_requires: false,
            graph: false,
            print_graph: false,
            write_graph: None,
            quiet: false,
            verbose: false,
        };

        assert_eq!(args.package, "test");
        assert!(!args.build_requires);
        assert!(!args.verbose);
    }
}
