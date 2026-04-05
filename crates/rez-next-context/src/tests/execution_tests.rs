// ── Phase 111: ExecutionConfig + ContextExecutor + ProcessResult tests ─────
#[cfg(test)]
mod execution_behavior_tests {

    use crate::execution::{
        ContextExecutionBuilder, ContextExecutor, ExecutionConfig, ExecutionStats, ProcessResult,
    };
    use crate::{ContextStatus, ResolvedContext, ShellType};
    use rez_next_package::Package;
    use rez_next_version::Version;
    use std::path::PathBuf;

    fn make_context_with_pkg(name: &str, ver: &str) -> ResolvedContext {
        let mut ctx = ResolvedContext::from_requirements(vec![]);
        ctx.status = ContextStatus::Resolved;
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(ver).unwrap());
        ctx.resolved_packages.push(pkg);
        ctx.set_env_var(
            format!("{}_ROOT", name.to_uppercase()),
            format!("/packages/{}/{}", name, ver),
        );
        ctx
    }

    /// ExecutionConfig default values
    #[test]
    fn test_execution_config_defaults() {
        let cfg = ExecutionConfig::default();
        assert!(
            cfg.inherit_parent_env,
            "Should inherit parent env by default"
        );
        assert!(cfg.additional_env_vars.is_empty());
        assert!(cfg.working_directory.is_none());
        assert_eq!(cfg.timeout_seconds, 300);
        assert!(cfg.capture_output);
    }

    /// ExecutionConfig with custom values
    #[test]
    fn test_execution_config_custom() {
        let mut additional_env_vars = std::collections::HashMap::new();
        additional_env_vars.insert("MY_VAR".to_string(), "hello".to_string());
        let cfg = ExecutionConfig {
            shell_type: ShellType::Bash,
            timeout_seconds: 60,
            working_directory: Some(PathBuf::from("/tmp")),
            additional_env_vars,
            ..Default::default()
        };
        assert_eq!(cfg.timeout_seconds, 60);
        assert!(cfg.working_directory.is_some());
        assert_eq!(
            cfg.additional_env_vars.get("MY_VAR"),
            Some(&"hello".to_string())
        );
    }

    /// ContextExecutor::new creates executor with default config
    #[test]
    fn test_context_executor_new() {
        let ctx = make_context_with_pkg("python", "3.9.0");
        let executor = ContextExecutor::new(ctx);
        assert!(executor.get_context().resolved_packages.len() == 1);
    }

    /// ContextExecutor get_execution_stats reports correct counts
    #[test]
    fn test_context_executor_stats() {
        let ctx = make_context_with_pkg("maya", "2023.0");
        let executor = ContextExecutor::new(ctx);
        let stats = executor.get_execution_stats();
        assert_eq!(stats.package_count, 1);
        assert_eq!(stats.env_var_count, 1, "Should have MAYA_ROOT env var");
    }

    /// ContextExecutionBuilder fluent API
    #[test]
    fn test_execution_builder_fluent_api() {
        let ctx = make_context_with_pkg("houdini", "20.0");
        let executor = ContextExecutionBuilder::new(ctx)
            .with_shell(ShellType::PowerShell)
            .with_timeout(120)
            .with_working_directory(PathBuf::from("/work"))
            .with_env_var("TEST_VAR".to_string(), "test_value".to_string())
            .with_capture_output(false)
            .build();
        let stats = executor.get_execution_stats();
        assert_eq!(stats.package_count, 1);
        assert!(stats.working_directory.is_some());
    }

    /// ProcessResult is_success reflects exit code 0
    #[test]
    fn test_process_result_is_success() {
        let result = ProcessResult {
            program: "python".to_string(),
            args: vec!["--version".to_string()],
            exit_code: 0,
            stdout: "Python 3.9.0".to_string(),
            stderr: "".to_string(),
            execution_time_ms: 50,
        };
        assert!(result.is_success());
        assert_eq!(result.command_line(), "python --version");
    }

    /// ProcessResult combined_output merges stdout and stderr
    #[test]
    fn test_process_result_combined_output() {
        let with_both = ProcessResult {
            program: "cmd".to_string(),
            args: vec![],
            exit_code: 1,
            stdout: "output".to_string(),
            stderr: "error".to_string(),
            execution_time_ms: 10,
        };
        let combined = with_both.combined_output();
        assert!(combined.contains("output"));
        assert!(combined.contains("error"));
        assert!(!with_both.is_success());

        // Only stdout
        let only_out = ProcessResult {
            program: "prog".to_string(),
            args: vec![],
            exit_code: 0,
            stdout: "hello".to_string(),
            stderr: "".to_string(),
            execution_time_ms: 5,
        };
        assert_eq!(only_out.combined_output(), "hello");
    }

    /// ExecutionStats serialization
    #[test]
    fn test_execution_stats_serialization() {
        let stats = ExecutionStats {
            context_id: "ctx-001".to_string(),
            package_count: 3,
            env_var_count: 10,
            tool_count: 5,
            shell_type: ShellType::Bash,
            working_directory: Some(PathBuf::from("/work")),
        };
        let json = serde_json::to_string(&stats).unwrap();
        assert!(json.contains("ctx-001"));
        assert!(json.contains("3"));
        let restored: ExecutionStats = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.context_id, "ctx-001");
        assert_eq!(restored.package_count, 3);
    }
}
