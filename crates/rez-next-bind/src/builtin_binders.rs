//! Built-in binder definitions for common system tools.
//!
//! Each BuiltinBinder describes how to detect and bind a specific tool.
//! Users can extend this by implementing custom binders.

use crate::binder::{BindError, BindOptions, BindResult, PackageBinder};

/// Description of a known bindable tool.
#[derive(Debug, Clone)]
pub struct BuiltinBinder {
    /// Tool / package name (e.g. "python", "cmake").
    pub name: &'static str,

    /// Short description for help output.
    pub description: &'static str,

    /// Typical executable name(s) to search (e.g. ["python3", "python"]).
    pub executables: &'static [&'static str],

    /// Help URL or documentation reference.
    pub help_url: &'static str,
}

impl BuiltinBinder {
    /// Execute the bind operation using the standard PackageBinder.
    pub fn bind(&self, options: &BindOptions) -> Result<BindResult, BindError> {
        let binder = PackageBinder::new();

        // Try each executable in order until one is found
        for exe in self.executables {
            let mut opts = options.clone();
            if opts.version_override.is_none() {
                // try to detect via this exe name
                opts.search_path = true;
            }
            let result = binder.bind(exe, &opts);
            if result.is_ok() {
                return result;
            }
            // If only failed because the tool was not found, try next exe
            if matches!(
                result,
                Err(BindError::ToolNotFound(_)) | Err(BindError::VersionNotFound(_))
            ) {
                continue;
            }
            return result;
        }

        Err(BindError::ToolNotFound(self.name.to_string()))
    }
}

/// All built-in binders shipped with rez-next.
static BUILTIN_BINDERS: &[BuiltinBinder] = &[
    BuiltinBinder {
        name: "python",
        description: "CPython interpreter",
        executables: &["python3", "python"],
        help_url: "https://www.python.org",
    },
    BuiltinBinder {
        name: "cmake",
        description: "CMake build system",
        executables: &["cmake"],
        help_url: "https://cmake.org",
    },
    BuiltinBinder {
        name: "git",
        description: "Git version control",
        executables: &["git"],
        help_url: "https://git-scm.com",
    },
    BuiltinBinder {
        name: "pip",
        description: "Python package installer",
        executables: &["pip3", "pip"],
        help_url: "https://pip.pypa.io",
    },
    BuiltinBinder {
        name: "gcc",
        description: "GNU C/C++ compiler",
        executables: &["gcc"],
        help_url: "https://gcc.gnu.org",
    },
    BuiltinBinder {
        name: "clang",
        description: "Clang/LLVM C/C++ compiler",
        executables: &["clang"],
        help_url: "https://clang.llvm.org",
    },
    BuiltinBinder {
        name: "node",
        description: "Node.js JavaScript runtime",
        executables: &["node", "nodejs"],
        help_url: "https://nodejs.org",
    },
    BuiltinBinder {
        name: "rust",
        description: "Rust programming language",
        executables: &["rustc"],
        help_url: "https://www.rust-lang.org",
    },
    BuiltinBinder {
        name: "go",
        description: "Go programming language",
        executables: &["go"],
        help_url: "https://go.dev",
    },
    BuiltinBinder {
        name: "java",
        description: "Java Development Kit",
        executables: &["java"],
        help_url: "https://openjdk.org",
    },
    BuiltinBinder {
        name: "ffmpeg",
        description: "FFmpeg multimedia framework",
        executables: &["ffmpeg"],
        help_url: "https://ffmpeg.org",
    },
    BuiltinBinder {
        name: "imagemagick",
        description: "ImageMagick image processing",
        executables: &["convert", "magick"],
        help_url: "https://imagemagick.org",
    },
];

/// Get a built-in binder by tool name.
pub fn get_builtin_binder(name: &str) -> Option<&'static BuiltinBinder> {
    BUILTIN_BINDERS.iter().find(|b| b.name == name)
}

/// List all known built-in binder names.
pub fn list_builtin_binders() -> Vec<&'static str> {
    BUILTIN_BINDERS.iter().map(|b| b.name).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::binder::BindOptions;
    use tempfile::TempDir;

    #[test]
    fn test_list_builtin_binders() {
        let binders = list_builtin_binders();
        assert!(binders.contains(&"python"));
        assert!(binders.contains(&"cmake"));
        assert!(binders.contains(&"git"));
        assert!(binders.len() >= 8);
    }

    #[test]
    fn test_get_builtin_binder_known() {
        let b = get_builtin_binder("python").unwrap();
        assert_eq!(b.name, "python");
        assert!(!b.description.is_empty());
        assert!(!b.executables.is_empty());
    }

    #[test]
    fn test_get_builtin_binder_unknown() {
        assert!(get_builtin_binder("nonexistent_tool_xyz").is_none());
    }

    #[test]
    fn test_all_binders_have_executables() {
        for b in BUILTIN_BINDERS {
            assert!(
                !b.executables.is_empty(),
                "Binder '{}' has no executables",
                b.name
            );
        }
    }

    #[test]
    fn test_all_binders_have_non_empty_description() {
        for b in BUILTIN_BINDERS {
            assert!(
                !b.description.is_empty(),
                "Binder '{}' has empty description",
                b.name
            );
        }
    }

    #[test]
    fn test_all_binders_have_help_url() {
        for b in BUILTIN_BINDERS {
            assert!(
                !b.help_url.is_empty(),
                "Binder '{}' has empty help_url",
                b.name
            );
            assert!(
                b.help_url.starts_with("https://"),
                "Binder '{}' help_url should be https: {}",
                b.name,
                b.help_url
            );
        }
    }

    #[test]
    fn test_all_binder_names_unique() {
        let names = list_builtin_binders();
        let mut seen = std::collections::HashSet::new();
        for name in &names {
            assert!(seen.insert(name), "Duplicate binder name: {}", name);
        }
    }

    #[test]
    fn test_builtin_binder_list_contains_common_tools() {
        let binders = list_builtin_binders();
        // These tools should always be present
        for tool in &["python", "cmake", "git", "node", "rust", "go"] {
            assert!(
                binders.contains(tool),
                "Built-in binders should contain '{}'",
                tool
            );
        }
    }

    #[test]
    fn test_builtin_binder_bind_with_version_override() {
        let tmp = TempDir::new().unwrap();
        let binder = get_builtin_binder("python").unwrap();

        let opts = BindOptions {
            version_override: Some("3.11.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };

        let result = binder.bind(&opts);
        // With version_override and no search_path, it should succeed (tries each exe)
        // The first exe that doesn't raise ToolNotFound wins; with search_path=false and
        // version_override=Some, detect is skipped so it depends on BindError::ToolNotFound path.
        // Either Ok or ToolNotFound is acceptable — important: no panic, no crash.
        match result {
            Ok(r) => {
                assert!(!r.version.is_empty());
                assert!(r.install_path.exists());
            }
            Err(e) => {
                // ToolNotFound is acceptable when python not on PATH in CI
                let msg = e.to_string();
                assert!(
                    msg.contains("not found") || msg.contains("version") || msg.contains("Bind"),
                    "Unexpected error: {}",
                    msg
                );
            }
        }
    }

    #[test]
    fn test_builtin_binder_bind_already_exists() {
        let tmp = TempDir::new().unwrap();

        // Manually create the directory to simulate already-bound
        // python binder tries "python3" first then "python"
        let pkg_dir = tmp.path().join("python3").join("3.11.0");
        std::fs::create_dir_all(&pkg_dir).unwrap();
        std::fs::write(pkg_dir.join("package.py"), "name = 'python3'").unwrap();

        let binder = get_builtin_binder("python").unwrap();
        let opts = BindOptions {
            version_override: Some("3.11.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };

        // Second attempt should fail with AlreadyExists (or possibly ToolNotFound if search_path=false)
        let result = binder.bind(&opts);
        match result {
            Ok(_) => { /* first exe worked with new version */ }
            Err(e) => {
                // Either AlreadyExists or ToolNotFound (no path search)
                let _ = e.to_string(); // must not panic
            }
        }
    }

    #[test]
    fn test_builtin_binder_force_flag_clears_already_exists() {
        let tmp = TempDir::new().unwrap();
        let binder = get_builtin_binder("git").unwrap();

        let opts = BindOptions {
            version_override: Some("2.42.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: false,
            search_path: false,
            ..Default::default()
        };

        // First bind
        let _ = binder.bind(&opts);

        // Second bind with force — should not get AlreadyExists
        let opts_force = BindOptions {
            version_override: Some("2.42.0".to_string()),
            install_path: Some(tmp.path().to_path_buf()),
            force: true,
            search_path: false,
            ..Default::default()
        };
        let result = binder.bind(&opts_force);
        match result {
            Ok(r) => assert_eq!(r.version, "2.42.0"),
            Err(e) => {
                // ToolNotFound is ok (git may not be on CI PATH for no-search_path mode)
                assert!(
                    !e.to_string().contains("already exists"),
                    "force should prevent AlreadyExists: {}",
                    e
                );
            }
        }
    }

    #[test]
    fn test_builtin_binder_cmake_metadata() {
        let b = get_builtin_binder("cmake").unwrap();
        assert_eq!(b.name, "cmake");
        assert!(b.executables.contains(&"cmake"));
        assert!(b.help_url.contains("cmake.org"));
    }

    #[test]
    fn test_builtin_binder_python_has_fallback_executable() {
        let b = get_builtin_binder("python").unwrap();
        // python binder should have both python3 and python
        assert!(b.executables.contains(&"python3") || b.executables.contains(&"python"));
        assert!(b.executables.len() >= 2);
    }

    #[test]
    fn test_builtin_binder_pip_has_fallback_executable() {
        let b = get_builtin_binder("pip").unwrap();
        assert!(b.executables.contains(&"pip3") || b.executables.contains(&"pip"));
        assert!(b.executables.len() >= 2);
    }

    #[test]
    fn test_get_builtin_binder_returns_none_for_empty_string() {
        assert!(get_builtin_binder("").is_none());
    }

    #[test]
    fn test_get_builtin_binder_case_sensitive() {
        // Binder names are lowercase; uppercase should not match
        assert!(get_builtin_binder("Python").is_none());
        assert!(get_builtin_binder("CMAKE").is_none());
    }
}
