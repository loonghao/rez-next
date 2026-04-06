//! # Rez Core Repository
//!
//! Repository scanning, caching, and management for Rez Core.
//!
//! This crate provides:
//! - Repository scanning and indexing
//! - Package discovery and caching
//! - Repository metadata management
//! - Async repository operations

pub mod cache;
pub mod filesystem;
pub mod high_performance_scanner;
pub mod repository;
pub mod scanner;
pub mod scanner_types;
pub mod simple_repository;

pub use cache::*;
pub use filesystem::*;
pub use high_performance_scanner::*;
pub use repository::{
    deduplicate_packages, PackageSearchCriteria, Repository, RepositoryMetadata, RepositoryStats,
    RepositoryType,
};
pub use scanner::*;
pub use scanner_types::{
    CacheStatistics, PackageScanResult, ScanError, ScanErrorType, ScanPerformanceMetrics,
    ScanResult, ScannerConfig, REZ_PACKAGE_FILENAMES,
};
pub use simple_repository::*;
