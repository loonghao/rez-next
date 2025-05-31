//! Intelligent caching system for rez-core
//!
//! This crate provides a unified, intelligent caching system that integrates
//! with existing rez-core caches (SolverCache, RepositoryCache, RexCache).
//! It features multi-level caching, predictive preheating, and adaptive tuning
//! to achieve >90% cache hit rates.
//!
//! # Features
//!
//! - **Unified Cache Interface**: Common trait for all cache types
//! - **Multi-level Caching**: L1 memory cache + L2 disk cache
//! - **Predictive Preheating**: ML-based access pattern prediction
//! - **Adaptive Tuning**: Real-time parameter optimization
//! - **Performance Monitoring**: Comprehensive statistics and metrics
//!
//! # Architecture
//!
//! The caching system is built on top of existing rez-core cache implementations,
//! reusing proven patterns and components while adding intelligent features.
//!
//! ```text
//! ┌─────────────────────────────────────────────────────────────┐
//! │                IntelligentCacheManager                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  L1 Cache (DashMap)  │  L2 Cache (RepositoryCache)        │
//! ├─────────────────────────────────────────────────────────────┤
//! │  PredictivePreheater │  AdaptiveTuner                     │
//! ├─────────────────────────────────────────────────────────────┤
//! │  SolverCache  │  RepositoryCache  │  RexCache             │
//! └─────────────────────────────────────────────────────────────┘
//! ```

pub mod unified_cache;
pub mod cache_config;
pub mod cache_stats;
pub mod error;
pub mod intelligent_manager;
pub mod predictive_preheater;
pub mod adaptive_tuner;
pub mod performance_monitor;
pub mod benchmarks;

#[cfg(test)]
mod tests;

// Re-export core types
pub use unified_cache::*;
pub use cache_config::*;
pub use cache_stats::*;
pub use error::*;
pub use intelligent_manager::*;
pub use predictive_preheater::*;
pub use adaptive_tuner::*;
pub use performance_monitor::*;
pub use benchmarks::*;

// Re-export existing cache components for compatibility
// Temporarily disabled due to compilation errors in other crates
// pub use rez_core_solver::cache::{EvictionStrategy, SolverCache, SolverCacheConfig, SolverCacheStats};
// pub use rez_core_repository::cache::{RepositoryCache, CacheConfig as RepositoryCacheConfig, CacheStats as RepositoryCacheStats};
// pub use rez_core_rex::cache::{RexCache, RexCacheConfig, RexCacheStats};

/// Cache eviction strategies (copied from SolverCache for now)
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub enum EvictionStrategy {
    /// Least Recently Used
    LRU,
    /// Least Frequently Used
    LFU,
    /// First In, First Out
    FIFO,
    /// Time-based expiration only
    TTL,
}



/// Version information for the cache system
pub const CACHE_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Default cache configuration
pub const DEFAULT_L1_CAPACITY: usize = 10000;
pub const DEFAULT_L2_CAPACITY: usize = 100000;
pub const DEFAULT_TTL_SECONDS: u64 = 3600;
pub const DEFAULT_MEMORY_LIMIT_MB: u64 = 100;


