"""
Error handling for rez-core.

This module provides access to all error types used by rez-core,
with proper inheritance and error handling patterns.
"""

from . import (
    RezCoreError,
    VersionParseError,
)

__all__ = [
    "RezCoreError",
    "VersionParseError",
    "VersionError",
    "RangeParseError",
    "TokenError",
]

# Additional error types for better error categorization
class VersionError(VersionParseError):
    """Raised when there's an error with version operations."""
    pass

class RangeParseError(VersionParseError):
    """Raised when there's an error parsing version ranges."""
    pass

class TokenError(VersionParseError):
    """Raised when there's an error with version tokens."""
    pass

def handle_version_error(func):
    """
    Decorator to handle version-related errors consistently.
    
    This decorator catches low-level errors and re-raises them
    as appropriate high-level error types.
    """
    def wrapper(*args, **kwargs):
        try:
            return func(*args, **kwargs)
        except VersionParseError:
            raise  # Re-raise as-is
        except ValueError as e:
            if "version" in str(e).lower():
                raise VersionError(str(e)) from e
            elif "range" in str(e).lower():
                raise RangeParseError(str(e)) from e
            elif "token" in str(e).lower():
                raise TokenError(str(e)) from e
            else:
                raise VersionError(str(e)) from e
        except Exception as e:
            raise RezCoreError(f"Unexpected error: {e}") from e
    
    return wrapper
