//! Python bindings for Package, PackageRequirement, and PackageFormat

use crate::version_bindings::PyVersion;
use pyo3::prelude::*;
use pyo3::types::PyDict;
use rez_next_package::{Package, PackageRequirement};
use rez_next_package::serialization::PackageFormat;

/// Python-accessible Package class, compatible with rez.packages.Package
#[pyclass(name = "Package", from_py_object)]
#[derive(Clone)]
pub struct PyPackage(pub Package);

#[pymethods]
impl PyPackage {
    /// Create a new Package with the given name
    #[new]
    pub fn new(name: String) -> Self {
        PyPackage(Package::new(name))
    }

    fn __str__(&self) -> String {
        match &self.0.version {
            Some(v) => format!("{}-{}", self.0.name, v.as_str()),
            None => self.0.name.clone(),
        }
    }

    fn __repr__(&self) -> String {
        format!("Package('{}')", self.__str__())
    }

    fn __eq__(&self, other: &PyPackage) -> bool {
        self.0.name == other.0.name
            && self.0.version.as_ref().map(|v| v.as_str())
                == other.0.version.as_ref().map(|v| v.as_str())
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.name.hash(&mut h);
        if let Some(ref v) = self.0.version {
            v.as_str().hash(&mut h);
        }
        h.finish()
    }

    /// Package name
    #[getter]
    fn name(&self) -> String {
        self.0.name.clone()
    }

    /// Package version as PyVersion (or None)
    #[getter]
    fn version(&self) -> Option<PyVersion> {
        self.0.version.as_ref().map(|v| PyVersion(v.clone()))
    }

    /// Package version as string
    #[getter]
    fn version_str(&self) -> Option<String> {
        self.0.version.as_ref().map(|v| v.as_str().to_string())
    }

    /// Qualified name (name-version)
    #[getter]
    fn qualified_name(&self) -> String {
        self.__str__()
    }

    /// Description
    #[getter]
    fn description(&self) -> Option<String> {
        self.0.description.clone()
    }

    #[setter]
    fn set_description(&mut self, value: Option<String>) {
        self.0.description = value;
    }

    /// Authors
    #[getter]
    fn authors(&self) -> Vec<String> {
        self.0.authors.clone()
    }

    #[setter]
    fn set_authors(&mut self, authors: Vec<String>) {
        self.0.authors = authors;
    }

    /// Runtime requires
    #[getter]
    fn requires(&self) -> Vec<String> {
        self.0.requires.clone()
    }

    #[setter]
    fn set_requires(&mut self, requires: Vec<String>) {
        self.0.requires = requires;
    }

    /// Build requires
    #[getter]
    fn build_requires(&self) -> Vec<String> {
        self.0.build_requires.clone()
    }

    #[setter]
    fn set_build_requires(&mut self, requires: Vec<String>) {
        self.0.build_requires = requires;
    }

    /// Private build requires
    #[getter]
    fn private_build_requires(&self) -> Vec<String> {
        self.0.private_build_requires.clone()
    }

    #[setter]
    fn set_private_build_requires(&mut self, requires: Vec<String>) {
        self.0.private_build_requires = requires;
    }

    /// Variants
    #[getter]
    fn variants(&self) -> Vec<Vec<String>> {
        self.0.variants.clone()
    }

    #[setter]
    fn set_variants(&mut self, variants: Vec<Vec<String>>) {
        self.0.variants = variants;
    }

    /// Tools
    #[getter]
    fn tools(&self) -> Vec<String> {
        self.0.tools.clone()
    }

    #[setter]
    fn set_tools(&mut self, tools: Vec<String>) {
        self.0.tools = tools;
    }

    /// Commands string
    #[getter]
    fn commands(&self) -> Option<String> {
        self.0.commands.clone()
    }

    #[setter]
    fn set_commands(&mut self, commands: Option<String>) {
        self.0.commands = commands;
    }

    /// Timestamp (Unix)
    #[getter]
    fn timestamp(&self) -> Option<i64> {
        self.0.timestamp
    }

    /// UUID
    #[getter]
    fn uuid(&self) -> Option<String> {
        self.0.uuid.clone()
    }

    #[setter]
    fn set_uuid(&mut self, uuid: Option<String>) {
        self.0.uuid = uuid;
    }

    /// Whether package is cachable
    #[getter]
    fn cachable(&self) -> Option<bool> {
        self.0.cachable
    }

    #[setter]
    fn set_cachable(&mut self, cachable: Option<bool>) {
        self.0.cachable = cachable;
    }

    /// Whether package is relocatable
    #[getter]
    fn relocatable(&self) -> Option<bool> {
        self.0.relocatable
    }

    #[setter]
    fn set_relocatable(&mut self, relocatable: Option<bool>) {
        self.0.relocatable = relocatable;
    }

    /// Whether this is a developer package (loaded from a working directory)
    #[getter]
    fn is_dev_package(&self) -> Option<bool> {
        self.0.is_dev_package
    }

    /// Set whether this is a developer package
    #[setter]
    fn set_is_dev_package(&mut self, value: Option<bool>) {
        self.0.is_dev_package = value;
    }

    /// File path to the package definition file (package.py or package.yaml)
    /// Aligns with Rez's DeveloperPackage.filepath attribute.
    #[getter]
    fn filepath(&self) -> Option<String> {
        self.0.filepath.clone()
    }

    /// Set the file path
    #[setter]
    fn set_filepath(&mut self, path: Option<String>) {
        self.0.filepath = path;
    }

    /// Set of included Python modules (from @include decorators)
    /// Aligns with Rez's DeveloperPackage.includes attribute.
    #[getter]
    fn includes(&self) -> Option<Vec<String>> {
        self.0.includes.as_ref().map(|set| set.iter().cloned().collect())
    }

    /// Set the includes set
    #[setter]
    fn set_includes(&mut self, includes: Option<Vec<String>>) {
        self.0.includes = includes.map(|v| v.into_iter().collect());
    }

    /// Get the root directory of the package (parent of filepath).
    /// Aligns with Rez's DeveloperPackage.root property.
    fn root(&self) -> Option<String> {
        self.0.root()
    }

    /// Set the version string (rez compat helper)
    fn set_version(&mut self, version_str: &str) -> PyResult<()> {
        use rez_next_version::Version;
        let v = Version::parse(version_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))?;
        self.0.version = Some(v);
        Ok(())
    }

    /// Load a package from file (package.py or package.yaml)
    #[staticmethod]
    fn load(path: &str) -> PyResult<PyPackage> {
        use rez_next_package::serialization::PackageSerializer;
        use std::path::PathBuf;

        let path_buf = PathBuf::from(path);
        let mut pkg = PackageSerializer::load_from_file(&path_buf)
            .map(PyPackage)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))?;

        // Set filepath to track where the package was loaded from
        pkg.0.filepath = Some(path_buf.to_string_lossy().to_string());

        Ok(pkg)
    }

    /// Create a Package from a Python dictionary.
    /// Equivalent to `rez.packages.create_package(data)`.
    ///
    /// The dict must contain at least a "name" key.
    /// Optional keys: version, description, authors, requires,
    /// build_requires, variants, tools, commands, uuid, timestamp, etc.
    #[staticmethod]
    pub fn from_dict(_py: Python<'_>, data: Bound<'_, PyDict>) -> PyResult<PyPackage> {
        use rez_next_package::Package;
        use rez_next_version::Version;

        // Extract required field: name
        let name = match data.get_item("name")? {
            Some(val) => val.extract::<String>()
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(
                    format!("Invalid 'name' field: {}", e)
                ))?,
            None => {
                return Err(pyo3::exceptions::PyValueError::new_err(
                    "Package dict must contain 'name' field"
                ))
            }
        };

        let mut pkg = Package::new(name);

        // Extract optional fields
        if let Some(val) = data.get_item("version")? {
            let version_str: String = val.extract()
                .map_err(|e| pyo3::exceptions::PyValueError::new_err(
                    format!("Invalid 'version' field: {}", e)
                ))?;
            pkg.version = Some(
                Version::parse(&version_str)
                    .map_err(|e| pyo3::exceptions::PyValueError::new_err(
                        format!("Failed to parse version '{}': {}", version_str, e)
                    ))?
            );
        }

        if let Some(val) = data.get_item("description")? {
            pkg.description = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("authors")? {
            pkg.authors = val.extract()?;
        }

        if let Some(val) = data.get_item("requires")? {
            pkg.requires = val.extract()?;
        }

        if let Some(val) = data.get_item("build_requires")? {
            pkg.build_requires = val.extract()?;
        }

        if let Some(val) = data.get_item("private_build_requires")? {
            pkg.private_build_requires = val.extract()?;
        }

        if let Some(val) = data.get_item("variants")? {
            pkg.variants = val.extract()?;
        }

        if let Some(val) = data.get_item("tools")? {
            pkg.tools = val.extract()?;
        }

        if let Some(val) = data.get_item("commands")? {
            pkg.commands = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("uuid")? {
            pkg.uuid = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("timestamp")? {
            pkg.timestamp = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("cachable")? {
            pkg.cachable = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("relocatable")? {
            pkg.relocatable = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("format_version")? {
            pkg.format_version = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("vcs")? {
            pkg.vcs = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("changelog")? {
            pkg.changelog = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("release_message")? {
            pkg.release_message = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("revision")? {
            pkg.revision = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("hashed_variants")? {
            pkg.hashed_variants = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("preprocess")? {
            pkg.preprocess = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("is_dev_package")? {
            pkg.is_dev_package = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("plugin_for")? {
            pkg.plugin_for = val.extract()?;
        }

        if let Some(val) = data.get_item("build_command")? {
            pkg.build_command = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("build_system")? {
            pkg.build_system = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("pre_commands")? {
            pkg.pre_commands = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("post_commands")? {
            pkg.post_commands = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("pre_test_commands")? {
            pkg.pre_test_commands = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("pre_build_commands")? {
            pkg.pre_build_commands = Some(val.extract()?);
        }

        if let Some(val) = data.get_item("requires_rez_version")? {
            pkg.requires_rez_version = Some(val.extract()?);
        }

        // Extract tests dict
        if let Some(val) = data.get_item("tests")? {
            pkg.tests = val.extract()?;
        }

        // Extract config dict
        if let Some(val) = data.get_item("config")? {
            pkg.config = val.extract()?;
        }

        Ok(PyPackage(pkg))
    }

    /// Validate the package definition
    fn validate(&self) -> PyResult<bool> {
        self.0
            .validate()
            .map(|_| true)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    /// Check if the package definition is valid without raising exceptions
    fn is_valid(&self) -> bool {
        self.0.is_valid()
    }

    /// Get the format version
    #[getter]
    fn format_version(&self) -> Option<i32> {
        self.0.format_version
    }

    /// Save the package to a file (auto-detects format from extension)
    fn save(&self, path: &str) -> PyResult<()> {
        use rez_next_package::serialization::{PackageFormat, PackageSerializer};
        use std::path::PathBuf;

        let path_buf = PathBuf::from(path);
        let format = PackageFormat::from_extension(&path_buf)
            .ok_or_else(|| {
                pyo3::exceptions::PyValueError::new_err(format!(
                    "Cannot detect format from path: {}",
                    path
                ))
            })?;

        PackageSerializer::save_to_file(&self.0, &path_buf, format)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Save the package to a file with explicit format
    fn save_as(&self, path: &str, format: &PyPackageFormat) -> PyResult<()> {
        use rez_next_package::serialization::PackageSerializer;
        use std::path::PathBuf;

        PackageSerializer::save_to_file(&self.0, &PathBuf::from(path), format.0)
            .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
    }

    /// Convert the package to a Python dictionary.
    /// Equivalent to `dict(package)` in Rez.
    fn to_dict<'a>(&self, py: Python<'a>) -> PyResult<Bound<'a, PyDict>> {
        use pyo3::types::PyDict;

        let dict = PyDict::new(py);

        // Add all non-None fields
        dict.set_item("name", self.0.name.clone())?;

        if let Some(ref v) = self.0.version {
            dict.set_item("version", v.as_str())?;
        }
        if let Some(ref d) = self.0.description {
            dict.set_item("description", d.clone())?;
        }
        if !self.0.authors.is_empty() {
            dict.set_item("authors", self.0.authors.clone())?;
        }
        if !self.0.requires.is_empty() {
            dict.set_item("requires", self.0.requires.clone())?;
        }
        if !self.0.build_requires.is_empty() {
            dict.set_item("build_requires", self.0.build_requires.clone())?;
        }
        if !self.0.private_build_requires.is_empty() {
            dict.set_item("private_build_requires", self.0.private_build_requires.clone())?;
        }
        if !self.0.variants.is_empty() {
            dict.set_item("variants", self.0.variants.clone())?;
        }
        if !self.0.tools.is_empty() {
            dict.set_item("tools", self.0.tools.clone())?;
        }
        if let Some(ref c) = self.0.commands {
            dict.set_item("commands", c.clone())?;
        }
        if let Some(ref u) = self.0.uuid {
            dict.set_item("uuid", u.clone())?;
        }
        if let Some(t) = self.0.timestamp {
            dict.set_item("timestamp", t)?;
        }
        if let Some(c) = self.0.cachable {
            dict.set_item("cachable", c)?;
        }
        if let Some(r) = self.0.relocatable {
            dict.set_item("relocatable", r)?;
        }
        if let Some(ref v) = self.0.format_version {
            dict.set_item("format_version", *v)?;
        }

        Ok(dict.into())
    }

    /// Convert the package to a Python-formatted string (package.py format).
    /// This generates a string that can be written to a package.py file.
    fn to_package_py(&self) -> PyResult<String> {
        let mut lines = Vec::new();

        // Add encoding declaration
        lines.push("# -*- coding: utf-8 -*-".to_string());
        lines.push("".to_string());

        // Add name (required)
        lines.push(format!("name = \"{}\"", self.0.name));

        // Add version
        if let Some(ref v) = self.0.version {
            lines.push(format!("version = \"{}\"", v.as_str()));
        }

        // Add description
        if let Some(ref d) = self.0.description {
            if d.len() > 40 {
                lines.push("".to_string());
                lines.push("description = \"\"\"".to_string());
                lines.push(d.clone());
                lines.push("\"\"\"".to_string());
            } else {
                lines.push(format!("description = \"{}\"", d));
            }
        }

        // Add authors
        if !self.0.authors.is_empty() {
            lines.push("".to_string());
            if self.0.authors.len() == 1 {
                lines.push(format!("authors = [\"{}\"]", self.0.authors[0]));
            } else {
                lines.push("authors = [".to_string());
                for author in &self.0.authors {
                    lines.push(format!("    \"{}\",", author));
                }
                lines.push("]".to_string());
            }
        }

        // Add requires
        if !self.0.requires.is_empty() {
            lines.push("".to_string());
            if self.0.requires.len() == 1 {
                lines.push(format!("requires = [\"{}\"]", self.0.requires[0]));
            } else {
                lines.push("requires = [".to_string());
                for req in &self.0.requires {
                    lines.push(format!("    \"{}\",", req));
                }
                lines.push("]".to_string());
            }
        }

        // Add build_requires
        if !self.0.build_requires.is_empty() {
            lines.push("".to_string());
            if self.0.build_requires.len() == 1 {
                lines.push(format!("build_requires = [\"{}\"]", self.0.build_requires[0]));
            } else {
                lines.push("build_requires = [".to_string());
                for req in &self.0.build_requires {
                    lines.push(format!("    \"{}\",", req));
                }
                lines.push("]".to_string());
            }
        }

        // Add variants
        if !self.0.variants.is_empty() {
            lines.push("".to_string());
            lines.push("variants = [".to_string());
            for variant in &self.0.variants {
                if variant.len() == 1 {
                    lines.push(format!("    [\"{}\"],", variant[0]));
                } else {
                    lines.push("    [".to_string());
                    for item in variant {
                        lines.push(format!("        \"{}\",", item));
                    }
                    lines.push("    ],".to_string());
                }
            }
            lines.push("]".to_string());
        }

        // Add tools
        if !self.0.tools.is_empty() {
            lines.push("".to_string());
            if self.0.tools.len() == 1 {
                lines.push(format!("tools = [\"{}\"]", self.0.tools[0]));
            } else {
                lines.push("tools = [".to_string());
                for tool in &self.0.tools {
                    lines.push(format!("    \"{}\",", tool));
                }
                lines.push("]".to_string());
            }
        }

        // Add uuid
        if let Some(ref u) = self.0.uuid {
            lines.push("".to_string());
            lines.push(format!("uuid = \"{}\"", u));
        }

        Ok(lines.join("\n"))
    }
}

/// PackageFormat enum for Python
#[pyclass(name = "PackageFormat", from_py_object)]
#[derive(Clone)]
pub struct PyPackageFormat(pub PackageFormat);

#[pymethods]
impl PyPackageFormat {
    #[classattr]
    fn yaml() -> Self {
        PyPackageFormat(PackageFormat::Yaml)
    }

    #[classattr]
    fn json() -> Self {
        PyPackageFormat(PackageFormat::Json)
    }

    #[classattr]
    fn python() -> Self {
        PyPackageFormat(PackageFormat::Python)
    }

    #[classattr]
    fn yaml_compressed() -> Self {
        PyPackageFormat(PackageFormat::YamlCompressed)
    }

    #[classattr]
    fn json_compressed() -> Self {
        PyPackageFormat(PackageFormat::JsonCompressed)
    }

    #[classattr]
    fn binary() -> Self {
        PyPackageFormat(PackageFormat::Binary)
    }

    #[classattr]
    fn toml() -> Self {
        PyPackageFormat(PackageFormat::Toml)
    }

    #[classattr]
    fn xml() -> Self {
        PyPackageFormat(PackageFormat::Xml)
    }

    fn __str__(&self) -> String {
        format!("{:?}", self.0)
    }
}

/// Load a package from file (package.py or package.yaml)
#[pyfunction]
pub fn load_package_from_file(path: &str) -> PyResult<PyPackage> {
    use rez_next_package::serialization::PackageSerializer;
    use std::path::PathBuf;

    PackageSerializer::load_from_file(&PathBuf::from(path))
        .map(PyPackage)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Save a package to file (auto-detects format from extension)
#[pyfunction]
pub fn save_package_to_file(package: &PyPackage, path: &str) -> PyResult<()> {
    use rez_next_package::serialization::{PackageFormat, PackageSerializer};
    use std::path::PathBuf;

    let path_buf = PathBuf::from(path);
    let format = PackageFormat::from_extension(&path_buf)
        .ok_or_else(|| {
            pyo3::exceptions::PyValueError::new_err(format!(
                "Cannot detect format from path: {}",
                path
            ))
        })?;

    PackageSerializer::save_to_file(&package.0, &path_buf, format)
        .map_err(|e| pyo3::exceptions::PyIOError::new_err(e.to_string()))
}

/// Python-accessible PackageRequirement class, compatible with rez.packages.PackageRequirement
#[pyclass(name = "PackageRequirement", from_py_object)]
#[derive(Clone)]
pub struct PyPackageRequirement(pub PackageRequirement);

#[pymethods]
impl PyPackageRequirement {
    /// Create a new PackageRequirement from a string like "python-3.9" or "maya>=2024"
    #[new]
    pub fn new(requirement_str: &str) -> PyResult<Self> {
        PackageRequirement::parse(requirement_str)
            .map(PyPackageRequirement)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(e.to_string()))
    }

    fn __str__(&self) -> String {
        self.0.to_string()
    }

    fn __repr__(&self) -> String {
        format!("PackageRequirement('{}')", self.__str__())
    }

    fn __eq__(&self, other: &PyPackageRequirement) -> bool {
        self.0.name == other.0.name
            && self.0.version_spec == other.0.version_spec
            && self.0.conflict == other.0.conflict
            && self.0.weak == other.0.weak
    }

    fn __hash__(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut h = DefaultHasher::new();
        self.0.name.hash(&mut h);
        if let Some(ref spec) = self.0.version_spec {
            spec.hash(&mut h);
        }
        self.0.conflict.hash(&mut h);
        self.0.weak.hash(&mut h);
        h.finish()
    }

    /// Package name
    #[getter]
    fn name(&self) -> String {
        self.0.name.clone()
    }

    /// Version specification string (rez compat: .range)
    #[getter]
    fn range(&self) -> Option<String> {
        self.0.version_spec.clone()
    }

    /// Version specification string (rez compat alias: .version_range)
    #[getter]
    fn version_range(&self) -> Option<String> {
        self.0.version_spec.clone()
    }

    /// Whether this is a conflict requirement (prefixed with `!`)
    #[getter]
    fn conflict(&self) -> bool {
        self.0.conflict
    }

    /// Whether this is a weak requirement (prefixed with `~`)
    #[getter]
    fn weak(&self) -> bool {
        self.0.weak
    }

    /// Check if a version satisfies this requirement
    fn satisfied_by(&self, version: &PyVersion) -> bool {
        self.0.satisfied_by(&version.0)
    }

    /// Convert to conflict requirement (negate range)
    fn conflict_requirement(&self) -> String {
        if self.0.conflict {
            // Already a conflict requirement, return as-is
            self.__str__()
        } else {
            format!("!{}", self.__str__())
        }
    }
}

#[cfg(test)]
#[path = "package_bindings_tests.rs"]
mod tests;
