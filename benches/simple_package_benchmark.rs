//! Simple Package System Benchmark
//!
//! A standalone benchmark for the Package system without external dependencies

use criterion::{black_box, criterion_group, criterion_main, Criterion, BenchmarkId};
use rez_core_package::{Package, PackageSerializer, PackageFormat};
use rez_core_version::Version;

/// Benchmark package creation with different complexity levels
fn bench_package_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_creation");
    
    // Simple package creation
    group.bench_function("simple_package", |b| {
        b.iter(|| {
            black_box(Package::new("test_package".to_string()))
        })
    });

    // Package with version
    group.bench_function("package_with_version", |b| {
        b.iter(|| {
            let mut package = Package::new("test_package".to_string());
            let version = Version::parse("1.0.0").unwrap();
            package.set_version(version);
            black_box(package)
        })
    });

    // Complex package creation
    group.bench_function("complex_package", |b| {
        b.iter(|| {
            let mut package = Package::new("complex_package".to_string());
            package.set_version(Version::parse("2.1.3").unwrap());
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

    // YAML serialization benchmarks
    group.bench_function("simple_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_yaml(&simple_package).unwrap())
        })
    });

    group.bench_function("complex_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_yaml(&complex_package).unwrap())
        })
    });

    group.bench_function("large_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_yaml(&large_package).unwrap())
        })
    });

    // JSON serialization benchmarks
    group.bench_function("simple_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_json(&simple_package).unwrap())
        })
    });

    group.bench_function("complex_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_json(&complex_package).unwrap())
        })
    });

    group.bench_function("large_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_json(&large_package).unwrap())
        })
    });

    // Python serialization benchmarks
    group.bench_function("simple_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_python(&simple_package).unwrap())
        })
    });

    group.bench_function("complex_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_python(&complex_package).unwrap())
        })
    });

    group.bench_function("large_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::save_to_python(&large_package).unwrap())
        })
    });

    group.finish();
}

/// Benchmark package deserialization performance
fn bench_package_deserialization(c: &mut Criterion) {
    let mut group = c.benchmark_group("package_deserialization");
    
    // Prepare serialized test data
    let simple_package = create_simple_package();
    let complex_package = create_complex_package();
    let large_package = create_large_package();

    let simple_yaml = PackageSerializer::save_to_yaml(&simple_package).unwrap();
    let complex_yaml = PackageSerializer::save_to_yaml(&complex_package).unwrap();
    let large_yaml = PackageSerializer::save_to_yaml(&large_package).unwrap();

    let simple_json = PackageSerializer::save_to_json(&simple_package).unwrap();
    let complex_json = PackageSerializer::save_to_json(&complex_package).unwrap();
    let large_json = PackageSerializer::save_to_json(&large_package).unwrap();

    let simple_python = PackageSerializer::save_to_python(&simple_package).unwrap();
    let complex_python = PackageSerializer::save_to_python(&complex_package).unwrap();
    let large_python = PackageSerializer::save_to_python(&large_package).unwrap();

    // YAML deserialization benchmarks
    group.bench_function("simple_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_yaml(&simple_yaml).unwrap())
        })
    });

    group.bench_function("complex_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_yaml(&complex_yaml).unwrap())
        })
    });

    group.bench_function("large_yaml", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_yaml(&large_yaml).unwrap())
        })
    });

    // JSON deserialization benchmarks
    group.bench_function("simple_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_json(&simple_json).unwrap())
        })
    });

    group.bench_function("complex_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_json(&complex_json).unwrap())
        })
    });

    group.bench_function("large_json", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_json(&large_json).unwrap())
        })
    });

    // Python deserialization benchmarks
    group.bench_function("simple_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_python(&simple_python).unwrap())
        })
    });

    group.bench_function("complex_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_python(&complex_python).unwrap())
        })
    });

    group.bench_function("large_python", |b| {
        b.iter(|| {
            black_box(PackageSerializer::load_from_python(&large_python).unwrap())
        })
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
        b.iter(|| {
            black_box(simple_package.validate().is_ok())
        })
    });

    group.bench_function("complex_valid", |b| {
        b.iter(|| {
            black_box(complex_package.validate().is_ok())
        })
    });

    group.bench_function("large_valid", |b| {
        b.iter(|| {
            black_box(large_package.validate().is_ok())
        })
    });

    group.bench_function("invalid_package", |b| {
        b.iter(|| {
            black_box(invalid_package.validate().is_err())
        })
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
                    let mut package = Package::new("test_package".to_string());
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
        b.iter(|| {
            black_box(package_with_variants.num_variants())
        })
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
        b.iter(|| {
            black_box(simple_package.clone())
        })
    });

    group.bench_function("complex_clone", |b| {
        b.iter(|| {
            black_box(complex_package.clone())
        })
    });

    group.bench_function("large_clone", |b| {
        b.iter(|| {
            black_box(large_package.clone())
        })
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
                    let mut package = Package::new("test_package".to_string());
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
                    let mut package = Package::new("test_package".to_string());
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

// Helper functions for creating test packages
fn create_simple_package() -> Package {
    let mut package = Package::new("simple_package".to_string());
    package.set_version(Version::parse("1.0.0").unwrap());
    package.set_description("A simple test package".to_string());
    package
}

fn create_complex_package() -> Package {
    let mut package = Package::new("complex_package".to_string());
    package.set_version(Version::parse("2.1.3").unwrap());
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

fn create_large_package() -> Package {
    let mut package = Package::new("large_package".to_string());
    package.set_version(Version::parse("5.2.1").unwrap());
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

fn create_invalid_package() -> Package {
    // Create a package with an empty name (invalid)
    Package::new("".to_string())
}

fn create_package_with_variants(variant_count: usize) -> Package {
    let mut package = Package::new("variant_test_package".to_string());
    package.set_version(Version::parse("1.0.0").unwrap());

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
    bench_package_deserialization,
    bench_package_validation,
    bench_package_variants,
    bench_package_cloning,
    bench_package_requirements
);

criterion_main!(package_benches);
