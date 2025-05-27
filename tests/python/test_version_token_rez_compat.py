"""
Test VersionToken compatibility with original rez.

This module tests the VersionToken system to ensure it behaves exactly
like the original rez implementation.
"""

import pytest
import rez_core


class TestVersionTokenCompatibility:
    """Test VersionToken compatibility with rez."""

    def test_numeric_token_creation(self):
        """Test NumericToken creation and basic functionality."""
        token = rez_core.NumericToken("123")
        assert str(token) == "123"
        
    def test_numeric_token_comparison(self):
        """Test NumericToken comparison logic."""
        t1 = rez_core.NumericToken("1")
        t2 = rez_core.NumericToken("2")
        t10 = rez_core.NumericToken("10")
        
        assert t1 < t2
        assert t2 < t10
        assert t1 <= t2
        assert t2 >= t1
        assert t1 != t2
        assert t1 == rez_core.NumericToken("1")
        
    def test_numeric_token_next(self):
        """Test NumericToken next() method."""
        token = rez_core.NumericToken("5")
        next_token = token.next()
        # The next token should be a new NumericToken with value 6
        assert str(next_token) == "6"
        
    def test_alphanumeric_token_creation(self):
        """Test AlphanumericVersionToken creation."""
        token = rez_core.AlphanumericVersionToken("alpha1")
        assert str(token) == "alpha1"
        
    def test_alphanumeric_token_comparison_basic(self):
        """Test basic AlphanumericVersionToken comparison."""
        # Test from rez documentation examples
        t3 = rez_core.AlphanumericVersionToken("3")
        t4 = rez_core.AlphanumericVersionToken("4")
        assert t3 < t4
        
        # Test padding sensitivity: "01" < "1"
        t01 = rez_core.AlphanumericVersionToken("01")
        t1 = rez_core.AlphanumericVersionToken("1")
        assert t01 < t1
        
    def test_alphanumeric_token_alpha_vs_numeric(self):
        """Test that alphas come before numbers."""
        # "beta" < "1"
        beta = rez_core.AlphanumericVersionToken("beta")
        one = rez_core.AlphanumericVersionToken("1")
        assert beta < one
        
    def test_alphanumeric_token_complex_comparison(self):
        """Test complex AlphanumericVersionToken comparisons."""
        # "alpha3" < "alpha4"
        alpha3 = rez_core.AlphanumericVersionToken("alpha3")
        alpha4 = rez_core.AlphanumericVersionToken("alpha4")
        assert alpha3 < alpha4
        
        # "alpha" < "alpha3"
        alpha = rez_core.AlphanumericVersionToken("alpha")
        assert alpha < alpha3
        
        # "gamma33" < "33gamma"
        gamma33 = rez_core.AlphanumericVersionToken("gamma33")
        gamma_33 = rez_core.AlphanumericVersionToken("33gamma")
        assert gamma33 < gamma_33
        
    def test_alphanumeric_token_next(self):
        """Test AlphanumericVersionToken next() method."""
        # Test with alpha ending
        alpha = rez_core.AlphanumericVersionToken("alpha")
        next_alpha = alpha.next()
        assert str(next_alpha) == "alpha_"
        
        # Test with numeric ending
        alpha1 = rez_core.AlphanumericVersionToken("alpha1")
        next_alpha1 = alpha1.next()
        assert str(next_alpha1) == "alpha1_"
        
    def test_token_random_string_generation(self):
        """Test random token string generation."""
        # NumericToken should generate numeric strings
        numeric_random = rez_core.NumericToken.create_random_token_string()
        assert len(numeric_random) == 8
        assert numeric_random.isdigit()
        
        # AlphanumericVersionToken should generate alphanumeric strings
        alpha_random = rez_core.AlphanumericVersionToken.create_random_token_string()
        assert len(alpha_random) == 8
        assert alpha_random.isalnum()
        
    def test_token_invalid_input(self):
        """Test that invalid inputs raise appropriate errors."""
        # NumericToken should reject non-numeric input
        with pytest.raises(ValueError):
            rez_core.NumericToken("abc")
            
        # AlphanumericVersionToken should reject invalid characters
        with pytest.raises(ValueError):
            rez_core.AlphanumericVersionToken("alpha-beta")  # dash not allowed
            
    def test_version_token_abstract(self):
        """Test that VersionToken is abstract and cannot be instantiated."""
        with pytest.raises(NotImplementedError):
            rez_core.VersionToken("test")


class TestVersionTokenEdgeCases:
    """Test edge cases for VersionToken system."""
    
    def test_empty_token(self):
        """Test handling of empty tokens."""
        # AlphanumericVersionToken should handle empty string
        empty_token = rez_core.AlphanumericVersionToken("")
        assert str(empty_token) == ""
        
    def test_leading_zeros_comparison(self):
        """Test detailed leading zeros comparison."""
        # This is a key rez behavior: "01" < "1" because of string comparison after numeric equality
        tokens = [
            rez_core.AlphanumericVersionToken("1"),
            rez_core.AlphanumericVersionToken("01"),
            rez_core.AlphanumericVersionToken("001"),
        ]
        
        # Sort and verify order: "001" < "01" < "1"
        sorted_tokens = sorted(tokens)
        assert str(sorted_tokens[0]) == "001"
        assert str(sorted_tokens[1]) == "01"
        assert str(sorted_tokens[2]) == "1"
        
    def test_mixed_alphanumeric_parsing(self):
        """Test parsing of mixed alphanumeric tokens."""
        # Test complex token like "alpha1beta2"
        complex_token = rez_core.AlphanumericVersionToken("alpha1beta2")
        assert str(complex_token) == "alpha1beta2"
        
        # Should be parsed as: ["alpha", "1", "beta", "2"]
        # We can't directly access subtokens, but we can test comparison behavior
        
    def test_underscore_handling(self):
        """Test underscore handling in tokens."""
        underscore_token = rez_core.AlphanumericVersionToken("test_version")
        assert str(underscore_token) == "test_version"
        
    def test_token_equality(self):
        """Test token equality behavior."""
        t1 = rez_core.AlphanumericVersionToken("alpha1")
        t2 = rez_core.AlphanumericVersionToken("alpha1")
        t3 = rez_core.AlphanumericVersionToken("alpha2")
        
        assert t1 == t2
        assert t1 != t3
        assert not (t1 != t2)
        assert not (t1 == t3)


class TestVersionTokenPerformance:
    """Test VersionToken performance characteristics."""
    
    def test_token_creation_performance(self):
        """Test that token creation is reasonably fast."""
        import time
        
        start_time = time.time()
        for i in range(1000):
            rez_core.AlphanumericVersionToken(f"alpha{i}")
        end_time = time.time()
        
        # Should be able to create 1000 tokens quickly
        assert (end_time - start_time) < 1.0
        
    def test_token_comparison_performance(self):
        """Test that token comparison is reasonably fast."""
        import time
        
        tokens = [rez_core.AlphanumericVersionToken(f"token{i}") for i in range(100)]
        
        start_time = time.time()
        for _ in range(10):
            sorted(tokens)
        end_time = time.time()
        
        # Should be able to sort tokens quickly
        assert (end_time - start_time) < 1.0
