//! Serde serialization/deserialization for Package.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::types::Package;

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
        // config excluded from serialization for compatibility
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
                            if name.is_some() { return Err(de::Error::duplicate_field("name")); }
                            name = Some(map.next_value()?);
                        }
                        Field::Version => {
                            if version.is_some() { return Err(de::Error::duplicate_field("version")); }
                            version = Some(map.next_value()?);
                        }
                        Field::Description => {
                            if description.is_some() { return Err(de::Error::duplicate_field("description")); }
                            description = Some(map.next_value()?);
                        }
                        Field::Authors => {
                            if authors.is_some() { return Err(de::Error::duplicate_field("authors")); }
                            authors = Some(map.next_value()?);
                        }
                        Field::Requires => {
                            if requires.is_some() { return Err(de::Error::duplicate_field("requires")); }
                            requires = Some(map.next_value()?);
                        }
                        Field::BuildRequires => {
                            if build_requires.is_some() { return Err(de::Error::duplicate_field("build_requires")); }
                            build_requires = Some(map.next_value()?);
                        }
                        Field::PrivateBuildRequires => {
                            if private_build_requires.is_some() { return Err(de::Error::duplicate_field("private_build_requires")); }
                            private_build_requires = Some(map.next_value()?);
                        }
                        Field::Variants => {
                            if variants.is_some() { return Err(de::Error::duplicate_field("variants")); }
                            variants = Some(map.next_value()?);
                        }
                        Field::Tools => {
                            if tools.is_some() { return Err(de::Error::duplicate_field("tools")); }
                            tools = Some(map.next_value()?);
                        }
                        Field::Commands => {
                            if commands.is_some() { return Err(de::Error::duplicate_field("commands")); }
                            commands = Some(map.next_value()?);
                        }
                        Field::BuildCommand => {
                            if build_command.is_some() { return Err(de::Error::duplicate_field("build_command")); }
                            build_command = Some(map.next_value()?);
                        }
                        Field::BuildSystem => {
                            if build_system.is_some() { return Err(de::Error::duplicate_field("build_system")); }
                            build_system = Some(map.next_value()?);
                        }
                        Field::PreCommands => {
                            if pre_commands.is_some() { return Err(de::Error::duplicate_field("pre_commands")); }
                            pre_commands = Some(map.next_value()?);
                        }
                        Field::PostCommands => {
                            if post_commands.is_some() { return Err(de::Error::duplicate_field("post_commands")); }
                            post_commands = Some(map.next_value()?);
                        }
                        Field::PreTestCommands => {
                            if pre_test_commands.is_some() { return Err(de::Error::duplicate_field("pre_test_commands")); }
                            pre_test_commands = Some(map.next_value()?);
                        }
                        Field::PreBuildCommands => {
                            if pre_build_commands.is_some() { return Err(de::Error::duplicate_field("pre_build_commands")); }
                            pre_build_commands = Some(map.next_value()?);
                        }
                        Field::Tests => {
                            if tests.is_some() { return Err(de::Error::duplicate_field("tests")); }
                            tests = Some(map.next_value()?);
                        }
                        Field::RequiresRezVersion => {
                            if requires_rez_version.is_some() { return Err(de::Error::duplicate_field("requires_rez_version")); }
                            requires_rez_version = Some(map.next_value()?);
                        }
                        Field::Uuid => {
                            if uuid.is_some() { return Err(de::Error::duplicate_field("uuid")); }
                            uuid = Some(map.next_value()?);
                        }
                        Field::Help => {
                            if help.is_some() { return Err(de::Error::duplicate_field("help")); }
                            help = Some(map.next_value()?);
                        }
                        Field::Relocatable => {
                            if relocatable.is_some() { return Err(de::Error::duplicate_field("relocatable")); }
                            relocatable = Some(map.next_value()?);
                        }
                        Field::Cachable => {
                            if cachable.is_some() { return Err(de::Error::duplicate_field("cachable")); }
                            cachable = Some(map.next_value()?);
                        }
                        Field::Timestamp => {
                            if timestamp.is_some() { return Err(de::Error::duplicate_field("timestamp")); }
                            timestamp = Some(map.next_value()?);
                        }
                        Field::Revision => {
                            if revision.is_some() { return Err(de::Error::duplicate_field("revision")); }
                            revision = Some(map.next_value()?);
                        }
                        Field::Changelog => {
                            if changelog.is_some() { return Err(de::Error::duplicate_field("changelog")); }
                            changelog = Some(map.next_value()?);
                        }
                        Field::ReleaseMessage => {
                            if release_message.is_some() { return Err(de::Error::duplicate_field("release_message")); }
                            release_message = Some(map.next_value()?);
                        }
                        Field::PreviousVersion => {
                            if previous_version.is_some() { return Err(de::Error::duplicate_field("previous_version")); }
                            previous_version = Some(map.next_value()?);
                        }
                        Field::PreviousRevision => {
                            if previous_revision.is_some() { return Err(de::Error::duplicate_field("previous_revision")); }
                            previous_revision = Some(map.next_value()?);
                        }
                        Field::Vcs => {
                            if vcs.is_some() { return Err(de::Error::duplicate_field("vcs")); }
                            vcs = Some(map.next_value()?);
                        }
                        Field::FormatVersion => {
                            if format_version.is_some() { return Err(de::Error::duplicate_field("format_version")); }
                            format_version = Some(map.next_value()?);
                        }
                        Field::Base => {
                            if base.is_some() { return Err(de::Error::duplicate_field("base")); }
                            base = Some(map.next_value()?);
                        }
                        Field::HasPlugins => {
                            if has_plugins.is_some() { return Err(de::Error::duplicate_field("has_plugins")); }
                            has_plugins = Some(map.next_value()?);
                        }
                        Field::PluginFor => {
                            if plugin_for.is_some() { return Err(de::Error::duplicate_field("plugin_for")); }
                            plugin_for = Some(map.next_value()?);
                        }
                        Field::HashedVariants => {
                            if hashed_variants.is_some() { return Err(de::Error::duplicate_field("hashed_variants")); }
                            hashed_variants = Some(map.next_value()?);
                        }
                        Field::Preprocess => {
                            if preprocess.is_some() { return Err(de::Error::duplicate_field("preprocess")); }
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
                    commands_function: None,
                    build_command: build_command.unwrap_or(None),
                    build_system: build_system.unwrap_or(None),
                    pre_commands: pre_commands.unwrap_or(None),
                    post_commands: post_commands.unwrap_or(None),
                    pre_test_commands: pre_test_commands.unwrap_or(None),
                    pre_build_commands: pre_build_commands.unwrap_or(None),
                    tests: tests.unwrap_or_default(),
                    requires_rez_version: requires_rez_version.unwrap_or(None),
                    uuid: uuid.unwrap_or(None),
                    config: HashMap::new(),
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
            "name", "version", "description", "authors", "requires",
            "build_requires", "private_build_requires", "variants", "tools",
            "commands", "build_command", "build_system", "pre_commands",
            "post_commands", "pre_test_commands", "pre_build_commands", "tests",
            "requires_rez_version", "uuid", "help", "relocatable", "cachable",
            "timestamp", "revision", "changelog", "release_message",
            "previous_version", "previous_revision", "vcs", "format_version",
            "base", "has_plugins", "plugin_for", "hashed_variants", "preprocess",
        ];
        deserializer.deserialize_struct("Package", FIELDS, PackageVisitor)
    }
}
