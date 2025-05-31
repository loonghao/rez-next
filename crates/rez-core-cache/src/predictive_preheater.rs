//! Predictive Preheater
//!
//! This module provides ML-based predictive preheating for cache optimization.
//! It analyzes access patterns and preheats cache with likely-to-be-accessed data.

use crate::PreheatingConfig;
use serde::{Serialize, Deserialize};
use std::{
    collections::{HashMap, VecDeque},
    hash::Hash,
    sync::{Arc, RwLock},
    time::{Duration, SystemTime},
};
use tokio::time::Interval;


/// Access pattern for a cache key
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AccessPattern {
    /// Key identifier
    pub key_hash: u64,
    /// Access timestamps
    pub access_times: VecDeque<SystemTime>,
    /// Access frequency (accesses per hour)
    pub frequency: f64,
    /// Access regularity score (0.0 to 1.0)
    pub regularity: f64,
    /// Prediction confidence (0.0 to 1.0)
    pub confidence: f64,
    /// Last prediction time
    pub last_prediction: SystemTime,
    /// Prediction accuracy history
    pub accuracy_history: VecDeque<f64>,
}

impl AccessPattern {
    /// Create a new access pattern
    pub fn new(key_hash: u64) -> Self {
        Self {
            key_hash,
            access_times: VecDeque::new(),
            frequency: 0.0,
            regularity: 0.0,
            confidence: 0.0,
            last_prediction: SystemTime::UNIX_EPOCH,
            accuracy_history: VecDeque::new(),
        }
    }

    /// Record a new access
    pub fn record_access(&mut self, time: SystemTime) {
        self.access_times.push_back(time);
        
        // Keep only recent accesses (last 24 hours)
        let cutoff = time - Duration::from_secs(24 * 3600);
        while let Some(&front_time) = self.access_times.front() {
            if front_time < cutoff {
                self.access_times.pop_front();
            } else {
                break;
            }
        }

        self.update_metrics();
    }

    /// Update frequency and regularity metrics
    fn update_metrics(&mut self) {
        if self.access_times.len() < 2 {
            return;
        }

        // Calculate frequency (accesses per hour)
        let time_span = self.access_times.back().unwrap()
            .duration_since(*self.access_times.front().unwrap())
            .unwrap_or(Duration::from_secs(1))
            .as_secs_f64() / 3600.0; // Convert to hours

        self.frequency = if time_span > 0.0 {
            self.access_times.len() as f64 / time_span
        } else {
            0.0
        };

        // Calculate regularity (consistency of intervals)
        if self.access_times.len() >= 3 {
            let intervals: Vec<f64> = self.access_times
                .iter()
                .zip(self.access_times.iter().skip(1))
                .map(|(a, b)| b.duration_since(*a).unwrap_or(Duration::from_secs(0)).as_secs_f64())
                .collect();

            let mean_interval = intervals.iter().sum::<f64>() / intervals.len() as f64;
            let variance = intervals.iter()
                .map(|&x| (x - mean_interval).powi(2))
                .sum::<f64>() / intervals.len() as f64;
            
            // Regularity is inverse of coefficient of variation
            self.regularity = if mean_interval > 0.0 {
                1.0 / (1.0 + (variance.sqrt() / mean_interval))
            } else {
                0.0
            };
        }

        // Update confidence based on frequency and regularity
        self.confidence = (self.frequency.min(10.0) / 10.0) * self.regularity;
    }

    /// Predict next access time
    pub fn predict_next_access(&self) -> Option<SystemTime> {
        if self.access_times.len() < 2 || self.confidence < 0.3 {
            return None;
        }

        // Calculate average interval
        let intervals: Vec<Duration> = self.access_times
            .iter()
            .zip(self.access_times.iter().skip(1))
            .map(|(a, b)| b.duration_since(*a).unwrap_or(Duration::from_secs(0)))
            .collect();

        if intervals.is_empty() {
            return None;
        }

        let avg_interval_secs = intervals.iter()
            .map(|d| d.as_secs_f64())
            .sum::<f64>() / intervals.len() as f64;

        let last_access = *self.access_times.back()?;
        let predicted_time = last_access + Duration::from_secs_f64(avg_interval_secs);

        Some(predicted_time)
    }

    /// Calculate cache score for prioritization
    pub fn calculate_cache_score(&self) -> f64 {
        // Score based on frequency, regularity, and recency
        let recency_factor = if let Some(last_access) = self.access_times.back() {
            let age = SystemTime::now()
                .duration_since(*last_access)
                .unwrap_or(Duration::from_secs(u64::MAX))
                .as_secs_f64() / 3600.0; // Hours
            
            // Decay factor: more recent = higher score
            (-age / 24.0).exp() // 24-hour half-life
        } else {
            0.0
        };

        self.frequency * self.regularity * self.confidence * recency_factor
    }
}

/// Predictive Preheater
///
/// Analyzes access patterns and predicts future cache needs for proactive preheating.
/// Uses machine learning techniques to optimize cache hit rates.
#[derive(Debug)]
pub struct PredictivePreheater<K>
where
    K: Clone + Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
{
    /// Configuration
    config: PreheatingConfig,
    /// Access patterns for each key
    patterns: Arc<RwLock<HashMap<K, AccessPattern>>>,
    /// Preheating queue
    preheat_queue: Arc<RwLock<VecDeque<(K, f64, SystemTime)>>>, // (key, score, predicted_time)
    /// Preheating statistics
    stats: Arc<RwLock<PreheatingStats>>,
    /// Background preheating interval
    _preheating_interval: Option<Interval>,
}

/// Preheating statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreheatingStats {
    /// Total predictions made
    pub predictions_made: u64,
    /// Successful predictions (cache hits after preheating)
    pub successful_predictions: u64,
    /// Failed predictions (cache misses after preheating)
    pub failed_predictions: u64,
    /// Prediction accuracy rate
    pub accuracy_rate: f64,
    /// Total preheating operations
    pub preheat_operations: u64,
    /// Average prediction confidence
    pub avg_confidence: f64,
    /// Patterns learned
    pub patterns_learned: usize,
}

impl Default for PreheatingStats {
    fn default() -> Self {
        Self {
            predictions_made: 0,
            successful_predictions: 0,
            failed_predictions: 0,
            accuracy_rate: 0.0,
            preheat_operations: 0,
            avg_confidence: 0.0,
            patterns_learned: 0,
        }
    }
}

impl<K> PredictivePreheater<K>
where
    K: Clone + Hash + Eq + Send + Sync + std::fmt::Debug + 'static,
{
    /// Create a new predictive preheater
    pub fn new(config: PreheatingConfig) -> Self {
        Self {
            config,
            patterns: Arc::new(RwLock::new(HashMap::new())),
            preheat_queue: Arc::new(RwLock::new(VecDeque::new())),
            stats: Arc::new(RwLock::new(PreheatingStats::default())),
            _preheating_interval: None,
        }
    }

    /// Record an access for pattern learning
    pub async fn record_access(&self, key: &K) {
        if !self.config.enable_pattern_learning {
            return;
        }

        let key_hash = self.calculate_key_hash(key);
        let now = SystemTime::now();

        let mut patterns = self.patterns.write().unwrap();
        let pattern = patterns
            .entry(key.clone())
            .or_insert_with(|| AccessPattern::new(key_hash));
        
        pattern.record_access(now);

        // Update statistics
        {
            let mut stats = self.stats.write().unwrap();
            stats.patterns_learned = patterns.len();
        }
    }

    /// Predict and schedule preheating for a key
    pub async fn predict_and_preheat(&self, key: &K) {
        if !self.config.enable_predictive_preheating {
            return;
        }

        let patterns = self.patterns.read().unwrap();
        if let Some(pattern) = patterns.get(key) {
            if let Some(predicted_time) = pattern.predict_next_access() {
                let score = pattern.calculate_cache_score();
                
                // Only queue if confidence is high enough
                if pattern.confidence >= self.config.min_confidence_threshold {
                    let mut queue = self.preheat_queue.write().unwrap();
                    queue.push_back((key.clone(), score, predicted_time));
                    
                    // Keep queue size manageable
                    if queue.len() > self.config.max_preheat_queue_size {
                        queue.pop_front();
                    }

                    // Update statistics
                    {
                        let mut stats = self.stats.write().unwrap();
                        stats.predictions_made += 1;
                    }
                }
            }
        }
    }

    /// Get preheating recommendations
    pub async fn get_preheat_recommendations(&self) -> Vec<(K, f64)> {
        let mut queue = self.preheat_queue.write().unwrap();
        let now = SystemTime::now();
        
        // Filter and sort by score
        let mut recommendations: Vec<(K, f64)> = queue
            .iter()
            .filter(|(_, _, predicted_time)| {
                // Only recommend if prediction time is near
                let time_diff = predicted_time
                    .duration_since(now)
                    .unwrap_or_else(|_| now.duration_since(*predicted_time).unwrap_or(Duration::from_secs(0)))
                    .as_secs();
                
                time_diff <= self.config.preheat_window_seconds
            })
            .map(|(key, score, _)| (key.clone(), *score))
            .collect();

        // Sort by score (highest first)
        recommendations.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

        // Remove processed items from queue
        queue.retain(|(_, _, predicted_time)| {
            let time_diff = predicted_time
                .duration_since(now)
                .unwrap_or_else(|_| now.duration_since(*predicted_time).unwrap_or(Duration::from_secs(0)))
                .as_secs();
            
            time_diff > self.config.preheat_window_seconds
        });

        recommendations.into_iter().take(self.config.max_concurrent_preheats).collect()
    }

    /// Calculate cache score for a key
    pub fn calculate_cache_score(&self, key: &K) -> f64 {
        let patterns = self.patterns.read().unwrap();
        patterns.get(key)
            .map(|pattern| pattern.calculate_cache_score())
            .unwrap_or(0.0)
    }

    /// Get preheating statistics
    pub fn get_stats(&self) -> PreheatingStats {
        let stats = self.stats.read().unwrap();
        let mut result = stats.clone();
        
        // Calculate accuracy rate
        if result.predictions_made > 0 {
            result.accuracy_rate = result.successful_predictions as f64 / result.predictions_made as f64;
        }

        // Calculate average confidence
        let patterns = self.patterns.read().unwrap();
        if !patterns.is_empty() {
            result.avg_confidence = patterns.values()
                .map(|p| p.confidence)
                .sum::<f64>() / patterns.len() as f64;
        }

        result
    }

    /// Calculate hash for a key (simplified)
    fn calculate_key_hash(&self, key: &K) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::Hasher;
        
        let mut hasher = DefaultHasher::new();
        key.hash(&mut hasher);
        hasher.finish()
    }

    /// Record prediction outcome for learning
    pub async fn record_prediction_outcome(&self, _key: &K, was_hit: bool) {
        let mut stats = self.stats.write().unwrap();
        if was_hit {
            stats.successful_predictions += 1;
        } else {
            stats.failed_predictions += 1;
        }
    }
}
