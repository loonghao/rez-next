//! System tool detection for the bind command.
//!
//! Each `detect_*` function probes the host system and returns a [`DetectedTool`]
//! when the corresponding executable/library is found.

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Command;

use super::utils::{extract_version_from_output, parse_version_from_string, which_executable};

/// Detected system tool information
#[derive(Debug, Clone)]
pub struct DetectedTool {
    /// Detected version string
    pub version: String,
    /// Executable path
    pub executable_path: Option<PathBuf>,
    /// Additional metadata
    pub metadata: HashMap<String, String>,
}

/// Dispatch to the appropriate detector based on `name`.
pub fn detect_system_tool(name: &str) -> Option<DetectedTool> {
    match name {
        "python" => detect_python(),
        "pip" => detect_pip(),
        "cmake" => detect_cmake(),
        "git" => detect_git(),
        "gcc" => detect_gcc(),
        "clang" => detect_clang(),
        "setuptools" => detect_setuptools(),
        "platform" | "arch" | "os" | "rez" => detect_platform_package(name),
        _ => None,
    }
}

// ─── Individual detectors ─────────────────────────────────────────────────────

fn detect_python() -> Option<DetectedTool> {
    let candidates = if cfg!(windows) {
        vec!["python", "python3", "py"]
    } else {
        vec!["python3", "python"]
    };

    for cmd in candidates {
        if let Ok(output) = Command::new(cmd).args(["--version"]).output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout).to_string()
                    + &String::from_utf8_lossy(&output.stderr);
                if let Some(ver) = extract_version_from_output(&version_output, "Python") {
                    return Some(DetectedTool {
                        version: ver,
                        executable_path: which_executable(cmd),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }
    None
}

fn detect_pip() -> Option<DetectedTool> {
    let candidates = if cfg!(windows) {
        vec!["pip", "pip3"]
    } else {
        vec!["pip3", "pip"]
    };

    for cmd in candidates {
        if let Ok(output) = Command::new(cmd).args(["--version"]).output() {
            if output.status.success() {
                let version_output = String::from_utf8_lossy(&output.stdout).to_string();
                if let Some(ver) = extract_version_from_output(&version_output, "pip") {
                    return Some(DetectedTool {
                        version: ver,
                        executable_path: which_executable(cmd),
                        metadata: HashMap::new(),
                    });
                }
            }
        }
    }
    None
}

fn detect_cmake() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("cmake").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "cmake version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("cmake"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

fn detect_git() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("git").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "git version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("git"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

fn detect_gcc() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("gcc").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            let first_line = version_output.lines().next().unwrap_or("");
            if let Some(ver) = parse_version_from_string(first_line) {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("gcc"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

fn detect_clang() -> Option<DetectedTool> {
    if let Ok(output) = Command::new("clang").args(["--version"]).output() {
        if output.status.success() {
            let version_output = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = extract_version_from_output(&version_output, "clang version") {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable("clang"),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

fn detect_setuptools() -> Option<DetectedTool> {
    let script = "import setuptools; print(setuptools.__version__)";
    let cmd = if cfg!(windows) { "python" } else { "python3" };
    if let Ok(output) = Command::new(cmd).args(["-c", script]).output() {
        if output.status.success() {
            let ver = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if !ver.is_empty() {
                return Some(DetectedTool {
                    version: ver,
                    executable_path: which_executable(cmd),
                    metadata: HashMap::new(),
                });
            }
        }
    }
    None
}

fn detect_platform_package(name: &str) -> Option<DetectedTool> {
    match name {
        "platform" => {
            let platform = if cfg!(windows) {
                "windows"
            } else if cfg!(target_os = "macos") {
                "osx"
            } else {
                "linux"
            };
            let mut m = HashMap::new();
            m.insert("platform".to_string(), platform.to_string());
            Some(DetectedTool {
                version: "1.0.0".to_string(),
                executable_path: None,
                metadata: m,
            })
        }
        "arch" => {
            let mut m = HashMap::new();
            m.insert("arch".to_string(), std::env::consts::ARCH.to_string());
            Some(DetectedTool {
                version: "1.0.0".to_string(),
                executable_path: None,
                metadata: m,
            })
        }
        "os" => Some(DetectedTool {
            version: get_os_version(),
            executable_path: None,
            metadata: HashMap::new(),
        }),
        "rez" => Some(DetectedTool {
            version: env!("CARGO_PKG_VERSION").to_string(),
            executable_path: std::env::current_exe().ok(),
            metadata: HashMap::new(),
        }),
        _ => None,
    }
}

/// Return a best-effort OS version string.
pub(crate) fn get_os_version() -> String {
    if cfg!(windows) {
        if let Ok(output) = Command::new("cmd").args(["/c", "ver"]).output() {
            let s = String::from_utf8_lossy(&output.stdout).to_string();
            if let Some(ver) = parse_version_from_string(&s) {
                return ver;
            }
        }
        "10.0".to_string()
    } else if cfg!(target_os = "macos") {
        if let Ok(output) = Command::new("sw_vers").args(["-productVersion"]).output() {
            return String::from_utf8_lossy(&output.stdout).trim().to_string();
        }
        "unknown".to_string()
    } else {
        if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
            for line in content.lines() {
                if line.starts_with("VERSION_ID=") {
                    return line
                        .trim_start_matches("VERSION_ID=")
                        .trim_matches('"')
                        .to_string();
                }
            }
        }
        "unknown".to_string()
    }
}
