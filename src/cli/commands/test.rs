//! # Test Command
//!
//! Run tests defined in a package.
//! This command handles package testing including:
//! - Loading package definition and extracting test names from `tests` field
//! - Test environment resolution
//! - Real test command execution
//! - Result reporting and filtering

use crate::cli::utils::expand_home_path;
use clap::Args;
use rez_next_common::{RezCoreError, config::RezCoreConfig, error::RezCoreResult};
use rez_next_context::{ContextConfig, EnvironmentManager, normalize_environment_paths};
use rez_next_package::{Requirement, serialization::PackageSerializer};
use rez_next_repository::simple_repository::{RepositoryManager, SimpleRepository};
use rez_next_solver::{DependencyResolver, SolverConfig};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::time::Instant;

/// Test command configuration
#[derive(Debug, Clone, Args)]
pub struct TestArgs {
    /// Package to test (can be a path or package name)
    #[arg(value_name = "PKG")]
    pub package: String,

    /// Test names to run (defaults to all tests)
    #[arg(value_name = "TEST")]
    pub tests: Vec<String>,

    /// List available tests and exit
    #[arg(short = 'l', long = "list")]
    pub list: bool,

    /// Run tests in current environment if possible
    #[arg(long = "inplace")]
    pub inplace: bool,

    /// Extra packages to add to test environment
    #[arg(long = "extra-packages")]
    pub extra_packages: Vec<String>,

    /// Package search paths (colon-separated)
    #[arg(long = "paths")]
    pub paths: Option<String>,

    /// Don't include local packages
    #[arg(long = "no-local")]
    pub no_local: bool,

    /// Dry run - don't actually execute tests
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Stop on first test failure
    #[arg(long = "stop-on-fail")]
    pub stop_on_fail: bool,

    /// Verbose output level (0-2)
    #[arg(short = 'v', long = "verbose", action = clap::ArgAction::Count)]
    pub verbose: u8,

    /// Working directory
    #[arg(short = 'w', long = "working-dir")]
    pub working_dir: Option<PathBuf>,
}

/// Test result status
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    Success,
    Failed,
    Skipped,
    Error,
}

/// Individual test result
#[derive(Debug, Clone)]
pub struct TestResult {
    pub name: String,
    pub status: TestStatus,
    pub output: String,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub exit_code: i32,
}

/// Test definition loaded from package.py
#[derive(Debug, Clone)]
pub struct TestDefinition {
    /// Test name
    pub name: String,
    /// Command to run (can be a string command or list)
    pub command: TestCommand,
    /// Required packages for this test (beyond normal requires)
    pub requires: Vec<String>,
    /// Whether to run in the package's context
    pub run_on: Vec<String>,
    /// Whether this test is on by default
    pub on_variants: Option<bool>,
}

/// Test command format
#[derive(Debug, Clone)]
pub enum TestCommand {
    /// Simple string command
    String(String),
    /// List of arguments
    List(Vec<String>),
}

/// Test runner for package tests
#[derive(Debug)]
pub struct PackageTestRunner {
    pub package_name: String,
    pub working_dir: PathBuf,
    pub verbose: u8,
    pub dry_run: bool,
    pub stop_on_fail: bool,
    pub test_results: Vec<TestResult>,
    /// Loaded test definitions from package.py
    test_definitions: HashMap<String, TestDefinition>,
    /// Resolved package environment used by test commands.
    test_environment: Option<HashMap<String, String>>,
    /// Why an inplace test cannot run in the current resolved environment.
    inplace_skip_reason: Option<String>,
}

impl PackageTestRunner {
    /// Create a new test runner
    pub fn new(package_name: String, args: &TestArgs) -> RezCoreResult<Self> {
        let working_dir = args
            .working_dir
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap());

        let mut runner = Self {
            package_name: package_name.clone(),
            working_dir: working_dir.clone(),
            verbose: args.verbose,
            dry_run: args.dry_run,
            stop_on_fail: args.stop_on_fail,
            test_results: Vec::new(),
            test_definitions: HashMap::new(),
            test_environment: None,
            inplace_skip_reason: None,
        };

        // Load test definitions from package file
        runner.load_test_definitions(&working_dir, &package_name, args)?;

        Ok(runner)
    }

    /// Load test definitions from package.py in working directory
    fn load_test_definitions(
        &mut self,
        working_dir: &Path,
        package_spec: &str,
        args: &TestArgs,
    ) -> RezCoreResult<()> {
        let config = RezCoreConfig::load();
        let mut search_paths: Vec<PathBuf> = args
            .paths
            .as_deref()
            .map(|paths| std::env::split_paths(paths).collect())
            .unwrap_or_default();
        let local_path = expand_home_path(&config.local_packages_path);
        for search_path in &config.packages_path {
            let path = expand_home_path(search_path);
            if (!args.no_local || path != local_path) && !search_paths.contains(&path) {
                search_paths.push(path);
            }
        }
        let candidates = [working_dir.join("package.py")];

        for candidate in &candidates {
            if candidate.exists() {
                let mut package = PackageSerializer::load_from_file(candidate)?;
                package.filepath = Some(candidate.to_string_lossy().into_owned());
                self.working_dir = candidate
                    .parent()
                    .map(Path::to_path_buf)
                    .unwrap_or_else(|| working_dir.to_path_buf());
                self.extract_tests_from_package(&package, args, &search_paths, false)?;
                return Ok(());
            }
        }

        // If not found in working_dir, try to find by name in configured paths
        for search_path in &search_paths {
            let pkg_path = search_path.join(package_spec);
            if pkg_path.exists() {
                // Look for latest version
                if let Ok(entries) = std::fs::read_dir(&pkg_path) {
                    let mut versions: Vec<PathBuf> = entries
                        .flatten()
                        .filter(|e| e.path().is_dir())
                        .map(|e| e.path())
                        .collect();
                    versions.sort();
                    if let Some(latest) = versions.last() {
                        let package_file = latest.join("package.py");
                        if package_file.exists() {
                            let mut package = PackageSerializer::load_from_file(&package_file)?;
                            package.filepath = Some(package_file.to_string_lossy().into_owned());
                            self.working_dir = latest.clone();
                            self.extract_tests_from_package(&package, args, &search_paths, true)?;
                            return Ok(());
                        }
                    }
                }
            }
        }

        Err(RezCoreError::CliError(format!(
            "Package '{package_spec}' not found in the working directory or configured package paths"
        )))
    }

    /// Extract test definitions from a loaded package
    fn extract_tests_from_package(
        &mut self,
        package: &rez_next_package::Package,
        args: &TestArgs,
        search_paths: &[PathBuf],
        from_repository: bool,
    ) -> RezCoreResult<()> {
        // Package.tests is HashMap<String, String> where value is the command
        for (test_name, command_str) in &package.tests {
            let cmd = TestCommand::String(command_str.clone());

            self.test_definitions.insert(
                test_name.clone(),
                TestDefinition {
                    name: test_name.clone(),
                    command: cmd,
                    requires: vec![],
                    run_on: vec!["default".to_string()],
                    on_variants: None,
                },
            );
        }

        if args.inplace {
            let mut environment = std::env::vars().collect();
            normalize_environment_paths(&mut environment);
            let root_variable = format!("{}_ROOT", package.name.to_uppercase());
            let version_variable = format!("{}_VERSION", package.name.to_uppercase());
            let current_root = environment.get(&root_variable).map(PathBuf::from);
            let expected_version = package.version.as_ref().map(|version| version.as_str());
            let current_version = environment.get(&version_variable).map(String::as_str);
            let version_mismatch = expected_version
                .zip(current_version)
                .is_some_and(|(expected, current)| expected != current);

            if current_root.as_ref().is_none_or(|root| !root.is_dir()) || version_mismatch {
                self.inplace_skip_reason = Some(format!(
                    "The current environment does not contain package '{}' matching the request",
                    package.qualified_name()
                ));
            } else if let Some(root) = current_root {
                self.working_dir = root;
            }
            self.test_environment = Some(environment);
            return Ok(());
        }

        let runtime = tokio::runtime::Runtime::new().map_err(|error| {
            RezCoreError::CliError(format!("Failed to create test runtime: {error}"))
        })?;
        let mut repositories = RepositoryManager::new();
        for (index, path) in search_paths.iter().filter(|path| path.exists()).enumerate() {
            repositories.add_repository(Box::new(SimpleRepository::new(
                path,
                format!("test_repo_{index}"),
            )));
        }
        let repositories = Arc::new(repositories);
        let requirement_sets = test_requirement_sets(package, args, from_repository);
        let mut last_error = None;
        let mut packages = None;
        for requirement_strings in requirement_sets {
            if requirement_strings.is_empty() {
                packages = Some(Vec::new());
                break;
            }
            let requirements = parse_test_requirements(&requirement_strings)?;
            let mut resolver = DependencyResolver::new(
                repositories.clone(),
                SolverConfig {
                    strict_mode: true,
                    ..SolverConfig::default()
                },
            );
            match runtime.block_on(resolver.resolve(requirements)) {
                Ok(result) => {
                    packages = Some(
                        result
                            .resolved_packages
                            .iter()
                            .map(|resolved| resolved.materialized_package())
                            .collect(),
                    );
                    break;
                }
                Err(error) => last_error = Some(error),
            }
        }
        let mut packages = packages.ok_or_else(|| {
            last_error.unwrap_or_else(|| {
                RezCoreError::CliError(format!(
                    "No compatible test variant found for package '{}'",
                    package.qualified_name()
                ))
            })
        })?;
        if from_repository {
            let resolved_package = packages
                .iter_mut()
                .find(|resolved| resolved.name == package.name)
                .ok_or_else(|| {
                    RezCoreError::CliError(format!(
                        "Resolved test environment omitted package '{}'",
                        package.qualified_name()
                    ))
                })?;
            apply_pre_test_commands(resolved_package);
        } else {
            let mut test_package = package.clone();
            apply_pre_test_commands(&mut test_package);
            packages.push(test_package);
        }
        let manager = EnvironmentManager::new(ContextConfig {
            inherit_parent_env: true,
            ..ContextConfig::default()
        });
        self.test_environment = Some(runtime.block_on(manager.generate_environment(&packages))?);
        Ok(())
    }

    /// Get available test names from loaded package definition
    pub fn get_test_names(&self) -> RezCoreResult<Vec<String>> {
        if self.test_definitions.is_empty() {
            return Ok(vec![]);
        }

        let mut names: Vec<String> = self.test_definitions.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Find requested test names (supports wildcards)
    pub fn find_requested_test_names(&self, requested: &[String]) -> RezCoreResult<Vec<String>> {
        let available_tests = self.get_test_names()?;

        if requested.is_empty() {
            return Ok(available_tests);
        }

        let mut matched_tests = Vec::new();

        for pattern in requested {
            if pattern.contains('*') {
                let regex_pattern = pattern.replace('*', ".*");
                let regex = regex::Regex::new(&regex_pattern).map_err(|e| {
                    RezCoreError::CliError(format!("Invalid pattern '{}': {}", pattern, e))
                })?;

                for test_name in &available_tests {
                    if regex.is_match(test_name) && !matched_tests.contains(test_name) {
                        matched_tests.push(test_name.clone());
                    }
                }
            } else if available_tests.contains(pattern) && !matched_tests.contains(pattern) {
                matched_tests.push(pattern.clone());
            }
        }

        Ok(matched_tests)
    }

    /// Run a specific test by name
    pub fn run_test(&mut self, test_name: &str) -> RezCoreResult<i32> {
        if self.verbose > 0 {
            println!("Running test: {}", test_name);
        }

        let start_time = Instant::now();

        let (status, output, error, exit_code) = if let Some(reason) = &self.inplace_skip_reason {
            (TestStatus::Skipped, reason.clone(), None, 0)
        } else if self.dry_run {
            (
                TestStatus::Skipped,
                "Dry run - test not executed".to_string(),
                None,
                0,
            )
        } else {
            // Look up the test definition
            if let Some(test_def) = self.test_definitions.get(test_name) {
                self.execute_test_command(test_def)
            } else {
                (
                    TestStatus::Error,
                    String::new(),
                    Some(format!(
                        "Test '{}' not found in package definition",
                        test_name
                    )),
                    -1,
                )
            }
        };

        let duration_ms = start_time.elapsed().as_millis() as u64;

        let result = TestResult {
            name: test_name.to_string(),
            status: status.clone(),
            output: output.clone(),
            error: error.clone(),
            duration_ms,
            exit_code,
        };

        // Print test result
        match &status {
            TestStatus::Success => {
                if self.verbose > 0 {
                    println!("  PASSED: {} ({}ms)", test_name, duration_ms);
                    if self.verbose > 1 && !output.is_empty() {
                        println!("{}", output);
                    }
                }
            }
            TestStatus::Failed => {
                println!("  FAILED: {} ({}ms)", test_name, duration_ms);
                if !output.is_empty() {
                    println!("{}", output);
                }
                if let Some(ref err) = error {
                    println!("  Error: {}", err);
                }
            }
            TestStatus::Skipped => {
                if self.verbose > 0 {
                    println!("  SKIPPED: {} (skipped)", test_name);
                    if !output.is_empty() {
                        println!("  Reason: {}", output);
                    }
                }
            }
            TestStatus::Error => {
                println!("  ERROR: {} (error)", test_name);
                if let Some(ref err) = error {
                    println!("  Error: {}", err);
                }
            }
        }

        self.test_results.push(result);

        if self.stop_on_fail && exit_code != 0 {
            return Ok(exit_code);
        }

        Ok(exit_code)
    }

    /// Execute a test command and return (status, stdout, stderr, exit_code)
    fn execute_test_command(
        &self,
        test_def: &TestDefinition,
    ) -> (TestStatus, String, Option<String>, i32) {
        let (program, args) = match &test_def.command {
            TestCommand::String(cmd) => {
                // Shell-execute the string command
                if cfg!(windows) {
                    ("cmd".to_string(), vec!["/c".to_string(), cmd.clone()])
                } else {
                    ("sh".to_string(), vec!["-c".to_string(), cmd.clone()])
                }
            }
            TestCommand::List(parts) if !parts.is_empty() => {
                (parts[0].clone(), parts[1..].to_vec())
            }
            _ => {
                return (
                    TestStatus::Error,
                    String::new(),
                    Some("Empty test command".to_string()),
                    -1,
                );
            }
        };

        if self.verbose > 0 {
            println!("  Executing: {} {}", program, args.join(" "));
        }

        let mut command = Command::new(&program);
        command
            .args(&args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        if let Some(environment) = &self.test_environment {
            command.env_clear().envs(environment);
        }
        let result = command.output();

        match result {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout).to_string();
                let stderr = String::from_utf8_lossy(&output.stderr).to_string();
                let exit_code = output.status.code().unwrap_or(-1);

                if output.status.success() {
                    (
                        TestStatus::Success,
                        stdout,
                        if stderr.is_empty() {
                            None
                        } else {
                            Some(stderr)
                        },
                        exit_code,
                    )
                } else {
                    (
                        TestStatus::Failed,
                        stdout,
                        Some(if stderr.is_empty() {
                            format!("Process exited with code {}", exit_code)
                        } else {
                            stderr
                        }),
                        exit_code,
                    )
                }
            }
            Err(e) => (
                TestStatus::Error,
                String::new(),
                Some(format!("Failed to execute test command: {}", e)),
                -1,
            ),
        }
    }

    /// Print test summary
    pub fn print_summary(&self) {
        let total = self.test_results.len();
        let passed = self
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Success)
            .count();
        let failed = self
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count();
        let errors = self
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Error)
            .count();
        let skipped = self
            .test_results
            .iter()
            .filter(|r| r.status == TestStatus::Skipped)
            .count();

        println!("Test Summary for package '{}':", self.package_name);
        println!("   Total:   {}", total);
        println!("   Passed:  {}", passed);
        println!("   Failed:  {}", failed);
        println!("   Errors:  {}", errors);
        println!("   Skipped: {}", skipped);

        if failed > 0 || errors > 0 {
            println!("\nSome tests failed!");
        } else if total > 0 {
            println!("\nAll tests passed!");
        }
    }
}

fn parse_test_requirements(requirements: &[String]) -> RezCoreResult<Vec<Requirement>> {
    requirements
        .iter()
        .map(|requirement| {
            requirement.parse::<Requirement>().map_err(|error| {
                RezCoreError::RequirementParse(format!(
                    "Invalid test requirement '{requirement}': {error}"
                ))
            })
        })
        .collect()
}

fn test_requirement_sets(
    package: &rez_next_package::Package,
    args: &TestArgs,
    from_repository: bool,
) -> Vec<Vec<String>> {
    if from_repository {
        return vec![
            std::iter::once(package.qualified_name())
                .chain(args.extra_packages.iter().cloned())
                .collect(),
        ];
    }

    let base: Vec<_> = package
        .requires
        .iter()
        .chain(args.extra_packages.iter())
        .cloned()
        .collect();
    if package.variants.is_empty() {
        vec![base]
    } else {
        package
            .variants
            .iter()
            .map(|variant| {
                base.iter()
                    .cloned()
                    .chain(variant.iter().cloned())
                    .collect()
            })
            .collect()
    }
}

fn apply_pre_test_commands(package: &mut rez_next_package::Package) {
    let Some(pre_test_commands) = package.pre_test_commands.take() else {
        return;
    };
    package.post_commands = Some(match package.post_commands.take() {
        Some(post_commands) => format!("{post_commands}\n{pre_test_commands}"),
        None => pre_test_commands,
    });
}

// Need Path for load_test_definitions parameter
use std::path::Path;

/// Execute the test command
pub fn execute(args: TestArgs) -> RezCoreResult<()> {
    println!("Running package tests...");

    // Validate arguments
    if args.inplace && (!args.extra_packages.is_empty() || args.paths.is_some() || args.no_local) {
        return Err(RezCoreError::CliError(
            "Cannot use --inplace in combination with --extra-packages/--paths/--no-local"
                .to_string(),
        ));
    }

    // Create test runner
    let mut runner = PackageTestRunner::new(args.package.clone(), &args)?;

    // Get available tests
    let available_tests = runner.get_test_names()?;

    if available_tests.is_empty() {
        println!("No tests found in package '{}'.", args.package);
        println!("Make sure the package has a 'tests' field in its package.py.");
        return Ok(());
    }

    // Handle list option
    if args.list {
        println!("Tests defined in package '{}':", args.package);
        for test_name in &available_tests {
            if let Some(def) = runner.test_definitions.get(test_name) {
                let cmd_str = match &def.command {
                    TestCommand::String(s) => s.clone(),
                    TestCommand::List(parts) => parts.join(" "),
                };
                println!("  {:<20} {}", test_name, cmd_str);
            } else {
                println!("  {}", test_name);
            }
        }
        return Ok(());
    }

    // Find tests to run
    let tests_to_run = runner.find_requested_test_names(&args.tests)?;

    if tests_to_run.is_empty() {
        println!("No tests found matching the specified criteria");
        return Ok(());
    }

    // Run tests
    let mut exit_code = 0;
    for test_name in &tests_to_run {
        let result = runner.run_test(test_name)?;
        if result != 0 && exit_code == 0 {
            exit_code = result;
        }

        if runner.stop_on_fail && result != 0 {
            break;
        }
    }

    // Print summary
    println!();
    runner.print_summary();

    if exit_code != 0 {
        std::process::exit(exit_code);
    }

    Ok(())
}
