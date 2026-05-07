//! Package help functionality.
//!
//! Implements the `PackageHelp` struct for extracting and viewing help for a package.

use crate::package::Package;
use rez_next_version::VersionRange;

/// Result of help search
#[derive(Debug, Clone)]
pub struct HelpSection {
    /// Section name
    pub name: String,
    /// Section URI or command
    pub uri: String,
}

/// Object for extracting and viewing help for a package.
///
/// Given a package and version range, help will be extracted from the latest
/// package in the version range that provides it.
#[derive(Debug, Clone)]
pub struct PackageHelp {
    /// The package that provides help
    pub package: Option<Package>,
    /// Help sections
    pub sections: Vec<HelpSection>,
}

impl PackageHelp {
    /// Create a new PackageHelp object.
    ///
    /// # Arguments
    /// * `package_name` - Package to search
    /// * `version_range` - Version range to search (optional)
    /// * `packages` - List of packages to search (typically from repository)
    ///
    /// # Returns
    /// A new PackageHelp object
    pub fn new(package_name: &str, version_range: Option<&VersionRange>, packages: &[Package]) -> Self {
        // Find latest package with a help entry
        let mut sorted_packages = packages.to_vec();
        sorted_packages.sort_by(|a, b| {
            match (&a.version, &b.version) {
                (Some(v1), Some(v2)) => v2.cmp(v1), // Descending order
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => std::cmp::Ordering::Equal,
            }
        });

        let mut package = None;
        let mut sections = Vec::new();

        for pkg in &sorted_packages {
            if pkg.name != package_name {
                continue;
            }

            // Check if version matches range
            if let Some(range) = version_range {
                if let Some(ref ver) = pkg.version {
                    if !range.contains(ver) {
                        continue;
                    }
                } else {
                    continue;
                }
            }

            // Check if package has help
            if let Some(ref help) = pkg.help {
                package = Some(pkg.clone());

                // Parse help field
                // Help can be a string or a list of [name, uri] pairs
                // For now, treat as string
                sections.push(HelpSection {
                    name: "Help".to_string(),
                    uri: help.clone(),
                });
                break;
            }
        }

        Self {
            package,
            sections,
        }
    }

    /// Check if help was found.
    pub fn success(&self) -> bool {
        !self.sections.is_empty()
    }

    /// Get help sections.
    pub fn sections(&self) -> &[HelpSection] {
        &self.sections
    }

    /// Format help URIs with package context.
    ///
    /// Replaces placeholders like $BASE, $ROOT, $VERSION, etc.
    pub fn format_uris(&mut self) {
        if let Some(ref pkg) = self.package {
            let base = pkg.base.clone().unwrap_or_default();
            let root = base.clone(); // Simplified: root == base for non-variant packages
            let version = pkg.version.as_ref().map(|v| v.as_str().to_string()).unwrap_or_default();

            for section in &mut self.sections {
                let mut uri = section.uri.clone();
                
                // Replace placeholders
                uri = uri.replace("$BASE", &base);
                uri = uri.replace("$ROOT", &root);
                uri = uri.replace("$VERSION", &version);
                
                // Remove $BROWSER prefix if present
                if uri.starts_with("$BROWSER") {
                    uri = uri.trim_start_matches("$BROWSER").trim().to_string();
                }

                section.uri = uri;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;

    fn create_test_package(name: &str, version: &str, help: Option<&str>) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = Some(Version::parse(version).unwrap());
        pkg.help = help.map(|h| h.to_string());
        pkg.base = Some("/packages/mypackage/1.0.0".to_string());
        pkg
    }

    #[test]
    fn test_package_help_no_help() {
        let packages = vec![
            create_test_package("mypackage", "1.0.0", None),
        ];

        let ph = PackageHelp::new("mypackage", None, &packages);
        assert!(!ph.success());
        assert!(ph.sections().is_empty());
    }

    #[test]
    fn test_package_help_found() {
        let packages = vec![
            create_test_package("mypackage", "1.0.0", Some("https://example.com/docs")),
        ];

        let ph = PackageHelp::new("mypackage", None, &packages);
        assert!(ph.success());
        assert_eq!(ph.sections().len(), 1);
        assert_eq!(ph.sections()[0].name, "Help");
        assert_eq!(ph.sections()[0].uri, "https://example.com/docs");
    }

    #[test]
    fn test_package_help_version_range() {
        let packages = vec![
            create_test_package("mypackage", "1.0.0", Some("https://example.com/docs/1.0")),
            create_test_package("mypackage", "2.0.0", Some("https://example.com/docs/2.0")),
        ];

        let version_range = VersionRange::parse(">=2.0.0").unwrap();
        let ph = PackageHelp::new("mypackage", Some(&version_range), &packages);
        assert!(ph.success());
        assert_eq!(ph.sections()[0].uri, "https://example.com/docs/2.0");
    }

    #[test]
    fn test_package_help_format_uris() {
        let packages = vec![
            create_test_package("mypackage", "1.0.0", Some("$BASE/docs/index.html")),
        ];

        let mut ph = PackageHelp::new("mypackage", None, &packages);
        ph.format_uris();

        assert_eq!(ph.sections()[0].uri, "/packages/mypackage/1.0.0/docs/index.html");
    }

    #[test]
    fn test_package_help_latest_version() {
        let packages = vec![
            create_test_package("mypackage", "1.0.0", Some("old docs")),
            create_test_package("mypackage", "2.0.0", Some("new docs")),
        ];

        let ph = PackageHelp::new("mypackage", None, &packages);
        assert!(ph.success());
        // Should find the latest version (2.0.0)
        assert_eq!(ph.package.unwrap().version.unwrap().as_str(), "2.0.0");
    }
}
