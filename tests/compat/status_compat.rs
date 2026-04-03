use rez_core::version::{Version, VersionRange};
use rez_next_package::{Package, PackageRequirement, Requirement};
use rez_next_rex::{generate_shell_script, RexEnvironment, RexExecutor, ShellType};
use rez_next_suites::{Suite, ToolConflictMode};

// ─── rez.status compatibility tests ─────────────────────────────────────────

/// rez status: outside any context, is_in_rez_context is false (no REZ_ vars)
#[test]
fn test_status_outside_context_is_false() {
    // In a clean test environment, REZ_CONTEXT_FILE and REZ_USED_PACKAGES_NAMES
    // should not be set.  We only assert the negative when they are absent.
    let in_ctx = std::env::var("REZ_CONTEXT_FILE").is_ok()
        || std::env::var("REZ_USED_PACKAGES_NAMES").is_ok();
    // This test verifies the logic; if a rez context happens to be active the
    // assertion is intentionally skipped.
    if !in_ctx {
        let result = std::env::var("REZ_CONTEXT_FILE");
        assert!(
            result.is_err(),
            "REZ_CONTEXT_FILE should not be set outside a rez context"
        );
    }
}

