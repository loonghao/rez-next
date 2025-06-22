//! Tests for VersionToken compatibility with rez

use crate::version_token::{AlphanumericVersionToken, NumericToken, SubToken};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_subtoken_comparison() {
        // Test basic string comparison
        let alpha1 = SubToken::new("alpha".to_string());
        let alpha2 = SubToken::new("beta".to_string());
        assert!(alpha1 < alpha2);

        // Test string vs number (alphas < numbers)
        let alpha = SubToken::new("alpha".to_string());
        let num = SubToken::new("1".to_string());
        assert!(alpha < num);

        // Test numeric comparison
        let num1 = SubToken::new("1".to_string());
        let num2 = SubToken::new("2".to_string());
        assert!(num1 < num2);

        // Test padding comparison - this is the key rez behavior
        let num_padded = SubToken::new("01".to_string());
        let num_unpadded = SubToken::new("1".to_string());
        // In rez: "01" < "1" because (1, "01") < (1, "1")
        assert!(num_padded < num_unpadded);

        // Test more padding cases
        let num_001 = SubToken::new("001".to_string());
        let num_01 = SubToken::new("01".to_string());
        let num_1 = SubToken::new("1".to_string());

        // Order should be: "001" < "01" < "1"
        assert!(num_001 < num_01);
        assert!(num_01 < num_1);
        assert!(num_001 < num_1);
    }

    #[test]
    fn test_alphanumeric_token_parsing() {
        // Test simple alphanumeric token
        let token = AlphanumericVersionToken::parse_token("alpha1beta2");
        assert_eq!(token.len(), 4);
        assert_eq!(token[0].as_str(), "alpha");
        assert_eq!(token[1].as_str(), "1");
        assert_eq!(token[2].as_str(), "beta");
        assert_eq!(token[3].as_str(), "2");

        // Test token starting with number
        let token = AlphanumericVersionToken::parse_token("1alpha");
        assert_eq!(token.len(), 2);
        assert_eq!(token[0].as_str(), "1");
        assert_eq!(token[1].as_str(), "alpha");

        // Test token ending with number
        let token = AlphanumericVersionToken::parse_token("alpha1");
        assert_eq!(token.len(), 2);
        assert_eq!(token[0].as_str(), "alpha");
        assert_eq!(token[1].as_str(), "1");
    }

    #[test]
    fn test_alphanumeric_token_comparison() {
        // Test basic comparison
        let token1 = AlphanumericVersionToken::parse_token("alpha");
        let token2 = AlphanumericVersionToken::parse_token("beta");
        assert!(token1 < token2);

        // Test alpha vs numeric
        let alpha_token = AlphanumericVersionToken::parse_token("alpha");
        let numeric_token = AlphanumericVersionToken::parse_token("1");
        assert!(alpha_token < numeric_token);

        // Test complex comparison
        let token1 = AlphanumericVersionToken::parse_token("1alpha");
        let token2 = AlphanumericVersionToken::parse_token("1beta");
        assert!(token1 < token2);

        // Test padding in complex tokens
        let token1 = AlphanumericVersionToken::parse_token("01alpha");
        let token2 = AlphanumericVersionToken::parse_token("1alpha");
        assert!(token1 < token2);
    }

    #[test]
    fn test_rez_compatibility_examples() {
        // These are examples from rez documentation and tests

        // Test that "01" < "1" for numeric subtokens
        let token1 = AlphanumericVersionToken::parse_token("01");
        let token2 = AlphanumericVersionToken::parse_token("1");
        assert!(token1 < token2);

        // Test alpha ordering
        let tokens = vec![
            AlphanumericVersionToken::parse_token("_"),
            AlphanumericVersionToken::parse_token("A"),
            AlphanumericVersionToken::parse_token("Z"),
            AlphanumericVersionToken::parse_token("a"),
            AlphanumericVersionToken::parse_token("z"),
            AlphanumericVersionToken::parse_token("1"),
        ];

        // Verify ordering: _, A-Z, a-z, then numbers
        for i in 0..tokens.len() - 1 {
            assert!(
                tokens[i] < tokens[i + 1],
                "Expected token {} < token {}",
                i,
                i + 1
            );
        }
    }
}
