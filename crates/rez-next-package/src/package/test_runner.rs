//! Package test runner implementation
//!
//! This module provides the `PackageTestRunner` and `PackageTestResults` types
//! for running tests defined in a package's `tests` field (from `package.py`).
//!
//! # Example
//!
//! See the Python bindings (`rez_next._native.test`) for usage examples.

use crate::serialization::PackageSerializer;
use crate::types::Package;
use rez_next_common::{RezCoreResult, error::RezCoreError};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Instant;
use tracing;

// ── Test Status ─────────────────────────────────────────────────────────────

/// Test result status.
#[derive(Debug, Clone, PartialEq)]
pub enum TestStatus {
    /// Test passed successfully.
    Success,
    /// Test failed (non-zero exit code).
    Failed,
    /// Test was skipped.
    Skipped,
    /// Test encountered an error (e.g., command not found).
    Error,
}

// ── Test Command ───────────────────────────────────────────────────────────

/// Test command format.
#[derive(Debug, Clone)]
pub enum TestCommand {
    /// Simple string command (will be executed via shell).
    String(String),
    /// List of arguments (will be executed directly without shell).
    List(Vec<String>),
}

// ── Test Definition ────────────────────────────────────────────────────────

/// Test definition loaded from `package.py` `tests` field.
#[derive(Debug, Clone)]
pub struct TestDefinition {
    /// Test name.
    pub name: String,
    /// Command to run.
    pub command: TestCommand,
    /// Additional required packages for this test.
    pub requires: Vec<String>,
    /// Run tags for this test (e.g., "default", "ci", "nightly").
    pub run_on: Vec<String>,
    /// Whether to run on variants (False = once on preferred variant).
    pub on_variants: Option<bool>,
}

// ── Test Result ────────────────────────────────────────────────────────────

/// Individual test result.
#[derive(Debug, Clone)]
pub struct TestResult {
    /// Test name.
    pub name: String,
    /// Test status.
    pub status: TestStatus,
    /// Test output (stdout).
    pub output: String,
    /// Test error (stderr or error message).
    pub error: Option<String>,
    /// Test duration in milliseconds.
    pub duration_ms: u64,
    /// Test process exit code.
    pub exit_code: i32,
    /// Variant this test ran on (if applicable).
    pub variant: Option<String>,
}

// ── Package Test Runner ───────────────────────────────────────────────────

/// Runs tests defined in a package.
#[derive(Clone, Debug)]
pub struct PackageTestRunner {
    /// Package name (can include version request).
    pub package_name: String,
    /// Working directory for test execution.
    pub working_dir: PathBuf,
    /// Verbose output level (0-2).
    pub verbose: u8,
    /// Dry run mode (don't actually execute tests).
    pub dry_run: bool,
    /// Stop on first test failure.
    pub stop_on_fail: bool,
    /// Collected test results.
    pub test_results: Vec<TestResult>,
    /// Loaded test definitions from package.
    test_definitions: HashMap<String, TestDefinition>,
}

impl PackageTestRunner {
    /// Create a new test runner for the given package.
    ///
    /// # Arguments
    ///
    /// * `package_spec` - Package name or request (e.g., "my_package" or "my_package-1.0").
    ///
    /// # Errors
    ///
    /// Returns an error if the package cannot be found or loaded.
    pub fn new(package_spec: String) -> RezCoreResult<Self> {
        let working_dir = std::env::current_dir().map_err(|e| RezCoreError::from(e))?;

        let mut runner = Self {
            package_name: package_spec.clone(),
            working_dir: working_dir.clone(),
            verbose: 0,
            dry_run: false,
            stop_on_fail: false,
            test_results: Vec::new(),
            test_definitions: HashMap::new(),
        };

        // Load test definitions from package file in working directory
        runner.load_test_definitions(&working_dir, &package_spec)?;

        Ok(runner)
    }

    /// Set the working directory for test execution.
    #[must_use]
    pub fn with_working_dir(mut self, dir: PathBuf) -> Self {
        self.working_dir = dir;
        self
    }

    /// Set verbose output level.
    #[must_use]
    pub fn with_verbose(mut self, verbose: u8) -> Self {
        self.verbose = verbose;
        self
    }

    /// Set dry run mode.
    #[must_use]
    pub fn with_dry_run(mut self, dry_run: bool) -> Self {
        self.dry_run = dry_run;
        self
    }

    /// Set stop on fail behavior.
    #[must_use]
    pub fn with_stop_on_fail(mut self, stop_on_fail: bool) -> Self {
        self.stop_on_fail = stop_on_fail;
        self
    }

    /// Load test definitions from package.py or package.yaml in the given directory.
    fn load_test_definitions(
        &mut self,
        working_dir: &Path,
        _package_spec: &str,
    ) -> RezCoreResult<()> {
        // Try to find package.py or package.yaml in working_dir
        let candidates = [
            working_dir.join("package.py"),
            working_dir.join("package.yaml"),
            working_dir.join("package.yml"),
        ];

        for candidate in &candidates {
            if candidate.exists() {
                if let Ok(package) = PackageSerializer::load_from_file(candidate) {
                    self.extract_tests_from_package(&package);
                    return Ok(());
                }
            }
        }

        // If not found in working_dir, try configured package paths
        // (This would require access to RezCoreConfig)
        // For now, just return Ok with empty test_definitions

        Ok(())
    }

    /// Extract test definitions from a loaded package.
    fn extract_tests_from_package(&mut self, package: &Package) {
        // Package.tests is HashMap<String, String> where value is the command
        for (test_name, command_str) in &package.tests {
            let cmd = if command_str.contains(' ') {
                // Multi-word command, split it
                let parts: Vec<String> = command_str
                    .split_whitespace()
                    .map(|s| s.to_string())
                    .collect();
                TestCommand::List(parts)
            } else {
                TestCommand::String(command_str.clone())
            };

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
    }

    /// Get available test names from loaded package definition.
    ///
    /// # Errors
    ///
    /// Returns an error if test definitions cannot be retrieved.
    pub fn get_test_names(&self) -> RezCoreResult<Vec<String>> {
        if self.test_definitions.is_empty() {
            return Ok(vec![]);
        }

        let mut names: Vec<String> = self.test_definitions.keys().cloned().collect();
        names.sort();
        Ok(names)
    }

    /// Find requested test names (supports wildcards via fnmatch-style matching).
    ///
    /// # Arguments
    ///
    /// * `requested` - List of test name patterns (supports `*` wildcard).
    ///
    /// # Errors
    ///
    /// Returns an error if a pattern is invalid.
    pub fn find_requested_test_names(&self, requested: &[String]) -> RezCoreResult<Vec<String>> {
        let available_tests = self.get_test_names()?;

        if requested.is_empty() {
            return Ok(available_tests);
        }

        let mut matched_tests = Vec::new();

        for pattern in requested {
            if pattern.contains('*') {
                // Simple wildcard matching (fnmatch-style)
                let regex_pattern = pattern.replace('*', ".*");
                let regex = regex::Regex::new(&format!("^{}$", regex_pattern)).map_err(|e| {
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

    /// Run a specific test by name.
    ///
    /// # Arguments
    ///
    /// * `test_name` - Name of the test to run.
    ///
    /// # Errors
    ///
    /// Returns an error if the test cannot be run.
    pub fn run_test(&mut self, test_name: &str) -> RezCoreResult<i32> {
        if self.verbose > 0 {
            tracing::info!("Running test: {}", test_name);
        }

        let start_time = Instant::now();

        let (status, output, error, exit_code) = if self.dry_run {
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
            variant: None,
        };

        // Print test result
        match &status {
            TestStatus::Success => {
                if self.verbose > 0 {
                    tracing::info!("PASSED: {} ({}ms)", test_name, duration_ms);
                    if self.verbose > 1 && !output.is_empty() {
                        tracing::debug!("{}", output);
                    }
                }
            }
            TestStatus::Failed => {
                tracing::info!("FAILED: {} ({}ms)", test_name, duration_ms);
                if !output.is_empty() {
                    tracing::debug!("{}", output);
                }
                if let Some(ref err) = error {
                    tracing::error!("Error: {}", err);
                }
            }
            TestStatus::Skipped => {
                if self.verbose > 0 {
                    tracing::info!("SKIPPED: {} (skipped)", test_name);
                }
            }
            TestStatus::Error => {
                tracing::error!("ERROR: {} (error)", test_name);
                if let Some(ref err) = error {
                    tracing::error!("Error: {}", err);
                }
            }
        }

        self.test_results.push(result);

        if self.stop_on_fail && exit_code != 0 {
            return Ok(exit_code);
        }

        Ok(exit_code)
    }

    /// Execute a test command and return (status, stdout, stderr, exit_code).
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
            tracing::debug!("Executing: {} {}", program, args.join(" "));
        }

        let result = Command::new(&program)
            .args(&args)
            .current_dir(&self.working_dir)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output();

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

    /// Format test summary as String.
    fn format_summary(&self) -> String {
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

        let mut summary = String::new();
        summary.push_str("Test Summary:\n");
        summary.push_str(&format!("   Total:   {}\n", total));
        summary.push_str(&format!("   Passed:  {}\n", passed));
        summary.push_str(&format!("   Failed:  {}\n", failed));
        summary.push_str(&format!("   Errors:  {}\n", errors));
        summary.push_str(&format!("   Skipped: {}\n", skipped));

        if failed > 0 || errors > 0 {
            summary.push_str("\nSome tests failed!\n");
        } else if total > 0 {
            summary.push_str("\nAll tests passed!\n");
        }
        summary
    }

    /// Print test summary.
    pub fn print_summary(&self) {
        print!("{}", self.format_summary());
    }
}

// ── Package Test Results ──────────────────────────────────────────────────

/// Stores and displays test results across multiple `PackageTestRunner` instances.
#[derive(Debug, Clone, Default)]
pub struct PackageTestResults {
    /// Collected test results.
    results: Vec<TestResult>,
}

impl PackageTestResults {
    /// Create a new empty test results collector.
    #[must_use]
    pub fn new() -> Self {
        Self {
            results: Vec::new(),
        }
    }

    /// Add a test result.
    pub fn add_test_result(
        &mut self,
        test_name: String,
        variant: Option<String>,
        status: TestStatus,
        description: String,
    ) {
        // Compute exit_code before moving status
        let exit_code = if status == TestStatus::Success { 0 } else { 1 };
        let result = TestResult {
            name: test_name,
            status,
            output: description,
            error: None,
            duration_ms: 0,
            exit_code,
            variant,
        };
        self.results.push(result);
    }

    /// Get the number of tests.
    #[must_use]
    pub fn num_tests(&self) -> usize {
        self.results.len()
    }

    /// Get the number of successful tests.
    #[must_use]
    pub fn num_success(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Success)
            .count()
    }

    /// Get the number of failed tests.
    #[must_use]
    pub fn num_failed(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Failed)
            .count()
    }

    /// Get the number of skipped tests.
    #[must_use]
    pub fn num_skipped(&self) -> usize {
        self.results
            .iter()
            .filter(|r| r.status == TestStatus::Skipped)
            .count()
    }

    /// Format test results summary as String.
    fn format_summary(&self) -> String {
        let total = self.num_tests();
        let passed = self.num_success();
        let failed = self.num_failed();
        let errors = self
            .results
            .iter()
            .filter(|r| r.status == TestStatus::Error)
            .count();
        let skipped = self.num_skipped();

        let mut summary = String::new();
        summary.push_str("Test Results Summary:\n");
        summary.push_str(&format!("   Total:   {}\n", total));
        summary.push_str(&format!("   Passed:  {}\n", passed));
        summary.push_str(&format!("   Failed:  {}\n", failed));
        summary.push_str(&format!("   Errors:  {}\n", errors));
        summary.push_str(&format!("   Skipped: {}\n", skipped));

        // Print detailed results
        if total > 0 {
            summary.push_str("\nDetailed Results:\n");
            for result in &self.results {
                let status_str = match result.status {
                    TestStatus::Success => "PASSED",
                    TestStatus::Failed => "FAILED",
                    TestStatus::Skipped => "SKIPPED",
                    TestStatus::Error => "ERROR",
                };
                let variant_str = result.variant.as_deref().unwrap_or("(no variant)");
                summary.push_str(&format!(
                    "   [{}] {} on {} - {}\n",
                    status_str, result.name, variant_str, result.output
                ));
            }
        }

        if failed > 0 || errors > 0 {
            summary.push_str("\nSome tests failed!\n");
        } else if total > 0 {
            summary.push_str("\nAll tests passed!\n");
        }
        summary
    }

    /// Print test summary.
    pub fn print_summary(&self) {
        print!("{}", self.format_summary());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ── TestStatus ─────────────────────────────────────────────────────────

    #[test]
    fn test_test_status_equality() {
        assert_eq!(TestStatus::Success, TestStatus::Success);
        assert_ne!(TestStatus::Success, TestStatus::Failed);
    }

    // ── TestCommand ───────────────────────────────────────────────────────

    #[test]
    fn test_test_command_string() {
        let cmd = TestCommand::String("echo hello".to_string());
        match cmd {
            TestCommand::String(s) => assert_eq!(s, "echo hello"),
            _ => panic!("Expected TestCommand::String"),
        }
    }

    #[test]
    fn test_test_command_list() {
        let cmd = TestCommand::List(vec!["echo".to_string(), "hello".to_string()]);
        match cmd {
            TestCommand::List(parts) => {
                assert_eq!(parts.len(), 2);
                assert_eq!(parts[0], "echo");
                assert_eq!(parts[1], "hello");
            }
            _ => panic!("Expected TestCommand::List"),
        }
    }

    // ── TestDefinition ────────────────────────────────────────────────────

    #[test]
    fn test_test_definition_creation() {
        let def = TestDefinition {
            name: "unit".to_string(),
            command: TestCommand::String("python -m unittest".to_string()),
            requires: vec![],
            run_on: vec!["default".to_string()],
            on_variants: None,
        };
        assert_eq!(def.name, "unit");
        assert_eq!(def.run_on.len(), 1);
    }

    // ── PackageTestRunner ─────────────────────────────────────────────────

    #[test]
    fn test_package_test_runner_new() {
        // This test creates a runner but may not find a package
        // In a real test environment, we would have a test package
        let result = PackageTestRunner::new("nonexistent_package".to_string());
        // Should not panic, just have empty test definitions
        assert!(result.is_ok());
        let runner = result.unwrap();
        assert!(runner.get_test_names().unwrap().is_empty());
    }

    #[test]
    fn test_package_test_runner_with_working_dir() {
        let runner = PackageTestRunner::new("test".to_string())
            .unwrap()
            .with_working_dir(std::env::current_dir().unwrap())
            .with_verbose(1)
            .with_dry_run(true)
            .with_stop_on_fail(true);
        assert_eq!(runner.verbose, 1);
        assert!(runner.dry_run);
        assert!(runner.stop_on_fail);
    }

    #[test]
    fn test_find_requested_test_names_empty() {
        let runner = PackageTestRunner::new("test".to_string()).unwrap();
        let result = runner.find_requested_test_names(&[]).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_find_requested_test_names_with_pattern() {
        // Create a runner with mock test definitions
        let mut runner = PackageTestRunner::new("test".to_string()).unwrap();
        runner.test_definitions.insert(
            "unit_test".to_string(),
            TestDefinition {
                name: "unit_test".to_string(),
                command: TestCommand::String("python -m unittest".to_string()),
                requires: vec![],
                run_on: vec!["default".to_string()],
                on_variants: None,
            },
        );
        runner.test_definitions.insert(
            "integration_test".to_string(),
            TestDefinition {
                name: "integration_test".to_string(),
                command: TestCommand::String("python -m pytest".to_string()),
                requires: vec![],
                run_on: vec!["default".to_string()],
                on_variants: None,
            },
        );

        // Test wildcard matching
        let result = runner
            .find_requested_test_names(&["*test".to_string()])
            .unwrap();
        assert_eq!(result.len(), 2);
        assert!(result.contains(&"unit_test".to_string()));
        assert!(result.contains(&"integration_test".to_string()));
    }

    // ── PackageTestResults ───────────────────────────────────────────────

    #[test]
    fn test_package_test_results_new() {
        let results = PackageTestResults::new();
        assert_eq!(results.num_tests(), 0);
        assert_eq!(results.num_success(), 0);
    }

    #[test]
    fn test_package_test_results_add_result() {
        let mut results = PackageTestResults::new();
        results.add_test_result(
            "unit_test".to_string(),
            None,
            TestStatus::Success,
            "Test passed".to_string(),
        );
        assert_eq!(results.num_tests(), 1);
        assert_eq!(results.num_success(), 1);
        assert_eq!(results.num_failed(), 0);
    }

    #[test]
    fn test_package_test_results_multiple_results() {
        let mut results = PackageTestResults::new();
        results.add_test_result(
            "test1".to_string(),
            None,
            TestStatus::Success,
            "Passed".to_string(),
        );
        results.add_test_result(
            "test2".to_string(),
            None,
            TestStatus::Failed,
            "Failed".to_string(),
        );
        results.add_test_result(
            "test3".to_string(),
            None,
            TestStatus::Skipped,
            "Skipped".to_string(),
        );

        assert_eq!(results.num_tests(), 3);
        assert_eq!(results.num_success(), 1);
        assert_eq!(results.num_failed(), 1);
        assert_eq!(results.num_skipped(), 1);
    }
}
