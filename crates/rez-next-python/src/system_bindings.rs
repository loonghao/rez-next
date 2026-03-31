//! Python bindings for rez system information
//!
//! Provides `rez.system` compatible API: platform, arch, os, rez_version, etc.

use pyo3::prelude::*;

/// System information class - compatible with rez.system
#[pyclass(name = "System")]
pub struct PySystem;

impl Default for PySystem {
    fn default() -> Self {
        Self::new()
    }
}

#[pymethods]
impl PySystem {
    #[new]
    pub fn new() -> Self {
        PySystem
    }

    fn __repr__(&self) -> String {
        format!(
            "System(platform={}, arch={}, os={})",
            Self::platform_str(),
            Self::arch_str(),
            Self::os_str()
        )
    }

    /// Current platform: linux, windows, osx
    #[getter]
    fn platform(&self) -> String {
        Self::platform_str()
    }

    /// Current CPU architecture
    #[getter]
    fn arch(&self) -> String {
        Self::arch_str()
    }

    /// Detailed OS name
    #[getter]
    fn os(&self) -> String {
        Self::os_str()
    }

    /// rez-next version
    #[getter]
    fn rez_version(&self) -> String {
        env!("CARGO_PKG_VERSION").to_string()
    }

    /// Python version (of the current interpreter)
    #[getter]
    fn python_version(&self, py: Python) -> String {
        py.version().to_string()
    }

    /// Number of logical CPUs
    #[getter]
    fn num_cpus(&self) -> usize {
        std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1)
    }

    /// Hostname
    #[getter]
    fn hostname(&self) -> String {
        std::env::var("COMPUTERNAME")
            .or_else(|_| std::env::var("HOSTNAME"))
            .unwrap_or_else(|_| "unknown".to_string())
    }
}

impl PySystem {
    fn platform_str() -> String {
        match std::env::consts::OS {
            "windows" => "windows".to_string(),
            "macos" => "osx".to_string(),
            "linux" => "linux".to_string(),
            other => other.to_string(),
        }
    }

    fn arch_str() -> String {
        match std::env::consts::ARCH {
            "x86_64" => "x86_64".to_string(),
            "aarch64" => "arm_64".to_string(),
            other => other.to_string(),
        }
    }

    fn os_str() -> String {
        // Attempt to read /etc/os-release on Linux, else use OS name
        #[cfg(target_os = "linux")]
        {
            if let Ok(content) = std::fs::read_to_string("/etc/os-release") {
                for line in content.lines() {
                    if line.starts_with("PRETTY_NAME=") {
                        return line
                            .trim_start_matches("PRETTY_NAME=")
                            .trim_matches('"')
                            .to_string();
                    }
                }
            }
            "linux".to_string()
        }
        #[cfg(target_os = "windows")]
        {
            // Try Windows version from registry or cmd
            "windows".to_string()
        }
        #[cfg(target_os = "macos")]
        {
            "osx".to_string()
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows", target_os = "macos")))]
        {
            std::env::consts::OS.to_string()
        }
    }
}

/// Global system singleton factory
#[pyfunction]
pub fn get_system() -> PySystem {
    PySystem::new()
}
