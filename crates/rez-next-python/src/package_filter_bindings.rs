//! Python bindings for PackageFilter and Rule types
//! Compatible with rez.package_filter API

use pyo3::prelude::*;
use pyo3::types::{PyAny, PyDict, PyList, PyString, PyType};
use rez_next_package::Package;
use rez_next_package_filter::PackageFilter;

/// Python-accessible PackageFilter class
#[pyclass(name = "PackageFilter", from_py_object)]
#[derive(Clone)]
pub struct PyPackageFilter(pub PackageFilter);

#[pymethods]
impl PyPackageFilter {
    /// Create a new empty package filter
    #[new]
    pub fn new() -> Self {
        PyPackageFilter(PackageFilter::new())
    }

    /// Check if the filter excludes the given package
    /// Returns the matching exclusion rule (as string), or None
    pub fn excludes(
        slf: &Bound<'_, Self>,
        _py: Python<'_>,
        pkg_dict: Bound<'_, PyDict>,
    ) -> Option<String> {
        let pkg = dict_to_package(&pkg_dict).ok()?;
        let filter = slf.borrow();
        let result = filter.0.excludes(&pkg);
        result.map(|rule_pod| format!("{}:{}", rule_pod.rule_type, rule_pod.pattern))
    }

    /// Check if the filter includes the given package
    pub fn includes(slf: &Bound<'_, Self>, _py: Python<'_>, pkg_dict: Bound<'_, PyDict>) -> bool {
        if let Ok(pkg) = dict_to_package(&pkg_dict) {
            slf.borrow().0.includes(&pkg)
        } else {
            false
        }
    }

    /// Add an exclusion rule from string
    pub fn add_exclusion(slf: &Bound<'_, Self>, txt: String) -> PyResult<()> {
        slf.borrow_mut()
            .0
            .add_exclusion_from_str(&txt)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, String>(e.to_string()))
    }

    /// Add an inclusion rule from string
    pub fn add_inclusion(slf: &Bound<'_, Self>, txt: String) -> PyResult<()> {
        slf.borrow_mut()
            .0
            .add_inclusion_from_str(&txt)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, String>(e.to_string()))
    }

    /// Convert to POD (Plain Old Data) for serialization
    pub fn to_pod<'py>(slf: &Bound<'py, Self>, _py: Python<'py>) -> PyResult<Bound<'py, PyDict>> {
        let pod = &slf.borrow().0.to_pod();
        let dict = PyDict::new(_py);
        for (key, values) in pod {
            let list = PyList::empty(_py);
            for value in values {
                list.append(PyString::new(_py, value))?;
            }
            dict.set_item(key, list)?;
        }
        Ok(dict)
    }

    /// Create a PackageFilter from POD
    #[classmethod]
    pub fn from_pod<'py>(
        _cls: &Bound<'py, PyType>,
        _py: Python<'py>,
        pod: Bound<'py, PyAny>,
    ) -> PyResult<Self> {
        let dict: std::collections::HashMap<String, Vec<String>> = pod.extract()?;
        let filter = PackageFilter::from_pod(&dict)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, String>(e.to_string()))?;
        Ok(PyPackageFilter(filter))
    }

    /// Calculate SHA1 hash of this filter
    pub fn sha1(slf: &Bound<'_, Self>) -> String {
        slf.borrow().0.sha1()
    }
}

/// Convert Python dict to Package struct
fn dict_to_package(pkg_dict: &Bound<'_, PyDict>) -> PyResult<Package> {
    let name: String = pkg_dict
        .get_item("name")?
        .ok_or_else(|| PyErr::new::<pyo3::exceptions::PyKeyError, &str>("name is required"))?
        .extract()?;

    let version = if let Some(v) = pkg_dict.get_item("version")? {
        let v_str: String = v.extract()?;
        Some(
            rez_next_version::Version::parse(&v_str)
                .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, String>(e.to_string()))?,
        )
    } else {
        None
    };

    let description = if let Some(d) = pkg_dict.get_item("description")? {
        Some(d.extract()?)
    } else {
        None
    };

    let timestamp = if let Some(ts) = pkg_dict.get_item("timestamp")? {
        Some(ts.extract()?)
    } else {
        None
    };

    Ok(Package {
        name,
        version,
        description,
        timestamp,
        ..Default::default()
    })
}

/// Register the module
pub fn register_module(m: &Bound<'_, PyModule>) -> PyResult<()> {
    m.add_class::<PyPackageFilter>()?;
    Ok(())
}
