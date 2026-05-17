//! Python bindings for PackageVariant and PackageVariantCache
//!
//! Defines `PyPackageVariant` and `PyPackageVariantCache`, mapping
//! `rez_next_solver::package_variant::{PackageVariant, PackageVariantCache}`.

use crate::package_bindings::PyPackage;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rez_next_solver::package_variant::{PackageVariant, PackageVariantCache};

#[pyclass(name = "PackageVariant", from_py_object)]
#[derive(Clone)]
pub struct PyPackageVariant {
    inner: PackageVariant,
}

#[pymethods]
impl PyPackageVariant {
    #[new]
    fn new(py_package: PyPackage, index: usize) -> Self {
        PyPackageVariant {
            inner: PackageVariant::new(py_package.0, index),
        }
    }

    #[getter]
    fn index(&self) -> usize {
        self.inner.index
    }

    #[getter]
    fn name(&self) -> Option<String> {
        self.inner.name.clone()
    }

    fn __repr__(&self) -> String {
        format!(
            "<PackageVariant package={} index={}>",
            self.inner.package_name(),
            self.inner.index
        )
    }
}

#[pyclass(name = "PackageVariantCache", skip_from_py_object)]
#[derive(Clone)]
pub struct PyPackageVariantCache {
    inner: PackageVariantCache,
}

#[pymethods]
impl PyPackageVariantCache {
    #[new]
    fn new() -> Self {
        PyPackageVariantCache {
            inner: PackageVariantCache::new(),
        }
    }

    fn get_variants<'a>(
        &mut self,
        py: Python<'a>,
        package_name: &str,
    ) -> PyResult<Bound<'a, PyAny>> {
        if let Some(variants) = self.inner.get_variants(package_name) {
            let py_list = PyList::empty(py);
            for variant in variants {
                let py_variant = PyPackageVariant {
                    inner: variant.clone(),
                };
                py_list.append(py_variant)?;
            }
            Ok(py_list.into_any())
        } else {
            Ok(py.None().into_bound(py))
        }
    }

    fn cache_variants(&mut self, package_name: String, variants: Vec<PyPackageVariant>) {
        let variants_inner: Vec<PackageVariant> = variants.into_iter().map(|v| v.inner).collect();
        self.inner.cache_variants(package_name, variants_inner);
    }

    fn invalidate(&mut self, package_name: &str) {
        self.inner.invalidate(package_name);
    }

    fn clear(&mut self) {
        self.inner.clear();
    }

    #[getter]
    fn hit_rate(&self) -> f64 {
        self.inner.hit_rate()
    }

    fn __repr__(&self) -> String {
        format!(
            "<PackageVariantCache hit_rate={:.2}>",
            self.inner.hit_rate()
        )
    }
}

/// Register PackageVariant and PackageVariantCache types with the parent module
pub fn register_package_variant_types(parent_module: &Bound<'_, PyModule>) -> PyResult<()> {
    parent_module.add_class::<PyPackageVariant>()?;
    parent_module.add_class::<PyPackageVariantCache>()?;
    Ok(())
}
