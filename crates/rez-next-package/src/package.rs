//! Package implementation

#[cfg(feature = "python-bindings")]
use pyo3::prelude::*;
use rez_next_common::RezCoreError;
use rez_next_version::Version;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Simple package requirement for basic functionality
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackageRequirement {
    /// Package name
    pub name: String,
    /// Version requirement (optional)
    pub version_spec: Option<String>,
    /// Whether this is a weak requirement
    pub weak: bool,
}

impl PackageRequirement {
    /// Create a new package requirement
    pub fn new(name: String) -> Self {
        Self {
            name,
            version_spec: None,
            weak: false,
        }
    }

    /// Create a package requirement with version specification
    pub fn with_version(name: String, version_spec: String) -> Self {
        Self {
            name,
            version_spec: Some(version_spec),
            weak: false,
        }
    }

    /// Parse a requirement string like "python-3.9" or "maya>=2023"
    pub fn parse(requirement_str: &str) -> Result<Self, RezCoreError> {
        // Simple parsing - can be enhanced later
        if let Some(dash_pos) = requirement_str.rfind('-') {
            let name = requirement_str[..dash_pos].to_string();
            let version = requirement_str[dash_pos + 1..].to_string();
            Ok(Self::with_version(name, version))
        } else {
            Ok(Self::new(requirement_str.to_string()))
        }
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version specification
    pub fn version_spec(&self) -> Option<&str> {
        self.version_spec.as_deref()
    }

    /// Convert to string representation
    pub fn to_string(&self) -> String {
        if let Some(ref version) = self.version_spec {
            format!("{}-{}", self.name, version)
        } else {
            self.name.clone()
        }
    }

    /// Get requirement string (for compatibility)
    pub fn requirement_string(&self) -> String {
        self.to_string()
    }

    /// Check if this requirement is satisfied by a version (simplified)
    pub fn satisfied_by(&self, version: &Version) -> bool {
        // Simplified implementation - can be enhanced later
        if let Some(ref version_spec) = self.version_spec {
            // For now, just check if the version string matches
            version.as_str() == version_spec
        } else {
            // No version constraint, always satisfied
            true
        }
    }
}

/// High-performance package representation compatible with rez
#[cfg_attr(feature = "python-bindings", pyclass)]
#[derive(Debug)]
pub struct Package {
    /// Package name
    #[cfg(feature = "python-bindings")]
    #[pyo3(get)]
    pub name: String,
    /// Package name (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub name: String,

    /// Package version
    #[cfg(feature = "python-bindings")]
    #[pyo3(get)]
    pub version: Option<Version>,
    /// Package version (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub version: Option<Version>,

    /// Package description
    #[cfg(feature = "python-bindings")]
    #[pyo3(get)]
    pub description: Option<String>,
    /// Package description (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub description: Option<String>,

    /// Package authors
    #[cfg(feature = "python-bindings")]
    #[pyo3(get)]
    pub authors: Vec<String>,
    /// Package authors (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub authors: Vec<String>,

    /// Package requirements
    pub requires: Vec<String>,

    /// Build requirements
    pub build_requires: Vec<String>,

    /// Private build requirements
    pub private_build_requires: Vec<String>,

    /// Package variants
    pub variants: Vec<Vec<String>>,

    /// Package tools
    pub tools: Vec<String>,

    /// Package commands
    pub commands: Option<String>,

    /// Build command for custom builds
    pub build_command: Option<String>,

    /// Build system type
    pub build_system: Option<String>,

    /// Pre commands (executed before main commands)
    pub pre_commands: Option<String>,

    /// Post commands (executed after main commands)
    pub post_commands: Option<String>,

    /// Pre test commands (executed before tests)
    pub pre_test_commands: Option<String>,

    /// Pre build commands (executed before build)
    pub pre_build_commands: Option<String>,

    /// Package tests
    pub tests: HashMap<String, String>,

    /// Required rez version
    pub requires_rez_version: Option<String>,

    /// Package UUID
    #[cfg(feature = "python-bindings")]
    #[pyo3(get)]
    pub uuid: Option<String>,
    /// Package UUID (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub uuid: Option<String>,

    /// Package config
    #[cfg(feature = "python-bindings")]
    pub config: HashMap<String, PyObject>,

    /// Package config (non-Python version)
    #[cfg(not(feature = "python-bindings"))]
    pub config: HashMap<String, String>,

    /// Package help
    pub help: Option<String>,

    /// Package relocatable flag
    pub relocatable: Option<bool>,

    /// Package cachable flag
    pub cachable: Option<bool>,

    /// Package timestamp
    pub timestamp: Option<i64>,

    /// Package revision
    pub revision: Option<String>,

    /// Package changelog
    pub changelog: Option<String>,

    /// Package release message
    pub release_message: Option<String>,

    /// Previous version
    pub previous_version: Option<Version>,

    /// Previous revision
    pub previous_revision: Option<String>,

    /// VCS type
    pub vcs: Option<String>,

    /// Package format version
    pub format_version: Option<i32>,

    /// Package base
    pub base: Option<String>,

    /// Package has plugins
    pub has_plugins: Option<bool>,

    /// Plugin for packages
    pub plugin_for: Vec<String>,

    /// Package hashed variants
    pub hashed_variants: Option<bool>,

    /// Package preprocess function
    pub preprocess: Option<String>,
}

#[cfg(feature = "python-bindings")]
impl Clone for Package {
    fn clone(&self) -> Self {
        Python::with_gil(|py| {
            let cloned_config: HashMap<String, PyObject> = self
                .config
                .iter()
                .map(|(k, v)| (k.clone(), v.clone_ref(py)))
                .collect();

            Self {
                name: self.name.clone(),
                version: self.version.clone(),
                description: self.description.clone(),
                authors: self.authors.clone(),
                requires: self.requires.clone(),
                build_requires: self.build_requires.clone(),
                private_build_requires: self.private_build_requires.clone(),
                variants: self.variants.clone(),
                tools: self.tools.clone(),
                commands: self.commands.clone(),
                build_command: self.build_command.clone(),
                build_system: self.build_system.clone(),
                pre_commands: self.pre_commands.clone(),
                post_commands: self.post_commands.clone(),
                pre_test_commands: self.pre_test_commands.clone(),
                pre_build_commands: self.pre_build_commands.clone(),
                tests: self.tests.clone(),
                requires_rez_version: self.requires_rez_version.clone(),
                uuid: self.uuid.clone(),
                config: cloned_config,
                help: self.help.clone(),
                relocatable: self.relocatable,
                cachable: self.cachable,
                timestamp: self.timestamp,
                revision: self.revision.clone(),
                changelog: self.changelog.clone(),
                release_message: self.release_message.clone(),
                previous_version: self.previous_version.clone(),
                previous_revision: self.previous_revision.clone(),
                vcs: self.vcs.clone(),
                format_version: self.format_version,
                base: self.base.clone(),
                has_plugins: self.has_plugins,
                plugin_for: self.plugin_for.clone(),
                hashed_variants: self.hashed_variants,
                preprocess: self.preprocess.clone(),
            }
        })
    }
}

#[cfg(not(feature = "python-bindings"))]
impl Clone for Package {
    fn clone(&self) -> Self {
        Self {
            name: self.name.clone(),
            version: self.version.clone(),
            description: self.description.clone(),
            authors: self.authors.clone(),
            requires: self.requires.clone(),
            build_requires: self.build_requires.clone(),
            private_build_requires: self.private_build_requires.clone(),
            variants: self.variants.clone(),
            tools: self.tools.clone(),
            commands: self.commands.clone(),
            build_command: self.build_command.clone(),
            build_system: self.build_system.clone(),
            pre_commands: self.pre_commands.clone(),
            post_commands: self.post_commands.clone(),
            pre_test_commands: self.pre_test_commands.clone(),
            pre_build_commands: self.pre_build_commands.clone(),
            tests: self.tests.clone(),
            requires_rez_version: self.requires_rez_version.clone(),
            uuid: self.uuid.clone(),
            config: self.config.clone(),
            help: self.help.clone(),
            relocatable: self.relocatable,
            cachable: self.cachable,
            timestamp: self.timestamp,
            revision: self.revision.clone(),
            changelog: self.changelog.clone(),
            release_message: self.release_message.clone(),
            previous_version: self.previous_version.clone(),
            previous_revision: self.previous_revision.clone(),
            vcs: self.vcs.clone(),
            format_version: self.format_version,
            base: self.base.clone(),
            has_plugins: self.has_plugins,
            plugin_for: self.plugin_for.clone(),
            hashed_variants: self.hashed_variants,
            preprocess: self.preprocess.clone(),
        }
    }
}

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Package", 24)?;
        state.serialize_field("name", &self.name)?;
        state.serialize_field("version", &self.version)?;
        state.serialize_field("description", &self.description)?;
        state.serialize_field("authors", &self.authors)?;
        state.serialize_field("requires", &self.requires)?;
        state.serialize_field("build_requires", &self.build_requires)?;
        state.serialize_field("private_build_requires", &self.private_build_requires)?;
        state.serialize_field("variants", &self.variants)?;
        state.serialize_field("tools", &self.tools)?;
        state.serialize_field("commands", &self.commands)?;
        state.serialize_field("build_command", &self.build_command)?;
        state.serialize_field("build_system", &self.build_system)?;
        state.serialize_field("pre_commands", &self.pre_commands)?;
        state.serialize_field("post_commands", &self.post_commands)?;
        state.serialize_field("pre_test_commands", &self.pre_test_commands)?;
        state.serialize_field("pre_build_commands", &self.pre_build_commands)?;
        state.serialize_field("tests", &self.tests)?;
        state.serialize_field("requires_rez_version", &self.requires_rez_version)?;
        state.serialize_field("uuid", &self.uuid)?;
        // Skip config field as PyObject cannot be serialized
        state.serialize_field("help", &self.help)?;
        state.serialize_field("relocatable", &self.relocatable)?;
        state.serialize_field("cachable", &self.cachable)?;
        state.serialize_field("timestamp", &self.timestamp)?;
        state.serialize_field("revision", &self.revision)?;
        state.serialize_field("changelog", &self.changelog)?;
        state.serialize_field("release_message", &self.release_message)?;
        state.serialize_field("previous_version", &self.previous_version)?;
        state.serialize_field("previous_revision", &self.previous_revision)?;
        state.serialize_field("vcs", &self.vcs)?;
        state.serialize_field("format_version", &self.format_version)?;
        state.serialize_field("base", &self.base)?;
        state.serialize_field("has_plugins", &self.has_plugins)?;
        state.serialize_field("plugin_for", &self.plugin_for)?;
        state.serialize_field("hashed_variants", &self.hashed_variants)?;
        state.serialize_field("preprocess", &self.preprocess)?;
        state.end()
    }
}

impl<'de> Deserialize<'de> for Package {
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
            Description,
            Authors,
            Requires,
            BuildRequires,
            PrivateBuildRequires,
            Variants,
            Tools,
            Commands,
            BuildCommand,
            BuildSystem,
            PreCommands,
            PostCommands,
            PreTestCommands,
            PreBuildCommands,
            Tests,
            RequiresRezVersion,
            Uuid,
            Help,
            Relocatable,
            Cachable,
            Timestamp,
            Revision,
            Changelog,
            ReleaseMessage,
            PreviousVersion,
            PreviousRevision,
            Vcs,
            FormatVersion,
            Base,
            HasPlugins,
            PluginFor,
            HashedVariants,
            Preprocess,
        }

        struct PackageVisitor;

        impl<'de> Visitor<'de> for PackageVisitor {
            type Value = Package;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Package")
            }

            fn visit_map<V>(self, mut map: V) -> Result<Package, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut name = None;
                let mut version = None;
                let mut description = None;
                let mut authors = None;
                let mut requires = None;
                let mut build_requires = None;
                let mut private_build_requires = None;
                let mut variants = None;
                let mut tools = None;
                let mut commands = None;
                let mut build_command = None;
                let mut build_system = None;
                let mut pre_commands = None;
                let mut post_commands = None;
                let mut pre_test_commands = None;
                let mut pre_build_commands = None;
                let mut tests = None;
                let mut requires_rez_version = None;
                let mut uuid = None;
                let mut help = None;
                let mut relocatable = None;
                let mut cachable = None;
                let mut timestamp = None;
                let mut revision = None;
                let mut changelog = None;
                let mut release_message = None;
                let mut previous_version = None;
                let mut previous_revision = None;
                let mut vcs = None;
                let mut format_version = None;
                let mut base = None;
                let mut has_plugins = None;
                let mut plugin_for = None;
                let mut hashed_variants = None;
                let mut preprocess = None;

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
                        Field::Description => {
                            if description.is_some() {
                                return Err(de::Error::duplicate_field("description"));
                            }
                            description = Some(map.next_value()?);
                        }
                        Field::Authors => {
                            if authors.is_some() {
                                return Err(de::Error::duplicate_field("authors"));
                            }
                            authors = Some(map.next_value()?);
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
                        Field::Variants => {
                            if variants.is_some() {
                                return Err(de::Error::duplicate_field("variants"));
                            }
                            variants = Some(map.next_value()?);
                        }
                        Field::Tools => {
                            if tools.is_some() {
                                return Err(de::Error::duplicate_field("tools"));
                            }
                            tools = Some(map.next_value()?);
                        }
                        Field::Commands => {
                            if commands.is_some() {
                                return Err(de::Error::duplicate_field("commands"));
                            }
                            commands = Some(map.next_value()?);
                        }
                        Field::BuildCommand => {
                            if build_command.is_some() {
                                return Err(de::Error::duplicate_field("build_command"));
                            }
                            build_command = Some(map.next_value()?);
                        }
                        Field::BuildSystem => {
                            if build_system.is_some() {
                                return Err(de::Error::duplicate_field("build_system"));
                            }
                            build_system = Some(map.next_value()?);
                        }
                        Field::PreCommands => {
                            if pre_commands.is_some() {
                                return Err(de::Error::duplicate_field("pre_commands"));
                            }
                            pre_commands = Some(map.next_value()?);
                        }
                        Field::PostCommands => {
                            if post_commands.is_some() {
                                return Err(de::Error::duplicate_field("post_commands"));
                            }
                            post_commands = Some(map.next_value()?);
                        }
                        Field::PreTestCommands => {
                            if pre_test_commands.is_some() {
                                return Err(de::Error::duplicate_field("pre_test_commands"));
                            }
                            pre_test_commands = Some(map.next_value()?);
                        }
                        Field::PreBuildCommands => {
                            if pre_build_commands.is_some() {
                                return Err(de::Error::duplicate_field("pre_build_commands"));
                            }
                            pre_build_commands = Some(map.next_value()?);
                        }
                        Field::Tests => {
                            if tests.is_some() {
                                return Err(de::Error::duplicate_field("tests"));
                            }
                            tests = Some(map.next_value()?);
                        }
                        Field::RequiresRezVersion => {
                            if requires_rez_version.is_some() {
                                return Err(de::Error::duplicate_field("requires_rez_version"));
                            }
                            requires_rez_version = Some(map.next_value()?);
                        }
                        Field::Uuid => {
                            if uuid.is_some() {
                                return Err(de::Error::duplicate_field("uuid"));
                            }
                            uuid = Some(map.next_value()?);
                        }
                        Field::Help => {
                            if help.is_some() {
                                return Err(de::Error::duplicate_field("help"));
                            }
                            help = Some(map.next_value()?);
                        }
                        Field::Relocatable => {
                            if relocatable.is_some() {
                                return Err(de::Error::duplicate_field("relocatable"));
                            }
                            relocatable = Some(map.next_value()?);
                        }
                        Field::Cachable => {
                            if cachable.is_some() {
                                return Err(de::Error::duplicate_field("cachable"));
                            }
                            cachable = Some(map.next_value()?);
                        }
                        Field::Timestamp => {
                            if timestamp.is_some() {
                                return Err(de::Error::duplicate_field("timestamp"));
                            }
                            timestamp = Some(map.next_value()?);
                        }
                        Field::Revision => {
                            if revision.is_some() {
                                return Err(de::Error::duplicate_field("revision"));
                            }
                            revision = Some(map.next_value()?);
                        }
                        Field::Changelog => {
                            if changelog.is_some() {
                                return Err(de::Error::duplicate_field("changelog"));
                            }
                            changelog = Some(map.next_value()?);
                        }
                        Field::ReleaseMessage => {
                            if release_message.is_some() {
                                return Err(de::Error::duplicate_field("release_message"));
                            }
                            release_message = Some(map.next_value()?);
                        }
                        Field::PreviousVersion => {
                            if previous_version.is_some() {
                                return Err(de::Error::duplicate_field("previous_version"));
                            }
                            previous_version = Some(map.next_value()?);
                        }
                        Field::PreviousRevision => {
                            if previous_revision.is_some() {
                                return Err(de::Error::duplicate_field("previous_revision"));
                            }
                            previous_revision = Some(map.next_value()?);
                        }
                        Field::Vcs => {
                            if vcs.is_some() {
                                return Err(de::Error::duplicate_field("vcs"));
                            }
                            vcs = Some(map.next_value()?);
                        }
                        Field::FormatVersion => {
                            if format_version.is_some() {
                                return Err(de::Error::duplicate_field("format_version"));
                            }
                            format_version = Some(map.next_value()?);
                        }
                        Field::Base => {
                            if base.is_some() {
                                return Err(de::Error::duplicate_field("base"));
                            }
                            base = Some(map.next_value()?);
                        }
                        Field::HasPlugins => {
                            if has_plugins.is_some() {
                                return Err(de::Error::duplicate_field("has_plugins"));
                            }
                            has_plugins = Some(map.next_value()?);
                        }
                        Field::PluginFor => {
                            if plugin_for.is_some() {
                                return Err(de::Error::duplicate_field("plugin_for"));
                            }
                            plugin_for = Some(map.next_value()?);
                        }
                        Field::HashedVariants => {
                            if hashed_variants.is_some() {
                                return Err(de::Error::duplicate_field("hashed_variants"));
                            }
                            hashed_variants = Some(map.next_value()?);
                        }
                        Field::Preprocess => {
                            if preprocess.is_some() {
                                return Err(de::Error::duplicate_field("preprocess"));
                            }
                            preprocess = Some(map.next_value()?);
                        }
                    }
                }

                let name = name.ok_or_else(|| de::Error::missing_field("name"))?;
                Ok(Package {
                    name,
                    version: version.unwrap_or(None),
                    description: description.unwrap_or(None),
                    authors: authors.unwrap_or_default(),
                    requires: requires.unwrap_or_default(),
                    build_requires: build_requires.unwrap_or_default(),
                    private_build_requires: private_build_requires.unwrap_or_default(),
                    variants: variants.unwrap_or_default(),
                    tools: tools.unwrap_or_default(),
                    commands: commands.unwrap_or(None),
                    build_command: build_command.unwrap_or(None),
                    build_system: build_system.unwrap_or(None),
                    pre_commands: pre_commands.unwrap_or(None),
                    post_commands: post_commands.unwrap_or(None),
                    pre_test_commands: pre_test_commands.unwrap_or(None),
                    pre_build_commands: pre_build_commands.unwrap_or(None),
                    tests: tests.unwrap_or_default(),
                    requires_rez_version: requires_rez_version.unwrap_or(None),
                    uuid: uuid.unwrap_or(None),
                    config: HashMap::new(), // Cannot deserialize PyObject
                    help: help.unwrap_or(None),
                    relocatable: relocatable.unwrap_or(None),
                    cachable: cachable.unwrap_or(None),
                    timestamp: timestamp.unwrap_or(None),
                    revision: revision.unwrap_or(None),
                    changelog: changelog.unwrap_or(None),
                    release_message: release_message.unwrap_or(None),
                    previous_version: previous_version.unwrap_or(None),
                    previous_revision: previous_revision.unwrap_or(None),
                    vcs: vcs.unwrap_or(None),
                    format_version: format_version.unwrap_or(None),
                    base: base.unwrap_or(None),
                    has_plugins: has_plugins.unwrap_or(None),
                    plugin_for: plugin_for.unwrap_or_default(),
                    hashed_variants: hashed_variants.unwrap_or(None),
                    preprocess: preprocess.unwrap_or(None),
                })
            }
        }

        const FIELDS: &'static [&'static str] = &[
            "name",
            "version",
            "description",
            "authors",
            "requires",
            "build_requires",
            "private_build_requires",
            "variants",
            "tools",
            "commands",
            "build_command",
            "build_system",
            "pre_commands",
            "post_commands",
            "pre_test_commands",
            "pre_build_commands",
            "tests",
            "requires_rez_version",
            "uuid",
            "help",
            "relocatable",
            "cachable",
            "timestamp",
            "revision",
            "changelog",
            "release_message",
            "previous_version",
            "previous_revision",
            "vcs",
            "format_version",
            "base",
            "has_plugins",
            "plugin_for",
            "hashed_variants",
            "preprocess",
        ];
        deserializer.deserialize_struct("Package", FIELDS, PackageVisitor)
    }
}

#[cfg(feature = "python-bindings")]
#[pymethods]
impl Package {
    #[new]
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            description: None,
            authors: Vec::new(),
            requires: Vec::new(),
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            variants: Vec::new(),
            tools: Vec::new(),
            commands: None,
            build_command: None,
            build_system: None,
            pre_commands: None,
            post_commands: None,
            pre_test_commands: None,
            pre_build_commands: None,
            tests: HashMap::new(),
            requires_rez_version: None,
            uuid: None,
            config: HashMap::new(),
            help: None,
            relocatable: None,
            cachable: None,
            timestamp: None,
            revision: None,
            changelog: None,
            release_message: None,
            previous_version: None,
            previous_revision: None,
            vcs: None,
            format_version: None,
            base: None,
            has_plugins: None,
            plugin_for: Vec::new(),
            hashed_variants: None,
            preprocess: None,
        }
    }

    /// Get the qualified name of the package (name-version)
    #[getter]
    pub fn qualified_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}-{}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Get the package as an exact requirement string
    pub fn as_exact_requirement(&self) -> String {
        match &self.version {
            Some(version) => format!("{}=={}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Check if this is a package (always true for Package)
    #[getter]
    pub fn is_package(&self) -> bool {
        true
    }

    /// Check if this is a variant (always false for Package)
    #[getter]
    pub fn is_variant(&self) -> bool {
        false
    }

    /// Get the number of variants
    #[getter]
    pub fn num_variants(&self) -> usize {
        self.variants.len()
    }

    /// Set the package version
    pub fn set_version(&mut self, version: Version) {
        self.version = Some(version);
    }

    /// Set the package description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Add an author
    pub fn add_author(&mut self, author: String) {
        self.authors.push(author);
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

    /// Add a variant
    pub fn add_variant(&mut self, variant: Vec<String>) {
        self.variants.push(variant);
    }

    /// Add a tool
    pub fn add_tool(&mut self, tool: String) {
        self.tools.push(tool);
    }

    /// Set commands
    pub fn set_commands(&mut self, commands: String) {
        self.commands = Some(commands);
    }

    /// Get string representation
    fn __str__(&self) -> String {
        self.qualified_name()
    }

    /// Get representation
    fn __repr__(&self) -> String {
        format!("Package('{}')", self.qualified_name())
    }

    /// Create a new package (static method)
    #[staticmethod]
    pub fn new_static(name: String) -> Self {
        Self::new(name)
    }
}

#[cfg(not(feature = "python-bindings"))]
impl Package {
    pub fn new(name: String) -> Self {
        Self {
            name,
            version: None,
            description: None,
            authors: Vec::new(),
            requires: Vec::new(),
            build_requires: Vec::new(),
            private_build_requires: Vec::new(),
            variants: Vec::new(),
            tools: Vec::new(),
            commands: None,
            build_command: None,
            build_system: None,
            pre_commands: None,
            post_commands: None,
            pre_test_commands: None,
            pre_build_commands: None,
            tests: HashMap::new(),
            requires_rez_version: None,
            uuid: None,
            config: HashMap::new(),
            help: None,
            relocatable: None,
            cachable: None,
            timestamp: None,
            revision: None,
            changelog: None,
            release_message: None,
            previous_version: None,
            previous_revision: None,
            vcs: None,
            format_version: None,
            base: None,
            has_plugins: None,
            plugin_for: Vec::new(),
            hashed_variants: None,
            preprocess: None,
        }
    }

    /// Get the qualified name of the package (name-version)
    pub fn qualified_name(&self) -> String {
        match &self.version {
            Some(version) => format!("{}-{}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Get the package as an exact requirement string
    pub fn as_exact_requirement(&self) -> String {
        match &self.version {
            Some(version) => format!("{}=={}", self.name, version.as_str()),
            None => self.name.clone(),
        }
    }

    /// Check if this is a package (always true for Package)
    pub fn is_package(&self) -> bool {
        true
    }

    /// Check if this is a variant (always false for Package)
    pub fn is_variant(&self) -> bool {
        false
    }

    /// Get the number of variants
    pub fn num_variants(&self) -> usize {
        self.variants.len()
    }

    /// Set the package version
    pub fn set_version(&mut self, version: Version) {
        self.version = Some(version);
    }

    /// Set the package description
    pub fn set_description(&mut self, description: String) {
        self.description = Some(description);
    }

    /// Add an author
    pub fn add_author(&mut self, author: String) {
        self.authors.push(author);
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

    /// Add a variant
    pub fn add_variant(&mut self, variant: Vec<String>) {
        self.variants.push(variant);
    }

    /// Add a tool
    pub fn add_tool(&mut self, tool: String) {
        self.tools.push(tool);
    }

    /// Set commands
    pub fn set_commands(&mut self, commands: String) {
        self.commands = Some(commands);
    }

    /// Validate the package definition
    pub fn validate(&self) -> Result<(), RezCoreError> {
        // Check required fields
        if self.name.is_empty() {
            return Err(RezCoreError::PackageParse(
                "Package name cannot be empty".to_string(),
            ));
        }

        // Validate name format (alphanumeric, underscore, hyphen)
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RezCoreError::PackageParse(format!(
                "Invalid package name '{}': only alphanumeric, underscore, and hyphen allowed",
                self.name
            )));
        }

        // Validate version if present
        if let Some(ref version) = self.version {
            // Version validation is handled by the Version type itself
            if version.as_str().is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Package version cannot be empty".to_string(),
                ));
            }
        }

        // Validate requirements format
        for req in &self.requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Build requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.private_build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Private build requirement cannot be empty".to_string(),
                ));
            }
        }

        // Validate variants
        for variant in &self.variants {
            for req in variant {
                if req.is_empty() {
                    return Err(RezCoreError::PackageParse(
                        "Variant requirement cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}

#[cfg(feature = "python-bindings")]
impl Package {
    /// Create a package from a dictionary/map
    pub fn from_dict(data: HashMap<String, PyObject>) -> Result<Self, RezCoreError> {
        Python::with_gil(|py| {
            let name = data
                .get("name")
                .ok_or_else(|| RezCoreError::PackageParse("Missing 'name' field".to_string()))?
                .extract::<String>(py)
                .map_err(|e| RezCoreError::PackageParse(format!("Invalid 'name' field: {}", e)))?;

            let mut package = Package::new(name);

            // Set version if present
            if let Some(version_obj) = data.get("version") {
                if let Ok(version_str) = version_obj.extract::<String>(py) {
                    let version = Version::parse(&version_str).map_err(|e| {
                        RezCoreError::PackageParse(format!("Invalid version: {}", e))
                    })?;
                    package.version = Some(version);
                }
            }

            // Set description if present
            if let Some(desc_obj) = data.get("description") {
                if let Ok(desc) = desc_obj.extract::<String>(py) {
                    package.description = Some(desc);
                }
            }

            // Set authors if present
            if let Some(authors_obj) = data.get("authors") {
                if let Ok(authors) = authors_obj.extract::<Vec<String>>(py) {
                    package.authors = authors;
                }
            }

            // Set requires if present
            if let Some(requires_obj) = data.get("requires") {
                if let Ok(requires) = requires_obj.extract::<Vec<String>>(py) {
                    package.requires = requires;
                }
            }

            // Set build_requires if present
            if let Some(build_requires_obj) = data.get("build_requires") {
                if let Ok(build_requires) = build_requires_obj.extract::<Vec<String>>(py) {
                    package.build_requires = build_requires;
                }
            }

            // Set variants if present
            if let Some(variants_obj) = data.get("variants") {
                if let Ok(variants) = variants_obj.extract::<Vec<Vec<String>>>(py) {
                    package.variants = variants;
                }
            }

            // Set tools if present
            if let Some(tools_obj) = data.get("tools") {
                if let Ok(tools) = tools_obj.extract::<Vec<String>>(py) {
                    package.tools = tools;
                }
            }

            Ok(package)
        })
    }

    /// Validate the package definition
    pub fn validate(&self) -> Result<(), RezCoreError> {
        // Check required fields
        if self.name.is_empty() {
            return Err(RezCoreError::PackageParse(
                "Package name cannot be empty".to_string(),
            ));
        }

        // Validate name format (alphanumeric, underscore, hyphen)
        if !self
            .name
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(RezCoreError::PackageParse(format!(
                "Invalid package name '{}': only alphanumeric, underscore, and hyphen allowed",
                self.name
            )));
        }

        // Validate version if present
        if let Some(ref version) = self.version {
            // Version validation is handled by the Version type itself
            if version.as_str().is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Package version cannot be empty".to_string(),
                ));
            }
        }

        // Validate requirements format
        for req in &self.requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Build requirement cannot be empty".to_string(),
                ));
            }
        }

        for req in &self.private_build_requires {
            if req.is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Private build requirement cannot be empty".to_string(),
                ));
            }
        }

        // Validate variants
        for variant in &self.variants {
            for req in variant {
                if req.is_empty() {
                    return Err(RezCoreError::PackageParse(
                        "Variant requirement cannot be empty".to_string(),
                    ));
                }
            }
        }

        Ok(())
    }
}
