//! PackageVariant and PackageVariantCache types
//!
//! Defines `PackageVariant` and `PackageVariantCache`,
//! mirroring `rez.solver.PackageVariant` and `PackageVariantCache`.

use rez_next_package::Package;
use std::collections::HashMap;

/// A package variant (a specific build/configuration).
///
/// Mirrors `rez.solver.PackageVariant`.
#[derive(Debug, Clone)]
pub struct PackageVariant {
    /// The package this variant belongs to.
    pub package: Package,

    /// Variant index (0-based).
    pub index: usize,

    /// Variant name (e.g., "variant1", "variant2").
    pub name: Option<String>,

    /// Requirements specific to this variant.
    pub requirements: Vec<String>,

    /// Environment variables set by this variant.
    pub environment: HashMap<String, String>,
}

impl PackageVariant {
    /// Create a new `PackageVariant`.
    pub fn new(package: Package, index: usize) -> Self {
        Self {
            package,
            index,
            name: None,
            requirements: Vec::new(),
            environment: HashMap::new(),
        }
    }

    /// Get the package name.
    pub fn package_name(&self) -> String {
        self.package.name.clone()
    }

    /// Get the package version (if available).
    pub fn version(&self) -> Option<String> {
        self.package.version.as_ref().map(|v| v.as_str().to_string())
    }
}

/// Cache of package variants.
///
/// Mirrors `rez.solver.PackageVariantCache`.
#[derive(Debug, Clone)]
pub struct PackageVariantCache {
    /// Cached variants: package_name -> Vec<PackageVariant>
    pub variants: HashMap<String, Vec<PackageVariant>>,

    /// Cache statistics.
    pub hits: usize,
    pub misses: usize,
}

impl PackageVariantCache {
    /// Create a new empty `PackageVariantCache`.
    pub fn new() -> Self {
        Self {
            variants: HashMap::new(),
            hits: 0,
            misses: 0,
        }
    }

    /// Get variants for a package.
    pub fn get_variants(&mut self, package_name: &str) -> Option<&Vec<PackageVariant>> {
        if self.variants.contains_key(package_name) {
            self.hits += 1;
            self.variants.get(package_name)
        } else {
            self.misses += 1;
            None
        }
    }

    /// Cache variants for a package.
    pub fn cache_variants(&mut self, package_name: String, variants: Vec<PackageVariant>) {
        self.variants.insert(package_name, variants);
    }

    /// Invalidate cache for a package.
    pub fn invalidate(&mut self, package_name: &str) {
        self.variants.remove(package_name);
    }

    /// Clear the entire cache.
    pub fn clear(&mut self) {
        self.variants.clear();
        self.hits = 0;
        self.misses = 0;
    }

    /// Cache hit rate.
    pub fn hit_rate(&self) -> f64 {
        let total = self.hits + self.misses;
        if total == 0 {
            0.0
        } else {
            self.hits as f64 / total as f64
        }
    }
}

impl Default for PackageVariantCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_package::Package;

    fn make_package(name: &str) -> Package {
        let mut pkg = Package::new(name.to_string());
        pkg.version = None;
        pkg.description = None;
        pkg.requires = vec![];
        pkg
    }

    #[test]
    fn test_package_variant_new() {
        let pkg = make_package("python");
        let pv = PackageVariant::new(pkg, 0);
        assert_eq!(pv.package_name(), "python");
        assert_eq!(pv.index, 0);
    }

    #[test]
    fn test_package_variant_cache_new() {
        let cache = PackageVariantCache::new();
        assert_eq!(cache.hits, 0);
        assert_eq!(cache.misses, 0);
    }

    #[test]
    fn test_package_variant_cache_get() {
        let mut cache = PackageVariantCache::new();
        let pkg = make_package("python");
        let pv = PackageVariant::new(pkg, 0);
        cache.cache_variants("python".to_string(), vec![pv]);

        let variants = cache.get_variants("python");
        assert!(variants.is_some());
        assert_eq!(variants.unwrap().len(), 1);
        assert_eq!(cache.hits, 1);
    }

    #[test]
    fn test_package_variant_cache_invalidate() {
        let mut cache = PackageVariantCache::new();
        let pkg = make_package("python");
        let pv = PackageVariant::new(pkg, 0);
        cache.cache_variants("python".to_string(), vec![pv]);

        cache.invalidate("python");
        assert!(cache.get_variants("python").is_none());
        assert_eq!(cache.misses, 1);
    }

    #[test]
    fn test_package_variant_cache_clear() {
        let mut cache = PackageVariantCache::new();
        let pkg = make_package("python");
        let pv = PackageVariant::new(pkg, 0);
        cache.cache_variants("python".to_string(), vec![pv]);

        cache.clear();
        assert!(cache.variants.is_empty());
        assert_eq!(cache.hits, 0);
        assert_eq!(cache.misses, 0);
    }
}
