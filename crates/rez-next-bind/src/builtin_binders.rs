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
}
