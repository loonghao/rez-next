//! Standalone Package System Benchmark
//!
//! A completely independent benchmark for Package system performance testing
//! without any external dependencies to avoid compilation issues.

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use std::collections::HashMap;

/// Simplified Package structure for benchmarking
#[derive(Debug, Clone)]
pub struct TestPackage {
    pub name: String,
    pub version: Option<String>,
    pub description: Option<String>,
    pub authors: Vec<String>,
    pub requires: Vec<String>,
    pub build_requires: Vec<String>,
    pub private_build_requires: Vec<String>,
    pub variants: Vec<Vec<String>>,
    pub tools: Vec<String>,
    pub commands: Option<String>,
    pub uuid: Option<String>,
    pub config: HashMap<String, String>,
    pub help: Option<String>,
    pub relocatable: Option<bool>,
    pub cachable: Option<bool>,
    pub timestamp: Option<i64>,
    pub revision: Option<String>,
    pub changelog: Option<String>,
    pub release_message: Option<String>,
    pub previous_version: Option<String>,
    pub previous_revision: Option<String>,
    pub vcs: Option<String>,
    pub format_version: Option<i32>,
    pub base: Option<String>,
    pub has_plugins: Option<bool>,
    pub plugin_for: Vec<String>,
    pub hashed_variants: Option<bool>,
}

impl TestPackage {
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            description: None,
            authors: Vec::new(),
            requires: Vec::new(),
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            variants: Vec::new(),
            tools: Vec::new(),
            commands: None,
            uuid: None,
            config: HashMap::new(),
            help: None,
            relocatable: None,
            cachable: None,
            timestamp: None,
            revision: None,
            changelog: None,
            release_message: None,
            previous_version: None,
            previous_revision: None,
            vcs: None,
            format_version: None,
            base: None,
            has_plugins: None,
            plugin_for: Vec::new(),
            hashed_variants: None,
        }
    }

    pub fn set_version(&mut self, version: String) {
        self.version = Some(version);
    }

    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    pub fn add_author(&mut self, author: String) {
        self.authors.push(author);
    }

    pub fn add_requirement(&mut self, requirement: String) {
        self.requires.push(requirement);
    }

    pub fn add_build_requirement(&mut self, requirement: String) {
        self.build_requires.push(requirement);
    }

    pub fn add_private_build_requirement(&mut self, requirement: String) {
        self.private_build_requires.push(requirement);
    }

    pub fn add_variant(&mut self, variant: Vec<String>) {
        self.variants.push(variant);
    }

    pub fn add_tool(&mut self, tool: String) {
        self.tools.push(tool);
    }

    pub fn set_commands(&mut self, commands: String) {
        self.commands = Some(commands);
    }

    pub fn num_variants(&self) -> usize {
        self.variants.len()
    }

    pub fn qualified_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}-{}", self.name, version),
            None => self.name.clone(),
        }
    }

    pub fn as_exact_requirement(&self) -> String {
        match &self.version {
            Some(version) => format!("{}=={}", self.name, version),
            None => self.name.clone(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        // Check required fields
        if self.name.is_empty() {
            return Err("Package name cannot be empty".to_string());
        }

        // Validate name format (alphanumeric, underscore, hyphen)
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(format!(
                "Invalid package name '{}': only alphanumeric, underscore, and hyphen allowed",
                self.name
            ));
        }

        // Validate version if present
        if let Some(ref version) = self.version {
            if version.is_empty() {
                return Err("Package version cannot be empty".to_string());
            }
        }

        // Validate requirements format
        for req in &self.requires {
            if req.is_empty() {
                return Err("Requirement cannot be empty".to_string());
            }
        }

        for req in &self.build_requires {
            if req.is_empty() {
                return Err("Build requirement cannot be empty".to_string());
            }
        }

        for req in &self.private_build_requires {
            if req.is_empty() {
                return Err("Private build requirement cannot be empty".to_string());
            }
        }

        // Validate variants
        for variant in &self.variants {
            for req in variant {
                if req.is_empty() {
                    return Err("Variant requirement cannot be empty".to_string());
                }
            }
        }

        Ok(())
    }

    /// Serialize to JSON-like string (simplified)
    pub fn to_json_string(&self) -> String {
        format!(
            r#"{{"name":"{}","version":"{}","description":"{}","authors":{:?},"requires":{:?},"variants":{:?}}}"#,
            self.name,
            self.version.as_deref().unwrap_or(""),
            self.description.as_deref().unwrap_or(""),
            self.authors,
            self.requires,
            self.variants
        )
    }

    /// Serialize to YAML-like string (simplified)
    pub fn to_yaml_string(&self) -> String {
        let mut yaml = format!("name: {}\n", self.name);
        if let Some(ref version) = self.version {
            yaml.push_str(&format!("version: {}\n", version));
        }
        if let Some(ref description) = self.description {
            yaml.push_str(&format!("description: {}\n", description));
        }
        if !self.authors.is_empty() {
            yaml.push_str("authors:\n");
            for author in &self.authors {
                yaml.push_str(&format!("  - {}\n", author));
            }
        }
        if !self.requires.is_empty() {
            yaml.push_str("requires:\n");
            for req in &self.requires {
                yaml.push_str(&format!("  - {}\n", req));
            }
        }
        yaml
    }

    /// Serialize to Python-like string (simplified)
    pub fn to_python_string(&self) -> String {
        let mut python = format!("name = \"{}\"\n", self.name);
        if let Some(ref version) = self.version {
            python.push_str(&format!("version = \"{}\"\n", version));
        }
        if let Some(ref description) = self.description {
            python.push_str(&format!("description = \"{}\"\n", description));
        }
        if !self.authors.is_empty() {
            python.push_str("authors = [\n");
            for author in &self.authors {
                python.push_str(&format!("    \"{}\",\n", author));
            }
            python.push_str("]\n");
        }
        if !self.requires.is_empty() {
            python.push_str("requires = [\n");
            for req in &self.requires {
                python.push_str(&format!("    \"{}\",\n", req));
            }
            python.push_str("]\n");
        }
        python
    }
}

/// Benchmark package creation with different complexity levels
fn bench_package_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_creation");

    // Simple package creation
    group.bench_function("simple_package", |b| {
        b.iter(|| black_box(TestPackage::new("test_package".to_string())))
    });

    // Package with version
    group.bench_function("package_with_version", |b| {
        b.iter(|| {
            let mut package = TestPackage::new("test_package".to_string());
            package.set_version("1.0.0".to_string());
            black_box(package)
        })
    });

    // Complex package creation
    group.bench_function("complex_package", |b| {
        b.iter(|| {
            let mut package = TestPackage::new("complex_package".to_string());
            package.set_version("2.1.3".to_string());
            package.set_description("A complex test package".to_string());
            package.add_author("Test Author".to_string());
            package.add_requirement("python>=3.8".to_string());
            package.add_build_requirement("cmake".to_string());
            package.add_tool("python".to_string());
            black_box(package)
        })
    });

    group.finish();
}

/// Benchmark package serialization performance across different formats
fn bench_package_serialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_serialization");

    // Create test packages of different complexity
    let simple_package = create_simple_package();
    let complex_package = create_complex_package();
    let large_package = create_large_package();

    // JSON serialization benchmarks
    group.bench_function("simple_json", |b| {
        b.iter(|| black_box(simple_package.to_json_string()))
    });

    group.bench_function("complex_json", |b| {
        b.iter(|| black_box(complex_package.to_json_string()))
    });

    group.bench_function("large_json", |b| {
        b.iter(|| black_box(large_package.to_json_string()))
    });

    // YAML serialization benchmarks
    group.bench_function("simple_yaml", |b| {
        b.iter(|| black_box(simple_package.to_yaml_string()))
    });

    group.bench_function("complex_yaml", |b| {
        b.iter(|| black_box(complex_package.to_yaml_string()))
    });

    group.bench_function("large_yaml", |b| {
        b.iter(|| black_box(large_package.to_yaml_string()))
    });

    // Python serialization benchmarks
    group.bench_function("simple_python", |b| {
        b.iter(|| black_box(simple_package.to_python_string()))
    });

    group.bench_function("complex_python", |b| {
        b.iter(|| black_box(complex_package.to_python_string()))
    });

    group.bench_function("large_python", |b| {
        b.iter(|| black_box(large_package.to_python_string()))
    });

    group.finish();
}

/// Benchmark package validation performance
fn bench_package_validation(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_validation");

    let simple_package = create_simple_package();
    let complex_package = create_complex_package();
    let large_package = create_large_package();
    let invalid_package = create_invalid_package();

    group.bench_function("simple_valid", |b| {
        b.iter(|| black_box(simple_package.validate().is_ok()))
    });

    group.bench_function("complex_valid", |b| {
        b.iter(|| black_box(complex_package.validate().is_ok()))
    });

    group.bench_function("large_valid", |b| {
        b.iter(|| black_box(large_package.validate().is_ok()))
    });

    group.bench_function("invalid_package", |b| {
        b.iter(|| black_box(invalid_package.validate().is_err()))
    });

    group.finish();
}

/// Benchmark package variant handling performance
fn bench_package_variants(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_variants");

    // Test with different numbers of variants
    for variant_count in [1, 5, 10, 25, 50].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_variants", variant_count),
            variant_count,
            |b, &variant_count| {
                b.iter(|| {
                    let mut package = TestPackage::new("test_package".to_string());
                    for i in 0..variant_count {
                        let variant = vec![
                            format!("python-{}", i % 3 + 3),
                            format!("platform-{}", if i % 2 == 0 { "linux" } else { "windows" }),
                        ];
                        package.add_variant(variant);
                    }
                    black_box(package)
                })
            },
        );
    }

    // Test variant access performance
    let package_with_variants = create_package_with_variants(50);
    group.bench_function("access_variants", |b| {
        b.iter(|| black_box(package_with_variants.num_variants()))
    });

    group.finish();
}

/// Benchmark package cloning performance
fn bench_package_cloning(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_cloning");

    let simple_package = create_simple_package();
    let complex_package = create_complex_package();
    let large_package = create_large_package();

    group.bench_function("simple_clone", |b| {
        b.iter(|| black_box(simple_package.clone()))
    });

    group.bench_function("complex_clone", |b| {
        b.iter(|| black_box(complex_package.clone()))
    });

    group.bench_function("large_clone", |b| {
        b.iter(|| black_box(large_package.clone()))
    });

    group.finish();
}

/// Benchmark package requirements processing
fn bench_package_requirements(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_requirements");

    // Test adding different numbers of requirements
    for req_count in [1, 10, 50, 100, 500].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_requirements", req_count),
            req_count,
            |b, &req_count| {
                b.iter(|| {
                    let mut package = TestPackage::new("test_package".to_string());
                    for i in 0..req_count {
                        package.add_requirement(format!("package{}>={}.0.0", i, i % 10));
                    }
                    black_box(package)
                })
            },
        );
    }

    // Test adding build requirements
    for req_count in [1, 10, 50, 100].iter() {
        group.bench_with_input(
            BenchmarkId::new("add_build_requirements", req_count),
            req_count,
            |b, &req_count| {
                b.iter(|| {
                    let mut package = TestPackage::new("test_package".to_string());
                    for i in 0..req_count {
                        package.add_build_requirement(format!("build_tool{}>={}.0", i, i % 5));
                    }
                    black_box(package)
                })
            },
        );
    }

    group.finish();
}

/// Benchmark package string operations
fn bench_package_string_operations(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_string_operations");

    let simple_package = create_simple_package();
    let complex_package = create_complex_package();
    let large_package = create_large_package();

    group.bench_function("simple_qualified_name", |b| {
        b.iter(|| black_box(simple_package.qualified_name()))
    });

    group.bench_function("complex_qualified_name", |b| {
        b.iter(|| black_box(complex_package.qualified_name()))
    });

    group.bench_function("large_qualified_name", |b| {
        b.iter(|| black_box(large_package.qualified_name()))
    });

    group.bench_function("simple_exact_requirement", |b| {
        b.iter(|| black_box(simple_package.as_exact_requirement()))
    });

    group.bench_function("complex_exact_requirement", |b| {
        b.iter(|| black_box(complex_package.as_exact_requirement()))
    });

    group.bench_function("large_exact_requirement", |b| {
        b.iter(|| black_box(large_package.as_exact_requirement()))
    });

    group.finish();
}

// Helper functions for creating test packages
fn create_simple_package() -> TestPackage {
    let mut package = TestPackage::new("simple_package".to_string());
    package.set_version("1.0.0".to_string());
    package.set_description("A simple test package".to_string());
    package
}

fn create_complex_package() -> TestPackage {
    let mut package = TestPackage::new("complex_package".to_string());
    package.set_version("2.1.3".to_string());
    package.set_description("A complex test package with multiple features".to_string());

    // Add authors
    package.add_author("John Doe".to_string());
    package.add_author("Jane Smith".to_string());

    // Add requirements
    package.add_requirement("python>=3.8".to_string());
    package.add_requirement("numpy>=1.20.0".to_string());
    package.add_requirement("scipy>=1.7.0".to_string());

    // Add build requirements
    package.add_build_requirement("cmake>=3.16".to_string());
    package.add_build_requirement("gcc>=9.0".to_string());

    // Add tools
    package.add_tool("python".to_string());
    package.add_tool("pip".to_string());

    // Add variants
    package.add_variant(vec!["python-3.8".to_string(), "platform-linux".to_string()]);
    package.add_variant(vec!["python-3.9".to_string(), "platform-linux".to_string()]);
    package.add_variant(vec!["python-3.8".to_string(), "platform-windows".to_string()]);

    package
}

fn create_large_package() -> TestPackage {
    let mut package = TestPackage::new("large_package".to_string());
    package.set_version("5.2.1".to_string());
    package.set_description("A large test package with many dependencies and variants".to_string());

    // Add many authors
    for i in 0..20 {
        package.add_author(format!("Author {}", i));
    }

    // Add many requirements
    for i in 0..100 {
        package.add_requirement(format!("package{}>={}.0.0", i, i % 10));
    }

    // Add many build requirements
    for i in 0..50 {
        package.add_build_requirement(format!("build_tool{}>={}.0", i, i % 5));
    }

    // Add many tools
    for i in 0..30 {
        package.add_tool(format!("tool{}", i));
    }

    // Add many variants
    for i in 0..50 {
        let variant = vec![
            format!("python-{}", i % 3 + 3),
            format!("platform-{}", if i % 2 == 0 { "linux" } else { "windows" }),
            format!("arch-{}", if i % 4 < 2 { "x86_64" } else { "aarch64" }),
        ];
        package.add_variant(variant);
    }

    package
}

fn create_invalid_package() -> TestPackage {
    // Create a package with an empty name (invalid)
    TestPackage::new("".to_string())
}

fn create_package_with_variants(variant_count: usize) -> TestPackage {
    let mut package = TestPackage::new("variant_test_package".to_string());
    package.set_version("1.0.0".to_string());

    for i in 0..variant_count {
        let variant = vec![
            format!("python-{}", i % 3 + 3),
            format!("platform-{}", if i % 2 == 0 { "linux" } else { "windows" }),
        ];
        package.add_variant(variant);
    }

    package
}

// Criterion benchmark groups
criterion_group!(
    package_benches,
    bench_package_creation,
    bench_package_serialization,
    bench_package_validation,
    bench_package_variants,
    bench_package_cloning,
    bench_package_requirements,
    bench_package_string_operations
);

criterion_main!(package_benches);
