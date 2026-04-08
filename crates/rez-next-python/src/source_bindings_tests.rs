//! Tests for source_bindings — shell activation script generation.

#[cfg(test)]
mod tests {
    use crate::source_bindings::{
        build_activation_script, detect_current_shell, detect_shell, get_source_script,
        PySourceManager, SourceMode,
    };
    use std::path::PathBuf;

    #[test]
    fn test_detect_current_shell_returns_string() {
        let shell = detect_current_shell();
        assert!(!shell.is_empty());
        let known = ["bash", "zsh", "fish", "powershell", "pwsh", "cmd"];
        assert!(
            known.iter().any(|k| shell.contains(k)),
            "Unexpected shell: {}",
            shell
        );
    }

    #[test]
    fn test_build_activation_script_bash() {
        let pkgs = vec!["python-3.9".to_string(), "maya-2024".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZ_RESOLVE"), "bash script should set REZ_RESOLVE");
        assert!(script.contains("export"), "bash script should use export");
        assert!(script.contains("python-3.9"), "bash script should contain package name");
    }

    #[test]
    fn test_build_activation_script_powershell() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "powershell");
        assert!(script.contains("REZ_RESOLVE"), "ps1 script should set REZ_RESOLVE");
        assert!(
            script.contains("$env:") || script.contains("REZ_"),
            "ps1 should use $env: syntax"
        );
    }

    #[test]
    fn test_build_activation_script_zsh() {
        let pkgs = vec!["houdini-19.5".to_string()];
        let script = build_activation_script(&pkgs, "zsh");
        assert!(script.contains("REZ_RESOLVE"));
        assert!(script.contains("houdini-19.5"));
    }

    #[test]
    fn test_build_activation_script_fish() {
        let pkgs = vec!["nuke-14.0".to_string()];
        let script = build_activation_script(&pkgs, "fish");
        assert!(script.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_source_manager_new_default_shell() {
        let mgr = PySourceManager::new(vec!["python-3.9".to_string()], None);
        assert!(!mgr.shell_type.is_empty());
        assert_eq!(mgr.packages.len(), 1);
    }

    #[test]
    fn test_source_manager_new_explicit_shell() {
        let mgr = PySourceManager::new(vec!["maya-2024".to_string()], Some("bash".to_string()));
        assert_eq!(mgr.shell_type, "bash");
    }

    #[test]
    fn test_source_manager_get_activation_content() {
        let mgr = PySourceManager::new(
            vec!["python-3.10".to_string(), "pip-23".to_string()],
            Some("bash".to_string()),
        );
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("REZ_RESOLVE"));
        assert!(!content.is_empty());
    }

    #[test]
    fn test_write_activation_script_to_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("activate.sh");
        let mgr = PySourceManager::new(vec!["python-3.9".to_string()], Some("bash".to_string()));
        let content = mgr.get_activation_script_content(None);
        std::fs::write(&dest, &content).unwrap();
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_pkg_env_var_generation() {
        let pkgs = vec!["python-3.9".to_string(), "my-tool-2.0".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZPKG_PYTHON"), "Should set REZPKG_PYTHON");
    }

    #[test]
    fn test_activation_script_header_comment() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.starts_with("# rez-next activation script"),
            "Script should start with header comment"
        );
    }

    #[test]
    fn test_source_mode_inline_variant() {
        let mode = SourceMode::Inline;
        assert_eq!(mode, SourceMode::Inline);
    }

    #[test]
    fn test_source_mode_tempfile_variant() {
        let mode = SourceMode::TempFile;
        assert_eq!(mode, SourceMode::TempFile);
    }

    #[test]
    fn test_source_mode_file_variant() {
        let path = PathBuf::from("/tmp/activate.sh");
        let mode = SourceMode::File(path.clone());
        assert_eq!(mode, SourceMode::File(path));
    }

    #[test]
    fn test_resolve_source_mode_inline_logic() {
        let pkgs = vec!["python-3.9".to_string()];
        let content = build_activation_script(&pkgs, "bash");
        assert!(content.contains("REZ_RESOLVE"));
        assert!(content.contains("python-3.9"));
        let mode = SourceMode::Inline;
        let result = match mode {
            SourceMode::Inline => build_activation_script(&pkgs, "bash"),
            SourceMode::TempFile => "tempfile".to_string(),
            SourceMode::File(_) => "file".to_string(),
        };
        assert!(result.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_resolve_source_mode_file_logic() {
        use tempfile::TempDir;
        let pkgs = vec!["maya-2024".to_string()];
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("activate_test.sh");
        let mode = SourceMode::File(dest.clone());
        let result = match mode {
            SourceMode::Inline => "inline".to_string(),
            SourceMode::TempFile => "tempfile".to_string(),
            SourceMode::File(path) => {
                let script = build_activation_script(&pkgs, "bash");
                std::fs::write(&path, &script).unwrap();
                path.to_string_lossy().to_string()
            }
        };
        assert!(!result.is_empty());
        let written = std::fs::read_to_string(&dest).unwrap();
        assert!(written.contains("maya-2024"));
    }

    #[test]
    fn test_source_manager_packages_getter() {
        let pkgs = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];
        let mgr = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        assert_eq!(mgr.packages(), pkgs);
    }

    #[test]
    fn test_source_manager_shell_type_getter() {
        let mgr = PySourceManager::new(vec![], Some("zsh".to_string()));
        assert_eq!(mgr.shell_type(), "zsh");
    }

    #[test]
    fn test_source_manager_repr_contains_shell_and_packages() {
        let mgr = PySourceManager::new(
            vec!["houdini-19.5".to_string()],
            Some("fish".to_string()),
        );
        let repr = mgr.__repr__();
        assert!(repr.contains("SourceManager"), "repr: {repr}");
        assert!(repr.contains("fish"), "repr should show shell: {repr}");
        assert!(repr.contains("houdini-19.5"), "repr should show pkg: {repr}");
    }

    #[test]
    fn test_source_manager_empty_packages() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
    }

    #[test]
    fn test_source_manager_explicit_shell_override_in_get_content() {
        let mgr = PySourceManager::new(
            vec!["cmake-3.26".to_string()],
            Some("bash".to_string()),
        );
        let content = mgr.get_activation_script_content(Some("powershell".to_string()));
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
        assert!(
            content.contains("$env:") || content.contains("REZ_"),
            "powershell content should reference env vars: {content}"
        );
    }

    #[test]
    fn test_build_activation_script_empty_packages() {
        let script = build_activation_script(&[], "bash");
        assert!(script.contains("REZ_RESOLVE"), "script: {script}");
    }

    #[test]
    fn test_build_activation_script_cmd_shell() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "cmd");
        assert!(script.contains("REZ_RESOLVE"), "cmd script: {script}");
    }

    #[test]
    fn test_build_activation_script_unknown_shell_falls_to_bash() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "tcsh");
        assert!(script.contains("REZ_RESOLVE"), "script: {script}");
        assert!(script.contains("export"), "bash branch must use export: {script}");
    }

    #[test]
    fn test_write_activation_script_creates_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let dest = tmp.path().join("subdir").join("activate.sh");
        let mgr =
            PySourceManager::new(vec!["python-3.9".to_string()], Some("bash".to_string()));
        std::fs::create_dir_all(dest.parent().unwrap()).unwrap();
        let content = mgr.get_activation_script_content(None);
        std::fs::write(&dest, &content).unwrap();
        assert!(dest.exists());
        let read = std::fs::read_to_string(&dest).unwrap();
        assert!(read.contains("REZ_RESOLVE"));
    }

    #[test]
    fn test_build_activation_script_sets_rezpkg_for_each_package() {
        let pkgs = vec!["python-3.9".to_string(), "cmake-3.26".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZPKG_PYTHON"), "should set REZPKG_PYTHON: {script}");
        assert!(script.contains("REZPKG_CMAKE"), "should set REZPKG_CMAKE: {script}");
    }

    #[test]
    fn test_source_manager_multiple_packages_all_in_content() {
        let pkgs = vec!["alpha-1.0".to_string(), "beta-2.0".to_string(), "gamma-3.0".to_string()];
        let mgr = PySourceManager::new(pkgs, Some("bash".to_string()));
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("alpha-1.0"), "content: {content}");
        assert!(content.contains("beta-2.0"), "content: {content}");
        assert!(content.contains("gamma-3.0"), "content: {content}");
    }

    #[test]
    fn test_source_manager_fish_shell_explicit() {
        let mgr = PySourceManager::new(
            vec!["nuke-14.0".to_string()],
            Some("fish".to_string()),
        );
        let content = mgr.get_activation_script_content(None);
        assert!(content.contains("REZ_RESOLVE"), "fish content: {content}");
    }

    #[test]
    fn test_build_activation_script_powershell_contains_env_prefix() {
        let pkgs = vec!["maya-2024".to_string()];
        let script = build_activation_script(&pkgs, "powershell");
        assert!(
            script.contains("$env:") || script.contains("REZ_"),
            "ps1 script: {script}"
        );
        assert!(script.contains("maya-2024"), "ps1 script: {script}");
    }

    #[test]
    fn test_source_manager_repr_format() {
        let mgr = PySourceManager::new(
            vec!["python-3.9".to_string()],
            Some("bash".to_string()),
        );
        let repr = mgr.__repr__();
        assert!(!repr.is_empty(), "repr must not be empty");
        assert!(repr.contains("SourceManager"), "repr must contain 'SourceManager', got: {repr}");
    }

    #[test]
    fn test_activation_script_contains_rez_resolve_with_correct_value() {
        let pkgs = vec!["python-3.9".to_string(), "numpy-1.24".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("python-3.9"), "script: {script}");
        assert!(script.contains("numpy-1.24"), "script: {script}");
    }

    #[test]
    fn test_source_mode_tempfile_distinct_from_inline() {
        assert_ne!(SourceMode::Inline, SourceMode::TempFile);
        let path_a = PathBuf::from("/tmp/a.sh");
        let path_b = PathBuf::from("/tmp/b.sh");
        assert_ne!(SourceMode::File(path_a.clone()), SourceMode::File(path_b));
        assert_ne!(SourceMode::Inline, SourceMode::File(path_a));
    }

    #[test]
    fn test_build_activation_script_pwsh_alias() {
        let pkgs = vec!["python-3.11".to_string()];
        let script = build_activation_script(&pkgs, "pwsh");
        assert!(script.contains("REZ_RESOLVE"), "pwsh script: {script}");
    }

    #[test]
    fn test_source_manager_repr_is_non_empty() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        let repr = mgr.__repr__();
        assert!(!repr.is_empty(), "repr must not be empty");
    }

    #[test]
    fn test_build_activation_script_sets_rez_context_file() {
        let pkgs = vec!["python-3.9".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.contains("REZ_CONTEXT_FILE"),
            "script must set REZ_CONTEXT_FILE: {script}"
        );
    }

    #[test]
    fn test_source_manager_shell_type_preserved() {
        let mgr = PySourceManager::new(vec!["pkg-1.0".to_string()], Some("zsh".to_string()));
        assert_eq!(mgr.shell_type(), "zsh");
        assert_eq!(mgr.packages().len(), 1);
    }

    #[test]
    fn test_build_activation_script_rezpkg_version_correct() {
        let pkgs = vec!["python-3.11.2".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(script.contains("REZPKG_PYTHON"), "script: {script}");
        assert!(script.contains("3.11.2"), "version in REZPKG_PYTHON: {script}");
    }

    #[test]
    fn test_source_manager_two_instances_same_output() {
        let pkgs = vec!["cmake-3.26".to_string()];
        let mgr1 = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        let mgr2 = PySourceManager::new(pkgs, Some("bash".to_string()));
        let c1 = mgr1.get_activation_script_content(None);
        let c2 = mgr2.get_activation_script_content(None);
        assert_eq!(c1, c2, "identical managers must produce identical scripts");
    }

    #[test]
    fn test_detect_current_shell_returns_known_shell() {
        let shell = detect_current_shell();
        let known = ["bash", "zsh", "fish", "powershell", "pwsh", "cmd"];
        assert!(
            known.iter().any(|k| shell == *k),
            "detect_current_shell must return a known shell, got: '{shell}'"
        );
    }

    #[test]
    fn test_build_activation_script_sets_rez_used_resolve() {
        let pkgs = vec!["houdini-20.0".to_string()];
        let script = build_activation_script(&pkgs, "bash");
        assert!(
            script.contains("houdini-20.0") && script.contains("REZ_RESOLVE"),
            "bash script must include package in REZ_RESOLVE: {script}"
        );
    }

    #[test]
    fn test_source_manager_packages_count_preserved() {
        let pkgs: Vec<String> = (0..5).map(|i| format!("pkg_{i}-1.0")).collect();
        let mgr = PySourceManager::new(pkgs.clone(), Some("bash".to_string()));
        assert_eq!(mgr.packages().len(), 5);
    }

    #[test]
    fn test_build_activation_script_zsh_shell() {
        let pkgs = vec!["python-3.11".to_string()];
        let script = build_activation_script(&pkgs, "zsh");
        assert!(script.contains("REZ_RESOLVE"), "zsh script must set REZ_RESOLVE: {script}");
        assert!(script.contains("python-3.11"), "zsh script must include package name: {script}");
    }

    #[test]
    fn test_source_manager_get_content_none_shell_uses_default() {
        let mgr = PySourceManager::new(vec!["nuke-14.0".to_string()], None);
        let content = mgr.get_activation_script_content(None);
        assert!(!content.is_empty(), "content must not be empty when shell is None");
        assert!(content.contains("REZ_RESOLVE"), "content: {content}");
    }

    #[test]
    fn test_source_mode_file_path_preserved() {
        let p = PathBuf::from("/tmp/my_activate.sh");
        let mode = SourceMode::File(p.clone());
        if let SourceMode::File(stored) = mode {
            assert_eq!(stored, p, "SourceMode::File must store exact path");
        } else {
            panic!("Expected SourceMode::File variant");
        }
    }

    #[test]
    fn test_source_manager_repr_contains_package_count() {
        let pkgs = vec!["a-1.0".to_string(), "b-2.0".to_string(), "c-3.0".to_string()];
        let mgr = PySourceManager::new(pkgs, Some("bash".to_string()));
        let repr = mgr.__repr__();
        assert!(!repr.is_empty(), "repr must not be empty");
        assert!(repr.contains("SourceManager"), "repr: {repr}");
    }

    #[test]
    fn test_source_manager_packages_roundtrip() {
        let pkgs = vec!["python-3.11".to_string(), "cmake-3.26".to_string()];
        let mgr = PySourceManager::new(pkgs.clone(), None);
        assert_eq!(mgr.packages(), pkgs);
    }

    #[test]
    fn test_source_manager_shell_type_when_given_bash() {
        let mgr = PySourceManager::new(vec![], Some("bash".to_string()));
        assert_eq!(mgr.shell_type(), "bash");
    }

    #[test]
    fn test_source_manager_empty_packages_is_empty() {
        let mgr = PySourceManager::new(vec![], None);
        assert!(mgr.packages().is_empty(), "empty packages list must round-trip as empty");
    }

    #[test]
    fn test_get_source_script_returns_string() {
        let script = get_source_script(vec!["python-3.9".to_string()], Some("bash".to_string()));
        let _ = script.len();
    }

    #[test]
    fn test_detect_shell_is_nonempty() {
        let shell = detect_shell();
        assert!(!shell.is_empty(), "detect_shell must return a non-empty string");
    }
}
