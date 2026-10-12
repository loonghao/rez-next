//! Environment precedence contracts checked against official Rez 3.4.0.

use serde_json::{Value, json};
use std::process::{Command, Output};

fn config_output(key: &str, plain: Option<&str>, json_value: &str) -> Output {
    let home = tempfile::tempdir().unwrap();
    let mut command = Command::new(env!("CARGO_BIN_EXE_rez-next"));
    command.args(["config", "--json", key]);
    // Run each case in a fresh process without inherited Rez configuration.
    for (name, _) in std::env::vars().filter(|(name, _)| name.starts_with("REZ_")) {
        command.env_remove(name);
    }
    command
        .env("HOME", home.path())
        .env("USERPROFILE", home.path())
        .env("REZ_DISABLE_HOME_CONFIG", "1");
    let variable = format!("REZ_{}", key.to_ascii_uppercase());
    if let Some(value) = plain {
        command.env(&variable, value);
    }
    command.env(format!("{variable}_JSON"), json_value);
    command.output().unwrap()
}

fn assert_value(key: &str, plain: Option<&str>, json_value: &str, expected: Value) {
    let output = config_output(key, plain, json_value);
    assert!(
        output.status.success(),
        "{key}: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        expected
    );
}

#[test]
fn plain_environment_wins_for_supported_types() {
    assert_value("default_shell", Some("plain"), r#""json""#, json!("plain"));
    assert_value(
        "packages_path",
        Some("/plain"),
        r#"["/json"]"#,
        json!(["/plain"]),
    );
    // RezNext-specific keys follow the same precedence rule.
    assert_value("use_rust_solver", Some("false"), "true", json!(false));
    assert_value("thread_count", Some("2"), "4", json!(2));
}

#[test]
fn empty_plain_environment_still_wins() {
    assert_value("default_shell", Some(""), r#""json""#, json!(""));
    assert_value("packages_path", Some(""), r#"["/json"]"#, json!([]));
}

#[test]
fn json_environment_is_used_when_plain_is_absent() {
    assert_value("default_shell", None, r#""json""#, json!("json"));
    assert_value("packages_path", None, r#"["/json"]"#, json!(["/json"]));
    assert_value("use_rust_solver", None, "false", json!(false));
    assert_value("thread_count", None, "4", json!(4));
}

#[test]
fn masked_invalid_json_is_not_parsed() {
    let output = config_output("default_shell", Some("plain"), "not-json");
    assert!(output.status.success());
    assert_eq!(
        serde_json::from_slice::<Value>(&output.stdout).unwrap(),
        json!("plain")
    );
    assert!(!String::from_utf8_lossy(&output.stderr).contains("invalid JSON"));
}
