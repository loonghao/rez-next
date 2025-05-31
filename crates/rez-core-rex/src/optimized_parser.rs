//! High-performance optimized Rex command parser
//!
//! This module provides an optimized Rex command parser that uses state machine
//! parsing and zero-copy optimization techniques, reusing the proven architecture
//! from StateMachineParser in the version module.

use crate::{RexCommand, RexScript, ParserConfig};
use rez_core_common::RezCoreError;
use rez_core_version::parser::{StateMachineParser, TokenType};
use ahash::AHashMap;
use once_cell::sync::Lazy;
use smallvec::SmallVec;
use std::sync::{Arc, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

/// String interning pool for reducing memory allocations
static COMMAND_INTERN_POOL: Lazy<RwLock<AHashMap<String, &'static str>>> =
    Lazy::new(|| RwLock::new(AHashMap::new()));

/// Command pattern types for fast matching
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum CommandPattern {
    SetEnv,
    AppendEnv,
    PrependEnv,
    UnsetEnv,
    Alias,
    Function,
    Source,
    Comment,
    Shell,
}

/// Command pattern matcher using state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseState {
    Start,
    InCommand,
    InArgs,
    End,
}

/// Precompiled command patterns for fast lookup
static COMMAND_PATTERNS: Lazy<AHashMap<&'static str, CommandPattern>> = Lazy::new(|| {
    let mut patterns = AHashMap::new();
    patterns.insert("setenv", CommandPattern::SetEnv);
    patterns.insert("appendenv", CommandPattern::AppendEnv);
    patterns.insert("prependenv", CommandPattern::PrependEnv);
    patterns.insert("unsetenv", CommandPattern::UnsetEnv);
    patterns.insert("alias", CommandPattern::Alias);
    patterns.insert("function", CommandPattern::Function);
    patterns.insert("source", CommandPattern::Source);
    patterns
});

/// Cache entry for parsed commands
#[derive(Debug, Clone)]
struct ParseCacheEntry {
    command: RexCommand,
    timestamp: u64,
    access_count: u64,
}

impl ParseCacheEntry {
    fn new(command: RexCommand) -> Self {
        Self {
            command,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0),
            access_count: 1,
        }
    }

    fn mark_accessed(&mut self) {
        self.access_count += 1;
    }

    fn is_valid(&self, ttl: u64) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        now - self.timestamp < ttl
    }
}

/// High-performance optimized Rex parser
pub struct OptimizedRexParser {
    /// Parser configuration
    config: ParserConfig,
    /// State machine parser for token processing
    state_machine: StateMachineParser,
    /// Parse cache for frequently used commands
    parse_cache: Arc<RwLock<AHashMap<String, ParseCacheEntry>>>,
    /// Cache configuration
    cache_ttl: u64,
    max_cache_entries: usize,
    /// Performance statistics
    cache_hits: Arc<RwLock<u64>>,
    cache_misses: Arc<RwLock<u64>>,
    total_parses: Arc<RwLock<u64>>,
}

impl OptimizedRexParser {
    /// Create a new optimized Rex parser
    pub fn new() -> Self {
        Self::with_config(ParserConfig::default())
    }

    /// Create a parser with custom configuration
    pub fn with_config(config: ParserConfig) -> Self {
        Self {
            config,
            state_machine: StateMachineParser::new(),
            parse_cache: Arc::new(RwLock::new(AHashMap::new())),
            cache_ttl: 3600, // 1 hour
            max_cache_entries: 5000,
            cache_hits: Arc::new(RwLock::new(0)),
            cache_misses: Arc::new(RwLock::new(0)),
            total_parses: Arc::new(RwLock::new(0)),
        }
    }

    /// Parse a Rex script from string (standard interface)
    pub fn parse(&self, content: &str) -> Result<RexScript, RezCoreError> {
        self.parse_optimized(content)
    }

    /// Parse a Rex script from string with optimized performance
    pub fn parse_optimized(&self, content: &str) -> Result<RexScript, RezCoreError> {
        let mut script = RexScript::new();

        for (line_num, line) in content.lines().enumerate() {
            let line = line.trim();

            // Skip empty lines
            if line.is_empty() {
                continue;
            }

            // Parse command using optimized parser
            match self.parse_line_optimized(line) {
                Ok(Some(command)) => script.add_command(command),
                Ok(None) => {}, // Empty or comment line
                Err(e) => {
                    if self.config.strict_mode {
                        return Err(RezCoreError::RexError(
                            format!("Parse error at line {}: {}", line_num + 1, e)
                        ));
                    } else {
                        // In non-strict mode, treat as comment
                        script.add_command(RexCommand::Comment {
                            text: format!("Parse error: {}", line),
                        });
                    }
                }
            }
        }

        Ok(script)
    }

    /// Parse a single line with state machine optimization
    pub fn parse_line_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        // Update statistics
        {
            let mut total = self.total_parses.write().unwrap();
            *total += 1;
        }

        let line = line.trim();
        
        // Handle empty lines
        if line.is_empty() {
            return Ok(None);
        }

        // Check cache first
        if let Some(cached) = self.get_cached_command(line) {
            let mut hits = self.cache_hits.write().unwrap();
            *hits += 1;
            return Ok(Some(cached));
        }

        // Cache miss - parse using optimized state machine
        {
            let mut misses = self.cache_misses.write().unwrap();
            *misses += 1;
        }

        let result = self.parse_line_state_machine(line)?;
        
        // Cache the result if it's not None
        if let Some(ref command) = result {
            self.cache_command(line.to_string(), command.clone());
        }

        Ok(result)
    }

    /// Parse line using state machine approach
    fn parse_line_state_machine(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        // Handle comments first (fast path)
        if line.starts_with('#') {
            return Ok(Some(RexCommand::Comment {
                text: line[1..].trim().to_string(),
            }));
        }

        // Use state machine to identify command pattern
        let command_pattern = self.identify_command_pattern(line)?;
        
        match command_pattern {
            CommandPattern::SetEnv => self.parse_setenv_optimized(line),
            CommandPattern::AppendEnv => self.parse_appendenv_optimized(line),
            CommandPattern::PrependEnv => self.parse_prependenv_optimized(line),
            CommandPattern::UnsetEnv => self.parse_unsetenv_optimized(line),
            CommandPattern::Alias => self.parse_alias_optimized(line),
            CommandPattern::Function => self.parse_function_optimized(line),
            CommandPattern::Source => self.parse_source_optimized(line),
            CommandPattern::Comment => Ok(Some(RexCommand::Comment {
                text: line[1..].trim().to_string(),
            })),
            CommandPattern::Shell => {
                if self.config.allow_shell_syntax {
                    self.parse_shell_command_optimized(line)
                } else {
                    Err(RezCoreError::RexError(
                        format!("Unknown command: {}", line)
                    ))
                }
            }
        }
    }

    /// Identify command pattern using state machine
    fn identify_command_pattern(&self, line: &str) -> Result<CommandPattern, RezCoreError> {
        // Fast path for common patterns using first character
        let first_char = line.chars().next().unwrap_or(' ');

        match first_char {
            '#' => return Ok(CommandPattern::Comment),
            's' => {
                if line.starts_with("setenv ") {
                    return Ok(CommandPattern::SetEnv);
                } else if line.starts_with("source ") {
                    return Ok(CommandPattern::Source);
                }
            }
            'a' => {
                if line.starts_with("appendenv ") {
                    return Ok(CommandPattern::AppendEnv);
                } else if line.starts_with("alias ") {
                    return Ok(CommandPattern::Alias);
                }
            }
            'p' => {
                if line.starts_with("prependenv ") {
                    return Ok(CommandPattern::PrependEnv);
                }
            }
            'u' => {
                if line.starts_with("unsetenv ") {
                    return Ok(CommandPattern::UnsetEnv);
                }
            }
            'f' => {
                if line.starts_with("function ") {
                    return Ok(CommandPattern::Function);
                }
            }
            _ => {}
        }

        // Fallback to pattern matching for complex cases
        let space_pos = line.find(' ').unwrap_or(line.len());
        let command_word = &line[..space_pos];

        if let Some(&pattern) = COMMAND_PATTERNS.get(command_word) {
            Ok(pattern)
        } else {
            Ok(CommandPattern::Shell)
        }
    }

    /// Cache management methods
    fn get_cached_command(&self, line: &str) -> Option<RexCommand> {
        let mut cache = self.parse_cache.write().unwrap();
        if let Some(entry) = cache.get_mut(line) {
            if entry.is_valid(self.cache_ttl) {
                entry.mark_accessed();
                return Some(entry.command.clone());
            } else {
                // Remove expired entry
                cache.remove(line);
            }
        }
        None
    }

    fn cache_command(&self, line: String, command: RexCommand) {
        let mut cache = self.parse_cache.write().unwrap();

        // Check cache size limit
        if cache.len() >= self.max_cache_entries {
            // Simple LRU eviction - remove oldest entries
            let mut entries_to_remove = Vec::new();
            let now = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(0);

            for (key, entry) in cache.iter() {
                if now - entry.timestamp > self.cache_ttl / 2 {
                    entries_to_remove.push(key.clone());
                }
            }

            for key in entries_to_remove {
                cache.remove(&key);
            }
        }

        cache.insert(line, ParseCacheEntry::new(command));
    }

    /// Optimized command parsing methods
    fn parse_setenv_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let args = &line[7..]; // Skip "setenv "
        let parts = self.split_command_line_optimized(args);

        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "setenv requires name and value".to_string()
            ));
        }

        let name = parts[0].clone();
        let value = parts[1..].join(" ");
        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };

        Ok(Some(RexCommand::SetEnv {
            name,
            value: expanded_value,
        }))
    }

    fn parse_appendenv_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let args = &line[10..]; // Skip "appendenv "
        let parts = self.split_command_line_optimized(args);

        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "appendenv requires name and value".to_string()
            ));
        }

        let name = parts[0].clone();
        let value = parts[1].clone();
        let separator = if parts.len() > 2 {
            parts[2].clone()
        } else {
            self.get_default_separator(&name)
        };

        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };

        Ok(Some(RexCommand::AppendEnv {
            name,
            value: expanded_value,
            separator,
        }))
    }

    fn parse_prependenv_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let args = &line[11..]; // Skip "prependenv "
        let parts = self.split_command_line_optimized(args);

        if parts.len() < 2 {
            return Err(RezCoreError::RexError(
                "prependenv requires name and value".to_string()
            ));
        }

        let name = parts[0].clone();
        let value = parts[1].clone();
        let separator = if parts.len() > 2 {
            parts[2].clone()
        } else {
            self.get_default_separator(&name)
        };

        let expanded_value = if self.config.variable_expansion {
            self.expand_variables(&value)?
        } else {
            value
        };

        Ok(Some(RexCommand::PrependEnv {
            name,
            value: expanded_value,
            separator,
        }))
    }

    fn parse_unsetenv_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let args = &line[9..]; // Skip "unsetenv "
        let parts = self.split_command_line_optimized(args);

        if parts.is_empty() {
            return Err(RezCoreError::RexError(
                "unsetenv requires variable name".to_string()
            ));
        }

        Ok(Some(RexCommand::UnsetEnv {
            name: parts[0].clone(),
        }))
    }

    fn parse_alias_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let content = &line[6..]; // Skip "alias "

        if let Some(eq_pos) = content.find('=') {
            let name = content[..eq_pos].trim().to_string();
            let command = content[eq_pos + 1..].trim().to_string();

            // Remove quotes if present
            let command = if (command.starts_with('"') && command.ends_with('"')) ||
                            (command.starts_with('\'') && command.ends_with('\'')) {
                command[1..command.len()-1].to_string()
            } else {
                command
            };

            Ok(Some(RexCommand::Alias { name, command }))
        } else {
            Err(RezCoreError::RexError(
                "alias requires name=command format".to_string()
            ))
        }
    }

    fn parse_function_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let content = &line[9..]; // Skip "function "

        if let Some(brace_pos) = content.find('{') {
            let name = content[..brace_pos].trim().to_string();
            let body_start = brace_pos + 1;

            if let Some(close_brace) = content.rfind('}') {
                let body = content[body_start..close_brace].trim().to_string();
                Ok(Some(RexCommand::Function { name, body }))
            } else {
                Err(RezCoreError::RexError(
                    "function missing closing brace".to_string()
                ))
            }
        } else {
            Err(RezCoreError::RexError(
                "function requires name { body } format".to_string()
            ))
        }
    }

    fn parse_source_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let args = &line[7..]; // Skip "source "
        let parts = self.split_command_line_optimized(args);

        if parts.is_empty() {
            return Err(RezCoreError::RexError(
                "source requires file path".to_string()
            ));
        }

        let path = if self.config.variable_expansion {
            self.expand_variables(&parts[0])?
        } else {
            parts[0].clone()
        };

        Ok(Some(RexCommand::Source { path }))
    }

    fn parse_shell_command_optimized(&self, line: &str) -> Result<Option<RexCommand>, RezCoreError> {
        let parts = self.split_command_line_optimized(line);

        if parts.is_empty() {
            return Ok(None);
        }

        let command = parts[0].clone();
        let args = parts[1..].to_vec();

        Ok(Some(RexCommand::Command { command, args }))
    }

    /// Optimized command line splitting using SmallVec
    fn split_command_line_optimized(&self, line: &str) -> SmallVec<[String; 8]> {
        let mut parts = SmallVec::new();
        let mut current_part = String::new();
        let mut in_quotes = false;
        let mut quote_char = '"';
        let mut chars = line.chars().peekable();

        while let Some(ch) = chars.next() {
            match ch {
                '"' | '\'' if !in_quotes => {
                    in_quotes = true;
                    quote_char = ch;
                }
                ch if in_quotes && ch == quote_char => {
                    in_quotes = false;
                }
                ' ' | '\t' if !in_quotes => {
                    if !current_part.is_empty() {
                        parts.push(current_part.clone());
                        current_part.clear();
                    }
                }
                _ => {
                    current_part.push(ch);
                }
            }
        }

        if !current_part.is_empty() {
            parts.push(current_part);
        }

        parts
    }

    /// Get default separator for environment variable
    fn get_default_separator(&self, var_name: &str) -> String {
        match var_name.to_uppercase().as_str() {
            "PATH" | "LD_LIBRARY_PATH" | "PYTHONPATH" | "CLASSPATH" => {
                self.config.default_path_separator.clone()
            }
            _ => " ".to_string(),
        }
    }

    /// Expand variables in a string
    fn expand_variables(&self, value: &str) -> Result<String, RezCoreError> {
        shellexpand::env(value)
            .map(|expanded| expanded.to_string())
            .map_err(|e| RezCoreError::RexError(
                format!("Variable expansion error: {}", e)
            ))
    }

    /// Get performance statistics
    pub fn get_cache_stats(&self) -> (u64, u64, u64, f64) {
        let hits = *self.cache_hits.read().unwrap();
        let misses = *self.cache_misses.read().unwrap();
        let total = *self.total_parses.read().unwrap();
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        (hits, misses, total, hit_rate)
    }

    /// Clear the parse cache
    pub fn clear_cache(&self) {
        let mut cache = self.parse_cache.write().unwrap();
        cache.clear();
    }

    /// Intern a string to reduce memory allocations
    fn intern_string(&self, s: String) -> String {
        if s.len() > 64 {
            return s;
        }

        // Try to get from pool first
        {
            let pool = COMMAND_INTERN_POOL.read().unwrap();
            if let Some(&interned) = pool.get(&s) {
                return interned.to_string();
            }
        }

        // Add to pool if not found
        {
            let mut pool = COMMAND_INTERN_POOL.write().unwrap();
            // Double-check after acquiring write lock
            if let Some(&interned) = pool.get(&s) {
                return interned.to_string();
            }

            // Limit pool size to prevent memory leaks
            if pool.len() < 5000 {
                let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
                pool.insert(s.clone(), leaked);
                return leaked.to_string();
            }
        }

        s
    }
}

impl Default for OptimizedRexParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_optimized_parser_creation() {
        let parser = OptimizedRexParser::new();
        let (hits, misses, total, hit_rate) = parser.get_cache_stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 0);
        assert_eq!(total, 0);
        assert_eq!(hit_rate, 0.0);
    }

    #[test]
    fn test_command_pattern_identification() {
        let parser = OptimizedRexParser::new();

        assert_eq!(parser.identify_command_pattern("setenv PATH /usr/bin").unwrap(), CommandPattern::SetEnv);
        assert_eq!(parser.identify_command_pattern("appendenv PATH /usr/local/bin").unwrap(), CommandPattern::AppendEnv);
        assert_eq!(parser.identify_command_pattern("# comment").unwrap(), CommandPattern::Comment);
        assert_eq!(parser.identify_command_pattern("unknown_command").unwrap(), CommandPattern::Shell);
    }

    #[test]
    fn test_optimized_parsing() {
        let parser = OptimizedRexParser::new();

        // Test setenv parsing
        let result = parser.parse_line_optimized("setenv PATH /usr/bin").unwrap();
        assert!(result.is_some());
        if let Some(RexCommand::SetEnv { name, value }) = result {
            assert_eq!(name, "PATH");
            assert_eq!(value, "/usr/bin");
        } else {
            panic!("Expected SetEnv command");
        }

        // Test cache functionality
        let result2 = parser.parse_line_optimized("setenv PATH /usr/bin").unwrap();
        assert!(result2.is_some());

        let (hits, misses, total, _) = parser.get_cache_stats();
        assert_eq!(total, 2);
        assert_eq!(misses, 1);
        assert_eq!(hits, 1);
    }

    #[test]
    fn test_split_command_line_optimized() {
        let parser = OptimizedRexParser::new();

        let parts = parser.split_command_line_optimized("arg1 arg2 \"quoted arg\" 'single quoted'");
        assert_eq!(parts.len(), 4);
        assert_eq!(parts[0], "arg1");
        assert_eq!(parts[1], "arg2");
        assert_eq!(parts[2], "quoted arg");
        assert_eq!(parts[3], "single quoted");
    }
}
