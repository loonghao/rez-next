//! Package implementation

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
    /// Whether this is a weak requirement (prefix ~)
    pub weak: bool,
    /// Whether this is a conflict requirement (prefix !)
    pub conflict: bool,
}

impl std::fmt::Display for PackageRequirement {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let base = if let Some(ref version) = self.version_spec {
            format!("{}-{}", self.name, version)
        } else {
            self.name.clone()
        };
        if self.conflict {
            write!(f, "!{}", base)
        } else if self.weak {
            write!(f, "~{}", base)
        } else {
            write!(f, "{}", base)
        }
    }
}

impl PackageRequirement {
    /// Create a new package requirement
    pub fn new(name: String) -> Self {
        Self {
            name,
            version_spec: None,
            weak: false,
            conflict: false,
        }
    }

    /// Create a package requirement with version specification
    pub fn with_version(name: String, version_spec: String) -> Self {
        Self {
            name,
            version_spec: Some(version_spec),
            weak: false,
            conflict: false,
        }
    }

    /// Parse a requirement string.
    ///
    /// Supports the following rez requirement formats:
    /// - `python` — plain name requirement
    /// - `python-3.9` — name with version
    /// - `python>=3.9` — name with operator-prefixed version spec
    /// - `~python` — weak (optional) requirement
    /// - `!python` — conflict requirement (must NOT be present)
    /// - `!python-3.9` — conflict requirement with version
    pub fn parse(requirement_str: &str) -> Result<Self, RezCoreError> {
        // Handle conflict requirement prefix (!)
        let (s, conflict) = if let Some(rest) = requirement_str.strip_prefix('!') {
            (rest, true)
        } else {
            (requirement_str, false)
        };

        // Handle weak requirement prefix (~), but not ~= which is a version operator
        let (s, weak) = if s.starts_with('~') && !s.starts_with("~=") {
            if let Some(rest) = s.strip_prefix('~') {
                (rest, true)
            } else {
                (s, false)
            }
        } else {
            (s, false)
        };

        // Parse name and version
        let mut req = if let Some(dash_pos) = s.rfind('-') {
            // Ensure it's a version separator and not part of the name
            let potential_name = &s[..dash_pos];
            let potential_version = &s[dash_pos + 1..];
            // A version separator dash is followed by a digit
            if potential_version.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                Self::with_version(potential_name.to_string(), potential_version.to_string())
            } else {
                Self::new(s.to_string())
            }
        } else {
            Self::new(s.to_string())
        };
        req.weak = weak;
        req.conflict = conflict;
        Ok(req)
    }

    /// Get the package name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Get the version specification
    pub fn version_spec(&self) -> Option<&str> {
        self.version_spec.as_deref()
    }

    /// Get requirement string (for compatibility)
    pub fn requirement_string(&self) -> String {
        self.to_string()
    }

    /// Check if this requirement is satisfied by a version
    pub fn satisfied_by(&self, version: &Version) -> bool {
        if let Some(ref version_spec) = self.version_spec {
            // Support range operators: >=, <=, >, <, ==, !=, ~=
            let spec = version_spec.trim();

            if spec.is_empty() {
                return true;
            }

            // Handle range with comma: ">=1.0,<2.0"
            if spec.contains(',') {
                return spec
                    .split(',')
                    .all(|part| Self::check_single_constraint(version, part.trim()));
            }

            Self::check_single_constraint(version, spec)
        } else {
            true
        }
    }

    /// Check a single version constraint like ">=1.0" or "2.1.0"
    fn check_single_constraint(version: &Version, spec: &str) -> bool {
        use rez_next_version::VersionRange;

        // Handle PEP 440 / rez operators first (before VersionRange stub)
        let (op, ver_str) = if spec.starts_with(">=") {
            (">=", &spec[2..])
        } else if spec.starts_with("<=") {
            ("<=", &spec[2..])
        } else if spec.starts_with("!=") {
            ("!=", &spec[2..])
        } else if spec.starts_with("~=") {
            ("~=", &spec[2..])
        } else if spec.starts_with("==") {
            ("==", &spec[2..])
        } else if spec.starts_with('>') {
            (">", &spec[1..])
        } else if spec.starts_with('<') {
            ("<", &spec[1..])
        } else {
            // No leading operator: try rez-style range first (e.g. "1.0+", "1.2<2.0")
            if let Ok(range) = VersionRange::parse(spec) {
                return range.contains(version);
            }
            // Plain version string: exact match
            ("==", spec)
        };

        let ver_str = ver_str.trim();
        if let Ok(constraint_ver) = Version::parse(ver_str) {
            use crate::requirement::VersionConstraint;
            let constraint = match op {
                ">=" => VersionConstraint::GreaterThanOrEqual(constraint_ver),
                "<=" => VersionConstraint::LessThanOrEqual(constraint_ver),
                ">" => VersionConstraint::GreaterThan(constraint_ver),
                "<" => VersionConstraint::LessThan(constraint_ver),
                "!=" => VersionConstraint::Exclude(vec![constraint_ver]),
                "~=" => VersionConstraint::Compatible(constraint_ver),
                _ => VersionConstraint::Exact(constraint_ver),
            };
            constraint.is_satisfied_by(version)
        } else {
            // Could not parse version spec as a version — exact string match
            version.as_str() == ver_str
        }
    }
}

/// High-performance package representation compatible with rez
#[derive(Debug, Clone)]
pub struct Package {
    /// Package name
    pub name: String,

    /// Package version
    pub version: Option<Version>,

    /// Package description
    pub description: Option<String>,

    /// Package authors
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

    /// Package commands (rex script string, set from `def commands():` body)
    pub commands: Option<String>,

    /// Package commands function body (alias for commands; used by validation layer)
    pub commands_function: Option<String>,

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
    pub uuid: Option<String>,

    /// Package config
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

/// Number of fields serialized in the Package struct (excludes `config` and `commands_function`).
const PACKAGE_SERIALIZED_FIELD_COUNT: usize = 35;

impl Serialize for Package {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeStruct;
        let mut state = serializer.serialize_struct("Package", PACKAGE_SERIALIZED_FIELD_COUNT)?;
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
        // Note: config field is excluded from serialization for compatibility
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
                    commands_function: None, // Not stored in serialized form; set during parse
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

        const FIELDS: &[&str] = &[
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
            commands_function: None,
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

    /// Check if the package definition is valid (convenience bool version of validate())
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
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

#[cfg(test)]
mod package_tests {
    use super::*;
    use rez_next_version::Version;

    fn ver(s: &str) -> Version {
        Version::parse(s).unwrap()
    }

    #[test]
    fn test_pkg_req_satisfied_no_constraint() {
        let r = PackageRequirement::parse("python").unwrap();
        assert!(r.satisfied_by(&ver("3.9.0")));
    }

    #[test]
    fn test_pkg_req_satisfied_ge() {
        // Use identical version string format in constraint and test version
        let r = PackageRequirement::with_version("python".into(), ">=3.8.0".into());
        assert!(r.satisfied_by(&ver("3.9.0")));
        assert!(r.satisfied_by(&ver("3.8.0")));
        assert!(!r.satisfied_by(&ver("3.7.0")));
    }

    #[test]
    fn test_pkg_req_satisfied_lt() {
        let r = PackageRequirement::with_version("python".into(), "<3.10.0".into());
        assert!(r.satisfied_by(&ver("3.9.0")));
        assert!(!r.satisfied_by(&ver("3.10.0")));
    }

    #[test]
    fn test_pkg_req_satisfied_ne() {
        // Use exact same string so version equality is reliable
        let r = PackageRequirement::with_version("python".into(), "!=3.8.0".into());
        assert!(r.satisfied_by(&ver("3.9.0")));
        assert!(!r.satisfied_by(&ver("3.8.0")));
    }

    #[test]
    fn test_pkg_req_satisfied_compatible() {
        // ~=1.4.0 means >=1.4.0 AND same minor (starts_with "1.4.")
        let r = PackageRequirement::with_version("mylib".into(), "~=1.4.0".into());
        assert!(r.satisfied_by(&ver("1.4.0")));
        assert!(r.satisfied_by(&ver("1.4.5")));
        assert!(!r.satisfied_by(&ver("1.5.0")));
    }

    #[test]
    fn test_package_new_and_validate() {
        let pkg = Package::new("mylib".to_string());
        assert_eq!(pkg.name, "mylib");
        assert!(pkg.version.is_none());
        assert!(pkg.validate().is_ok());
    }

    #[test]
    fn test_package_empty_name_invalid() {
        assert!(Package::new("".to_string()).validate().is_err());
    }

    // ── conflict requirement (!pkg) ───────────────────────────────────

    #[test]
    fn test_conflict_requirement_parse() {
        let req = PackageRequirement::parse("!python").unwrap();
        assert_eq!(req.name, "python");
        assert!(req.conflict, "!python must be a conflict requirement");
        assert!(!req.weak);
        assert!(req.version_spec.is_none());
    }

    #[test]
    fn test_conflict_requirement_with_version() {
        let req = PackageRequirement::parse("!python-3.9").unwrap();
        assert_eq!(req.name, "python");
        assert!(req.conflict);
        assert_eq!(req.version_spec.as_deref(), Some("3.9"));
    }

    #[test]
    fn test_conflict_requirement_to_string() {
        let req = PackageRequirement::parse("!python").unwrap();
        assert_eq!(req.to_string(), "!python");
    }

    #[test]
    fn test_conflict_requirement_with_version_to_string() {
        let req = PackageRequirement::parse("!python-3.9").unwrap();
        assert_eq!(req.to_string(), "!python-3.9");
    }

    #[test]
    fn test_weak_requirement_to_string() {
        let req = PackageRequirement::parse("~numpy").unwrap();
        assert_eq!(req.name, "numpy");
        assert!(req.weak);
        assert!(!req.conflict);
        assert_eq!(req.to_string(), "~numpy");
    }

    #[test]
    fn test_normal_requirement_not_conflict_not_weak() {
        let req = PackageRequirement::parse("maya-2024").unwrap();
        assert!(!req.conflict);
        assert!(!req.weak);
        assert_eq!(req.name, "maya");
        assert_eq!(req.version_spec.as_deref(), Some("2024"));
    }

    #[test]
    fn test_conflict_takes_priority_over_weak() {
        // "!" prefix is checked before "~", so "!~pkg" would be conflict + name "~pkg"
        // but "~!pkg" is NOT valid rez syntax (~ before ! is not standard)
        let req = PackageRequirement::parse("!python").unwrap();
        assert!(req.conflict);
        assert!(!req.weak);
    }
}
