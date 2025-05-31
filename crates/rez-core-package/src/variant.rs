//! Package variant implementation

use rez_core_common::RezCoreError;
use rez_core_version::Version;
#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Package variant representation
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct PackageVariant {
    /// Parent package name
    #[cfg_attr(feature = "python-bindings", pyo3(get))]
    pub name: String,

    /// Parent package version
    #[cfg_attr(feature = "python-bindings", pyo3(get))]
    pub version: Option<Version>,

    /// Variant index (None for packages without variants)
    #[cfg_attr(feature = "python-bindings", pyo3(get))]
    pub index: Option<usize>,
    
    /// Variant requirements (overrides parent package requirements)
    pub requires: Vec<String>,
    
    /// Variant build requirements
    pub build_requires: Vec<String>,
    
    /// Variant private build requirements
    pub private_build_requires: Vec<String>,
    
    /// Variant-specific commands
    pub commands: Option<String>,
    
    /// Variant root path
    pub root: Option<String>,
    
    /// Variant subpath
    pub subpath: Option<String>,
    
    /// Variant metadata
    pub metadata: HashMap<String, PyObject>,
}

impl Clone for PackageVariant {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            let cloned_metadata: HashMap<String, PyObject> = self.metadata
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect();

            Self {
                name: self.name.clone(),
                version: self.version.clone(),
                index: self.index,
                requires: self.requires.clone(),
                build_requires: self.build_requires.clone(),
                private_build_requires: self.private_build_requires.clone(),
                commands: self.commands.clone(),
                root: self.root.clone(),
                subpath: self.subpath.clone(),
                metadata: cloned_metadata,
            }
        })
    }
}

impl Serialize for PackageVariant {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PackageVariant", 9)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("index", &self.index)?;
        state.serialize_field("requires", &self.requires)?;
        state.serialize_field("build_requires", &self.build_requires)?;
        state.serialize_field("private_build_requires", &self.private_build_requires)?;
        state.serialize_field("commands", &self.commands)?;
        state.serialize_field("root", &self.root)?;
        state.serialize_field("subpath", &self.subpath)?;
        // Skip metadata field as PyObject cannot be serialized
        state.end()
    }
}

impl<'de> Deserialize<'de> for PackageVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Deserializer, Visitor, MapAccess};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name, Version, Index, Requires, BuildRequires,
            PrivateBuildRequires, Commands, Root, Subpath,
        }

        struct PackageVariantVisitor;

        impl<'de> Visitor<'de> for PackageVariantVisitor {
            type Value = PackageVariant;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PackageVariant")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PackageVariant, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut version = None;
                let mut index = None;
                let mut requires = None;
                let mut build_requires = None;
                let mut private_build_requires = None;
                let mut commands = None;
                let mut root = None;
                let mut subpath = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Version => {
                            if version.is_some() {
                                return Err(de::Error::duplicate_field("version"));
                            }
                            version = Some(map.next_value()?);
                        }
                        Field::Index => {
                            if index.is_some() {
                                return Err(de::Error::duplicate_field("index"));
                            }
                            index = Some(map.next_value()?);
                        }
                        Field::Requires => {
                            if requires.is_some() {
                                return Err(de::Error::duplicate_field("requires"));
                            }
                            requires = Some(map.next_value()?);
                        }
                        Field::BuildRequires => {
                            if build_requires.is_some() {
                                return Err(de::Error::duplicate_field("build_requires"));
                            }
                            build_requires = Some(map.next_value()?);
                        }
                        Field::PrivateBuildRequires => {
                            if private_build_requires.is_some() {
                                return Err(de::Error::duplicate_field("private_build_requires"));
                            }
                            private_build_requires = Some(map.next_value()?);
                        }
                        Field::Commands => {
                            if commands.is_some() {
                                return Err(de::Error::duplicate_field("commands"));
                            }
                            commands = Some(map.next_value()?);
                        }
                        Field::Root => {
                            if root.is_some() {
                                return Err(de::Error::duplicate_field("root"));
                            }
                            root = Some(map.next_value()?);
                        }
                        Field::Subpath => {
                            if subpath.is_some() {
                                return Err(de::Error::duplicate_field("subpath"));
                            }
                            subpath = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                Ok(PackageVariant {
                    name,
                    version: version.unwrap_or(None),
                    index: index.unwrap_or(None),
                    requires: requires.unwrap_or_default(),
                    build_requires: build_requires.unwrap_or_default(),
                    private_build_requires: private_build_requires.unwrap_or_default(),
                    commands: commands.unwrap_or(None),
                    root: root.unwrap_or(None),
                    subpath: subpath.unwrap_or(None),
                    metadata: HashMap::new(), // Cannot deserialize PyObject
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "name", "version", "index", "requires", "build_requires",
            "private_build_requires", "commands", "root", "subpath",
        ];
        deserializer.deserialize_struct("PackageVariant", FIELDS, PackageVariantVisitor)
    }
}

#[pymethods]
impl PackageVariant {
    #[new]
    pub fn new(name: String, index: Option<usize>) -> Self {
        Self {
            name,
            version: None,
            index,
            requires: Vec::new(),
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            commands: None,
            root: None,
            subpath: None,
            metadata: HashMap::new(),
        }
    }

    /// Get the qualified name of the variant
    #[getter]
    pub fn qualified_name(&self) -> String {
        let base_name = match &self.version {
            Some(version) => format!("{}-{}", self.name, version.as_str()),
            None => self.name.clone(),
        };
        
        match self.index {
            Some(idx) => format!("{}[{}]", base_name, idx),
            None => base_name,
        }
    }

    /// Get the qualified package name (without variant index)
    #[getter]
    pub fn qualified_package_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}-{}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Check if this is a package (always false for PackageVariant)
    #[getter]
    pub fn is_package(&self) -> bool {
        false
    }

    /// Check if this is a variant (always true for PackageVariant)
    #[getter]
    pub fn is_variant(&self) -> bool {
        true
    }

    /// Set the package version
    pub fn set_version(&mut self, version: Version) {
        self.version = Some(version);
    }

    /// Add a requirement
    pub fn add_requirement(&mut self, requirement: String) {
        self.requires.push(requirement);
    }

    /// Add a build requirement
    pub fn add_build_requirement(&mut self, requirement: String) {
        self.build_requires.push(requirement);
    }

    /// Add a private build requirement
    pub fn add_private_build_requirement(&mut self, requirement: String) {
        self.private_build_requires.push(requirement);
    }

    /// Set commands
    pub fn set_commands(&mut self, commands: String) {
        self.commands = Some(commands);
    }

    /// Set root path
    pub fn set_root(&mut self, root: String) {
        self.root = Some(root);
    }

    /// Set subpath
    pub fn set_subpath(&mut self, subpath: String) {
        self.subpath = Some(subpath);
    }

    /// Get string representation
    fn __str__(&self) -> String {
        self.qualified_name()
    }

    /// Get representation
    fn __repr__(&self) -> String {
        format!("PackageVariant('{}')", self.qualified_name())
    }
}

impl PackageVariant {
    /// Create a variant from a package and variant definition
    pub fn from_package_and_variant(
        package_name: String,
        package_version: Option<Version>,
        index: Option<usize>,
        variant_requirements: Vec<String>,
    ) -> Self {
        Self {
            name: package_name,
            version: package_version,
            index,
            requires: variant_requirements,
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            commands: None,
            root: None,
            subpath: None,
            metadata: HashMap::new(),
        }
    }

    /// Validate the variant definition
    pub fn validate(&self) -> Result<(), RezCoreError> {
        // Check required fields
        if self.name.is_empty() {
            return Err(RezCoreError::PackageParse("Variant name cannot be empty".to_string()));
        }

        // Validate name format (alphanumeric, underscore, hyphen)
        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(RezCoreError::PackageParse(
                format!("Invalid variant name '{}': only alphanumeric, underscore, and hyphen allowed", self.name)
            ));
        }

        // Validate version if present
        if let Some(ref version) = self.version {
            if version.as_str().is_empty() {
                return Err(RezCoreError::PackageParse("Variant version cannot be empty".to_string()));
            }
        }

        // Validate requirements format
        for req in &self.requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse("Variant requirement cannot be empty".to_string()));
            }
        }

        for req in &self.build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse("Variant build requirement cannot be empty".to_string()));
            }
        }

        for req in &self.private_build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse("Variant private build requirement cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Check if this variant matches the given requirements
    pub fn matches_requirements(&self, requirements: &[String]) -> bool {
        // Simple implementation: check if all requirements are satisfied
        // TODO: Implement proper requirement matching logic
        for req in requirements {
            if !self.requires.contains(req) {
                return false;
            }
        }
        true
    }

    /// Get the variant as a requirement string
    pub fn as_requirement(&self) -> String {
        match &self.version {
            Some(version) => format!("{}=={}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }
}
