use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Context serialization round-trip tests ───────────────────────────────────

/// rez context: JSON serialization round-trip preserves context ID
#[test]
fn test_context_json_roundtrip_preserves_id() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let original = ResolvedContext::from_requirements(vec![]);
    let bytes = ContextSerializer::serialize(&original, ContextFormat::Json).unwrap();
    let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Json).unwrap();
    assert_eq!(
        restored.id, original.id,
        "JSON round-trip must preserve context ID"
    );
}

/// rez context: JSON serialization output is valid UTF-8 and non-empty
#[test]
fn test_context_json_output_is_valid_utf8() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let ctx = ResolvedContext::from_requirements(vec![]);
    let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
    assert!(!bytes.is_empty(), "Serialized context must not be empty");
    let s = String::from_utf8(bytes);
    assert!(s.is_ok(), "Serialized context must be valid UTF-8");
}

/// rez context: deserialization of corrupt bytes returns Err, not panic
#[test]
fn test_context_deserialize_corrupt_no_panic() {
    use rez_next_context::{ContextFormat, ContextSerializer};

    let result = ContextSerializer::deserialize(b"{broken json{{{{", ContextFormat::Json);
    assert!(result.is_err(), "Corrupt JSON must return Err");
}

/// rez context: environment_vars are preserved across JSON round-trip
#[test]
fn test_context_env_vars_roundtrip() {
    use rez_next_context::{ContextFormat, ContextSerializer, ResolvedContext};

    let mut ctx = ResolvedContext::from_requirements(vec![]);
    ctx.environment_vars
        .insert("MY_TOOL_ROOT".to_string(), "/opt/my_tool/1.0".to_string());
    ctx.environment_vars
        .insert("PYTHONPATH".to_string(), "/opt/python/lib".to_string());

    let bytes = ContextSerializer::serialize(&ctx, ContextFormat::Json).unwrap();
    let restored = ContextSerializer::deserialize(&bytes, ContextFormat::Json).unwrap();

    assert_eq!(
        restored.environment_vars.get("MY_TOOL_ROOT"),
        Some(&"/opt/my_tool/1.0".to_string()),
        "MY_TOOL_ROOT must survive JSON round-trip"
    );
    assert_eq!(
        restored.environment_vars.get("PYTHONPATH"),
        Some(&"/opt/python/lib".to_string()),
        "PYTHONPATH must survive JSON round-trip"
    );
}

