use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use rstest::rstest;

use rez_next_context::{ContextConfig, EnvironmentManager, PathStrategy};
use rez_next_package::Package;

fn package_with_tools(root: &Path) -> Package {
    // A real root-level executable makes the old discovery fallback observable.
    let executable = if cfg!(windows) {
        "runtime.exe"
    } else {
        "runtime"
    };
    fs::write(
        root.join(executable),
        b"tool metadata must not add this path",
    )
    .unwrap();
    Package {
        name: "runtime".to_string(),
        tools: vec!["runtime".to_string()],
        filepath: Some(root.join("package.py").to_string_lossy().into_owned()),
        ..Default::default()
    }
}

fn manager(strategy: PathStrategy, parent: HashMap<String, String>) -> EnvironmentManager {
    EnvironmentManager::with_base_environment(
        ContextConfig {
            path_strategy: strategy,
            ..Default::default()
        },
        parent,
    )
}

fn paths(environment: &HashMap<String, String>) -> Vec<PathBuf> {
    std::env::split_paths(environment.get("PATH").expect("explicit package PATH")).collect()
}

#[rstest]
#[case(PathStrategy::Prepend)]
#[case(PathStrategy::Append)]
#[case(PathStrategy::Replace)]
#[case(PathStrategy::NoModify)]
#[tokio::test]
async fn test_generate_environment_explicit_payload_path_is_authoritative(
    #[case] strategy: PathStrategy,
) {
    let temporary = tempfile::tempdir().unwrap();
    let mut package = package_with_tools(temporary.path());
    package.commands = Some("env.prepend_path('PATH', '{root}/payload/bin')".to_string());

    let environment = manager(strategy, HashMap::new())
        .generate_environment(&[package])
        .await
        .unwrap();

    assert_eq!(paths(&environment), [temporary.path().join("payload/bin")]);
}

#[rstest]
#[case(PathStrategy::Prepend)]
#[case(PathStrategy::Append)]
#[case(PathStrategy::Replace)]
#[case(PathStrategy::NoModify)]
#[tokio::test]
async fn test_generate_environment_explicit_append_preserves_parent_order(
    #[case] strategy: PathStrategy,
) {
    let temporary = tempfile::tempdir().unwrap();
    let mut package = package_with_tools(temporary.path());
    package.commands = Some("env.append_path('PATH', '{root}/payload/bin')".to_string());
    let parent_bin = temporary.path().join("parent-bin");
    let parent = HashMap::from([(
        "PATH".to_string(),
        parent_bin.to_string_lossy().into_owned(),
    )]);

    let environment = manager(strategy, parent)
        .generate_environment(&[package])
        .await
        .unwrap();

    assert_eq!(
        paths(&environment),
        [parent_bin, temporary.path().join("payload/bin")]
    );
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[tokio::test]
async fn test_generate_environment_tools_metadata_does_not_undo_phase_path_unset(
    #[case] phase: usize,
) {
    let temporary = tempfile::tempdir().unwrap();
    let mut package = package_with_tools(temporary.path());
    let commands = Some("unsetenv('PATH')".to_string());
    match phase {
        0 => package.pre_commands = commands,
        1 => package.commands = commands,
        2 => package.post_commands = commands,
        _ => unreachable!(),
    }
    let parent = HashMap::from([("PATH".to_string(), "parent-bin".to_string())]);

    let environment = manager(PathStrategy::Prepend, parent)
        .generate_environment(&[package])
        .await
        .unwrap();

    assert!(!environment.contains_key("PATH"));
}

#[rstest]
#[case("")]
#[case("env.setenv('ACTIVATED', 'yes')")]
#[tokio::test]
async fn test_generate_environment_defined_commands_do_not_infer_a_tool_path(
    #[case] commands: &str,
) {
    let temporary = tempfile::tempdir().unwrap();
    let mut package = package_with_tools(temporary.path());
    package.commands = Some(commands.to_string());

    let environment = manager(PathStrategy::Prepend, HashMap::new())
        .generate_environment(&[package])
        .await
        .unwrap();

    assert!(!environment.contains_key("PATH"));
}
