"""
Version token system for rez-core.

This module provides access to the rez-compatible version token system,
which is used internally by the Version class for parsing and comparison.
"""

from . import (
    AlphanumericVersionToken,
    NumericToken,
    VersionToken,
)

__all__ = [
    "VersionToken",
    "NumericToken",
    "AlphanumericVersionToken",
    "create_token",
    "parse_token_string",
]


def create_token(token_str):
    """
    Create the appropriate token type for a given string.

    Args:
        token_str (str): The token string to parse

    Returns:
        VersionToken: Either NumericToken or AlphanumericVersionToken

    Raises:
        ValueError: If the token string is invalid
    """
    if not token_str:
        raise ValueError("Token string cannot be empty")

    # Try numeric token first
    if token_str.isdigit():
        return NumericToken(token_str)
    else:
        return AlphanumericVersionToken(token_str)


def parse_token_string(token_str):
    """
    Parse a token string and return information about its structure.

    Args:
        token_str (str): The token string to analyze

    Returns:
        dict: Information about the token including type and properties
    """
    if not token_str:
        return {
            "type": "empty",
            "is_numeric": False,
            "is_alphanumeric": False,
            "length": 0,
        }

    is_numeric = token_str.isdigit()
    is_alphanumeric = token_str.replace("_", "").isalnum()

    return {
        "type": "numeric" if is_numeric else "alphanumeric",
        "is_numeric": is_numeric,
        "is_alphanumeric": is_alphanumeric,
        "length": len(token_str),
        "has_leading_zeros": is_numeric and len(token_str) > 1 and token_str[0] == "0",
        "token_class": NumericToken if is_numeric else AlphanumericVersionToken,
    }
