use std::fs;

/// Helper: create a temp dir with a specific marker file and return the path string.
fn make_temp_dir_with_file(dir_name: &str, marker: &str) -> std::path::PathBuf {
    let tmp = std::env::temp_dir().join(dir_name);
    let _ = fs::remove_dir_all(&tmp);
    fs::create_dir_all(&tmp).unwrap();
    if !marker.is_empty() {
        fs::write(tmp.join(marker), b"").unwrap();
    }
    tmp
}

mod test_get_build_system {
    use super::*;
    use crate::build_functions::get_build_system;

    #[test]
    fn test_cmake_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_cmake", "CMakeLists.txt");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_makefile_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_make", "Makefile");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "make");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_setup_py_detected_as_python() {
        let tmp = make_temp_dir_with_file("rez_bs_setup_py", "setup.py");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_pyproject_toml_detected_as_python() {
        let tmp = make_temp_dir_with_file("rez_bs_pyproject", "pyproject.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_package_json_detected_as_nodejs() {
        let tmp = make_temp_dir_with_file("rez_bs_pkgjson", "package.json");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "nodejs");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cargo_toml_detected_as_cargo() {
        let tmp = make_temp_dir_with_file("rez_bs_cargo", "Cargo.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cargo");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_empty_directory_returns_unknown() {
        let tmp = make_temp_dir_with_file("rez_bs_unknown", "");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "unknown");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_rezbuild_py_has_highest_priority() {
        let tmp = make_temp_dir_with_file("rez_bs_priority", "rezbuild.py");
        fs::write(tmp.join("CMakeLists.txt"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py should take priority over CMakeLists.txt");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_sh_detected_as_custom_script() {
        let tmp = make_temp_dir_with_file("rez_bs_build_sh", "build.sh");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script", "build.sh should map to custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_bat_detected_as_custom_script() {
        let tmp = make_temp_dir_with_file("rez_bs_build_bat", "build.bat");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script", "build.bat should map to custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_lowercase_makefile_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_makefile_lower", "makefile");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "make", "lowercase makefile should map to make");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_nonexistent_directory_returns_unknown() {
        let missing = std::env::temp_dir()
            .join("rez_bs_missing_parent")
            .join("rez_bs_missing_dir_xyz_abc_999");
        let result = get_build_system(Some(missing.to_str().unwrap())).unwrap();
        assert_eq!(result, "unknown");
    }

    #[test]
    fn test_cmake_priority_over_makefile() {
        let tmp = make_temp_dir_with_file("rez_bs_cmake_priority", "CMakeLists.txt");
        fs::write(tmp.join("Makefile"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake", "cmake should take priority over Makefile");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_pyproject_toml_coexists_with_setup_py() {
        let tmp = make_temp_dir_with_file("rez_bs_both_py", "pyproject.toml");
        fs::write(tmp.join("setup.py"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python", "python build system for both py files");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_rezbuild_py_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_rezbuild", "rezbuild.py");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py must map to python_rezbuild");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cmake_priority_over_python() {
        let tmp = make_temp_dir_with_file("rez_bs_cmake_vs_py", "CMakeLists.txt");
        fs::write(tmp.join("setup.py"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake", "cmake should take priority over setup.py");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cargo_toml_with_package_json_nodejs_wins() {
        // package.json is checked before Cargo.toml in get_build_system priority order
        let tmp = make_temp_dir_with_file("rez_bs_cargo_vs_node", "Cargo.toml");
        fs::write(tmp.join("package.json"), b"{}").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "nodejs", "package.json takes priority over Cargo.toml");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_source_dir_none_uses_cwd() {
        let result = get_build_system(None);
        assert!(result.is_ok(), "get_build_system(None) must return Ok");
    }

    #[test]
    fn test_all_known_build_system_types_exact_mapping() {
        for (marker, expected) in &[
            ("CMakeLists.txt", "cmake"),
            ("Makefile", "make"),
            ("rezbuild.py", "python_rezbuild"),
            ("setup.py", "python"),
            ("package.json", "nodejs"),
            ("Cargo.toml", "cargo"),
            ("build.sh", "custom_script"),
        ] {
            let dir_name = format!("rez_bs_type_check_{}", marker.replace('.', "_"));
            let tmp = make_temp_dir_with_file(&dir_name, marker);
            let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
            assert_eq!(result, *expected, "marker '{}' must map to build system '{}'", marker, expected);
            let _ = fs::remove_dir_all(&tmp);
        }
    }

    #[test]
    fn test_rezbuild_py_beats_cmake_and_makefile() {
        let tmp = make_temp_dir_with_file("rez_bs_reb_prio_all", "rezbuild.py");
        fs::write(tmp.join("CMakeLists.txt"), b"").unwrap();
        fs::write(tmp.join("Makefile"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py must have highest priority");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_sh_beats_empty_dir() {
        let tmp = make_temp_dir_with_file("rez_bs_buildsh_only", "build.sh");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script", "build.sh must map to custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_pyproject_toml_alone_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_pyproj_only", "pyproject.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python", "pyproject.toml alone should map to python");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cargo_toml_alone_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_cargo_only", "Cargo.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cargo", "Cargo.toml alone should map to cargo");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_package_json_alone_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_pkgjson_only", "package.json");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "nodejs", "package.json alone should map to nodejs");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_build_system_result_is_non_empty() {
        let tmp = make_temp_dir_with_file("rez_bs_non_empty", "");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert!(!result.is_empty(), "get_build_system must always return non-empty string");
        let _ = fs::remove_dir_all(&tmp);
    }

    // ─────── Cycle 118 additions ──────────────────────────────────────────

    #[test]
    fn test_cmake_detected_by_exact_filename() {
        let tmp = make_temp_dir_with_file("rez_bs_cmake_exact", "CMakeLists.txt");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake", "CMakeLists.txt should detect cmake");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_makefile_uppercase_detected() {
        let tmp = make_temp_dir_with_file("rez_bs_Makefile_upper", "Makefile");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "make", "Makefile should detect make");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_setup_py_yields_python_build_system() {
        let tmp = make_temp_dir_with_file("rez_bs_setup_py2", "setup.py");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python", "setup.py should detect python build system");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cargo_toml_yields_cargo_build_system() {
        let tmp = make_temp_dir_with_file("rez_bs_cargo2", "Cargo.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cargo", "Cargo.toml should detect cargo build system");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_package_json_yields_nodejs_build_system() {
        let tmp = make_temp_dir_with_file("rez_bs_pkgjson2", "package.json");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "nodejs", "package.json should detect nodejs build system");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_unknown_dir_returns_unknown_string() {
        let tmp = make_temp_dir_with_file("rez_bs_unk_str", "");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "unknown", "empty dir should return 'unknown'");
        let _ = fs::remove_dir_all(&tmp);
    }

    // ─────── Cycle 122 additions ──────────────────────────────────────────

    #[test]
    fn test_get_build_system_returns_ok_always() {
        let cases = [
            Some("/nonexistent_dir_rez_bs_abc"),
            None,
        ];
        for case in cases {
            let result = get_build_system(case);
            assert!(result.is_ok(), "get_build_system must always return Ok, got: {:?}", result);
        }
    }

    #[test]
    fn test_rezbuild_py_priority_over_all_build_markers() {
        let tmp = make_temp_dir_with_file("rez_bs_rebpy_wins", "rezbuild.py");
        for m in &["CMakeLists.txt", "Makefile", "setup.py", "Cargo.toml", "package.json"] {
            fs::write(tmp.join(m), b"").unwrap();
        }
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py must have highest priority");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_bat_yields_custom_script() {
        let tmp = make_temp_dir_with_file("rez_bs_bat_cs", "build.bat");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script", "build.bat must map to custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_build_sh_yields_custom_script() {
        let tmp = make_temp_dir_with_file("rez_bs_sh_cs", "build.sh");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script", "build.sh must map to custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_lowercase_makefile_yields_make() {
        let tmp = make_temp_dir_with_file("rez_bs_lc_mk", "makefile");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "make", "lowercase makefile should map to make");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_cmake_priority_when_multiple_markers_present() {
        let tmp = make_temp_dir_with_file("rez_bs_cmake_multi", "CMakeLists.txt");
        fs::write(tmp.join("Makefile"), b"").unwrap();
        fs::write(tmp.join("setup.py"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake", "cmake should take priority when multiple markers present");
        let _ = fs::remove_dir_all(&tmp);
    }

    // ─────── Cycle 130 additions ──────────────────────────────────────────

    #[test]
    fn test_build_bat_takes_priority_over_unknown() {
        let tmp = make_temp_dir_with_file("rez_bs_cy130_bat", "build.bat");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "custom_script");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_rezbuild_py_beats_cargo_toml() {
        let tmp = make_temp_dir_with_file("rez_bs_cy130_reb_cargo", "rezbuild.py");
        fs::write(tmp.join("Cargo.toml"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py must beat Cargo.toml");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_rezbuild_py_beats_package_json() {
        let tmp = make_temp_dir_with_file("rez_bs_cy130_reb_pkgjson", "rezbuild.py");
        fs::write(tmp.join("package.json"), b"{}").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python_rezbuild", "rezbuild.py must beat package.json");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_build_system_deep_path_still_works() {
        let base = std::env::temp_dir()
            .join("rez_bs_cy130_deep")
            .join("a")
            .join("b")
            .join("c");
        let _ = fs::remove_dir_all(&base);
        fs::create_dir_all(&base).unwrap();
        fs::write(base.join("Makefile"), b"").unwrap();
        let result = get_build_system(Some(base.to_str().unwrap())).unwrap();
        assert_eq!(result, "make", "nested path should still detect Makefile");
        let _ = fs::remove_dir_all(std::env::temp_dir().join("rez_bs_cy130_deep"));
    }

    #[test]
    fn test_get_build_system_pyproject_beats_package_json_when_checked_first() {
        let tmp = make_temp_dir_with_file("rez_bs_cy130_pyproj_only", "pyproject.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "python", "pyproject.toml must yield python build system");
        let _ = fs::remove_dir_all(&tmp);
    }

    // ─────── Cycle 132 additions ──────────────────────────────────────────

    #[test]
    fn test_get_build_system_result_is_known_value() {
        let known = [
            "cmake", "make", "python_rezbuild", "python", "nodejs", "cargo", "custom_script", "unknown",
        ];
        let tmp = make_temp_dir_with_file("rez_bs_cy132_known", "");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert!(
            known.contains(&result.as_str()),
            "result '{}' must be one of the known build system identifiers",
            result
        );
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_build_system_cmake_over_build_sh() {
        let tmp = make_temp_dir_with_file("rez_bs_cy132_cmake_sh", "CMakeLists.txt");
        fs::write(tmp.join("build.sh"), b"").unwrap();
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cmake", "cmake must take priority over build.sh");
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn test_get_build_system_cargo_toml_alone_is_cargo() {
        let tmp = make_temp_dir_with_file("rez_bs_cy132_cargo_alone", "Cargo.toml");
        let result = get_build_system(Some(tmp.to_str().unwrap())).unwrap();
        assert_eq!(result, "cargo", "Cargo.toml alone must be detected as cargo");
        let _ = fs::remove_dir_all(&tmp);
    }
}
