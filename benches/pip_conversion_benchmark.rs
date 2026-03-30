//! pip-to-rez conversion performance benchmarks
//!
//! Measures throughput of:
//! - Package name normalization
//! - pip version specifier → rez version range conversion
//! - Bulk package requirement parsing after pip conversion
//! - Pip-converted package resolution simulation

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement};

// ── Name normalization ─────────────────────────────────────────────────────

fn normalize_pip_name(name: &str) -> String {
    name.to_lowercase().replace('_', "-")
}

fn bench_name_normalization(c: &mut Criterion) {
    let names = vec![
        "NumPy", "Pillow", "PyYAML", "scikit_learn", "Django",
        "Twisted", "SQLAlchemy", "Werkzeug", "MarkupSafe", "Jinja2",
        "cryptography", "cffi", "six", "python_dateutil", "pytz",
        "certifi", "urllib3", "idna", "charset_normalizer", "requests",
    ];

    c.bench_function("pip_name_normalization_20_pkgs", |b| {
        b.iter(|| {
            for name in &names {
                black_box(normalize_pip_name(black_box(name)));
            }
        })
    });

    let mut group = c.benchmark_group("pip_name_normalization_scale");
    for count in [10, 100, 1000].iter() {
        let large_names: Vec<String> = (0..*count)
            .map(|i| format!("Package_Name_{}", i))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                for name in &large_names {
                    black_box(normalize_pip_name(black_box(name.as_str())));
                }
            })
        });
    }
    group.finish();
}

// ── Version specifier conversion ───────────────────────────────────────────

fn pip_ver_to_rez(spec: &str) -> String {
    if let Some(ver) = spec.strip_prefix("==") {
        return ver.to_string();
    }
    if let Some(ver) = spec.strip_prefix(">=") {
        return format!("{}+", ver);
    }
    if let Some(ver) = spec.strip_prefix("<") {
        return format!("<{}", ver);
    }
    spec.to_string()
}

fn bench_version_conversion(c: &mut Criterion) {
    let pip_specs = vec![
        "==1.25.0",
        ">=3.9",
        ">=1.0,<2.0",
        "<2.0",
        ">=2.28.0",
        "==6.0",
        ">=1.7.0",
        ">=0.23.0,<0.24.0",
        ">=21.1",
        ">=2021.3",
    ];

    c.bench_function("pip_version_conversion_10_specs", |b| {
        b.iter(|| {
            for spec in &pip_specs {
                black_box(pip_ver_to_rez(black_box(spec)));
            }
        })
    });
}

// ── Requirement parsing after conversion ──────────────────────────────────

fn bench_pip_converted_req_parsing(c: &mut Criterion) {
    // Simulate pip packages converted to rez requires
    let rez_requires = vec![
        "numpy-1.25.0",
        "scipy-1.11.0",
        "matplotlib-3.7.0",
        "pandas-2.0.0",
        "requests-2.31.0",
        "pillow-10.0.0",
        "pyyaml-6.0",
        "cryptography-41.0.0",
        "certifi-2023.7.22",
        "urllib3-2.0.0",
        "django-4.2.0",
        "flask-2.3.0",
        "sqlalchemy-2.0.0",
        "celery-5.3.0",
        "boto3-1.28.0",
    ];

    c.bench_function("pip_converted_req_parse_15_pkgs", |b| {
        b.iter(|| {
            let reqs: Vec<PackageRequirement> = rez_requires
                .iter()
                .map(|r| PackageRequirement::parse(black_box(r)).unwrap())
                .collect();
            black_box(reqs);
        })
    });

    let mut group = c.benchmark_group("pip_converted_req_parse_scale");
    for count in [10, 50, 100, 500].iter() {
        let large_reqs: Vec<String> = (0..*count)
            .map(|i| format!("package-{}-{}.{}.0", i, i % 10, i % 5))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let reqs: Vec<_> = large_reqs
                    .iter()
                    .filter_map(|r| PackageRequirement::parse(black_box(r.as_str())).ok())
                    .collect();
                black_box(reqs);
            })
        });
    }
    group.finish();
}

// ── Version range satisfaction check (like solver would do) ───────────────

fn bench_pip_version_satisfaction(c: &mut Criterion) {
    // Simulate checking if pip-installed versions satisfy rez requirements
    let installed_versions: Vec<(String, Version)> = vec![
        ("numpy", "1.25.0"),
        ("scipy", "1.11.0"),
        ("matplotlib", "3.7.0"),
        ("pandas", "2.0.0"),
        ("requests", "2.31.0"),
    ].into_iter().map(|(n, v)| (n.to_string(), Version::parse(v).unwrap())).collect();

    let requirements: Vec<(String, VersionRange)> = vec![
        ("numpy", "1.20+"),
        ("scipy", "1.10+"),
        ("matplotlib", "3.0+<4.0"),
        ("pandas", "1.5+"),
        ("requests", "2.28+"),
    ].into_iter().map(|(n, r)| (n.to_string(), VersionRange::parse(r).unwrap())).collect();

    c.bench_function("pip_version_satisfaction_5_pkgs", |b| {
        b.iter(|| {
            let all_satisfied = requirements.iter().all(|(req_name, range)| {
                installed_versions
                    .iter()
                    .find(|(name, _)| name == req_name)
                    .map(|(_, ver)| range.contains(ver))
                    .unwrap_or(false)
            });
            black_box(all_satisfied)
        })
    });
}

// ── Bulk pip metadata processing ──────────────────────────────────────────

fn bench_bulk_pip_metadata_conversion(c: &mut Criterion) {
    // Simulate processing a large pip freeze output
    let pip_packages: Vec<(&str, &str)> = vec![
        ("numpy", "1.25.0"), ("scipy", "1.11.0"), ("matplotlib", "3.7.0"),
        ("pandas", "2.0.0"), ("requests", "2.31.0"), ("pillow", "10.0.0"),
        ("pyyaml", "6.0.0"), ("cryptography", "41.0.0"), ("certifi", "2023.7.22"),
        ("urllib3", "2.0.0"), ("django", "4.2.0"), ("flask", "2.3.0"),
        ("sqlalchemy", "2.0.0"), ("celery", "5.3.0"), ("boto3", "1.28.0"),
        ("paramiko", "3.3.0"), ("aiohttp", "3.8.0"), ("fastapi", "0.103.0"),
        ("pydantic", "2.3.0"), ("httpx", "0.25.0"),
    ];

    c.bench_function("pip_bulk_metadata_conversion_20_pkgs", |b| {
        b.iter(|| {
            let rez_pkgs: Vec<Package> = pip_packages
                .iter()
                .map(|(name, ver)| {
                    let rez_name = normalize_pip_name(black_box(name));
                    let mut pkg = Package::new(rez_name);
                    pkg.version = Some(Version::parse(black_box(ver)).unwrap());
                    pkg
                })
                .collect();
            black_box(rez_pkgs)
        })
    });

    let mut group = c.benchmark_group("pip_bulk_metadata_scale");
    for count in [20, 100, 500, 1000].iter() {
        let large_pkgs: Vec<(String, String)> = (0..*count)
            .map(|i| (format!("package_{}", i), format!("{}.{}.0", i / 10, i % 10)))
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(count), count, |b, _| {
            b.iter(|| {
                let rez_pkgs: Vec<Package> = large_pkgs
                    .iter()
                    .map(|(name, ver)| {
                        let rez_name = normalize_pip_name(black_box(name.as_str()));
                        let mut pkg = Package::new(rez_name);
                        pkg.version = Version::parse(black_box(ver.as_str())).ok();
                        pkg
                    })
                    .collect();
                black_box(rez_pkgs)
            })
        });
    }
    group.finish();
}

criterion_group!(
    pip_benches,
    bench_name_normalization,
    bench_version_conversion,
    bench_pip_converted_req_parsing,
    bench_pip_version_satisfaction,
    bench_bulk_pip_metadata_conversion,
);
criterion_main!(pip_benches);
