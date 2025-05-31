//! Rex command caching system
//!
//! This module provides high-performance caching for Rex command parsing and execution results.
//! It reuses the proven caching architecture from SolverCache and RepositoryCache.

use crate::{RexCommand, ExecutionResult};
use rez_core_common::RezCoreError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

/// Cache entry for Rex command parsing results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexParseCacheEntry {
    /// Cached parsed command
    pub command: RexCommand,
    /// Cache timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_access: u64,
}

impl RexParseCacheEntry {
    /// Create a new parse cache entry
    pub fn new(command: RexCommand, ttl: u64) -> Result<Self, RezCoreError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RezCoreError::CacheError(format!("Failed to get current time: {}", e)))?
            .as_secs();

        Ok(Self {
            command,
            timestamp: now,
            ttl,
            access_count: 0,
            last_access: now,
        })
    }

    /// Check if the cache entry is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now <= self.timestamp + self.ttl
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Cache entry for Rex command execution results
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexExecutionCacheEntry {
    /// Cached execution result
    pub result: ExecutionResult,
    /// Cache timestamp (Unix timestamp)
    pub timestamp: u64,
    /// Time-to-live in seconds
    pub ttl: u64,
    /// Access count
    pub access_count: u64,
    /// Last access time
    pub last_access: u64,
}

impl RexExecutionCacheEntry {
    /// Create a new execution cache entry
    pub fn new(result: ExecutionResult, ttl: u64) -> Result<Self, RezCoreError> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| RezCoreError::CacheError(format!("Failed to get current time: {}", e)))?
            .as_secs();

        Ok(Self {
            result,
            timestamp: now,
            ttl,
            access_count: 0,
            last_access: now,
        })
    }

    /// Check if the cache entry is still valid
    pub fn is_valid(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        now <= self.timestamp + self.ttl
    }

    /// Mark the entry as accessed
    pub fn mark_accessed(&mut self) {
        self.access_count += 1;
        self.last_access = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
}

/// Rex cache configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexCacheConfig {
    /// Maximum number of parse cache entries
    pub max_parse_entries: usize,
    /// Maximum number of execution cache entries
    pub max_execution_entries: usize,
    /// Default TTL for parse cache entries (in seconds)
    pub parse_ttl: u64,
    /// Default TTL for execution cache entries (in seconds)
    pub execution_ttl: u64,
    /// Maximum memory usage in bytes
    pub max_memory_bytes: u64,
    /// Cache eviction strategy
    pub eviction_strategy: EvictionStrategy,
    /// Enable cache persistence
    pub enable_persistence: bool,
    /// Cache file path (if persistence is enabled)
    pub cache_file_path: Option<String>,
}

impl Default for RexCacheConfig {
    fn default() -> Self {
        Self {
            max_parse_entries: 5000,     // Parse results are small, can cache more
            max_execution_entries: 1000, // Execution results are larger
            parse_ttl: 7200,             // 2 hours for parse results
            execution_ttl: 1800,         // 30 minutes for execution results
            max_memory_bytes: 20 * 1024 * 1024, // 20 MB
            eviction_strategy: EvictionStrategy::LRU,
            enable_persistence: false,
            cache_file_path: None,
        }
    }
}

/// Cache eviction strategies (reused from SolverCache)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
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

/// Rex cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RexCacheStats {
    /// Parse cache hits
    pub parse_hits: u64,
    /// Parse cache misses
    pub parse_misses: u64,
    /// Execution cache hits
    pub execution_hits: u64,
    /// Execution cache misses
    pub execution_misses: u64,
    /// Total parse cache entries
    pub parse_entries: usize,
    /// Total execution cache entries
    pub execution_entries: usize,
    /// Estimated memory usage in bytes
    pub memory_usage_bytes: u64,
    /// Cache evictions
    pub evictions: u64,
    /// Parse cache hit rate (0.0 to 1.0)
    pub parse_hit_rate: f64,
    /// Execution cache hit rate (0.0 to 1.0)
    pub execution_hit_rate: f64,
}

impl Default for RexCacheStats {
    fn default() -> Self {
        Self {
            parse_hits: 0,
            parse_misses: 0,
            execution_hits: 0,
            execution_misses: 0,
            parse_entries: 0,
            execution_entries: 0,
            memory_usage_bytes: 0,
            evictions: 0,
            parse_hit_rate: 0.0,
            execution_hit_rate: 0.0,
        }
    }
}

/// High-performance Rex cache
#[derive(Debug)]
pub struct RexCache {
    /// Cache configuration
    config: RexCacheConfig,
    /// Parse cache entries
    parse_cache: Arc<RwLock<HashMap<String, RexParseCacheEntry>>>,
    /// Execution cache entries
    execution_cache: Arc<RwLock<HashMap<String, RexExecutionCacheEntry>>>,
    /// Access order for LRU eviction (parse cache)
    parse_access_order: Arc<RwLock<Vec<String>>>,
    /// Access order for LRU eviction (execution cache)
    execution_access_order: Arc<RwLock<Vec<String>>>,
    /// Cache statistics
    stats: Arc<RwLock<RexCacheStats>>,
}

impl RexCache {
    /// Create a new Rex cache
    pub fn new() -> Self {
        Self::with_config(RexCacheConfig::default())
    }

    /// Create a new Rex cache with custom configuration
    pub fn with_config(config: RexCacheConfig) -> Self {
        Self {
            config,
            parse_cache: Arc::new(RwLock::new(HashMap::new())),
            execution_cache: Arc::new(RwLock::new(HashMap::new())),
            parse_access_order: Arc::new(RwLock::new(Vec::new())),
            execution_access_order: Arc::new(RwLock::new(Vec::new())),
            stats: Arc::new(RwLock::new(RexCacheStats::default())),
        }
    }

    /// Get a cached parsed command
    pub async fn get_parsed_command(&self, line: &str) -> Option<RexCommand> {
        let mut cache = self.parse_cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(line) {
            if entry.is_valid() {
                entry.mark_accessed();
                stats.parse_hits += 1;
                self.update_parse_access_order(line).await;
                self.update_parse_hit_rate(&mut stats);
                return Some(entry.command.clone());
            } else {
                // Remove expired entry
                cache.remove(line);
                self.remove_from_parse_access_order(line).await;
            }
        }

        stats.parse_misses += 1;
        self.update_parse_hit_rate(&mut stats);
        None
    }

    /// Put a parsed command into the cache
    pub async fn put_parsed_command(&self, line: String, command: RexCommand) {
        let entry = match RexParseCacheEntry::new(command, self.config.parse_ttl) {
            Ok(entry) => entry,
            Err(_) => return, // Failed to create entry
        };

        {
            let mut cache = self.parse_cache.write().await;
            
            // Check if we need to evict entries
            if cache.len() >= self.config.max_parse_entries {
                self.evict_parse_entries(&mut cache).await;
            }

            cache.insert(line.clone(), entry);
        }

        // Update access order
        self.update_parse_access_order(&line).await;

        // Update statistics
        self.update_cache_stats().await;
    }

    /// Get a cached execution result
    pub async fn get_execution_result(&self, key: &str) -> Option<ExecutionResult> {
        let mut cache = self.execution_cache.write().await;
        let mut stats = self.stats.write().await;

        if let Some(entry) = cache.get_mut(key) {
            if entry.is_valid() {
                entry.mark_accessed();
                stats.execution_hits += 1;
                self.update_execution_access_order(key).await;
                self.update_execution_hit_rate(&mut stats);
                return Some(entry.result.clone());
            } else {
                // Remove expired entry
                cache.remove(key);
                self.remove_from_execution_access_order(key).await;
            }
        }

        stats.execution_misses += 1;
        self.update_execution_hit_rate(&mut stats);
        None
    }

    /// Put an execution result into the cache
    pub async fn put_execution_result(&self, key: String, result: ExecutionResult) {
        let entry = match RexExecutionCacheEntry::new(result, self.config.execution_ttl) {
            Ok(entry) => entry,
            Err(_) => return, // Failed to create entry
        };

        {
            let mut cache = self.execution_cache.write().await;

            // Check if we need to evict entries
            if cache.len() >= self.config.max_execution_entries {
                self.evict_execution_entries(&mut cache).await;
            }

            cache.insert(key.clone(), entry);
        }

        // Update access order
        self.update_execution_access_order(&key).await;

        // Update statistics
        self.update_cache_stats().await;
    }

    /// Remove a parsed command from cache
    pub async fn remove_parsed_command(&self, line: &str) -> bool {
        let mut cache = self.parse_cache.write().await;
        let removed = cache.remove(line).is_some();

        if removed {
            self.remove_from_parse_access_order(line).await;
            self.update_cache_stats().await;
        }

        removed
    }

    /// Remove an execution result from cache
    pub async fn remove_execution_result(&self, key: &str) -> bool {
        let mut cache = self.execution_cache.write().await;
        let removed = cache.remove(key).is_some();

        if removed {
            self.remove_from_execution_access_order(key).await;
            self.update_cache_stats().await;
        }

        removed
    }

    /// Clear all cache entries
    pub async fn clear(&self) {
        {
            let mut parse_cache = self.parse_cache.write().await;
            parse_cache.clear();
        }

        {
            let mut execution_cache = self.execution_cache.write().await;
            execution_cache.clear();
        }

        {
            let mut parse_access_order = self.parse_access_order.write().await;
            parse_access_order.clear();
        }

        {
            let mut execution_access_order = self.execution_access_order.write().await;
            execution_access_order.clear();
        }

        {
            let mut stats = self.stats.write().await;
            *stats = RexCacheStats::default();
        }
    }

    /// Get cache statistics
    pub async fn get_stats(&self) -> RexCacheStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Cleanup expired entries
    pub async fn cleanup(&self) {
        // Cleanup parse cache
        {
            let mut cache = self.parse_cache.write().await;
            let mut expired_keys = Vec::new();

            for (key, entry) in cache.iter() {
                if !entry.is_valid() {
                    expired_keys.push(key.clone());
                }
            }

            for key in &expired_keys {
                cache.remove(key);
                self.remove_from_parse_access_order(key).await;
            }
        }

        // Cleanup execution cache
        {
            let mut cache = self.execution_cache.write().await;
            let mut expired_keys = Vec::new();

            for (key, entry) in cache.iter() {
                if !entry.is_valid() {
                    expired_keys.push(key.clone());
                }
            }

            for key in &expired_keys {
                cache.remove(key);
                self.remove_from_execution_access_order(key).await;
            }
        }

        // Update statistics
        self.update_cache_stats().await;
    }

    // Private helper methods

    /// Evict parse cache entries based on the configured strategy
    async fn evict_parse_entries(&self, cache: &mut HashMap<String, RexParseCacheEntry>) {
        let evict_count = (cache.len() as f64 * 0.1).max(1.0) as usize; // Evict 10% or at least 1

        match self.config.eviction_strategy {
            EvictionStrategy::LRU => {
                self.evict_parse_lru(cache, evict_count).await;
            }
            EvictionStrategy::LFU => {
                self.evict_parse_lfu(cache, evict_count).await;
            }
            EvictionStrategy::FIFO => {
                self.evict_parse_fifo(cache, evict_count).await;
            }
            EvictionStrategy::TTL => {
                self.evict_parse_expired(cache).await;
            }
        }

        // Update eviction statistics
        {
            let mut stats = self.stats.write().await;
            stats.evictions += evict_count as u64;
        }
    }

    /// Evict execution cache entries based on the configured strategy
    async fn evict_execution_entries(&self, cache: &mut HashMap<String, RexExecutionCacheEntry>) {
        let evict_count = (cache.len() as f64 * 0.1).max(1.0) as usize; // Evict 10% or at least 1

        match self.config.eviction_strategy {
            EvictionStrategy::LRU => {
                self.evict_execution_lru(cache, evict_count).await;
            }
            EvictionStrategy::LFU => {
                self.evict_execution_lfu(cache, evict_count).await;
            }
            EvictionStrategy::FIFO => {
                self.evict_execution_fifo(cache, evict_count).await;
            }
            EvictionStrategy::TTL => {
                self.evict_execution_expired(cache).await;
            }
        }

        // Update eviction statistics
        {
            let mut stats = self.stats.write().await;
            stats.evictions += evict_count as u64;
        }
    }

    /// Evict least recently used parse entries
    async fn evict_parse_lru(&self, cache: &mut HashMap<String, RexParseCacheEntry>, count: usize) {
        let access_order = self.parse_access_order.read().await;
        let keys_to_evict: Vec<String> = access_order.iter().take(count).cloned().collect();

        for key in &keys_to_evict {
            cache.remove(key);
        }

        drop(access_order);

        // Remove from access order
        for key in &keys_to_evict {
            self.remove_from_parse_access_order(key).await;
        }
    }

    /// Evict least recently used execution entries
    async fn evict_execution_lru(&self, cache: &mut HashMap<String, RexExecutionCacheEntry>, count: usize) {
        let access_order = self.execution_access_order.read().await;
        let keys_to_evict: Vec<String> = access_order.iter().take(count).cloned().collect();

        for key in &keys_to_evict {
            cache.remove(key);
        }

        drop(access_order);

        // Remove from access order
        for key in &keys_to_evict {
            self.remove_from_execution_access_order(key).await;
        }
    }

    /// Evict least frequently used parse entries
    async fn evict_parse_lfu(&self, cache: &mut HashMap<String, RexParseCacheEntry>, count: usize) {
        let mut entries_by_frequency: Vec<_> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.access_count))
            .collect();

        entries_by_frequency.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_frequency.iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            cache.remove(key);
            self.remove_from_parse_access_order(key).await;
        }
    }

    /// Evict least frequently used execution entries
    async fn evict_execution_lfu(&self, cache: &mut HashMap<String, RexExecutionCacheEntry>, count: usize) {
        let mut entries_by_frequency: Vec<_> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.access_count))
            .collect();

        entries_by_frequency.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_frequency.iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            cache.remove(key);
            self.remove_from_execution_access_order(key).await;
        }
    }

    /// Evict oldest parse entries (FIFO)
    async fn evict_parse_fifo(&self, cache: &mut HashMap<String, RexParseCacheEntry>, count: usize) {
        let mut entries_by_age: Vec<_> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.timestamp))
            .collect();

        entries_by_age.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_age.iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            cache.remove(key);
            self.remove_from_parse_access_order(key).await;
        }
    }

    /// Evict oldest execution entries (FIFO)
    async fn evict_execution_fifo(&self, cache: &mut HashMap<String, RexExecutionCacheEntry>, count: usize) {
        let mut entries_by_age: Vec<_> = cache.iter()
            .map(|(key, entry)| (key.clone(), entry.timestamp))
            .collect();

        entries_by_age.sort_by(|a, b| a.1.cmp(&b.1));

        let keys_to_evict: Vec<String> = entries_by_age.iter()
            .take(count)
            .map(|(key, _)| key.clone())
            .collect();

        for key in &keys_to_evict {
            cache.remove(key);
            self.remove_from_execution_access_order(key).await;
        }
    }

    /// Evict expired parse entries
    async fn evict_parse_expired(&self, cache: &mut HashMap<String, RexParseCacheEntry>) {
        let expired_keys: Vec<String> = cache.iter()
            .filter(|(_, entry)| !entry.is_valid())
            .map(|(key, _)| key.clone())
            .collect();

        for key in &expired_keys {
            cache.remove(key);
            self.remove_from_parse_access_order(key).await;
        }
    }

    /// Evict expired execution entries
    async fn evict_execution_expired(&self, cache: &mut HashMap<String, RexExecutionCacheEntry>) {
        let expired_keys: Vec<String> = cache.iter()
            .filter(|(_, entry)| !entry.is_valid())
            .map(|(key, _)| key.clone())
            .collect();

        for key in &expired_keys {
            cache.remove(key);
            self.remove_from_execution_access_order(key).await;
        }
    }

    /// Update parse access order for LRU tracking
    async fn update_parse_access_order(&self, key: &str) {
        let mut access_order = self.parse_access_order.write().await;

        // Remove key if it already exists
        access_order.retain(|k| k != key);

        // Add to the end (most recently used)
        access_order.push(key.to_string());
    }

    /// Update execution access order for LRU tracking
    async fn update_execution_access_order(&self, key: &str) {
        let mut access_order = self.execution_access_order.write().await;

        // Remove key if it already exists
        access_order.retain(|k| k != key);

        // Add to the end (most recently used)
        access_order.push(key.to_string());
    }

    /// Remove a key from parse access order
    async fn remove_from_parse_access_order(&self, key: &str) {
        let mut access_order = self.parse_access_order.write().await;
        access_order.retain(|k| k != key);
    }

    /// Remove a key from execution access order
    async fn remove_from_execution_access_order(&self, key: &str) {
        let mut access_order = self.execution_access_order.write().await;
        access_order.retain(|k| k != key);
    }

    /// Update cache statistics
    async fn update_cache_stats(&self) {
        let mut stats = self.stats.write().await;

        // Update entry counts
        {
            let parse_cache = self.parse_cache.read().await;
            stats.parse_entries = parse_cache.len();
        }

        {
            let execution_cache = self.execution_cache.read().await;
            stats.execution_entries = execution_cache.len();
        }

        // Estimate memory usage
        stats.memory_usage_bytes = self.estimate_memory_usage().await;
    }

    /// Estimate memory usage of the cache
    async fn estimate_memory_usage(&self) -> u64 {
        let mut total_size = 0u64;

        // Estimate parse cache memory usage
        {
            let parse_cache = self.parse_cache.read().await;
            for (key, entry) in parse_cache.iter() {
                total_size += key.len() as u64;
                total_size += self.estimate_command_size(&entry.command);
                total_size += 64; // Overhead for entry metadata
            }
        }

        // Estimate execution cache memory usage
        {
            let execution_cache = self.execution_cache.read().await;
            for (key, entry) in execution_cache.iter() {
                total_size += key.len() as u64;
                total_size += self.estimate_execution_result_size(&entry.result);
                total_size += 64; // Overhead for entry metadata
            }
        }

        total_size
    }

    /// Estimate the memory size of a RexCommand
    fn estimate_command_size(&self, command: &RexCommand) -> u64 {
        match command {
            RexCommand::SetEnv { name, value } => {
                (name.len() + value.len()) as u64
            }
            RexCommand::AppendEnv { name, value, separator } => {
                (name.len() + value.len() + separator.len()) as u64
            }
            RexCommand::PrependEnv { name, value, separator } => {
                (name.len() + value.len() + separator.len()) as u64
            }
            RexCommand::UnsetEnv { name } => {
                name.len() as u64
            }
            RexCommand::Alias { name, command } => {
                (name.len() + command.len()) as u64
            }
            RexCommand::Function { name, body } => {
                (name.len() + body.len()) as u64
            }
            RexCommand::Source { path } => {
                path.len() as u64
            }
            RexCommand::Command { command, args } => {
                command.len() as u64 + args.iter().map(|arg| arg.len()).sum::<usize>() as u64
            }
            RexCommand::If { condition, then_commands, else_commands } => {
                let mut size = condition.len() as u64;
                size += then_commands.iter().map(|cmd| self.estimate_command_size(cmd)).sum::<u64>();
                if let Some(else_cmds) = else_commands {
                    size += else_cmds.iter().map(|cmd| self.estimate_command_size(cmd)).sum::<u64>();
                }
                size
            }
            RexCommand::Comment { text } => {
                text.len() as u64
            }
        }
    }

    /// Estimate the memory size of an ExecutionResult
    fn estimate_execution_result_size(&self, result: &ExecutionResult) -> u64 {
        let mut size = 0u64;

        // Output messages
        size += result.output.iter().map(|msg| msg.len()).sum::<usize>() as u64;

        // Error messages
        size += result.errors.iter().map(|msg| msg.len()).sum::<usize>() as u64;

        // Environment changes
        for (key, value) in &result.env_changes {
            size += key.len() as u64;
            if let Some(val) = value {
                size += val.len() as u64;
            }
        }

        size += 32; // Overhead for other fields
        size
    }

    /// Update parse hit rate statistics
    fn update_parse_hit_rate(&self, stats: &mut RexCacheStats) {
        let total_requests = stats.parse_hits + stats.parse_misses;
        if total_requests > 0 {
            stats.parse_hit_rate = stats.parse_hits as f64 / total_requests as f64;
        }
    }

    /// Update execution hit rate statistics
    fn update_execution_hit_rate(&self, stats: &mut RexCacheStats) {
        let total_requests = stats.execution_hits + stats.execution_misses;
        if total_requests > 0 {
            stats.execution_hit_rate = stats.execution_hits as f64 / total_requests as f64;
        }
    }
}

impl Default for RexCache {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    fn create_test_command() -> RexCommand {
        RexCommand::SetEnv {
            name: "TEST_VAR".to_string(),
            value: "test_value".to_string(),
        }
    }

    fn create_test_execution_result() -> ExecutionResult {
        ExecutionResult {
            success: true,
            output: vec!["test output".to_string()],
            errors: vec![],
            env_changes: HashMap::new(),
            execution_time_ms: 0,
        }
    }

    #[tokio::test]
    async fn test_cache_creation() {
        let cache = RexCache::new();
        let stats = cache.get_stats().await;

        assert_eq!(stats.parse_entries, 0);
        assert_eq!(stats.execution_entries, 0);
        assert_eq!(stats.parse_hits, 0);
        assert_eq!(stats.parse_misses, 0);
    }

    #[tokio::test]
    async fn test_parse_cache_operations() {
        let cache = RexCache::new();
        let command = create_test_command();
        let line = "setenv TEST_VAR test_value";

        // Test cache miss
        let result = cache.get_parsed_command(line).await;
        assert!(result.is_none());

        // Test cache put and hit
        cache.put_parsed_command(line.to_string(), command.clone()).await;
        let result = cache.get_parsed_command(line).await;
        assert!(result.is_some());

        let stats = cache.get_stats().await;
        assert_eq!(stats.parse_hits, 1);
        assert_eq!(stats.parse_misses, 1);
        assert_eq!(stats.parse_entries, 1);
    }

    #[tokio::test]
    async fn test_execution_cache_operations() {
        let cache = RexCache::new();
        let result = create_test_execution_result();
        let key = "test_execution_key";

        // Test cache miss
        let cached_result = cache.get_execution_result(key).await;
        assert!(cached_result.is_none());

        // Test cache put and hit
        cache.put_execution_result(key.to_string(), result.clone()).await;
        let cached_result = cache.get_execution_result(key).await;
        assert!(cached_result.is_some());

        let stats = cache.get_stats().await;
        assert_eq!(stats.execution_hits, 1);
        assert_eq!(stats.execution_misses, 1);
        assert_eq!(stats.execution_entries, 1);
    }

    #[tokio::test]
    async fn test_cache_eviction() {
        let config = RexCacheConfig {
            max_parse_entries: 2,
            max_execution_entries: 2,
            ..Default::default()
        };
        let cache = RexCache::with_config(config);

        // Fill parse cache beyond capacity
        for i in 0..3 {
            let line = format!("setenv TEST_VAR_{} value_{}", i, i);
            let command = RexCommand::SetEnv {
                name: format!("TEST_VAR_{}", i),
                value: format!("value_{}", i),
            };
            cache.put_parsed_command(line, command).await;
        }

        let stats = cache.get_stats().await;
        assert!(stats.parse_entries <= 2); // Should have evicted entries
        assert!(stats.evictions > 0);
    }

    #[tokio::test]
    async fn test_cache_cleanup() {
        let config = RexCacheConfig {
            parse_ttl: 1, // 1 second TTL
            execution_ttl: 1,
            ..Default::default()
        };
        let cache = RexCache::with_config(config);

        // Add entries
        let command = create_test_command();
        let result = create_test_execution_result();
        cache.put_parsed_command("test_line".to_string(), command).await;
        cache.put_execution_result("test_key".to_string(), result).await;

        // Wait for expiration
        tokio::time::sleep(Duration::from_secs(2)).await;

        // Cleanup should remove expired entries
        cache.cleanup().await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.parse_entries, 0);
        assert_eq!(stats.execution_entries, 0);
    }

    #[tokio::test]
    async fn test_cache_clear() {
        let cache = RexCache::new();
        let command = create_test_command();
        let result = create_test_execution_result();

        // Add entries
        cache.put_parsed_command("test_line".to_string(), command).await;
        cache.put_execution_result("test_key".to_string(), result).await;

        // Clear cache
        cache.clear().await;

        let stats = cache.get_stats().await;
        assert_eq!(stats.parse_entries, 0);
        assert_eq!(stats.execution_entries, 0);
        assert_eq!(stats.parse_hits, 0);
        assert_eq!(stats.parse_misses, 0);
    }

    #[tokio::test]
    async fn test_hit_rate_calculation() {
        let cache = RexCache::new();
        let command = create_test_command();
        let line = "test_line";

        // Add to cache
        cache.put_parsed_command(line.to_string(), command).await;

        // Generate hits and misses
        cache.get_parsed_command(line).await; // hit
        cache.get_parsed_command(line).await; // hit
        cache.get_parsed_command("missing_line").await; // miss

        let stats = cache.get_stats().await;
        assert_eq!(stats.parse_hits, 2);
        assert_eq!(stats.parse_misses, 1);
        assert!((stats.parse_hit_rate - 0.6666666666666666).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_memory_estimation() {
        let cache = RexCache::new();
        let command = RexCommand::SetEnv {
            name: "TEST_VAR".to_string(),
            value: "test_value".to_string(),
        };

        cache.put_parsed_command("test_line".to_string(), command).await;

        let memory_usage = cache.estimate_memory_usage().await;
        assert!(memory_usage > 0);

        let stats = cache.get_stats().await;
        assert_eq!(stats.memory_usage_bytes, memory_usage);
    }
}
