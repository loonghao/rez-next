//! PyO3 bindings for BuildType and BuildSystem.
//!
//! Exposes rez_next_build::BuildType and rez_next_build::BuildSystem to Python.

use pyo3::prelude::*;
use rez_next_build::BuildSystem;
use rez_next_build::BuildType;

// ============================================================================
/// BuildType enumeration (local or central build)
// ============================================================================
#[pyclass(name = "BuildType", from_py_object)]
#[derive(Clone)]
pub struct PyBuildType {
    inner: BuildType,
}

#[pymethods]
impl PyBuildType {
    #[new]
    pub fn new(name: &str) -> PyResult<Self> {
        match BuildType::from_str_opt(name) {
            Some(bt) => Ok(Self { inner: bt }),
            None => Err(pyo3::exceptions::PyValueError::new_err(format!(
                "Invalid BuildType: '{}', expected 'local' or 'central'",
                name
            ))),
        }
    }

    pub fn __str__(&self) -> String {
        format!("BuildType.{}", self.inner.name())
    }

    pub fn __repr__(&self) -> String {
        format!("BuildType.{}", self.inner.name())
    }

    pub fn __eq__(&self, other: &Bound<'_, PyAny>) -> PyResult<bool> {
        if let Ok(other_bt) = other.extract::<PyBuildType>() {
            Ok(self.inner == other_bt.inner)
        } else {
            Ok(false)
        }
    }

    #[getter]
    pub fn value(&self) -> i32 {
        match self.inner {
            BuildType::Local => 0,
            BuildType::Central => 1,
        }
    }

    #[getter]
    pub fn name(&self) -> String {
        self.inner.name().to_string()
    }
}

impl From<&PyBuildType> for BuildType {
    fn from(py_bt: &PyBuildType) -> Self {
        py_bt.inner.clone()
    }
}

impl From<BuildType> for PyBuildType {
    fn from(bt: BuildType) -> Self {
        Self { inner: bt }
    }
}

// ============================================================================
/// BuildSystem abstraction (detect and wrap build systems)
// ============================================================================
#[pyclass(name = "BuildSystem", from_py_object)]
#[derive(Clone)]
pub struct PyBuildSystem {
    inner: BuildSystem,
}

#[pymethods]
impl PyBuildSystem {
    /// Detect build system from source directory
    #[staticmethod]
    pub fn detect(source_dir: &str) -> PyResult<Self> {
        let path = std::path::Path::new(source_dir);
        match BuildSystem::detect(path) {
            Ok(bs) => Ok(Self { inner: bs }),
            Err(e) => Err(pyo3::exceptions::PyRuntimeError::new_err(e.to_string())),
        }
    }

    /// Get the build system type name
    pub fn get_type(&self) -> String {
        match self.inner {
            BuildSystem::CMake(_) => "cmake".to_string(),
            BuildSystem::Make(_) => "make".to_string(),
            BuildSystem::Python(_) => "python".to_string(),
            BuildSystem::NodeJs(_) => "nodejs".to_string(),
            BuildSystem::Cargo(_) => "cargo".to_string(),
            BuildSystem::Custom(_) => "custom".to_string(),
        }
    }

    pub fn __str__(&self) -> String {
        format!("BuildSystem(type={})", self.get_type())
    }

    pub fn __repr__(&self) -> String {
        self.__str__()
    }
}

impl From<&PyBuildSystem> for BuildSystem {
    fn from(py_bs: &PyBuildSystem) -> Self {
        py_bs.inner.clone()
    }
}

impl From<BuildSystem> for PyBuildSystem {
    fn from(bs: BuildSystem) -> Self {
        Self { inner: bs }
    }
}

// ============================================================================
/// Get BuildType enum values for Python (like rez.build_process.BuildType)
// ============================================================================
#[pyfunction]
pub fn get_build_type_local() -> PyBuildType {
    PyBuildType {
        inner: BuildType::Local,
    }
}

#[pyfunction]
pub fn get_build_type_central() -> PyBuildType {
    PyBuildType {
        inner: BuildType::Central,
    }
}
