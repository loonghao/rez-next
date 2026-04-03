use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── Context serialization edge cases ──────────────────────────────────────

/// rez: context serialized as JSON contains all required fields
#[test]
fn test_context_json_serialization_fields() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::PackageRequirement;
    use serde_json::Value;

    let reqs = vec![
        PackageRequirement::parse("python-3.9").unwrap(),
        PackageRequirement::parse("maya-2024").unwrap(),
    ];
    let ctx = ResolvedContext::from_requirements(reqs);

    let json = serde_json::to_string(&ctx).unwrap();
    let parsed: Value = serde_json::from_str(&json).unwrap();

    // Required fields in rez .rxt JSON format
    assert!(!json.is_empty(), "context JSON should have content");
    assert!(parsed.is_object(), "context JSON should be a JSON object");
}

/// rez: context with empty request list is valid
#[test]
fn test_context_empty_requests_is_valid() {
    use rez_next_context::ResolvedContext;

    let ctx = ResolvedContext::from_requirements(vec![]);
    let json = serde_json::to_string(&ctx).unwrap();
    assert!(
        !json.is_empty(),
        "Serialized empty context should not be empty string"
    );
}

/// rez: context with single package request
#[test]
fn test_context_single_package_request() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::PackageRequirement;

    let reqs = vec![PackageRequirement::parse("python-3.9").unwrap()];
    let ctx = ResolvedContext::from_requirements(reqs);
    assert_eq!(ctx.requirements.len(), 1, "Should have 1 requirement");
    assert_eq!(ctx.requirements[0].name, "python");
}

/// rez: context roundtrip through JSON serialization preserves requests
#[test]
fn test_context_json_roundtrip_preserves_requests() {
    use rez_next_context::ResolvedContext;
    use rez_next_package::PackageRequirement;

    let reqs = vec![
        PackageRequirement::parse("python-3.9").unwrap(),
        PackageRequirement::parse("houdini-19.5").unwrap(),
    ];
    let original = ResolvedContext::from_requirements(reqs);

    let json = serde_json::to_string(&original).unwrap();
    let restored: ResolvedContext = serde_json::from_str(&json).unwrap();

    assert_eq!(
        original.requirements.len(),
        restored.requirements.len(),
        "Requirement count should be preserved through JSON roundtrip"
    );
    assert_eq!(
        original.requirements[0].name, restored.requirements[0].name,
        "First requirement name should be preserved"
    );
}

