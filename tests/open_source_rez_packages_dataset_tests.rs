//! Offline fixture tests for UTS-AnimalLogicAcademy/open-source-rez-packages.

use rez_next_build::{BuildConfig, BuildManager, BuildRequest};
use rez_next_context::{ContextConfig, ContextStatus, EnvironmentManager, ResolvedContext};
use rez_next_package::Package;
use rez_next_package::PackageRequirement;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

fn fixture_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/open_source_rez_packages")
}

fn aces_container_source() -> PathBuf {
    fixture_root().join("aces_container").join("1.0.2")
}

fn make_build_manager(temp_dir: &TempDir) -> BuildManager {
    let config = BuildConfig {
        build_dir: temp_dir.path().join("build"),
        temp_dir: temp_dir.path().join("tmp"),
        keep_artifacts: true,
        ..BuildConfig::default()
    };
    BuildManager::with_config(config)
}

async fn run_single_build(
    package: Package,
    source_dir: PathBuf,
    temp_dir: &TempDir,
) -> rez_next_build::BuildResult {
    let mut manager = make_build_manager(temp_dir);
    let request = BuildRequest::new(package, None, source_dir);
    let build_ids = manager.start_build(request).await.unwrap();
    assert_eq!(build_ids.len(), 1);
    manager.wait_for_build(&build_ids[0]).await.unwrap()
}

async fn run_single_build_with_context(
    package: Package,
    source_dir: PathBuf,
    temp_dir: &TempDir,
    context: ResolvedContext,
) -> rez_next_build::BuildResult {
    let mut manager = make_build_manager(temp_dir);
    let request = BuildRequest::new(package, Some(context), source_dir);
    let build_ids = manager.start_build(request).await.unwrap();
    assert_eq!(build_ids.len(), 1);
    manager.wait_for_build(&build_ids[0]).await.unwrap()
}

fn copy_fixture_dir(src: &Path, dest: &Path) {
    std::fs::create_dir_all(dest).unwrap();
    for entry in std::fs::read_dir(src).unwrap() {
        let entry = entry.unwrap();
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());
        if src_path.is_dir() {
            copy_fixture_dir(&src_path, &dest_path);
        } else {
            std::fs::copy(&src_path, &dest_path).unwrap();
        }
    }
}

fn vx_fixture_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/vx_rez_package")
}

fn build_deps_repo_source() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/build_deps_repo")
}

fn python_executable() -> String {
    let output = Command::new("python")
        .args(["-c", "import sys; print(sys.executable)"])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "failed to locate python executable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout).trim().to_string()
}

fn create_python_shim(repo: &Path) {
    let bin_dir = repo.join("python").join("3.11.0").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let python = python_executable();
    if cfg!(windows) {
        std::fs::write(
            bin_dir.join("python.cmd"),
            format!("@echo off\r\n\"{}\" %*\r\n", python),
        )
        .unwrap();
    } else {
        let shim = bin_dir.join("python");
        std::fs::write(&shim, format!("#!/bin/sh\n\"{}\" \"$@\"\n", python)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
}

fn create_bootstrap_python_bin(temp_dir: &TempDir) -> PathBuf {
    let bin_dir = temp_dir.path().join("bootstrap_python").join("bin");
    std::fs::create_dir_all(&bin_dir).unwrap();
    let python = python_executable();
    if cfg!(windows) {
        std::fs::write(
            bin_dir.join("python.cmd"),
            format!("@echo off\r\n\"{}\" %*\r\n", python),
        )
        .unwrap();
    } else {
        let shim = bin_dir.join("python");
        std::fs::write(&shim, format!("#!/bin/sh\n\"{}\" \"$@\"\n", python)).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&shim, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
    }
    bin_dir
}

fn create_python_standalone_artifact(temp_dir: &TempDir) -> (PathBuf, String) {
    let artifact = temp_dir.path().join("python-standalone.tar.gz");
    let script = format!(
        r##"
import hashlib
import pathlib
import tarfile
import tempfile

artifact = pathlib.Path(r"{artifact}")
with tempfile.TemporaryDirectory() as tmp:
    root = pathlib.Path(tmp) / "python" / "install" / "bin"
    root.mkdir(parents=True)
    (root / "python.cmd").write_text("@echo off\r\necho python standalone fixture\r\n", encoding="utf-8")
    (root / "python").write_text("#!/bin/sh\necho python standalone fixture\n", encoding="utf-8")
    with tarfile.open(artifact, "w:gz") as archive:
        archive.add(pathlib.Path(tmp) / "python", arcname="python")
print(hashlib.sha256(artifact.read_bytes()).hexdigest())
"##,
        artifact = artifact.to_string_lossy()
    );
    let output = Command::new("python")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "failed to create python standalone artifact: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sha256 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (artifact, sha256)
}

async fn vx_build_context(temp_dir: &TempDir) -> ResolvedContext {
    let repo = temp_dir.path().join("build_deps_repo_context");
    copy_fixture_dir(&build_deps_repo_source(), &repo);
    create_python_shim(&repo);
    let python = Package::from_path(repo.join("python").join("3.11.0")).unwrap();
    let rez_builder = Package::from_path(repo.join("rez_builder").join("0.1.0")).unwrap();
    let mut context = ResolvedContext::from_requirements(vec![
        PackageRequirement::parse("python-3+").unwrap(),
        PackageRequirement::parse("rez_builder-0").unwrap(),
    ]);
    context.resolved_packages = vec![python, rez_builder];
    context.status = ContextStatus::Resolved;

    let manager = EnvironmentManager::new(ContextConfig {
        inherit_parent_env: false,
        ..Default::default()
    });
    context.environment_vars = manager
        .generate_environment(&context.resolved_packages)
        .await
        .unwrap();
    context
}

async fn python_build_context(temp_dir: &TempDir) -> ResolvedContext {
    let repo = temp_dir.path().join("python_build_deps_repo_context");
    copy_fixture_dir(&build_deps_repo_source(), &repo);
    create_python_shim(&repo);
    let python = Package::from_path(repo.join("python").join("3.11.0")).unwrap();
    let mut context =
        ResolvedContext::from_requirements(vec![PackageRequirement::parse("python-3+").unwrap()]);
    context.resolved_packages = vec![python];
    context.status = ContextStatus::Resolved;

    let manager = EnvironmentManager::new(ContextConfig {
        inherit_parent_env: false,
        ..Default::default()
    });
    context.environment_vars = manager
        .generate_environment(&context.resolved_packages)
        .await
        .unwrap();
    context
}

fn create_vx_artifact(temp_dir: &TempDir) -> (PathBuf, String) {
    let artifact = temp_dir.path().join("vx-artifact.zip");
    let script = format!(
        r##"
import hashlib
import pathlib
import zipfile

artifact = pathlib.Path(r"{artifact}")
artifact.parent.mkdir(parents=True, exist_ok=True)
with zipfile.ZipFile(artifact, "w") as archive:
    archive.writestr("bin/vx.cmd", "@echo off\r\necho vx 0.0.1-test\r\n")
    archive.writestr("bin/vx", "#!/bin/sh\necho vx 0.0.1-test\n")
print(hashlib.sha256(artifact.read_bytes()).hexdigest())
"##,
        artifact = artifact.to_string_lossy()
    );
    let output = Command::new("python")
        .arg("-c")
        .arg(script)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "failed to create vx artifact: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let sha256 = String::from_utf8_lossy(&output.stdout).trim().to_string();
    (artifact, sha256)
}

async fn run_vx_build(
    package: Package,
    source_dir: PathBuf,
    temp_dir: &TempDir,
    artifact: &Path,
    sha256: &str,
) -> rez_next_build::BuildResult {
    let mut manager = make_build_manager(temp_dir);
    let mut request =
        BuildRequest::new(package, Some(vx_build_context(temp_dir).await), source_dir);
    request.options.env_vars.insert(
        "REZ_BINARY_ARCHIVE_ARTIFACT".to_string(),
        artifact.to_string_lossy().to_string(),
    );
    request
        .options
        .env_vars
        .insert("REZ_BINARY_ARCHIVE_SHA256".to_string(), sha256.to_string());

    let build_ids = manager.start_build(request).await.unwrap();
    assert_eq!(build_ids.len(), 1);
    manager.wait_for_build(&build_ids[0]).await.unwrap()
}

#[test]
fn parses_real_open_source_rez_package_fixture() {
    let source = aces_container_source();
    let package = Package::from_path(&source).unwrap();

    assert_eq!(package.name, "aces_container");
    assert_eq!(
        package.version.as_ref().map(|version| version.as_str()),
        Some("1.0.3")
    );
    assert!(
        package
            .requires
            .contains(&"os-RedHatEnterprise-8.10+".to_string())
    );
    assert!(
        package
            .private_build_requires
            .contains(&"cmake-3".to_string())
    );
    assert_eq!(package.root(), Some(source.to_string_lossy().to_string()));
}

#[test]
fn real_open_source_rez_package_uses_actual_root_in_environment() {
    let source = aces_container_source();
    let package = Package::from_path(&source).unwrap();
    let manager = EnvironmentManager::new(ContextConfig {
        inherit_parent_env: false,
        ..Default::default()
    });

    let env = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(manager.generate_environment(&[package]))
        .unwrap();

    assert_eq!(
        env.get("ACES_CONTAINER_ROOT"),
        Some(&source.to_string_lossy().to_string())
    );
}

#[tokio::test]
async fn builds_real_open_source_rez_package_fixture_successfully() {
    let source = aces_container_source();
    let package = Package::from_path(&source).unwrap();
    let temp_dir = TempDir::new().unwrap();

    let result = run_single_build(package, source, &temp_dir).await;

    assert!(result.success, "build should succeed: {}", result.errors);
    assert!(
        result.artifacts.install_dir.join("package.py").exists(),
        "copy-only build should install the package definition"
    );
}

#[tokio::test]
async fn builds_python_standalone_package_from_artifact() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("python_rez_package");
    copy_fixture_dir(
        &build_deps_repo_source().join("python").join("3.11.0"),
        &source,
    );
    let (artifact, sha256) = create_python_standalone_artifact(&temp_dir);
    let package = Package::from_path(&source).unwrap();
    let mut manager = make_build_manager(&temp_dir);
    let mut request = BuildRequest::new(package, None, source);
    request.options.env_vars.insert(
        "REZ_BINARY_ARCHIVE_ARTIFACT".to_string(),
        artifact.to_string_lossy().to_string(),
    );
    request.options.env_vars.insert(
        "REZ_BINARY_ARCHIVE_PAYLOAD_PREFIX".to_string(),
        "python/install".to_string(),
    );
    request
        .options
        .env_vars
        .insert("REZ_BINARY_ARCHIVE_SHA256".to_string(), sha256);
    request.options.env_vars.insert(
        "PATH".to_string(),
        create_bootstrap_python_bin(&temp_dir)
            .to_string_lossy()
            .to_string(),
    );
    request
        .options
        .env_vars
        .insert("PATHEXT".to_string(), ".COM;.EXE;.BAT;.CMD".to_string());

    let build_ids = manager.start_build(request).await.unwrap();
    assert_eq!(build_ids.len(), 1);
    let result = manager.wait_for_build(&build_ids[0]).await.unwrap();

    assert!(
        result.success,
        "python standalone build should succeed: {}",
        result.errors
    );
    assert!(
        result
            .artifacts
            .install_dir
            .join("bin")
            .join(if cfg!(windows) {
                "python.cmd"
            } else {
                "python"
            })
            .exists(),
        "python standalone payload should be installed"
    );
}

#[tokio::test]
async fn build_failure_reports_step_command_exit_code_and_stderr() {
    let fixture_source = aces_container_source();
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("aces_container_failure");
    copy_fixture_dir(&fixture_source, &source);
    std::fs::write(
        source.join("rezbuild.py"),
        r#"
import sys

print("OSRP_FIXTURE_SENTINEL: missing cmake package", file=sys.stderr)
sys.exit(1)
"#,
    )
    .unwrap();

    let package = Package::from_path(&source).unwrap();
    let result = run_single_build_with_context(
        package,
        source,
        &temp_dir,
        python_build_context(&temp_dir).await,
    )
    .await;

    assert!(!result.success, "failing rezbuild.py should fail the build");
    assert!(
        result.errors.contains("Installing Errors"),
        "{}",
        result.errors
    );
    assert!(
        result.errors.contains("Command failed with exit code 1"),
        "{}",
        result.errors
    );
    assert!(result.errors.contains("rezbuild.py"), "{}", result.errors);
    assert!(
        result
            .errors
            .contains("OSRP_FIXTURE_SENTINEL: missing cmake package"),
        "{}",
        result.errors
    );
}

#[tokio::test]
async fn builds_vx_style_executable_rez_package_from_local_artifact() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("vx_rez_package");
    copy_fixture_dir(&vx_fixture_source(), &source);
    let (artifact, sha256) = create_vx_artifact(&temp_dir);
    let package = Package::from_path(&source).unwrap();

    let result = run_vx_build(package, source, &temp_dir, &artifact, &sha256).await;

    assert!(
        result.success,
        "vx-style build should succeed: {}",
        result.errors
    );
    assert!(
        result
            .artifacts
            .install_dir
            .join("bin")
            .join("vx.cmd")
            .exists()
    );
    assert!(result.artifacts.install_dir.join("bin").join("vx").exists());

    let output = if cfg!(windows) {
        Command::new("cmd")
            .args([
                "/C",
                &result
                    .artifacts
                    .install_dir
                    .join("bin")
                    .join("vx.cmd")
                    .to_string_lossy(),
            ])
            .output()
            .unwrap()
    } else {
        Command::new("sh")
            .arg(result.artifacts.install_dir.join("bin").join("vx"))
            .output()
            .unwrap()
    };

    assert!(output.status.success());
    assert_eq!(
        String::from_utf8_lossy(&output.stdout).trim(),
        "vx 0.0.1-test"
    );
}

#[test]
fn vx_style_package_exposes_tests_field() {
    let package = Package::from_path(vx_fixture_source()).unwrap();

    assert_eq!(package.requires, vec!["python-3+"]);
    assert_eq!(package.build_requires, vec!["python-3+"]);
    assert_eq!(package.private_build_requires, vec!["rez_builder-0"]);
    assert_eq!(
        package.tests.get("artifact").map(String::as_str),
        Some("python tests/check_artifact.py")
    );
    assert_eq!(
        package.tests.get("zipfile").map(String::as_str),
        Some("python tests/check_zipfile.py")
    );
}

#[tokio::test]
async fn vx_style_rezbuild_reports_checksum_mismatch_precisely() {
    let temp_dir = TempDir::new().unwrap();
    let source = temp_dir.path().join("vx_rez_package");
    copy_fixture_dir(&vx_fixture_source(), &source);
    let (artifact, _sha256) = create_vx_artifact(&temp_dir);
    let package = Package::from_path(&source).unwrap();

    let result = run_vx_build(package, source, &temp_dir, &artifact, "bad-sha256").await;

    assert!(!result.success, "checksum mismatch should fail the build");
    assert!(
        result.errors.contains("Installing Errors"),
        "{}",
        result.errors
    );
    assert!(
        result.errors.contains("binary archive sha256 mismatch"),
        "{}",
        result.errors
    );
}
