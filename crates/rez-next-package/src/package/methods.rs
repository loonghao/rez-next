//! Package method implementations.

use rez_next_common::RezCoreError;
use rez_next_version::Version;
use std::collections::HashMap;

use super::types::Package;

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

    /// Check if the package definition is valid
    pub fn is_valid(&self) -> bool {
        self.validate().is_ok()
    }

    /// Validate the package definition
    pub fn validate(&self) -> Result<(), RezCoreError> {
        if self.name.is_empty() {
            return Err(RezCoreError::PackageParse(
                "Package name cannot be empty".to_string(),
            ));
        }

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

        if let Some(ref version) = self.version {
            if version.as_str().is_empty() {
                return Err(RezCoreError::PackageParse(
                    "Package version cannot be empty".to_string(),
                ));
            }
        }

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
