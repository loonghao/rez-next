//! Simple parser test without Python dependencies

use std::time::Instant;

// Simplified version of our parser for testing
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Numeric(u64),
    Alphanumeric(String),
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ParseState {
    Start,
    InToken,
    InSeparator,
}

pub struct SimplifiedParser;

impl Default for SimplifiedParser {
    fn default() -> Self {
        Self::new()
    }
}

impl SimplifiedParser {
    pub fn new() -> Self {
        Self
    }

    #[inline(always)]
    fn is_valid_separator(c: char) -> bool {
        matches!(c, '.' | '-' | '_' | '+')
    }

    #[inline(always)]
    fn is_token_char(c: char) -> bool {
        c.is_ascii_alphanumeric() || c == '_'
    }

    pub fn parse_tokens(&self, input: &str) -> Result<(Vec<TokenType>, Vec<char>), String> {
        if input.is_empty() {
            return Ok((Vec::new(), Vec::new()));
        }

        let mut tokens = Vec::new();
        let mut separators = Vec::new();
        let mut state = ParseState::Start;
        let mut current_token = String::new();

        let chars: Vec<char> = input.chars().collect();
        let mut i = 0;

        while i < chars.len() {
            let c = chars[i];

            match state {
                ParseState::Start => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                        state = ParseState::InToken;
                    } else if Self::is_valid_separator(c) {
                        return Err(format!("Version cannot start with separator '{}'", c));
                    } else {
                        return Err(format!("Invalid character '{}' at start of version", c));
                    }
                }

                ParseState::InToken => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                    } else if Self::is_valid_separator(c) {
                        // Finalize current token
                        self.finalize_token(&mut current_token, &mut tokens)?;
                        separators.push(c);
                        state = ParseState::InSeparator;
                    } else {
                        return Err(format!("Invalid character '{}' in token", c));
                    }
                }

                ParseState::InSeparator => {
                    if Self::is_token_char(c) {
                        current_token.push(c);
                        state = ParseState::InToken;
                    } else {
                        return Err(format!(
                            "Expected token character after separator, found '{}'",
                            c
                        ));
                    }
                }
            }

            i += 1;
        }

        // Finalize last token if we're in a token state
        if state == ParseState::InToken && !current_token.is_empty() {
            self.finalize_token(&mut current_token, &mut tokens)?;
        } else if state == ParseState::InSeparator {
            return Err("Version cannot end with separator".to_string());
        }

        Ok((tokens, separators))
    }

    fn finalize_token(
        &self,
        current_token: &mut String,
        tokens: &mut Vec<TokenType>,
    ) -> Result<(), String> {
        if current_token.is_empty() {
            return Err("Empty token found".to_string());
        }

        // Try to parse as numeric first (fast path)
        if current_token.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(num) = current_token.parse::<u64>() {
                tokens.push(TokenType::Numeric(num));
            } else {
                // Number too large, treat as alphanumeric
                tokens.push(TokenType::Alphanumeric(current_token.clone()));
            }
        } else {
            // Alphanumeric token
            tokens.push(TokenType::Alphanumeric(current_token.clone()));
        }

        current_token.clear();
        Ok(())
    }
}

fn main() {
    println!("🚀 Simplified Parser Performance Test");
    println!("====================================");

    let parser = SimplifiedParser::new();

    // Test cases
    let test_versions = vec![
        "1.2.3",
        "1.2.3-alpha.1",
        "2.0.0-beta.2",
        "1.0.0-rc.1",
        "3.1.4-dev.123",
        "10.20.30",
        "1.2.3-alpha1.beta2.gamma3",
        "0.1.0",
        "1.0.0-alpha",
        "2.1.0-beta.1",
    ];

    println!("\n📊 Testing Parser Performance");
    println!("-----------------------------");

    // Warm up
    for _ in 0..1000 {
        for version_str in &test_versions {
            let _ = parser.parse_tokens(version_str);
        }
    }

    // Performance test
    let iterations = 100000;
    let start = Instant::now();

    for _ in 0..iterations {
        for version_str in &test_versions {
            match parser.parse_tokens(version_str) {
                Ok((tokens, _separators)) => {
                    // Verify basic parsing worked
                    if tokens.is_empty() && !version_str.is_empty() {
                        eprintln!("❌ Failed to parse: {}", version_str);
                    }
                }
                Err(e) => {
                    eprintln!("❌ Error parsing '{}': {}", version_str, e);
                }
            }
        }
    }

    let duration = start.elapsed();
    let total_parses = iterations * test_versions.len();
    let parses_per_second = total_parses as f64 / duration.as_secs_f64();

    println!("✅ Completed {} parses in {:?}", total_parses, duration);
    println!("🎯 Performance: {:.0} parses/second", parses_per_second);
    println!(
        "📈 Average time per parse: {:.2} μs",
        duration.as_micros() as f64 / total_parses as f64
    );

    // Test individual version parsing
    println!("\n🔍 Individual Version Analysis");
    println!("------------------------------");

    for version_str in &test_versions {
        let start = Instant::now();
        match parser.parse_tokens(version_str) {
            Ok((tokens, separators)) => {
                let duration = start.elapsed();
                println!(
                    "✅ '{}' -> {} tokens, {} separators ({:.2} μs)",
                    version_str,
                    tokens.len(),
                    separators.len(),
                    duration.as_micros()
                );
            }
            Err(e) => {
                println!("❌ '{}' -> Error: {}", version_str, e);
            }
        }
    }

    println!("\n🎉 Performance test completed!");

    // Performance target check
    if parses_per_second > 5000.0 {
        println!("🎯 SUCCESS: Achieved target of >5000 parses/second!");
        println!(
            "🚀 Current performance: {:.0}x faster than target!",
            parses_per_second / 5000.0
        );
    } else {
        println!(
            "⚠️  Target not met: {} < 5000 parses/second",
            parses_per_second as u64
        );
    }
}
