//! Package ordering module for Rez.
//!
//! This module provides version ordering strategies for packages,
//! allowing users to customize how package versions are sorted.

use std::collections::HashMap;

use crate::package::Package;
use rez_next_version::Version;
use std::fmt;

/// Trait for package ordering strategies.
///
/// Implement this trait to create custom package ordering strategies.
pub trait PackageOrder: Send + Sync + fmt::Debug {
    /// Returns the name of this ordering strategy.
    fn name(&self) -> &str;

    /// Returns the list of package families this orderer applies to.
    /// Returns None if it applies to all packages.
    fn packages(&self) -> Option<&[String]>;

    /// Reorder a list of packages.
    ///
    /// Returns None if this orderer doesn't handle the packages,
    /// or Some(reordered_list) if it does.
    fn reorder(&self, packages: &[Package]) -> Option<Vec<Package>>;

    /// Convert to plain old data for serialization.
    fn to_pod(&self) -> serde_json::Value;

    /// Clone this orderer as a `Box<dyn PackageOrder>`.
    fn clone_box(&self) -> Box<dyn PackageOrder>;

    /// Get SHA1 hash of this orderer for caching.
    fn sha1(&self) -> String {
        use sha1::{Digest, Sha1};
        let mut hasher = Sha1::new();
        let pod_str = self.to_pod().to_string();
        hasher.update(pod_str.as_bytes());
        hasher
            .finalize()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<Vec<_>>()
            .join("")
    }
}

// Implement Clone for Box<dyn PackageOrder>
impl Clone for Box<dyn PackageOrder> {
    fn clone(&self) -> Box<dyn PackageOrder> {
        self.clone_box()
    }
}

/// Null order - no reordering.
#[derive(Debug, Clone)]
pub struct NullPackageOrder {
    pub packages: Option<Vec<String>>,
}

impl NullPackageOrder {
    pub fn new(packages: Option<Vec<String>>) -> Self {
        Self { packages }
    }
}

impl PackageOrder for NullPackageOrder {
    fn name(&self) -> &str {
        "no_order"
    }

    fn packages(&self) -> Option<&[String]> {
        self.packages.as_deref()
    }

    fn reorder(&self, _packages: &[Package]) -> Option<Vec<Package>> {
        // Null order doesn't reorder - returns None to let others handle
        None
    }

    fn to_pod(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "no_order",
            "packages": self.packages.clone()
        })
    }

    fn clone_box(&self) -> Box<dyn PackageOrder> {
        Box::new(self.clone())
    }
}

/// Sorted order - order by version.
#[derive(Debug, Clone)]
pub struct SortedOrder {
    pub descending: bool,
    pub packages: Option<Vec<String>>,
}

impl SortedOrder {
    pub fn new(descending: bool, packages: Option<Vec<String>>) -> Self {
        Self {
            descending,
            packages,
        }
    }
}

impl PackageOrder for SortedOrder {
    fn name(&self) -> &str {
        "sorted"
    }

    fn packages(&self) -> Option<&[String]> {
        self.packages.as_deref()
    }

    fn reorder(&self, packages: &[Package]) -> Option<Vec<Package>> {
        let mut result: Vec<Package> = packages.to_vec();
        if self.descending {
            result.sort_by(|a, b| {
                let a_ver = a.version.as_ref();
                let b_ver = b.version.as_ref();
                // Handle None versions
                match (a_ver, b_ver) {
                    (Some(av), Some(bv)) => bv.cmp(av),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        } else {
            result.sort_by(|a, b| {
                let a_ver = a.version.as_ref();
                let b_ver = b.version.as_ref();
                match (a_ver, b_ver) {
                    (Some(av), Some(bv)) => av.cmp(bv),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                }
            });
        }
        Some(result)
    }

    fn to_pod(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "sorted",
            "descending": self.descending,
            "packages": self.packages.clone()
        })
    }

    fn clone_box(&self) -> Box<dyn PackageOrder> {
        Box::new(self.clone())
    }
}

/// Version split order - packages with version <= split point come first.
#[derive(Debug, Clone)]
pub struct VersionSplitPackageOrder {
    pub first_version: Version,
    pub packages: Option<Vec<String>>,
}

impl VersionSplitPackageOrder {
    pub fn new(first_version: Version, packages: Option<Vec<String>>) -> Self {
        Self {
            first_version,
            packages,
        }
    }
}

impl PackageOrder for VersionSplitPackageOrder {
    fn name(&self) -> &str {
        "version_split"
    }

    fn packages(&self) -> Option<&[String]> {
        self.packages.as_deref()
    }

    fn reorder(&self, packages: &[Package]) -> Option<Vec<Package>> {
        let mut before = Vec::new();
        let mut after = Vec::new();

        for pkg in packages {
            if let Some(ref ver) = pkg.version {
                if *ver <= self.first_version {
                    before.push(pkg.clone());
                } else {
                    after.push(pkg.clone());
                }
            } else {
                after.push(pkg.clone());
            }
        }

        before.sort_by(|a, b| a.version.as_ref().unwrap().cmp(b.version.as_ref().unwrap()));
        after.sort_by(|a, b| a.version.as_ref().unwrap().cmp(b.version.as_ref().unwrap()));

        let mut result = before;
        result.extend(after);
        Some(result)
    }

    fn to_pod(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "version_split",
            "first_version": self.first_version.to_string(),
            "packages": self.packages.clone()
        })
    }

    fn clone_box(&self) -> Box<dyn PackageOrder> {
        Box::new(self.clone())
    }
}

/// Per-family order - different orders for different package families.
#[derive(Debug, Clone)]
pub struct PerFamilyOrder {
    pub order_dict: HashMap<String, Box<dyn PackageOrder>>,
    pub default_order: Option<Box<dyn PackageOrder>>,
}

impl PerFamilyOrder {
    pub fn new(
        order_dict: HashMap<String, Box<dyn PackageOrder>>,
        default_order: Option<Box<dyn PackageOrder>>,
    ) -> Self {
        Self {
            order_dict,
            default_order,
        }
    }
}

impl PackageOrder for PerFamilyOrder {
    fn name(&self) -> &str {
        "per_family"
    }

    fn packages(&self) -> Option<&[String]> {
        // PerFamilyOrder applies to all packages in the dict
        None
    }

    fn reorder(&self, packages: &[Package]) -> Option<Vec<Package>> {
        // Group by family
        let mut grouped: HashMap<String, Vec<Package>> = HashMap::new();
        for pkg in packages {
            grouped
                .entry(pkg.name.clone())
                .or_default()
                .push(pkg.clone());
        }

        let mut result = Vec::new();
        for (family, mut pkgs) in grouped {
            // Get orderer for this family
            let orderer: Option<&Box<dyn PackageOrder>> =
                self.order_dict.get(&family).or(self.default_order.as_ref());

            if let Some(o) = orderer {
                if let Some(reordered) = o.reorder(&pkgs) {
                    result.extend(reordered);
                } else {
                    result.append(&mut pkgs);
                }
            } else {
                result.append(&mut pkgs);
            }
        }

        Some(result)
    }

    fn to_pod(&self) -> serde_json::Value {
        let mut dict = HashMap::new();
        for (k, v) in &self.order_dict {
            dict.insert(k.clone(), v.to_pod());
        }

        serde_json::json!({
            "type": "per_family",
            "order_dict": dict,
            "default_order": self.default_order.as_ref().map(|o| o.to_pod())
        })
    }

    fn clone_box(&self) -> Box<dyn PackageOrder> {
        let mut new_dict = HashMap::new();
        for (k, v) in &self.order_dict {
            new_dict.insert(k.clone(), v.clone_box());
        }
        Box::new(PerFamilyOrder {
            order_dict: new_dict,
            default_order: self.default_order.as_ref().map(|o| o.clone_box()),
        })
    }
}

/// Timestamp-based package order.
#[derive(Debug, Clone)]
pub struct TimestampPackageOrder {
    pub timestamp: i64,
    pub rank: i32,
    pub packages: Option<Vec<String>>,
}

impl TimestampPackageOrder {
    pub fn new(timestamp: i64, rank: i32, packages: Option<Vec<String>>) -> Self {
        Self {
            timestamp,
            rank,
            packages,
        }
    }
}

impl PackageOrder for TimestampPackageOrder {
    fn name(&self) -> &str {
        "soft_timestamp"
    }

    fn packages(&self) -> Option<&[String]> {
        self.packages.as_deref()
    }

    fn reorder(&self, packages: &[Package]) -> Option<Vec<Package>> {
        // Sort by timestamp proximity
        let mut result: Vec<Package> = packages.to_vec();
        result.sort_by(|a, b| {
            let a_time = a.timestamp.unwrap_or(0);
            let b_time = b.timestamp.unwrap_or(0);

            // Packages before timestamp come first, sorted by proximity
            let a_before = a_time <= self.timestamp;
            let b_before = b_time <= self.timestamp;

            match (a_before, b_before) {
                (true, false) => std::cmp::Ordering::Less,
                (false, true) => std::cmp::Ordering::Greater,
                _ => {
                    // Both before or both after - sort by time descending
                    b_time.cmp(&a_time)
                }
            }
        });
        Some(result)
    }

    fn to_pod(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "soft_timestamp",
            "timestamp": self.timestamp,
            "rank": self.rank,
            "packages": self.packages.clone()
        })
    }

    fn clone_box(&self) -> Box<dyn PackageOrder> {
        Box::new(self.clone())
    }
}

/// List of package orderers.
#[derive(Debug, Clone)]
pub struct PackageOrderList {
    pub orderers: Vec<Box<dyn PackageOrder>>,
}

impl PackageOrderList {
    pub fn new() -> Self {
        Self {
            orderers: Vec::new(),
        }
    }

    pub fn from_pod(data: &[serde_json::Value]) -> Self {
        let mut list = Self::new();
        for item in data {
            if let Some(orderer) = from_pod(item) {
                list.orderers.push(orderer);
            }
        }
        list
    }

    pub fn to_pod(&self) -> Vec<serde_json::Value> {
        self.orderers.iter().map(|o| o.to_pod()).collect()
    }

    pub fn get(&self, package_name: &str) -> Option<&dyn PackageOrder> {
        for orderer in &self.orderers {
            if let Some(pkgs) = orderer.packages() {
                if pkgs.contains(&package_name.to_string()) {
                    return Some(&**orderer);
                }
            } else {
                // None means applies to all
                return Some(&**orderer);
            }
        }
        None
    }

    pub fn append(&mut self, orderer: Box<dyn PackageOrder>) {
        self.orderers.push(orderer);
    }
}

impl Default for PackageOrderList {
    fn default() -> Self {
        Self::new()
    }
}

/// Convert a PackageOrder to plain old data.
pub fn to_pod(orderer: &dyn PackageOrder) -> serde_json::Value {
    orderer.to_pod()
}

/// Create a PackageOrder from plain old data.
pub fn from_pod(data: &serde_json::Value) -> Option<Box<dyn PackageOrder>> {
    let type_name = data.get("type")?.as_str()?;

    match type_name {
        "no_order" => {
            let packages = data.get("packages").and_then(|p| p.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(Box::new(NullPackageOrder::new(packages)))
        }
        "sorted" => {
            let descending = data.get("descending")?.as_bool()?;
            let packages = data.get("packages").and_then(|p| p.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(Box::new(SortedOrder::new(descending, packages)))
        }
        "version_split" => {
            let version_str = data.get("first_version")?.as_str()?;
            let first_version = Version::parse(version_str).ok()?;
            let packages = data.get("packages").and_then(|p| p.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(Box::new(VersionSplitPackageOrder::new(
                first_version,
                packages,
            )))
        }
        "soft_timestamp" => {
            let timestamp = data.get("timestamp")?.as_i64()?;
            let rank = data.get("rank").and_then(|r| r.as_i64()).unwrap_or(0) as i32;
            let packages = data.get("packages").and_then(|p| p.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            });

            Some(Box::new(TimestampPackageOrder::new(
                timestamp, rank, packages,
            )))
        }
        _ => None,
    }
}

/// Get the orderer for a package.
pub fn get_orderer(
    package_name: &str,
    orderers: Option<&PackageOrderList>,
) -> Box<dyn PackageOrder> {
    if let Some(list) = orderers
        && let Some(orderer) = list.get(package_name)
    {
        return orderer.clone_box();
    }

    // Default: no ordering
    Box::new(NullPackageOrder::new(None))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rez_next_version::Version;

    #[test]
    fn test_null_order() {
        let order = NullPackageOrder::new(None);
        assert_eq!(order.name(), "no_order");
        assert!(order.reorder(&[]).is_none());
    }

    #[test]
    fn test_sorted_order_ascending() {
        let order = SortedOrder::new(false, None);
        let pkg1 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            ..Default::default()
        };
        let pkg2 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("2.0.0").unwrap()),
            ..Default::default()
        };

        let result = order.reorder(&[pkg2.clone(), pkg1.clone()]).unwrap();
        assert_eq!(
            result[0].version.as_ref().unwrap(),
            &Version::parse("1.0.0").unwrap()
        );
        assert_eq!(
            result[1].version.as_ref().unwrap(),
            &Version::parse("2.0.0").unwrap()
        );
    }

    #[test]
    fn test_sorted_order_descending() {
        let order = SortedOrder::new(true, None);
        let pkg1 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            ..Default::default()
        };
        let pkg2 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("2.0.0").unwrap()),
            ..Default::default()
        };

        let result = order.reorder(&[pkg1.clone(), pkg2.clone()]).unwrap();
        assert_eq!(
            result[0].version.as_ref().unwrap(),
            &Version::parse("2.0.0").unwrap()
        );
        assert_eq!(
            result[1].version.as_ref().unwrap(),
            &Version::parse("1.0.0").unwrap()
        );
    }

    #[test]
    fn test_version_split_order() {
        let split_version = Version::parse("2.0.0").unwrap();
        let order = VersionSplitPackageOrder::new(split_version.clone(), None);

        let pkg1 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            ..Default::default()
        };
        let pkg2 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("2.0.0").unwrap()),
            ..Default::default()
        };
        let pkg3 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("3.0.0").unwrap()),
            ..Default::default()
        };

        let result = order
            .reorder(&[pkg3.clone(), pkg1.clone(), pkg2.clone()])
            .unwrap();
        // pkg1 and pkg2 should come before pkg3
        assert!(result[0].version.as_ref().unwrap() <= &split_version);
        assert!(result[1].version.as_ref().unwrap() <= &split_version);
        assert!(result[2].version.as_ref().unwrap() > &split_version);
    }

    #[test]
    fn test_per_family_order() {
        let mut order_dict = HashMap::new();
        order_dict.insert(
            "foo".to_string(),
            Box::new(SortedOrder::new(true, None)) as Box<dyn PackageOrder>,
        );

        let order = PerFamilyOrder::new(order_dict, None);

        let foo_pkg1 = Package {
            name: "foo".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            ..Default::default()
        };
        let foo_pkg2 = Package {
            name: "foo".to_string(),
            version: Some(Version::parse("2.0.0").unwrap()),
            ..Default::default()
        };

        assert_eq!(order.name(), "per_family");
        // Test that PerFamilyOrder can reorder packages
        let result = order.reorder(&[foo_pkg2.clone(), foo_pkg1.clone()]);
        assert!(result.is_some());
        // Descending order: 2.0.0 should come before 1.0.0
        let sorted = result.unwrap();
        assert_eq!(
            sorted[0].version.as_ref().unwrap(),
            &Version::parse("2.0.0").unwrap()
        );
    }

    #[test]
    fn test_timestamp_order() {
        let order = TimestampPackageOrder::new(1000, 0, None);

        let pkg1 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("1.0.0").unwrap()),
            timestamp: Some(500),
            ..Default::default()
        };
        let pkg2 = Package {
            name: "test".to_string(),
            version: Some(Version::parse("2.0.0").unwrap()),
            timestamp: Some(1500),
            ..Default::default()
        };

        let result = order.reorder(&[pkg2.clone(), pkg1.clone()]).unwrap();
        // pkg1 (timestamp 500 <= 1000) should come before pkg2 (timestamp 1500 > 1000)
        assert!(result[0].timestamp.unwrap() <= 1000);
        assert!(result[1].timestamp.unwrap() > 1000);
    }

    #[test]
    fn test_package_order_list() {
        let mut list = PackageOrderList::new();
        list.append(Box::new(SortedOrder::new(
            false,
            Some(vec!["foo".to_string()]),
        )));

        assert!(list.get("foo").is_some());
        assert!(list.get("bar").is_none());
    }

    #[test]
    fn test_to_pod_and_from_pod() {
        let order = SortedOrder::new(true, Some(vec!["foo".to_string()]));
        let pod = to_pod(&order);
        assert_eq!(pod.get("type").unwrap().as_str().unwrap(), "sorted");
        assert!(pod.get("descending").unwrap().as_bool().unwrap());

        let restored = from_pod(&pod).unwrap();
        assert_eq!(restored.name(), "sorted");
    }
}
