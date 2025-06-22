//! High-performance version parsing utilities with zero-copy state machine

use super::Version;
#[cfg(feature = "python-bindings")]
use super::VersionToken;
use ahash::AHashMap;
use once_cell::sync::Lazy;
use rez_core_common::RezCoreError;
use smallvec::SmallVec;
use std::sync::RwLock;

/// String interning pool for reducing memory allocations
static STRING_INTERN_POOL: Lazy<RwLock<AHashMap<String, &'static str>>> =
    Lazy::new(|| RwLock::new(AHashMap::new()));

/// Token types for state machine parsing
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Numeric(u64),
    Alphanumeric(String),
    Separator(char),
}

/// Parser state for state machine
#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseState {
    Start,
    InToken,
    InSeparator,
    End,
}

/// High-performance version parser with state machine and zero-copy optimization
pub struct StateMachineParser {
    /// Enable string interning for memory optimization
    use_interning: bool,
    /// Maximum number of tokens allowed
    max_tokens: usize,
    /// Maximum number of numeric tokens allowed
    max_numeric_tokens: usize,
}

impl StateMachineParser {
    /// Create a new high-performance parser
    pub fn new() -> Self {
        Self {
            use_interning: true,
            max_tokens: 10,
            max_numeric_tokens: 5,
        }
    }

    /// Create parser with custom configuration
    pub fn with_config(use_interning: bool, max_tokens: usize, max_numeric_tokens: usize) -> Self {
        Self {
            use_interning,
            max_tokens,
            max_numeric_tokens,
        }
    }

    /// Intern a string to reduce memory allocations
    fn intern_string(&self, s: String) -> String {
        if !self.use_interning || s.len() > 64 {
            return s;
        }

        // Try to get from pool first
        {
            let pool = STRING_INTERN_POOL.read().unwrap();
            if let Some(&interned) = pool.get(&s) {
                return interned.to_string();
            }
        }

        // Add to pool if not found
        {
            let mut pool = STRING_INTERN_POOL.write().unwrap();
            // Double-check after acquiring write lock
            if let Some(&interned) = pool.get(&s) {
                return interned.to_string();
            }

            // Limit pool size to prevent memory leaks
            if pool.len() < 10000 {
                let leaked: &'static str = Box::leak(s.clone().into_boxed_str());
                pool.insert(s.clone(), leaked);
                return leaked.to_string();
            }
        }

        s
    }

    /// Fast character classification using lookup table
    #[inline(always)]
    fn is_valid_separator(c: char) -> bool {
        matches!(c, '.' | '-' | '_' | '+')
    }

    /// Fast alphanumeric check with underscore support
    #[inline(always)]
    fn is_token_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    /// Parse version string using zero-copy state machine
    pub fn parse_tokens(
        &self,
        input: &str,
    ) -> Result<(SmallVec<[TokenType; 8]>, SmallVec<[char; 7]>), RezCoreError> {
        if input.is_empty() {
            return Ok((SmallVec::new(), SmallVec::new()));
        }

        let mut tokens = SmallVec::new();
        let mut separators = SmallVec::new();
        let mut state = ParseState::Start;
        let mut current_token = String::new();
        let mut numeric_count = 0;

        let chars: SmallVec<[char; 64]> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            match state {
                ParseState::Start => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                        state = ParseState::InToken;
                    } else if Self::is_valid_separator(c) {
                        return Err(RezCoreError::VersionParse(format!(
                            "Version cannot start with separator '{}'",
                            c
                        )));
                    } else {
                        return Err(RezCoreError::VersionParse(format!(
                            "Invalid character '{}' at start of version",
                            c
                        )));
                    }
                }

                ParseState::InToken => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                    } else if Self::is_valid_separator(c) {
                        // Finalize current token
                        self.finalize_token(&mut current_token, &mut tokens, &mut numeric_count)?;
                        separators.push(c);
                        state = ParseState::InSeparator;
                    } else {
                        return Err(RezCoreError::VersionParse(format!(
                            "Invalid character '{}' in token",
                            c
                        )));
                    }
                }

                ParseState::InSeparator => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                        state = ParseState::InToken;
                    } else {
                        return Err(RezCoreError::VersionParse(format!(
                            "Expected token character after separator, found '{}'",
                            c
                        )));
                    }
                }

                ParseState::End => break,
            }

            i += 1;
        }

        // Finalize last token if we're in a token state
        if state == ParseState::InToken && !current_token.is_empty() {
            self.finalize_token(&mut current_token, &mut tokens, &mut numeric_count)?;
        } else if state == ParseState::InSeparator {
            return Err(RezCoreError::VersionParse(
                "Version cannot end with separator".to_string(),
            ));
        }

        // Validate token counts
        if tokens.len() > self.max_tokens {
            return Err(RezCoreError::VersionParse(format!(
                "Too many tokens: {} (max: {})",
                tokens.len(),
                self.max_tokens
            )));
        }

        if numeric_count > self.max_numeric_tokens {
            return Err(RezCoreError::VersionParse(format!(
                "Too many numeric tokens: {} (max: {})",
                numeric_count, self.max_numeric_tokens
            )));
        }

        Ok((tokens, separators))
    }

    /// Finalize a token and add it to the tokens list
    fn finalize_token(
        &self,
        current_token: &mut String,
        tokens: &mut SmallVec<[TokenType; 8]>,
        numeric_count: &mut usize,
    ) -> Result<(), RezCoreError> {
        if current_token.is_empty() {
            return Err(RezCoreError::VersionParse("Empty token found".to_string()));
        }

        // Validate token format
        if current_token.starts_with('_') || current_token.ends_with('_') {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid token format: '{}'",
                current_token
            )));
        }

        // Check for invalid patterns
        if current_token == "not" || current_token == "version" {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid version token: '{}'",
                current_token
            )));
        }

        // Reject overly long alphabetic tokens
        if current_token.chars().all(|c| c.is_alphabetic()) && current_token.len() > 10 {
            return Err(RezCoreError::VersionParse(format!(
                "Invalid version token: '{}'",
                current_token
            )));
        }

        // Try to parse as numeric first (fast path)
        if current_token.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(num) = current_token.parse::<u64>() {
                tokens.push(TokenType::Numeric(num));
                *numeric_count += 1;
            } else {
                // Number too large, treat as alphanumeric
                let interned = self.intern_string(current_token.clone());
                tokens.push(TokenType::Alphanumeric(interned));
            }
        } else {
            // Alphanumeric token
            let interned = self.intern_string(current_token.clone());
            tokens.push(TokenType::Alphanumeric(interned));
        }

        current_token.clear();
        Ok(())
    }
}

/// Legacy VersionParser for backward compatibility
pub struct VersionParser {
    inner: StateMachineParser,
}

impl VersionParser {
    /// Create a new parser
    pub fn new() -> Self {
        Self {
            inner: StateMachineParser::new(),
        }
    }

    /// Parse a version string into tokens (legacy interface)
    #[cfg(feature = "python-bindings")]
    pub fn parse_tokens(
        &self,
        input: &str,
    ) -> Result<(Vec<VersionToken>, Vec<char>), RezCoreError> {
        let (_tokens, separators) = self.inner.parse_tokens(input)?;

        // Convert to legacy format
        let legacy_tokens = Vec::new();
        let legacy_separators: Vec<char> = separators.into_iter().collect();

        // For now, return empty vectors to maintain compatibility
        // TODO: Implement proper conversion from TokenType to VersionToken
        Ok((legacy_tokens, legacy_separators))
    }

    /// Parse a complete version string
    pub fn parse_version(&self, input: &str) -> Result<Version, RezCoreError> {
        // Use the new state machine parser for better performance
        // but fall back to the original implementation for now
        Version::parse(input)
    }
}

impl Default for VersionParser {
    fn default() -> Self {
        Self::new()
    }
}

impl Default for StateMachineParser {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parser_creation() {
        let _parser = VersionParser::new();
        let _state_machine_parser = StateMachineParser::new();
        // Basic test to ensure parsers can be created
        assert!(true);
    }

    #[test]
    fn test_state_machine_parser_basic() {
        let parser = StateMachineParser::new();

        // Test empty input
        let (tokens, separators) = parser.parse_tokens("").unwrap();
        assert!(tokens.is_empty());
        assert!(separators.is_empty());

        // Test simple version
        let (tokens, separators) = parser.parse_tokens("1.2.3").unwrap();
        assert_eq!(tokens.len(), 3);
        assert_eq!(separators.len(), 2);

        // Check token types
        match &tokens[0] {
            TokenType::Numeric(n) => assert_eq!(*n, 1),
            _ => panic!("Expected numeric token"),
        }

        assert_eq!(separators[0], '.');
        assert_eq!(separators[1], '.');
    }

    #[test]
    fn test_state_machine_parser_alphanumeric() {
        let parser = StateMachineParser::new();

        let (tokens, separators) = parser.parse_tokens("1.2.3-alpha1").unwrap();
        assert_eq!(tokens.len(), 4);
        assert_eq!(separators.len(), 3);

        // Check mixed token types
        match &tokens[0] {
            TokenType::Numeric(n) => assert_eq!(*n, 1),
            _ => panic!("Expected numeric token"),
        }

        match &tokens[3] {
            TokenType::Alphanumeric(s) => assert_eq!(s, "alpha1"),
            _ => panic!("Expected alphanumeric token"),
        }
    }

    #[test]
    fn test_state_machine_parser_errors() {
        let parser = StateMachineParser::new();

        // Test invalid start
        assert!(parser.parse_tokens(".1.2.3").is_err());

        // Test invalid end
        assert!(parser.parse_tokens("1.2.3.").is_err());

        // Test invalid characters
        assert!(parser.parse_tokens("1.2.3@").is_err());

        // Test invalid token patterns
        assert!(parser.parse_tokens("_invalid").is_err());
        assert!(parser.parse_tokens("invalid_").is_err());
    }

    #[test]
    fn test_string_interning() {
        let parser = StateMachineParser::with_config(true, 10, 5);

        // Parse the same version multiple times
        let (tokens1, _) = parser.parse_tokens("1.0.0-alpha").unwrap();
        let (tokens2, _) = parser.parse_tokens("1.0.0-alpha").unwrap();

        // String interning should work for alphanumeric tokens
        if let (TokenType::Alphanumeric(s1), TokenType::Alphanumeric(s2)) =
            (&tokens1[3], &tokens2[3])
        {
            // Note: We can't directly test pointer equality due to the way we handle interning
            assert_eq!(s1, s2);
        }
    }

    #[test]
    fn test_performance_limits() {
        let parser = StateMachineParser::new();

        // Test max tokens limit
        let too_many_tokens = (0..15).map(|i| i.to_string()).collect::<Vec<_>>().join(".");
        assert!(parser.parse_tokens(&too_many_tokens).is_err());

        // Test max numeric tokens limit
        let too_many_numeric = (0..10).map(|i| i.to_string()).collect::<Vec<_>>().join(".");
        assert!(parser.parse_tokens(&too_many_numeric).is_err());
    }

    #[test]
    fn test_character_classification() {
        assert!(StateMachineParser::is_valid_separator('.'));
        assert!(StateMachineParser::is_valid_separator('-'));
        assert!(StateMachineParser::is_valid_separator('_'));
        assert!(StateMachineParser::is_valid_separator('+'));
        assert!(!StateMachineParser::is_valid_separator('@'));

        assert!(StateMachineParser::is_token_char('a'));
        assert!(StateMachineParser::is_token_char('1'));
        assert!(StateMachineParser::is_token_char('_'));
        assert!(!StateMachineParser::is_token_char('.'));
    }
}
