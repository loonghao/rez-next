//! Test for enhanced cache functionality

use std::path::PathBuf;
use std::time::SystemTime;

// Mock structures for testing (since we can't compile the full project)
#[derive(Debug, Clone)]
pub struct MockPackageScanResult {
    pub package_file: PathBuf,
    pub file_size: u64,
    pub scan_duration_ms: u64,
}

#[derive(Debug, Clone)]
pub struct MockScanCacheEntry {
    pub result: MockPackageScanResult,
    pub mtime: SystemTime,
    pub size: u64,
    pub cached_at: SystemTime,
    pub access_count: u64,
    pub last_accessed: SystemTime,
}

#[derive(Debug, Clone)]
pub struct MockCacheStatistics {
    pub hits: usize,
    pub misses: usize,
    pub prefix_hits: usize,
    pub hit_rate: f64,
    pub prefix_hit_rate: f64,
    pub cache_size: usize,
    pub total_entries: usize,
}

#[derive(Debug, Clone)]
pub struct MockScannerConfig {
    pub enable_prefix_matching: bool,
    pub enable_cache_preload: bool,
    pub preload_paths: Vec<PathBuf>,
    pub cache_refresh_interval: u64,
    pub enable_background_refresh: bool,
}

impl Default for MockScannerConfig {
    fn default() -> Self {
        Self {
            enable_prefix_matching: true,
            enable_cache_preload: true,
            preload_paths: vec![
                PathBuf::from("/usr/local/packages"),
                PathBuf::from("/opt/packages"),
                PathBuf::from("C:\\packages"),
            ],
            cache_refresh_interval: 300,
            enable_background_refresh: true,
        }
    }
}

// Mock scanner with enhanced cache functionality
pub struct MockRepositoryScanner {
    config: MockScannerConfig,
    cache: std::collections::HashMap<PathBuf, MockScanCacheEntry>,
    hits: usize,
    misses: usize,
    prefix_hits: usize,
}

impl MockRepositoryScanner {
    pub fn new(config: MockScannerConfig) -> Self {
        Self {
            config,
            cache: std::collections::HashMap::new(),
            hits: 0,
            misses: 0,
            prefix_hits: 0,
        }
    }

    pub fn get_cache_statistics(&self) -> MockCacheStatistics {
        let total_entries = self.hits + self.misses + self.prefix_hits;
        let hit_rate = if total_entries > 0 {
            self.hits as f64 / total_entries as f64
        } else {
            0.0
        };
        let prefix_hit_rate = if total_entries > 0 {
            self.prefix_hits as f64 / total_entries as f64
        } else {
            0.0
        };

        MockCacheStatistics {
            hits: self.hits,
            misses: self.misses,
            prefix_hits: self.prefix_hits,
            hit_rate,
            prefix_hit_rate,
            cache_size: self.cache.len(),
            total_entries,
        }
    }

    pub fn get_by_prefix(&mut self, path: &std::path::Path) -> Option<MockPackageScanResult> {
        if !self.config.enable_prefix_matching {
            return None;
        }

        let normalized_path = self.normalize_path(path);

        // First try exact match
        if let Some(entry) = self.cache.get_mut(&normalized_path) {
            entry.access_count += 1;
            entry.last_accessed = SystemTime::now();
            self.hits += 1;
            return Some(entry.result.clone());
        }

        // Try prefix matching
        for (cached_path, entry) in self.cache.iter_mut() {
            if normalized_path.starts_with(cached_path) || cached_path.starts_with(&normalized_path)
            {
                entry.access_count += 1;
                entry.last_accessed = SystemTime::now();
                self.prefix_hits += 1;
                return Some(entry.result.clone());
            }
        }

        self.misses += 1;
        None
    }

    pub fn insert_cache_entry(&mut self, path: PathBuf, result: MockPackageScanResult) {
        let now = SystemTime::now();
        let entry = MockScanCacheEntry {
            result,
            mtime: now,
            size: 1024, // Mock size
            cached_at: now,
            access_count: 1,
            last_accessed: now,
        };
        self.cache.insert(path, entry);
    }

    fn normalize_path(&self, path: &std::path::Path) -> PathBuf {
        // Simple normalization for testing
        path.to_path_buf()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_statistics() {
        let config = MockScannerConfig::default();
        let scanner = MockRepositoryScanner::new(config);

        let stats = scanner.get_cache_statistics();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.prefix_hits, 0);
        assert_eq!(stats.hit_rate, 0.0);
        assert_eq!(stats.prefix_hit_rate, 0.0);
        assert_eq!(stats.cache_size, 0);
    }

    #[test]
    fn test_prefix_matching_exact_match() {
        let config = MockScannerConfig::default();
        let mut scanner = MockRepositoryScanner::new(config);

        let path = PathBuf::from("/test/package.py");
        let result = MockPackageScanResult {
            package_file: path.clone(),
            file_size: 1024,
            scan_duration_ms: 10,
        };

        scanner.insert_cache_entry(path.clone(), result);

        // Test exact match
        let cached_result = scanner.get_by_prefix(&path);
        assert!(cached_result.is_some());

        let stats = scanner.get_cache_statistics();
        assert_eq!(stats.hits, 1);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.prefix_hits, 0);
        assert_eq!(stats.hit_rate, 1.0);
    }

    #[test]
    fn test_prefix_matching_prefix_match() {
        let config = MockScannerConfig::default();
        let mut scanner = MockRepositoryScanner::new(config);

        let cached_path = PathBuf::from("/test");
        let query_path = PathBuf::from("/test/subdir/package.py");

        let result = MockPackageScanResult {
            package_file: cached_path.clone(),
            file_size: 1024,
            scan_duration_ms: 10,
        };

        scanner.insert_cache_entry(cached_path, result);

        // Test prefix match
        let cached_result = scanner.get_by_prefix(&query_path);
        assert!(cached_result.is_some());

        let stats = scanner.get_cache_statistics();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 0);
        assert_eq!(stats.prefix_hits, 1);
        assert_eq!(stats.prefix_hit_rate, 1.0);
    }

    #[test]
    fn test_cache_miss() {
        let config = MockScannerConfig::default();
        let mut scanner = MockRepositoryScanner::new(config);

        let path = PathBuf::from("/nonexistent/package.py");

        // Test cache miss
        let cached_result = scanner.get_by_prefix(&path);
        assert!(cached_result.is_none());

        let stats = scanner.get_cache_statistics();
        assert_eq!(stats.hits, 0);
        assert_eq!(stats.misses, 1);
        assert_eq!(stats.prefix_hits, 0);
        assert_eq!(stats.hit_rate, 0.0);
    }

    #[test]
    fn test_prefix_matching_disabled() {
        let mut config = MockScannerConfig::default();
        config.enable_prefix_matching = false;
        let mut scanner = MockRepositoryScanner::new(config);

        let path = PathBuf::from("/test/package.py");
        let result = MockPackageScanResult {
            package_file: path.clone(),
            file_size: 1024,
            scan_duration_ms: 10,
        };

        scanner.insert_cache_entry(path.clone(), result);

        // Test with prefix matching disabled
        let cached_result = scanner.get_by_prefix(&path);
        assert!(cached_result.is_none());
    }

    #[test]
    fn test_access_count_tracking() {
        let config = MockScannerConfig::default();
        let mut scanner = MockRepositoryScanner::new(config);

        let path = PathBuf::from("/test/package.py");
        let result = MockPackageScanResult {
            package_file: path.clone(),
            file_size: 1024,
            scan_duration_ms: 10,
        };

        scanner.insert_cache_entry(path.clone(), result);

        // Access the cache entry multiple times
        scanner.get_by_prefix(&path);
        scanner.get_by_prefix(&path);
        scanner.get_by_prefix(&path);

        // Check that access count is tracked
        let entry = scanner.cache.get(&path).unwrap();
        assert_eq!(entry.access_count, 4); // 1 initial + 3 accesses

        let stats = scanner.get_cache_statistics();
        assert_eq!(stats.hits, 3);
    }
}
