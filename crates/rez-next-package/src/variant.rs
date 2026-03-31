//! Package variant implementation

use crate::requirement::Requirement;
use rez_next_common::RezCoreError;
use rez_next_version::Version;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fmt;

/// Package variant representation
#[derive(Debug)]
pub struct PackageVariant {
    /// Parent package name
    pub name: String,

    /// Parent package version
    pub version: Option<Version>,

    /// Variant index (None for packages without variants)
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
    pub metadata: HashMap<String, String>,
}

impl Clone for PackageVariant {
    fn clone(&self) -> Self {
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
            metadata: self.metadata.clone(),
        }
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
        // Note: metadata field is excluded from serialization for compatibility
        state.end()
    }
}

impl<'de> Deserialize<'de> for PackageVariant {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        use serde::de::{self, MapAccess, Visitor};
        use std::fmt;

        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "snake_case")]
        enum Field {
            Name,
            Version,
            Index,
            Requires,
            BuildRequires,
            PrivateBuildRequires,
            Commands,
            Root,
            Subpath,
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
                    metadata: HashMap::new(),
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "name",
            "version",
            "index",
            "requires",
            "build_requires",
            "private_build_requires",
            "commands",
            "root",
            "subpath",
        ];
        deserializer.deserialize_struct("PackageVariant", FIELDS, PackageVariantVisitor)
    }
}

impl PackageVariant {
    /// Create a new package variant
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
    pub fn qualified_package_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}-{}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Check if this is a package (always false for PackageVariant)
    pub fn is_package(&self) -> bool {
        false
    }

    /// Check if this is a variant (always true for PackageVariant)
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
            return Err(RezCoreError::PackageParse(
                "Variant name cannot be empty".to_string(),
            ));
        }

        // Validate name format (alphanumeric, underscore, hyphen)
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RezCoreError::PackageParse(format!(
                "Invalid variant name '{}': only alphanumeric, underscore, and hyphen allowed",
                self.name
            )));
        }

        // Validate version if present
        if let Some(ref version) = self.version {
            if version.as_str().is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Variant version cannot be empty".to_string(),
                ));
            }
        }

        // Validate requirements format
        for req in &self.requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Variant requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Variant build requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.private_build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Variant private build requirement cannot be empty".to_string(),
                ));
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

    /// Check if this variant is compatible with another variant
    pub fn is_compatible_with(&self, other: &PackageVariant) -> bool {
        // Same package name and version
        if self.name != other.name || self.version != other.version {
            return false;
        }

        // Check for conflicting requirements
        let self_reqs: HashSet<_> = self.requires.iter().collect();
        let other_reqs: HashSet<_> = other.requires.iter().collect();

        // If they have different requirements, they might be incompatible
        // This is a simplified check - in reality, we'd need to resolve requirements
        self_reqs == other_reqs
    }

    /// Get the variant's effective requirements (including inherited ones)
    pub fn effective_requirements(&self, parent_requirements: &[String]) -> Vec<String> {
        let mut effective = parent_requirements.to_vec();
        effective.extend(self.requires.clone());
        effective.sort();
        effective.dedup();
        effective
    }

    /// Check if this variant satisfies the given requirement
    pub fn satisfies_requirement(&self, requirement: &Requirement) -> bool {
        // Check package name
        if self.name != requirement.package_name() {
            return false;
        }

        // Check version constraint
        if let Some(ref version) = self.version {
            requirement.is_satisfied_by(version)
        } else {
            // No version means any version constraint is satisfied
            true
        }
    }

    /// Get variant hash for caching and comparison
    pub fn variant_hash(&self) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.name.hash(&mut hasher);
        self.version.hash(&mut hasher);
        self.index.hash(&mut hasher);
        self.requires.hash(&mut hasher);
        self.build_requires.hash(&mut hasher);
        self.private_build_requires.hash(&mut hasher);

        format!("{:x}", hasher.finish())
    }

    /// Create a variant from variant definition list
    pub fn from_variant_list(
        package_name: String,
        package_version: Option<Version>,
        variant_index: usize,
        variant_list: &[Vec<String>],
    ) -> Result<Self, RezCoreError> {
        if variant_index >= variant_list.len() {
            return Err(RezCoreError::PackageParse(format!(
                "Variant index {} out of bounds for {} variants",
                variant_index,
                variant_list.len()
            )));
        }

        let variant_requirements = variant_list[variant_index].clone();

        Ok(Self {
            name: package_name,
            version: package_version,
            index: Some(variant_index),
            requires: variant_requirements,
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            commands: None,
            root: None,
            subpath: None,
            metadata: HashMap::new(),
        })
    }
}

/// Variant manager for handling multiple variants
#[derive(Debug, Clone)]
pub struct VariantManager {
    /// All variants for a package
    variants: Vec<PackageVariant>,
    /// Variant index mapping
    index_map: HashMap<usize, usize>, // variant_index -> position in variants vec
}

impl VariantManager {
    /// Create a new variant manager
    pub fn new() -> Self {
        Self {
            variants: Vec::new(),
            index_map: HashMap::new(),
        }
    }

    /// Add a variant
    pub fn add_variant(&mut self, variant: PackageVariant) -> Result<(), RezCoreError> {
        // Validate the variant
        variant.validate()?;

        // Check for duplicate indices
        if let Some(index) = variant.index {
            if self.index_map.contains_key(&index) {
                return Err(RezCoreError::PackageParse(format!(
                    "Variant with index {} already exists",
                    index
                )));
            }
            self.index_map.insert(index, self.variants.len());
        }

        self.variants.push(variant);
        Ok(())
    }

    /// Get variant by index
    pub fn get_variant(&self, index: usize) -> Option<&PackageVariant> {
        self.index_map
            .get(&index)
            .and_then(|&pos| self.variants.get(pos))
    }

    /// Get all variants
    pub fn get_all_variants(&self) -> &[PackageVariant] {
        &self.variants
    }

    /// Filter variants by requirements
    pub fn filter_by_requirements(&self, requirements: &[String]) -> Vec<&PackageVariant> {
        self.variants
            .iter()
            .filter(|variant| variant.matches_requirements(requirements))
            .collect()
    }

    /// Get compatible variants for a given variant
    pub fn get_compatible_variants(&self, target: &PackageVariant) -> Vec<&PackageVariant> {
        self.variants
            .iter()
            .filter(|variant| variant.is_compatible_with(target))
            .collect()
    }

    /// Get the number of variants
    pub fn variant_count(&self) -> usize {
        self.variants.len()
    }

    /// Check if manager has any variants
    pub fn is_empty(&self) -> bool {
        self.variants.is_empty()
    }

    /// Remove variant by index
    pub fn remove_variant(&mut self, index: usize) -> Result<PackageVariant, RezCoreError> {
        if let Some(&pos) = self.index_map.get(&index) {
            self.index_map.remove(&index);

            // Update index map for remaining variants
            for (_, position) in self.index_map.iter_mut() {
                if *position > pos {
                    *position -= 1;
                }
            }

            Ok(self.variants.remove(pos))
        } else {
            Err(RezCoreError::PackageParse(format!(
                "Variant with index {} not found",
                index
            )))
        }
    }

    /// Clear all variants
    pub fn clear(&mut self) {
        self.variants.clear();
        self.index_map.clear();
    }

    /// Create variants from variant definition matrix
    pub fn from_variant_matrix(
        package_name: String,
        package_version: Option<Version>,
        variant_matrix: &[Vec<String>],
    ) -> Result<Self, RezCoreError> {
        let mut manager = Self::new();

        for (index, variant_def) in variant_matrix.iter().enumerate() {
            let variant = PackageVariant::from_variant_list(
                package_name.clone(),
                package_version.clone(),
                index,
                variant_matrix,
            )?;
            manager.add_variant(variant)?;
        }

        Ok(manager)
    }
}

impl Default for VariantManager {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for VariantManager {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "VariantManager({} variants)", self.variants.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_package_variant_creation() {
        let variant = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string(), "platform-linux".to_string()],
        );

        assert_eq!(variant.name, "test_package");
        assert_eq!(variant.index, Some(0));
        assert_eq!(variant.requires.len(), 2);
        assert!(variant.requires.contains(&"python-3.8".to_string()));
        assert!(variant.requires.contains(&"platform-linux".to_string()));
    }

    #[test]
    fn test_variant_validation() {
        let mut variant = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        // Valid variant should pass validation
        assert!(variant.validate().is_ok());

        // Empty name should fail validation
        variant.name = String::new();
        assert!(variant.validate().is_err());

        // Invalid name characters should fail validation
        variant.name = "test@package".to_string();
        assert!(variant.validate().is_err());

        // Valid name should pass
        variant.name = "test_package-v2".to_string();
        assert!(variant.validate().is_ok());
    }

    #[test]
    fn test_variant_qualified_name() {
        let variant = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        // Test basic properties
        assert_eq!(variant.name, "test_package");
        assert_eq!(variant.index, Some(0));

        assert_eq!(variant.qualified_name(), "test_package-1.0.0[0]");
        assert_eq!(variant.qualified_package_name(), "test_package-1.0.0");
    }

    #[test]
    fn test_variant_compatibility() {
        let variant1 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        let variant2 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(1),
            vec!["python-3.8".to_string()],
        );

        let variant3 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(2),
            vec!["python-3.9".to_string()],
        );

        // Same requirements should be compatible
        assert!(variant1.is_compatible_with(&variant2));

        // Different requirements should not be compatible
        assert!(!variant1.is_compatible_with(&variant3));
    }

    #[test]
    fn test_variant_manager() {
        let mut manager = VariantManager::new();
        assert!(manager.is_empty());
        assert_eq!(manager.variant_count(), 0);

        let variant1 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        let variant2 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(1),
            vec!["python-3.9".to_string()],
        );

        // Add variants
        assert!(manager.add_variant(variant1).is_ok());
        assert!(manager.add_variant(variant2).is_ok());

        assert!(!manager.is_empty());
        assert_eq!(manager.variant_count(), 2);

        // Get variant by index
        let retrieved = manager.get_variant(0);
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().index, Some(0));

        // Filter by requirements
        let filtered = manager.filter_by_requirements(&["python-3.8".to_string()]);
        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].index, Some(0));
    }

    #[test]
    fn test_variant_from_matrix() {
        let variant_matrix = vec![
            vec!["python-3.8".to_string(), "platform-linux".to_string()],
            vec!["python-3.9".to_string(), "platform-linux".to_string()],
            vec!["python-3.8".to_string(), "platform-windows".to_string()],
        ];

        let manager = VariantManager::from_variant_matrix(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            &variant_matrix,
        )
        .unwrap();

        assert_eq!(manager.variant_count(), 3);

        let variant0 = manager.get_variant(0).unwrap();
        assert_eq!(variant0.requires.len(), 2);
        assert!(variant0.requires.contains(&"python-3.8".to_string()));
        assert!(variant0.requires.contains(&"platform-linux".to_string()));

        let variant2 = manager.get_variant(2).unwrap();
        assert!(variant2.requires.contains(&"python-3.8".to_string()));
        assert!(variant2.requires.contains(&"platform-windows".to_string()));
    }

    #[test]
    fn test_variant_hash() {
        let variant1 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        let variant2 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        let variant3 = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.9".to_string()],
        );

        // Same variants should have same hash
        assert_eq!(variant1.variant_hash(), variant2.variant_hash());

        // Different variants should have different hashes
        assert_ne!(variant1.variant_hash(), variant3.variant_hash());
    }

    #[test]
    fn test_effective_requirements() {
        let variant = PackageVariant::from_package_and_variant(
            "test_package".to_string(),
            Some(Version::parse("1.0.0").unwrap()),
            Some(0),
            vec!["python-3.8".to_string()],
        );

        let parent_requirements = vec!["cmake".to_string(), "gcc".to_string()];
        let effective = variant.effective_requirements(&parent_requirements);

        assert_eq!(effective.len(), 3);
        assert!(effective.contains(&"cmake".to_string()));
        assert!(effective.contains(&"gcc".to_string()));
        assert!(effective.contains(&"python-3.8".to_string()));
    }
}
