//! Package metadata helpers and `package.py` generator for the bind command.

use std::path::PathBuf;

// ─── package.py generation ───────────────────────────────────────────────────

/// Generate the content of a `package.py` file.
pub fn generate_package_py(
    name: &str,
    version: &str,
    description: &str,
    requires: &[String],
    tools: &[String],
    commands: Option<&str>,
) -> String {
    let mut content = String::new();
    content.push_str(&format!("name = '{}'\n", name));
    content.push_str(&format!("version = '{}'\n\n", version));
    content.push_str(&format!("description = '{}'\n\n", description));

    if !requires.is_empty() {
        content.push_str("requires = [\n");
        for req in requires {
            content.push_str(&format!("    '{}',\n", req));
        }
        content.push_str("]\n\n");
    }

    if !tools.is_empty() {
        content.push_str("tools = [\n");
        for tool in tools {
            content.push_str(&format!("    '{}',\n", tool));
        }
        content.push_str("]\n\n");
    }

    if let Some(cmds) = commands {
        content.push_str(&format!("def commands():\n{}\n", cmds));
    }

    content
}

// ─── Per-package metadata ─────────────────────────────────────────────────────

/// Build the `def commands():` body for a system package, or `None` if not needed.
pub fn get_package_commands(name: &str, exe_path: Option<&PathBuf>) -> Option<String> {
    match name {
        "python" => {
            if let Some(path) = exe_path {
                let dir = path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !dir.is_empty() {
                    return Some(format!(
                        "    import os\n    env.PATH.prepend('{}')\n",
                        dir.replace('\\', "/")
                    ));
                }
            }
            Some("    env.PATH.prepend('{root}/bin')\n".to_string())
        }
        "pip" | "cmake" | "git" | "gcc" | "clang" => {
            if let Some(path) = exe_path {
                let dir = path
                    .parent()
                    .map(|p| p.to_string_lossy().to_string())
                    .unwrap_or_default();
                if !dir.is_empty() {
                    return Some(format!(
                        "    import os\n    env.PATH.prepend('{}')\n",
                        dir.replace('\\', "/")
                    ));
                }
            }
            None
        }
        _ => None,
    }
}

/// Return a human-readable description for a known package name.
pub fn get_package_description(name: &str) -> String {
    match name {
        "platform" => "System platform package",
        "arch" => "System architecture package",
        "os" => "Operating system package",
        "python" => "Python interpreter",
        "rez" => "Rez package manager (rez-next)",
        "pip" => "Python package installer",
        "setuptools" => "Python build and packaging utilities",
        "cmake" => "CMake build system",
        "git" => "Git version control system",
        "gcc" => "GNU Compiler Collection",
        "clang" => "Clang/LLVM compiler",
        _ => return format!("System package: {}", name),
    }
    .to_string()
}

/// Return the default `requires` list for a known package.
pub fn get_default_requirements(name: &str) -> Vec<String> {
    match name {
        "os" => vec!["platform".to_string(), "arch".to_string()],
        "python" => vec!["os".to_string()],
        "pip" => vec!["python".to_string()],
        "setuptools" => vec!["python".to_string()],
        _ => vec![],
    }
}

/// Return the default `tools` list for a known package.
pub fn get_default_tools(name: &str) -> Vec<String> {
    match name {
        "python" => vec!["python".to_string(), "python3".to_string()],
        "pip" => vec!["pip".to_string(), "pip3".to_string()],
        "cmake" => vec![
            "cmake".to_string(),
            "ctest".to_string(),
            "cpack".to_string(),
        ],
        "git" => vec!["git".to_string()],
        "gcc" => vec!["gcc".to_string(), "g++".to_string(), "cpp".to_string()],
        "clang" => vec!["clang".to_string(), "clang++".to_string()],
        _ => vec![],
    }
}
