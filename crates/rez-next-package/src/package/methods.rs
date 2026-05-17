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
            is_dev_package: None,
            filepath: None,
            includes: None,
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

    /// Load a developer package from a path.
    ///
    /// This aligns with Rez's `DeveloperPackage.from_path()` interface.
    /// Supports both file path (package.py/package.yaml) and directory path.
    ///
    /// # Arguments
    /// * `path` - Directory containing package definition, or path to the file itself
    ///
    /// # Returns
    /// * `Result<Package, RezCoreError>` - The loaded package
    ///
    /// # Example
    /// ```
    /// # use rez_next_package::package::types::Package;
    /// # use std::path::PathBuf;
    /// # use tempfile::TempDir;
    /// # let dir = TempDir::new().unwrap();
    /// # let pkg_path = dir.path().join("package.py");
    /// # std::fs::write(&pkg_path, r#"name = "mypackage""#).unwrap();
    /// let pkg = Package::from_path(pkg_path).unwrap();
    /// assert_eq!(pkg.name, "mypackage");
    /// ```
    pub fn from_path<P: AsRef<std::path::Path>>(path: P) -> Result<Self, RezCoreError> {
        let path = path.as_ref();

        // Determine if path is a directory or file
        let file_path = if path.is_dir() {
            // Look for package.py or package.yaml in directory
            let package_py = path.join("package.py");
            let package_yaml = path.join("package.yaml");
            let package_yml = path.join("package.yml");

            if package_py.exists() {
                package_py
            } else if package_yaml.exists() {
                package_yaml
            } else if package_yml.exists() {
                package_yml
            } else {
                return Err(RezCoreError::PackageParse(
                    format!("No package definition file found in {}", path.display())
                ));
            }
        } else {
            path.to_path_buf()
        };

        // Load package from file
        let mut pkg = crate::serialization::PackageSerializer::load_from_file(&file_path)
            .map_err(|e| RezCoreError::PackageParse(format!("Failed to load package: {}", e)))?;

        // Set filepath
        pkg.filepath = Some(file_path.to_string_lossy().to_string());

        // Set is_dev_package flag
        pkg.is_dev_package = Some(true);

        // TODO: Collect includes from SourceCode objects
        // This requires parsing the package data to find all @include decorators
        // For now, we leave includes as None

        Ok(pkg)
    }

    /// Get the root directory of the package (parent of filepath).
    ///
    /// Aligns with Rez's `DeveloperPackage.root` property.
    pub fn root(&self) -> Option<String> {
        self.filepath.as_ref().and_then(|fp| {
            std::path::Path::new(fp).parent().map(|p| p.to_string_lossy().to_string())
        })
    }

    /// Serialize the package to package.py format string.
    /// Compatible with rez package.py format.
    pub fn to_package_py(&self) -> String {
        let mut lines = Vec::new();

        // name (required)
        lines.push(format!("name = \"{}\"", self.name));

        // version (optional)
        if let Some(ref version) = self.version {
            lines.push(format!("version = \"{}\"", version.as_str()));
        }

        // description (optional)
        if let Some(ref desc) = self.description {
            // Escape quotes in description
            let desc_escaped = desc.replace("\"", "\\\"");
            lines.push(format!("description = \"{}\"", desc_escaped));
        }

        // authors (optional)
        if !self.authors.is_empty() {
            let authors_str = self
                .authors
                .iter()
                .map(|a| format!("\"{}\"", a.replace("\"", "\\\"")))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("authors = [{}]", authors_str));
        }

        // requires (optional)
        if !self.requires.is_empty() {
            let req_str = self
                .requires
                .iter()
                .map(|r| format!("    \"{}\"", r))
                .collect::<Vec<_>>()
                .join(",\n");
            lines.push(format!("requires = [\n{}\n]", req_str));
        }

        // build_requires (optional)
        if !self.build_requires.is_empty() {
            let req_str = self
                .build_requires
                .iter()
                .map(|r| format!("    \"{}\"", r))
                .collect::<Vec<_>>()
                .join(",\n");
            lines.push(format!("build_requires = [\n{}\n]", req_str));
        }

        // private_build_requires (optional)
        if !self.private_build_requires.is_empty() {
            let req_str = self
                .private_build_requires
                .iter()
                .map(|r| format!("    \"{}\"", r))
                .collect::<Vec<_>>()
                .join(",\n");
            lines.push(format!("private_build_requires = [\n{}\n]", req_str));
        }

        // variants (optional)
        if !self.variants.is_empty() {
            let mut variant_lines = Vec::new();
            for variant in &self.variants {
                let var_str = variant
                    .iter()
                    .map(|r| format!("\"{}\"", r))
                    .collect::<Vec<_>>()
                    .join(", ");
                variant_lines.push(format!("    [{}]", var_str));
            }
            lines.push(format!("variants = [\n{}\n]", variant_lines.join(",\n")));
        }

        // tools (optional)
        if !self.tools.is_empty() {
            let tools_str = self
                .tools
                .iter()
                .map(|t| format!("\"{}\"", t))
                .collect::<Vec<_>>()
                .join(", ");
            lines.push(format!("tools = [{}]", tools_str));
        }

        // uuid (optional)
        if let Some(ref uuid) = self.uuid {
            lines.push(format!("uuid = \"{}\"", uuid));
        }

        // relocatable (optional)
        if let Some(ref relocatable) = self.relocatable {
            lines.push(format!("relocatable = {}", relocatable));
        }

        // cachable (optional)
        if let Some(ref cachable) = self.cachable {
            lines.push(format!("cachable = {}", cachable));
        }

        // commands function (optional)
        if let Some(ref commands) = self.commands {
            lines.push("\ndef commands():".to_string());
            // Add commands function body with proper indentation
            for line in commands.lines() {
                if line.trim().is_empty() {
                    lines.push("".to_string());
                } else {
                    lines.push(format!("    {}", line));
                }
            }
        }

        // pre_commands (optional)
        if let Some(ref pre_commands) = self.pre_commands {
            lines.push("\ndef pre_commands():".to_string());
            for line in pre_commands.lines() {
                if line.trim().is_empty() {
                    lines.push("".to_string());
                } else {
                    lines.push(format!("    {}", line));
                }
            }
        }

        // post_commands (optional)
        if let Some(ref post_commands) = self.post_commands {
            lines.push("\ndef post_commands():".to_string());
            for line in post_commands.lines() {
                if line.trim().is_empty() {
                    lines.push("".to_string());
                } else {
                    lines.push(format!("    {}", line));
                }
            }
        }

        // tests (optional) - simplified serialization
        if !self.tests.is_empty() {
            lines.push("\ntests = {".to_string());
            for test_name in self.tests.keys() {
                // Simplified: just add a comment, full serialization is complex
                lines.push(format!("    \"{}\": {{", test_name));
                lines.push("        # test definition".to_string());
                lines.push("    },".to_string());
            }
            lines.push("}".to_string());
        }

        lines.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;

    #[test]
    fn test_to_package_py_basic() {
        let pkg = Package::new("test_pkg".to_string());
        let result = pkg.to_package_py();
        assert!(result.contains("name = \"test_pkg\""));
        assert!(!result.contains("version = "));
    }

    #[test]
    fn test_to_package_py_with_version() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        let result = pkg.to_package_py();
        assert!(result.contains("name = \"test_pkg\""));
        assert!(result.contains("version = \"1.0.0\""));
    }

    #[test]
    fn test_to_package_py_with_description() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.description = Some("A test package".to_string());
        let result = pkg.to_package_py();
        assert!(result.contains("description = \"A test package\""));
    }

    #[test]
    fn test_to_package_py_with_authors() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.authors = vec!["Author One".to_string(), "Author Two".to_string()];
        let result = pkg.to_package_py();
        assert!(result.contains("authors = ["));
        assert!(result.contains("\"Author One\""));
        assert!(result.contains("\"Author Two\""));
    }

    #[test]
    fn test_to_package_py_with_requires() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.requires = vec!["python-3.9".to_string(), "maya-2024".to_string()];
        let result = pkg.to_package_py();
        assert!(result.contains("requires = ["));
        assert!(result.contains("\"python-3.9\""));
        assert!(result.contains("\"maya-2024\""));
    }

    #[test]
    fn test_to_package_py_with_variants() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.variants = vec![
            vec!["python-3.9".to_string()],
            vec!["python-3.10".to_string()],
        ];
        let result = pkg.to_package_py();
        assert!(result.contains("variants = ["));
        assert!(result.contains("[\"python-3.9\"]"));
        assert!(result.contains("[\"python-3.10\"]"));
    }

    #[test]
    fn test_to_package_py_with_tools() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.tools = vec!["mytool".to_string(), "another_tool".to_string()];
        let result = pkg.to_package_py();
        assert!(result.contains("tools = ["));
        assert!(result.contains("\"mytool\""));
        assert!(result.contains("\"another_tool\""));
    }

    #[test]
    fn test_to_package_py_with_commands() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.commands = Some("    env.PATH.prepend(\"{root}/bin\")\n".to_string());
        let result = pkg.to_package_py();
        assert!(result.contains("def commands():"));
        assert!(result.contains("env.PATH.prepend"));
    }

    #[test]
    fn test_to_package_py_with_uuid() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.uuid = Some("12345678-1234-1234-1234-123456789012".to_string());
        let result = pkg.to_package_py();
        assert!(result.contains("uuid = \"12345678-1234-1234-1234-123456789012\""));
    }

    #[test]
    fn test_to_package_py_with_relocatable() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.relocatable = Some(true);
        let result = pkg.to_package_py();
        assert!(result.contains("relocatable = true"));
    }

    #[test]
    fn test_to_package_py_with_cachable() {
        let mut pkg = Package::new("test_pkg".to_string());
        pkg.cachable = Some(false);
        let result = pkg.to_package_py();
        assert!(result.contains("cachable = false"));
    }

    #[test]
    fn test_to_package_py_complete() {
        let mut pkg = Package::new("complete_pkg".to_string());
        pkg.version = Some(Version::parse("2.1.0").unwrap());
        pkg.description = Some("A complete test package".to_string());
        pkg.authors = vec!["Test Author".to_string()];
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.build_requires = vec!["cmake-3.20".to_string()];
        pkg.variants = vec![vec!["python-3.9".to_string()], vec!["python-3.10".to_string()]];
        pkg.tools = vec!["mytool".to_string()];
        pkg.commands = Some("    env.PATH.prepend(\"{root}/bin\")\n".to_string());
        pkg.uuid = Some("12345678-1234-1234-1234-123456789012".to_string());
        pkg.relocatable = Some(true);
        pkg.cachable = Some(true);

        let result = pkg.to_package_py();

        // Verify all fields are present
        assert!(result.contains("name = \"complete_pkg\""));
        assert!(result.contains("version = \"2.1.0\""));
        assert!(result.contains("description = \"A complete test package\""));
        assert!(result.contains("authors = ["));
        assert!(result.contains("requires = ["));
        assert!(result.contains("build_requires = ["));
        assert!(result.contains("variants = ["));
        assert!(result.contains("tools = ["));
        assert!(result.contains("def commands():"));
        assert!(result.contains("uuid = "));
        assert!(result.contains("relocatable = true"));
        assert!(result.contains("cachable = true"));
    }

    #[test]
    fn test_to_package_py_output_format_valid() {
        let mut pkg = Package::new("format_test".to_string());
        pkg.version = Some(Version::parse("1.0.0").unwrap());
        pkg.requires = vec!["python-3.9".to_string()];
        pkg.commands = Some("    env.PATH.prepend(\"{root}/bin\")\n".to_string());

        let result = pkg.to_package_py();

        // Verify output can be parsed as valid Python (basic check)
        assert!(result.starts_with("name = "));
        assert!(result.contains("def commands():"));
        assert!(result.contains("    env.PATH.prepend"));
    }

    #[test]
    fn test_to_package_py_escapes_description() {
        let mut pkg = Package::new("escape_test".to_string());
        pkg.description = Some("Description with \"quotes\"".to_string());
        let result = pkg.to_package_py();
        assert!(result.contains("Description with \\\"quotes\\\""));
    }

    // ── Phase N: from_path() and root() tests ─────────────────────────────

    #[test]
    fn test_from_path_with_package_py() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let pkg_path = tmp.path().join("package.py");
        let mut file = File::create(&pkg_path).unwrap();
        writeln!(file, "name = 'mypackage'").unwrap();
        writeln!(file, "version = '1.0.0'").unwrap();
        file.flush().unwrap();

        let pkg = Package::from_path(pkg_path.to_str().unwrap()).unwrap();
        assert_eq!(pkg.name, "mypackage");
        assert_eq!(pkg.version.as_ref().unwrap().as_str(), "1.0.0");
        assert_eq!(pkg.filepath, Some(pkg_path.to_string_lossy().to_string()));
        assert_eq!(pkg.is_dev_package, Some(true));
    }

    #[test]
    fn test_from_path_with_directory() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let pkg_path = tmp.path().join("package.py");
        let mut file = File::create(&pkg_path).unwrap();
        writeln!(file, "name = 'dirpkg'").unwrap();
        writeln!(file, "version = '2.0.0'").unwrap();
        file.flush().unwrap();

        // Pass directory path
        let pkg = Package::from_path(tmp.path().to_str().unwrap()).unwrap();
        assert_eq!(pkg.name, "dirpkg");
        assert_eq!(pkg.filepath, Some(pkg_path.to_string_lossy().to_string()));
    }

    #[test]
    fn test_root() {
        use std::fs::File;
        use std::io::Write;
        use tempfile::TempDir;

        let tmp = TempDir::new().unwrap();
        let pkg_path = tmp.path().join("package.py");
        let mut file = File::create(&pkg_path).unwrap();
        writeln!(file, "name = 'rootpkg'").unwrap();
        file.flush().unwrap();

        let pkg = Package::from_path(pkg_path.to_str().unwrap()).unwrap();
        let root = pkg.root();
        assert_eq!(root, Some(tmp.path().to_string_lossy().to_string()));
    }

    #[test]
    fn test_from_path_no_package_file() {
        use tempfile::TempDir;
        let tmp = TempDir::new().unwrap();
        let result = Package::from_path(tmp.path().to_str().unwrap());
        assert!(result.is_err());
    }
}
