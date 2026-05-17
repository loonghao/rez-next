//! Python bindings for package_order module.
//!
//! Exposes PackageOrder, NullPackageOrder, SortedOrder, etc. to Python.

use pyo3::prelude::*;
use rez_next_package::package_order::*;
use rez_next_version::Version;

/// NullPackageOrder - no reordering.
#[pyclass(name = "NullPackageOrder", skip_from_py_object)]
#[derive(Clone)]
struct PyNullPackageOrder {
    inner: NullPackageOrder,
}

#[pymethods]
impl PyNullPackageOrder {
    #[new]
    #[pyo3(signature = (packages = None))]
    fn new(packages: Option<Vec<String>>) -> Self {
        Self {
            inner: NullPackageOrder::new(packages),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn packages(&self) -> Option<Vec<String>> {
        self.inner.packages().map(|p| p.to_vec())
    }

    fn to_pod(&self) -> PyResult<String> {
        Ok(self.inner.to_pod().to_string())
    }

    fn sha1(&self) -> String {
        self.inner.sha1()
    }
}

/// SortedOrder - order by version.
#[pyclass(name = "SortedOrder", skip_from_py_object)]
#[derive(Clone)]
struct PySortedOrder {
    inner: SortedOrder,
}

#[pymethods]
impl PySortedOrder {
    #[new]
    #[pyo3(signature = (descending, packages = None))]
    fn new(descending: bool, packages: Option<Vec<String>>) -> Self {
        Self {
            inner: SortedOrder::new(descending, packages),
        }
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn descending(&self) -> bool {
        self.inner.descending
    }

    #[getter]
    fn packages(&self) -> Option<Vec<String>> {
        self.inner.packages().map(|p| p.to_vec())
    }

    fn to_pod(&self) -> PyResult<String> {
        Ok(self.inner.to_pod().to_string())
    }

    fn sha1(&self) -> String {
        self.inner.sha1()
    }
}

/// VersionSplitPackageOrder - split at a version.
#[pyclass(name = "VersionSplitPackageOrder", skip_from_py_object)]
#[derive(Clone)]
struct PyVersionSplitPackageOrder {
    inner: VersionSplitPackageOrder,
}

#[pymethods]
impl PyVersionSplitPackageOrder {
    #[new]
    #[pyo3(signature = (first_version, packages = None))]
    fn new(first_version: &str, packages: Option<Vec<String>>) -> PyResult<Self> {
        let ver = Version::parse(first_version)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))?;
        Ok(Self {
            inner: VersionSplitPackageOrder::new(ver, packages),
        })
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn first_version(&self) -> String {
        self.inner.first_version.to_string()
    }

    #[getter]
    fn packages(&self) -> Option<Vec<String>> {
        self.inner.packages().map(|p| p.to_vec())
    }

    fn to_pod(&self) -> PyResult<String> {
        Ok(self.inner.to_pod().to_string())
    }

    fn sha1(&self) -> String {
        self.inner.sha1()
    }
}

/// TimestampPackageOrder - order by timestamp proximity.
#[pyclass(name = "TimestampPackageOrder", skip_from_py_object)]
#[derive(Clone)]
struct PyTimestampPackageOrder {
    inner: TimestampPackageOrder,
}

#[pymethods]
impl PyTimestampPackageOrder {
    #[new]
    #[pyo3(signature = (timestamp, rank = 0, packages = None))]
    fn new(timestamp: i64, rank: i32, packages: Option<Vec<String>>) -> PyResult<Self> {
        Ok(Self {
            inner: TimestampPackageOrder::new(timestamp, rank, packages),
        })
    }

    #[getter]
    fn name(&self) -> &str {
        self.inner.name()
    }

    #[getter]
    fn timestamp(&self) -> i64 {
        self.inner.timestamp
    }

    #[getter]
    fn rank(&self) -> i32 {
        self.inner.rank
    }

    #[getter]
    fn packages(&self) -> Option<Vec<String>> {
        self.inner.packages().map(|p| p.to_vec())
    }

    fn to_pod(&self) -> PyResult<String> {
        Ok(self.inner.to_pod().to_string())
    }

    fn sha1(&self) -> String {
        self.inner.sha1()
    }
}

/// Register the package_order submodule.
pub fn register_package_order_submodule(py: Python, parent: &Bound<'_, PyModule>) -> PyResult<()> {
    let m = PyModule::new(py, "package_order")?;

    // Add classes
    m.add_class::<PyNullPackageOrder>()?;
    m.add_class::<PySortedOrder>()?;
    m.add_class::<PyVersionSplitPackageOrder>()?;
    m.add_class::<PyTimestampPackageOrder>()?;

    // Add module to parent
    parent.add_submodule(&m)?;

    // Register in sys.modules
    let sys = py.import("sys")?;
    let modules = sys.getattr("modules")?;
    let parent_name = parent.name()?;
    let full_name = format!("{}.{}", parent_name, "package_order");
    modules.set_item(full_name, &m)?;

    Ok(())
}
