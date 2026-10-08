use std::collections::HashMap;
use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};

use rez_next_context::ContextConfig;
use rez_next_runtime::{RezRuntime, RezRuntimeError};
use rez_next_solver::SolverConfig;

const CHILD_ARGUMENTS: [&str; 11] = [
    "--exact",
    "test_command_child_probe",
    "--nocapture",
    "--skip",
    "with spaces",
    "--skip",
    "\"quoted\"",
    "--skip",
    "$(literal)",
    "--skip",
    "&literal",
];

fn write_package(repository: &Path, name: &str, version: &str, body: &str) -> PathBuf {
    let root = repository.join(name).join(version);
    fs::create_dir_all(&root).expect("package directory");
    fs::write(
        root.join("package.py"),
        format!("name = '{name}'\nversion = '{version}'\n{body}\n"),
    )
    .expect("package definition");
    root
}

fn runtime() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().expect("test runtime")
}

#[test]
fn test_resolve_uses_materialized_roots_and_records_context_environment() {
    let temporary = tempfile::tempdir().unwrap();
    let repository = temporary.path().join("arbitrary-cache").join("unpacked");
    let root = write_package(
        &repository,
        "tool",
        "1.0",
        "def commands():\n    env.setenv('CUSTOM_ROOT', '{root}')",
    );
    let resolved = runtime()
        .block_on(
            RezRuntime::new([&repository])
                .unwrap()
                .resolve(["tool-1.0"]),
        )
        .unwrap();

    assert_eq!(
        resolved.environment().get("CUSTOM_ROOT"),
        Some(&root.to_string_lossy().into_owned())
    );
    assert_eq!(
        resolved.environment().get("TOOL_ROOT"),
        Some(&root.to_string_lossy().into_owned())
    );
    assert_eq!(
        resolved
            .environment()
            .get("REZ_USED_REQUEST")
            .map(String::as_str),
        Some("tool-1.0")
    );
    assert_eq!(
        resolved
            .environment()
            .get("REZ_USED_RESOLVE")
            .map(String::as_str),
        Some("tool-1.0")
    );
    assert_eq!(
        resolved.environment().get("REZ_USED_PACKAGES_PATH"),
        Some(
            &std::env::join_paths([repository])
                .unwrap()
                .to_string_lossy()
                .into_owned()
        )
    );
    let (context, environment) = resolved.into_parts();
    assert_eq!(context.environment_vars, environment);
    assert_eq!(
        context.get_package("tool").unwrap().root(),
        Some(root.to_string_lossy().into_owned())
    );
}

#[test]
fn test_resolve_includes_transitive_dependencies_and_environment_actions() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(
        temporary.path(),
        "base",
        "1.0",
        "def commands():\n    env.setenv('SDK_BASE', 'active')",
    );
    write_package(temporary.path(), "middle", "1.0", "requires = ['base-1.0']");
    write_package(temporary.path(), "tool", "1.0", "requires = ['middle-1.0']");
    let resolved = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["tool-1.0"]),
        )
        .unwrap();

    assert_eq!(resolved.context().package_count(), 3);
    assert!(resolved.context().contains_package("base"));
    assert!(resolved.context().contains_package("middle"));
    assert_eq!(
        resolved.environment().get("SDK_BASE").map(String::as_str),
        Some("active")
    );
}

#[test]
fn test_resolve_rejects_transitive_conflicts() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(temporary.path(), "base", "1.0", "");
    write_package(temporary.path(), "tool", "1.0", "requires = ['base-1.0']");
    let error = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["tool-1.0", "!base"]),
        )
        .unwrap_err();
    assert!(matches!(error, RezRuntimeError::Resolve(_)));
}

#[test]
fn test_with_solver_config_cannot_enable_partial_environment() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(
        temporary.path(),
        "tool",
        "1.0",
        "requires = ['missing-1.0']",
    );
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_solver_config(SolverConfig::default());
    let error = runtime()
        .block_on(resolver.resolve(["tool-1.0"]))
        .unwrap_err();
    assert!(matches!(error, RezRuntimeError::Resolve(_)));
}

#[test]
fn test_with_target_selects_matching_platform_and_arch_variant() {
    let temporary = tempfile::tempdir().unwrap();
    for (name, version) in [
        ("platform", "linux"),
        ("platform", "windows"),
        ("platform", "windows.11"),
        ("arch", "aarch64"),
        ("arch", "x86_64"),
    ] {
        write_package(temporary.path(), name, version, "");
    }
    let root = write_package(
        temporary.path(),
        "tool",
        "1.0",
        "variants = [['platform-linux', 'arch-x86_64'], ['platform-windows', 'arch-aarch64'], ['platform-windows', 'arch-x86_64']]\ndef commands():\n    env.setenv('SELECTED_ROOT', '{root}')",
    );
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap();
    let resolved = runtime()
        .block_on(resolver.resolve(["tool-1.0", "platform", "arch-x86_64"]))
        .unwrap();
    let expected_root = root.join("platform-windows").join("arch-x86_64");

    assert_eq!(resolved.context().platform.as_deref(), Some("windows"));
    assert_eq!(resolved.context().arch.as_deref(), Some("x86_64"));
    assert_eq!(
        resolved
            .context()
            .get_package("platform")
            .unwrap()
            .version
            .as_ref()
            .unwrap()
            .as_str(),
        "windows"
    );
    assert_eq!(
        resolved
            .context()
            .get_package("arch")
            .unwrap()
            .version
            .as_ref()
            .unwrap()
            .as_str(),
        "x86_64"
    );
    assert_eq!(
        resolved.environment().get("SELECTED_ROOT"),
        Some(&expected_root.to_string_lossy().into_owned())
    );
}

#[test]
fn test_with_target_rejects_explicit_incompatible_weak_and_conflict_requests() {
    let temporary = tempfile::tempdir().unwrap();
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap();
    let executor = runtime();
    for request in [
        "platform-linux",
        "arch-aarch64",
        "~platform-windows",
        "!platform-linux",
        "!arch",
        "~arch",
    ] {
        let error = executor.block_on(resolver.resolve([request])).unwrap_err();
        assert!(
            matches!(error, RezRuntimeError::IncompatibleTargetRequirement { requirement, .. } if requirement == request)
        );
    }
}

#[test]
fn test_with_target_rejects_versionless_target_packages() {
    let temporary = tempfile::tempdir().unwrap();
    let platform = temporary.path().join("platform");
    fs::create_dir_all(&platform).unwrap();
    fs::write(platform.join("package.py"), "name = 'platform'\n").unwrap();
    write_package(temporary.path(), "arch", "x86_64", "");
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap();
    let error = runtime()
        .block_on(resolver.resolve(Vec::<&str>::new()))
        .unwrap_err();
    assert!(matches!(error, RezRuntimeError::Resolve(_)));
}

#[test]
fn test_resolve_preserves_operator_requirements() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(temporary.path(), "tool", "1.0", "");
    write_package(temporary.path(), "tool", "2.0", "");
    let resolved = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["tool>=1.0,<2.0"]),
        )
        .unwrap();
    assert_eq!(
        resolved
            .context()
            .get_package("tool")
            .unwrap()
            .version
            .as_ref()
            .unwrap()
            .as_str(),
        "1.0"
    );
}

#[test]
fn test_resolve_rejects_malformed_conflict_and_weak_requests() {
    let temporary = tempfile::tempdir().unwrap();
    let resolver = RezRuntime::new([temporary.path()]).unwrap();
    let executor = runtime();
    for request in [
        "!!! not a request",
        "!bad family",
        "~bad family",
        "",
        ".feature>=1",
    ] {
        assert!(matches!(
            executor.block_on(resolver.resolve([request])).unwrap_err(),
            RezRuntimeError::InvalidRequirement { .. }
        ));
    }
}

#[test]
fn test_resolve_weak_absent_package_does_not_introduce_a_dependency() {
    let temporary = tempfile::tempdir().unwrap();
    let resolver = RezRuntime::new([temporary.path()]).unwrap();
    let resolved = runtime().block_on(resolver.resolve(["~absent"])).unwrap();
    assert!(resolved.context().resolved_packages.is_empty());
    assert_eq!(
        resolved
            .environment()
            .get("REZ_USED_REQUEST")
            .map(String::as_str),
        Some("~absent")
    );
}

#[test]
fn test_resolve_excludes_descriptors_nested_inside_serialized_package_payloads() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("application");
    let payload = root.join("payload");
    fs::create_dir_all(&payload).unwrap();
    fs::write(
        root.join("package.yaml"),
        "name: application\nversion: '1.0'\n",
    )
    .unwrap();
    fs::write(
        payload.join("package.py"),
        "name = 'application'\nversion = '99.0'\n",
    )
    .unwrap();
    let resolved = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["application"]),
        )
        .unwrap();
    assert_eq!(resolved.context().package_count(), 1);
    assert_eq!(
        resolved
            .context()
            .get_package("application")
            .unwrap()
            .version
            .as_ref()
            .unwrap()
            .as_str(),
        "1.0"
    );
}

#[test]
fn test_resolve_loads_serialized_packages_with_transitive_dependencies() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(temporary.path(), "base", "1.0", "");
    let root = temporary.path().join("application").join("1.0");
    fs::create_dir_all(&root).unwrap();
    fs::write(root.join("package.yaml"), "name: application\nversion: '1.0'\nrequires: ['base-1.0']\ncommands: |\n  env.setenv('YAML_ROOT', '{root}')\n").unwrap();
    let resolved = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["application-1.0"]),
        )
        .unwrap();
    assert_eq!(resolved.context().package_count(), 2);
    assert_eq!(
        resolved.environment().get("YAML_ROOT"),
        Some(&root.to_string_lossy().into_owned())
    );
}

#[test]
fn test_resolve_repeated_yaml_only_transitive_graph_preserves_every_family() {
    const FAMILIES: usize = 32;
    let temporary = tempfile::tempdir().unwrap();
    let mut roots = Vec::new();
    for index in 0..FAMILIES {
        let name = format!("yaml_dep_{index}");
        let root = temporary.path().join(&name).join("1.0");
        fs::create_dir_all(&root).unwrap();
        let requires = if index + 1 < FAMILIES {
            format!("requires: ['yaml_dep_{}-1.0']\n", index + 1)
        } else {
            String::new()
        };
        fs::write(
            root.join("package.yaml"),
            format!(
                "name: {name}\nversion: '1.0'\n{requires}commands: |\n  env.setenv('YAML_DEP_{index}', '{{root}}')\n"
            ),
        )
        .unwrap();
        roots.push(root);
    }
    let resolver = RezRuntime::new([temporary.path()]).unwrap();
    let executor = runtime();
    for iteration in 0..8 {
        let resolved = executor
            .block_on(resolver.resolve(["yaml_dep_0-1.0"]))
            .unwrap_or_else(|error| panic!("YAML graph iteration {iteration}: {error}"));
        assert_eq!(resolved.context().package_count(), FAMILIES);
        for (index, root) in roots.iter().enumerate() {
            assert_eq!(
                resolved.environment().get(&format!("YAML_DEP_{index}")),
                Some(&root.to_string_lossy().into_owned()),
                "YAML graph iteration {iteration}, family {index}"
            );
        }
    }
}

#[test]
fn test_resolve_prefers_python_descriptor_to_malformed_serialized_sibling() {
    let temporary = tempfile::tempdir().unwrap();
    let root = write_package(temporary.path(), "tool", "1.0", "");
    fs::write(root.join("package.yaml"), "[invalid yaml").unwrap();
    let resolved = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["tool-1.0"]),
        )
        .unwrap();
    assert_eq!(resolved.context().package_count(), 1);
    assert_eq!(
        resolved.context().get_package("tool").unwrap().root(),
        Some(root.to_string_lossy().into_owned())
    );
}

#[test]
fn test_resolve_rejects_malformed_preferred_descriptor_without_format_fallback() {
    let executor = runtime();
    for (preferred, malformed, fallback) in [
        ("package.py", "name = [", "package.yaml"),
        ("package.yaml", "[invalid yaml", "package.yml"),
    ] {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("tool").join("1.0");
        fs::create_dir_all(&root).unwrap();
        fs::write(root.join(preferred), malformed).unwrap();
        fs::write(root.join(fallback), "name: tool\nversion: '1.0'\n").unwrap();
        let error = executor
            .block_on(
                RezRuntime::new([temporary.path()])
                    .unwrap()
                    .resolve(["tool-1.0"]),
            )
            .unwrap_err();
        assert!(matches!(error, RezRuntimeError::Repository(_)));
    }
}

#[test]
fn test_resolve_ignores_descriptors_outside_canonical_package_locations() {
    let temporary = tempfile::tempdir().unwrap();
    fs::write(
        temporary.path().join("package.py"),
        "name = 'outside'\nversion = '1.0'\n",
    )
    .unwrap();
    let nested = temporary.path().join("tool").join("1.0").join("payload");
    fs::create_dir_all(&nested).unwrap();
    fs::write(
        nested.join("package.yaml"),
        "name: outside\nversion: '2.0'\n",
    )
    .unwrap();
    let error = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["outside"]),
        )
        .unwrap_err();
    assert!(matches!(error, RezRuntimeError::MissingPackage { .. }));
}

#[test]
fn test_with_target_is_exact_with_earliest_version_preference() {
    let temporary = tempfile::tempdir().unwrap();
    for (name, version) in [
        ("platform", "windows"),
        ("platform", "windows.11"),
        ("arch", "x86_64"),
    ] {
        write_package(temporary.path(), name, version, "");
    }
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap()
        .with_solver_config(SolverConfig {
            prefer_latest: false,
            ..SolverConfig::default()
        });
    let resolved = runtime()
        .block_on(resolver.resolve(Vec::<&str>::new()))
        .unwrap();
    assert_eq!(
        resolved
            .context()
            .get_package("platform")
            .unwrap()
            .version
            .as_ref()
            .unwrap()
            .as_str(),
        "windows"
    );
}

#[test]
fn test_relative_repository_root_survives_child_working_directory_change() {
    let current = std::env::current_dir().unwrap();
    let temporary = tempfile::tempdir_in(&current).unwrap();
    let root = write_package(
        temporary.path(),
        "probe",
        "1.0",
        "def commands():\n    env.setenv('SDK_CHILD_PROBE', 'active')",
    );
    let relative = temporary.path().strip_prefix(&current).unwrap();
    let resolver = RezRuntime::new([relative])
        .unwrap()
        .with_parent_environment(HashMap::from([(
            "SDK_PARENT".to_string(),
            "preserved".to_string(),
        )]));
    let resolved = runtime().block_on(resolver.resolve(["probe-1.0"])).unwrap();
    assert_eq!(
        resolved.environment().get("PROBE_ROOT"),
        Some(&root.to_string_lossy().into_owned())
    );
    assert!(Path::new(resolved.environment().get("PROBE_ROOT").unwrap()).is_absolute());
    let other_directory = temporary.path().join("other-cwd");
    fs::create_dir(&other_directory).unwrap();
    let status = resolved
        .command(std::env::current_exe().unwrap())
        .args(CHILD_ARGUMENTS)
        .current_dir(other_directory)
        .status()
        .unwrap();
    assert_eq!(status.code(), Some(17));
}

#[test]
fn test_with_parent_environment_appends_path_after_explicit_parent() {
    let temporary = tempfile::tempdir().unwrap();
    let root = write_package(
        temporary.path(),
        "tool",
        "1.0",
        "def commands():\n    env.append_path('PATH', '{root}/bin')",
    );
    let parent_path = temporary.path().join("parent-bin");
    let mut parent = HashMap::from([
        (
            "PATH".to_string(),
            parent_path.to_string_lossy().into_owned(),
        ),
        ("SDK_PARENT".to_string(), "preserved".to_string()),
    ]);
    if cfg!(windows) {
        parent.insert("Path".to_string(), "shadow-path".to_string());
    }
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_parent_environment(parent)
        .with_context_config(ContextConfig::default());
    let resolved = runtime().block_on(resolver.resolve(["tool-1.0"])).unwrap();
    let path: Vec<_> = std::env::split_paths(resolved.environment().get("PATH").unwrap()).collect();
    assert_eq!(path, [parent_path, root.join("bin")]);
    assert_eq!(
        resolved.environment().get("SDK_PARENT").map(String::as_str),
        Some("preserved")
    );
    assert_eq!(
        resolved
            .environment()
            .keys()
            .filter(|name| name.eq_ignore_ascii_case("PATH"))
            .count(),
        1
    );
}

#[test]
fn test_with_empty_parent_environment_excludes_ambient_variables() {
    let temporary = tempfile::tempdir().unwrap();
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_parent_environment(HashMap::new());
    let resolved = runtime()
        .block_on(resolver.resolve(Vec::<&str>::new()))
        .unwrap();
    assert!(
        resolved
            .environment()
            .keys()
            .all(|key| key.starts_with("REZ_USED_"))
    );
}

#[test]
fn test_command_preserves_arguments_and_returns_child_exit_code() {
    let temporary = tempfile::tempdir().unwrap();
    write_package(
        temporary.path(),
        "probe",
        "1.0",
        "def commands():\n    env.setenv('SDK_CHILD_PROBE', 'active')",
    );
    let resolver = RezRuntime::new([temporary.path()])
        .unwrap()
        .with_parent_environment(HashMap::from([(
            "SDK_PARENT".to_string(),
            "preserved".to_string(),
        )]));
    let resolved = runtime().block_on(resolver.resolve(["probe-1.0"])).unwrap();
    let program = std::env::current_exe().unwrap();
    let raw_arguments = ["with spaces", "\"quoted\"", "$(literal)", "&literal", ""];
    let mut command = resolved.command(&program);
    command.args(raw_arguments);
    assert_eq!(command.get_program(), program.as_os_str());
    assert_eq!(
        command.get_args().map(OsString::from).collect::<Vec<_>>(),
        raw_arguments.map(OsString::from)
    );

    let output = resolved
        .command(program)
        .args(CHILD_ARGUMENTS)
        .output()
        .expect("child starts");
    assert_eq!(
        output.status.code(),
        Some(17),
        "child output: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_command_child_probe() {
    if std::env::var("SDK_CHILD_PROBE").as_deref() != Ok("active") {
        return;
    }
    assert_eq!(
        std::env::args().skip(1).collect::<Vec<_>>(),
        CHILD_ARGUMENTS
    );
    assert_eq!(std::env::var("SDK_PARENT").as_deref(), Ok("preserved"));
    assert!(std::env::var_os("CARGO").is_none());
    assert_eq!(std::env::var("PROBE_VERSION").as_deref(), Ok("1.0"));
    std::process::exit(17);
}

#[test]
fn test_runtime_errors_classify_invalid_bundle_and_missing_package() {
    let temporary = tempfile::tempdir().unwrap();
    let missing = temporary.path().join("missing");
    assert!(
        matches!(RezRuntime::new([&missing]).unwrap_err(), RezRuntimeError::InvalidBundle { path } if path == missing)
    );
    assert!(matches!(
        RezRuntime::new(Vec::<PathBuf>::new()).unwrap_err(),
        RezRuntimeError::NoRepositories
    ));
    let error = runtime()
        .block_on(
            RezRuntime::new([temporary.path()])
                .unwrap()
                .resolve(["missing-1.0"]),
        )
        .unwrap_err();
    assert!(
        matches!(error, RezRuntimeError::MissingPackage { requirement } if requirement == "missing-1.0")
    );
    assert!(
        matches!(RezRuntime::new([temporary.path()]).unwrap().with_target("plan9", "amd64").unwrap_err(), RezRuntimeError::UnsupportedPlatform { platform } if platform == "plan9")
    );
}
