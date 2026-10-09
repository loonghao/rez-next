use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use rstest::rstest;

use rez_next_runtime::{InstallationPlan, InstallationPlanError, RezRuntime};

// Exact definition used by vx-org/witr's six-platform binary distribution.
const WITR: &str = include_str!("fixtures/witr_package.py");

fn executor() -> tokio::runtime::Runtime {
    tokio::runtime::Runtime::new().unwrap()
}

fn write_definition(directory: &Path, content: &str) -> PathBuf {
    let definition = directory.join("package.py");
    fs::write(&definition, content).unwrap();
    definition
}

#[rstest]
#[case("windows", "x86_64", 0)]
#[case("windows", "arm_64", 1)]
#[case("linux", "x86_64", 2)]
#[case("linux", "arm_64", 3)]
#[case("osx", "x86_64", 4)]
#[case("osx", "arm_64", 5)]
fn test_installation_plan_real_witr_variants_preserve_definition(
    #[case] platform: &str,
    #[case] architecture: &str,
    #[case] index: usize,
) {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), WITR);
    let plan = executor()
        .block_on(InstallationPlan::from_definition(
            &definition,
            platform,
            architecture,
        ))
        .unwrap();

    assert_eq!(plan.name, "witr");
    assert_eq!(plan.platform, platform);
    assert_eq!(plan.arch, architecture);
    assert_eq!(plan.version.as_deref(), Some("0.3.4"));
    assert_eq!(plan.variant_index, Some(index));
    assert_eq!(
        plan.variant_requirements,
        [
            format!("platform-{platform}"),
            format!("arch-{architecture}")
        ]
    );
    assert_eq!(plan.package_relative_path, "witr/0.3.4");
    assert_eq!(
        plan.variant_relative_path,
        format!("witr/0.3.4/platform-{platform}/arch-{architecture}")
    );
    assert_eq!(fs::read(definition).unwrap(), WITR.as_bytes());
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[rstest]
fn test_installation_plan_payload_root_matches_runtime_materialization() {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    fs::create_dir(&source).unwrap();
    let definition = write_definition(&source, WITR);
    let executor = executor();
    let plan = executor
        .block_on(InstallationPlan::from_definition(
            definition, "windows", "x86_64",
        ))
        .unwrap();
    let repository = temporary.path().join("repository");
    let package_base = repository.join(&plan.package_relative_path);
    let variant_root = repository.join(&plan.variant_relative_path);
    let binary_directory = variant_root.join("payload").join("bin");
    fs::create_dir_all(&binary_directory).unwrap();
    write_definition(&package_base, WITR);
    for (name, version) in [("platform", "windows"), ("arch", "x86_64")] {
        let binding = repository.join(name).join(version);
        fs::create_dir_all(&binding).unwrap();
        write_definition(
            &binding,
            &format!("name = '{name}'\nversion = '{version}'\n"),
        );
    }
    let marker = binary_directory.join("installed-payload-marker");
    fs::write(&marker, b"selected variant payload").unwrap();
    let runtime = RezRuntime::new([repository])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap()
        .with_parent_environment(Default::default());
    let resolved = executor.block_on(runtime.resolve(["witr-0.3.4"])).unwrap();
    assert_eq!(
        PathBuf::from(
            resolved
                .context()
                .get_package("witr")
                .unwrap()
                .root()
                .unwrap()
        ),
        variant_root
    );
    let paths: Vec<_> =
        std::env::split_paths(resolved.environment().get("PATH").unwrap()).collect();
    assert_eq!(paths, [binary_directory]);
    assert_eq!(
        fs::read(paths[0].join("installed-payload-marker")).unwrap(),
        b"selected variant payload"
    );
}

#[rstest]
#[case("macos")]
#[case("darwin")]
fn test_installation_plan_normalizes_platform_aliases(#[case] alias: &str) {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), WITR);
    let plan = executor()
        .block_on(InstallationPlan::from_definition(
            definition, alias, "x86_64",
        ))
        .unwrap();
    assert_eq!(plan.platform, "osx");
    assert_eq!(plan.arch, "x86_64");
    assert_eq!(plan.variant_index, Some(4));
    assert_eq!(
        plan.variant_relative_path,
        "witr/0.3.4/platform-osx/arch-x86_64"
    );
}

#[rstest]
fn test_installation_plan_unsupported_variant_fails_strictly() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), WITR);
    let error = executor()
        .block_on(InstallationPlan::from_definition(
            definition, "linux", "armv7",
        ))
        .unwrap_err();
    assert!(matches!(error, InstallationPlanError::Resolve(_)));
}

#[rstest]
fn test_installation_plan_leaves_commands_and_process_environment_unchanged() {
    let temporary = tempfile::tempdir().unwrap();
    const SENTINEL: &str = "REZ_NEXT_INSTALLATION_PLAN_READONLY_SENTINEL";
    const CONTENT: &str = "name = 'readonly'\nversion = '1.0'\ndef commands():\n    env.REZ_NEXT_INSTALLATION_PLAN_READONLY_SENTINEL.set('must-not-run')\n    env.PATH.prepend('{root}/payload/bin')\n";
    let original_environment = std::env::var_os(SENTINEL);
    let definition = write_definition(temporary.path(), CONTENT);
    let plan = executor()
        .block_on(InstallationPlan::from_definition(
            &definition,
            "windows",
            "x86_64",
        ))
        .unwrap();
    assert_eq!(plan.package_relative_path, "readonly/1.0");
    assert_eq!(plan.variant_relative_path, "readonly/1.0");
    assert_eq!(plan.variant_index, None);
    assert!(plan.variant_requirements.is_empty());
    assert_eq!(std::env::var_os(SENTINEL), original_environment);
    assert_eq!(fs::read(definition).unwrap(), CONTENT.as_bytes());
    assert_eq!(fs::read_dir(temporary.path()).unwrap().count(), 1);
}

#[rstest]
fn test_installation_plan_preserves_versionless_identity() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), "name = 'versionless'\n");
    let plan = executor()
        .block_on(InstallationPlan::from_definition(
            definition, "linux", "x86_64",
        ))
        .unwrap();
    assert_eq!(plan.version, None);
    assert_eq!(plan.package_relative_path, "versionless");
    assert_eq!(plan.variant_relative_path, "versionless");
}

#[rstest]
fn test_installation_plan_target_binding_definition_is_not_shadowed() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), "name = 'platform'\nversion = 'windows'\n");
    let plan = executor()
        .block_on(InstallationPlan::from_definition(
            &definition,
            "windows",
            "x86_64",
        ))
        .unwrap();
    assert_eq!(plan.package_relative_path, "platform/windows");
    assert!(
        executor()
            .block_on(InstallationPlan::from_definition(
                definition, "linux", "x86_64"
            ))
            .is_err()
    );
}

#[rstest]
fn test_installation_plan_missing_dependency_is_not_fabricated() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(
        temporary.path(),
        "name = 'application'\nversion = '1.0'\nrequires = ['actual_runtime-2']\n",
    );
    let error = executor()
        .block_on(InstallationPlan::from_definition(
            definition, "linux", "x86_64",
        ))
        .unwrap_err();
    assert!(matches!(error, InstallationPlanError::Resolve(_)));
}

#[rstest]
fn test_installation_plan_resolves_actual_repository_dependencies() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(
        temporary.path(),
        "name = 'application'\nversion = '1.0'\nvariants = [['platform-windows', 'arch-x86_64', 'actual_runtime-2']]\n",
    );
    let repository = temporary.path().join("repository");
    let dependency = repository.join("actual_runtime").join("2.0");
    fs::create_dir_all(&dependency).unwrap();
    write_definition(&dependency, "name = 'actual_runtime'\nversion = '2.0'\n");
    let plan = executor()
        .block_on(InstallationPlan::from_definition_with_repositories(
            definition,
            "windows",
            "x86_64",
            &[repository],
        ))
        .unwrap();
    assert_eq!(
        plan.variant_relative_path,
        "application/1.0/platform-windows/arch-x86_64/actual_runtime-2"
    );
    assert_eq!(
        plan.variant_requirements.last().map(String::as_str),
        Some("actual_runtime-2")
    );
}

#[rstest]
fn test_installation_plan_rejects_traversing_family() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), "name = '../escape'\nversion = '1.0'\n");
    assert!(
        executor()
            .block_on(InstallationPlan::from_definition(
                definition, "linux", "x86_64"
            ))
            .is_err()
    );
}

#[rstest]
fn test_installation_plan_cli_emits_deterministic_json() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), WITR);
    let run = || {
        Command::new(env!("CARGO_BIN_EXE_rez-next-runtime"))
            .args(["installation-plan", "--definition"])
            .arg(&definition)
            .args(["--platform", "windows", "--arch", "x86_64", "--json"])
            .output()
            .unwrap()
    };
    let first = run();
    let second = run();
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        second.status.success(),
        "{}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert_eq!(first.stdout, second.stdout);
    assert_eq!(
        String::from_utf8(first.stdout).unwrap(),
        "{\"schema_version\":1,\"platform\":\"windows\",\"arch\":\"x86_64\",\"name\":\"witr\",\"version\":\"0.3.4\",\"variant_index\":0,\"variant_requirements\":[\"platform-windows\",\"arch-x86_64\"],\"package_relative_path\":\"witr/0.3.4\",\"variant_relative_path\":\"witr/0.3.4/platform-windows/arch-x86_64\"}\n"
    );
    assert_eq!(fs::read(definition).unwrap(), WITR.as_bytes());
}

#[rstest]
fn test_installation_plan_cli_unsupported_target_has_no_json() {
    let temporary = tempfile::tempdir().unwrap();
    let definition = write_definition(temporary.path(), WITR);
    let output = Command::new(env!("CARGO_BIN_EXE_rez-next-runtime"))
        .args(["installation-plan", "--definition"])
        .arg(definition)
        .args(["--platform", "linux", "--arch", "armv7", "--json"])
        .output()
        .unwrap();
    assert!(!output.status.success());
    assert!(output.stdout.is_empty());
    assert!(!output.stderr.is_empty());
}

#[rstest]
#[case(
    "yaml",
    "name: serialized_runtime\nversion: '1.0'\nvariants: [['platform-windows', 'arch-x86_64']]\nhashed_variants: true\ntools: ['runtime']\npre_commands: \"env.setenv('SERIALIZED_PRE', 'yes')\"\ncommands: \"env.prepend_path('PATH', '{root}/payload/bin')\"\npost_commands: \"env.setenv('SERIALIZED_POST', 'yes')\"\n"
)]
#[case("json", r#"{"name":"serialized_runtime","version":"1.0","variants":[["platform-windows","arch-x86_64"]],"hashed_variants":true,"tools":["runtime"],"pre_commands":"env.setenv('SERIALIZED_PRE', 'yes')","commands":"env.prepend_path('PATH', '{root}/payload/bin')","post_commands":"env.setenv('SERIALIZED_POST', 'yes')"}"#)]
fn test_installation_plan_serialized_hashed_layout_matches_runtime(
    #[case] extension: &str,
    #[case] content: &str,
) {
    let temporary = tempfile::tempdir().unwrap();
    let source = temporary.path().join("source");
    fs::create_dir(&source).unwrap();
    let filename = format!("package.{extension}");
    let definition = source.join(&filename);
    fs::write(&definition, content).unwrap();
    let executor = executor();
    let plan = executor
        .block_on(InstallationPlan::from_definition(
            &definition,
            "windows",
            "x86_64",
        ))
        .unwrap();
    assert_eq!(plan.variant_index, Some(0));
    assert_eq!(
        plan.variant_relative_path,
        "serialized_runtime/1.0/c05c599993c2b1ed88babc35858f1d00995605f0"
    );

    let repository = temporary.path().join("repository");
    let package_base = repository.join(&plan.package_relative_path);
    let variant_root = repository.join(&plan.variant_relative_path);
    let binary_directory = variant_root.join("payload/bin");
    fs::create_dir_all(&binary_directory).unwrap();
    fs::write(package_base.join(filename), content).unwrap();
    fs::write(
        binary_directory.join("payload-marker"),
        b"actual hashed variant",
    )
    .unwrap();
    for (name, version) in [("platform", "windows"), ("arch", "x86_64")] {
        let binding = repository.join(name).join(version);
        fs::create_dir_all(&binding).unwrap();
        write_definition(
            &binding,
            &format!("name = '{name}'\nversion = '{version}'\n"),
        );
    }
    let runtime = RezRuntime::new([repository])
        .unwrap()
        .with_target("windows", "x86_64")
        .unwrap()
        .with_parent_environment(Default::default());
    let resolved = executor
        .block_on(runtime.resolve(["serialized_runtime-1.0"]))
        .unwrap();
    assert_eq!(
        PathBuf::from(
            resolved
                .context()
                .get_package("serialized_runtime")
                .unwrap()
                .root()
                .unwrap()
        ),
        variant_root
    );
    let paths: Vec<_> =
        std::env::split_paths(resolved.environment().get("PATH").unwrap()).collect();
    assert_eq!(paths, [binary_directory]);
    assert_eq!(
        fs::read(paths[0].join("payload-marker")).unwrap(),
        b"actual hashed variant"
    );
    assert_eq!(
        resolved
            .environment()
            .get("SERIALIZED_PRE")
            .map(String::as_str),
        Some("yes")
    );
    assert_eq!(
        resolved
            .environment()
            .get("SERIALIZED_POST")
            .map(String::as_str),
        Some("yes")
    );
    assert_eq!(fs::read(definition).unwrap(), content.as_bytes());
}
