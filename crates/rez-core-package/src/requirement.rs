//! Package requirement implementation

use rez_core_common::RezCoreError;
use rez_core_version::{Version, VersionRange};
use pyo3::prelude::*;
use serde::{Deserialize, Serialize};
use regex::Regex;

/// Package requirement representation
#[pyclass]
#[derive(Debug, Clone)]
pub struct PackageRequirement {
    /// Package name
    #[pyo3(get)]
    pub name: String,
    
    /// Version range requirement
    #[pyo3(get)]
    pub range: Option<VersionRange>,
    
    /// Original requirement string
    #[pyo3(get)]
    pub requirement_string: String,
    
    /// Whether this is a weak requirement
    #[pyo3(get)]
    pub weak: bool,
    
    /// Conflict flag (for conflict requirements)
    #[pyo3(get)]
    pub conflict: bool,
}

impl Serialize for PackageRequirement {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("PackageRequirement", 5)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("range", &self.range)?;
        state.serialize_field("requirement_string", &self.requirement_string)?;
        state.serialize_field("weak", &self.weak)?;
        state.serialize_field("conflict", &self.conflict)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for PackageRequirement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, Deserializer, Visitor, MapAccess};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name, Range, RequirementString, Weak, Conflict,
        }

        struct PackageRequirementVisitor;

        impl<'de> Visitor<'de> for PackageRequirementVisitor {
            type Value = PackageRequirement;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct PackageRequirement")
            }

            fn visit_map<V>(self, mut map: V) -> Result<PackageRequirement, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut range = None;
                let mut requirement_string = None;
                let mut weak = None;
                let mut conflict = None;

                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Name => {
                            if name.is_some() {
                                return Err(de::Error::duplicate_field("name"));
                            }
                            name = Some(map.next_value()?);
                        }
                        Field::Range => {
                            if range.is_some() {
                                return Err(de::Error::duplicate_field("range"));
                            }
                            range = Some(map.next_value()?);
                        }
                        Field::RequirementString => {
                            if requirement_string.is_some() {
                                return Err(de::Error::duplicate_field("requirement_string"));
                            }
                            requirement_string = Some(map.next_value()?);
                        }
                        Field::Weak => {
                            if weak.is_some() {
                                return Err(de::Error::duplicate_field("weak"));
                            }
                            weak = Some(map.next_value()?);
                        }
                        Field::Conflict => {
                            if conflict.is_some() {
                                return Err(de::Error::duplicate_field("conflict"));
                            }
                            conflict = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                let requirement_string = requirement_string.ok_or_else(|| de::Error::missing_field("requirement_string"))?;
                Ok(PackageRequirement {
                    name,
                    range: range.unwrap_or(None),
                    requirement_string,
                    weak: weak.unwrap_or(false),
                    conflict: conflict.unwrap_or(false),
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "name", "range", "requirement_string", "weak", "conflict",
        ];
        deserializer.deserialize_struct("PackageRequirement", FIELDS, PackageRequirementVisitor)
    }
}

#[pymethods]
impl PackageRequirement {
    #[new]
    pub fn new(requirement_string: String) -> PyResult<Self> {
        Self::parse(&requirement_string)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyValueError, _>(e.to_string()))
    }

    /// Check if a version satisfies this requirement
    pub fn satisfied_by(&self, version: &Version) -> bool {
        match &self.range {
            Some(range) => range.contains_version(version),
            None => true, // No version constraint means any version is acceptable
        }
    }

    /// Check if this requirement conflicts with another
    pub fn conflicts_with(&self, other: &PackageRequirement) -> bool {
        // Same package name but incompatible version ranges
        if self.name == other.name {
            if let (Some(self_range), Some(other_range)) = (&self.range, &other.range) {
                return !self_range.intersects(other_range);
            }
        }
        false
    }

    /// Get string representation
    fn __str__(&self) -> String {
        self.requirement_string.clone()
    }

    /// Get representation
    fn __repr__(&self) -> String {
        format!("PackageRequirement('{}')", self.requirement_string)
    }
}

impl PackageRequirement {
    /// Parse a requirement string into a PackageRequirement
    pub fn parse(requirement_string: &str) -> Result<Self, RezCoreError> {
        let requirement_string = requirement_string.trim();
        
        if requirement_string.is_empty() {
            return Err(RezCoreError::RequirementParse("Empty requirement string".to_string()));
        }

        // Check for weak requirement prefix (~)
        let (weak, requirement_string) = if requirement_string.starts_with('~') {
            (true, &requirement_string[1..])
        } else {
            (false, requirement_string)
        };

        // Check for conflict prefix (!)
        let (conflict, requirement_string) = if requirement_string.starts_with('!') {
            (true, &requirement_string[1..])
        } else {
            (false, requirement_string)
        };

        // Parse package name and version range
        let (name, range) = Self::parse_name_and_range(requirement_string)?;

        Ok(Self {
            name,
            range,
            requirement_string: requirement_string.to_string(),
            weak,
            conflict,
        })
    }

    /// Parse package name and version range from requirement string
    fn parse_name_and_range(requirement_string: &str) -> Result<(String, Option<VersionRange>), RezCoreError> {
        // Regex to match package name and optional version range
        let re = Regex::new(r"^([a-zA-Z_][a-zA-Z0-9_-]*)(.*)?$").unwrap();
        
        if let Some(captures) = re.captures(requirement_string) {
            let name = captures.get(1).unwrap().as_str().to_string();
            let version_part = captures.get(2).map(|m| m.as_str().trim()).unwrap_or("");
            
            let range = if version_part.is_empty() {
                None
            } else {
                Some(VersionRange::parse(version_part)
                    .map_err(|e| RezCoreError::RequirementParse(
                        format!("Invalid version range '{}': {}", version_part, e)
                    ))?)
            };
            
            Ok((name, range))
        } else {
            Err(RezCoreError::RequirementParse(
                format!("Invalid requirement format: '{}'", requirement_string)
            ))
        }
    }

    /// Create a requirement from package name and version range
    pub fn from_name_and_range(name: String, range: Option<VersionRange>) -> Self {
        let requirement_string = match &range {
            Some(r) => format!("{}{}", name, r.as_str()),
            None => name.clone(),
        };

        Self {
            name,
            range,
            requirement_string,
            weak: false,
            conflict: false,
        }
    }

    /// Create an exact requirement for a specific version
    pub fn exact(name: String, version: Version) -> Self {
        let requirement_string = format!("{}=={}", name, version.as_str());
        let range = VersionRange::parse(&format!("=={}", version.as_str())).ok();

        Self {
            name,
            range,
            requirement_string,
            weak: false,
            conflict: false,
        }
    }

    /// Create a weak requirement
    pub fn weak(name: String, range: Option<VersionRange>) -> Self {
        let mut req = Self::from_name_and_range(name, range);
        req.weak = true;
        req.requirement_string = format!("~{}", req.requirement_string);
        req
    }

    /// Create a conflict requirement
    pub fn conflict(name: String, range: Option<VersionRange>) -> Self {
        let mut req = Self::from_name_and_range(name, range);
        req.conflict = true;
        req.requirement_string = format!("!{}", req.requirement_string);
        req
    }

    /// Validate the requirement
    pub fn validate(&self) -> Result<(), RezCoreError> {
        // Check package name
        if self.name.is_empty() {
            return Err(RezCoreError::RequirementParse("Package name cannot be empty".to_string()));
        }

        // Validate name format (must start with letter or underscore, then alphanumeric, underscore, hyphen)
        if !self.name.chars().next().unwrap().is_alphabetic() && !self.name.starts_with('_') {
            return Err(RezCoreError::RequirementParse(
                format!("Invalid package name '{}': must start with letter or underscore", self.name)
            ));
        }

        if !self.name.chars().all(|c| c.is_alphanumeric() || c == '_' || c == '-') {
            return Err(RezCoreError::RequirementParse(
                format!("Invalid package name '{}': only alphanumeric, underscore, and hyphen allowed", self.name)
            ));
        }

        // Validate version range if present
        if let Some(ref range) = self.range {
            // Version range validation is handled by the VersionRange type itself
            if range.as_str().is_empty() {
                return Err(RezCoreError::RequirementParse("Version range cannot be empty".to_string()));
            }
        }

        Ok(())
    }

    /// Check if this requirement is compatible with another requirement
    pub fn is_compatible_with(&self, other: &PackageRequirement) -> bool {
        // Different packages are always compatible
        if self.name != other.name {
            return true;
        }

        // Same package: check if version ranges intersect
        match (&self.range, &other.range) {
            (Some(self_range), Some(other_range)) => self_range.intersects(other_range),
            _ => true, // If either has no version constraint, they're compatible
        }
    }
}
