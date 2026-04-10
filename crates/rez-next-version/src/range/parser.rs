//! Version range string parsing — converts range strings into BoundSets.

use super::types::{Bound, BoundSet};
use super::super::Version;
use rez_next_common::RezCoreError;

/// Parse a range string into a vector of BoundSets (disjunction)
pub(super) fn parse_range_str(s: &str) -> Result<Vec<BoundSet>, RezCoreError> {
    if s.is_empty() || s == "*" {
        return Ok(vec![BoundSet::any()]);
    }
    if s == "empty" || s == "!*" {
        return Ok(vec![BoundSet::none()]);
    }

    // Handle rez ".." interval syntax: "1.0..2.0" = ">=1.0,<2.0"
    // Note: must check before splitting on |
    if s.contains("..") && !s.starts_with('.') {
        // Only handle if it's a simple "a..b" form (no | or other operators in the whole string)
        if let Some(dot_pos) = s.find("..") {
            let left = &s[..dot_pos];
            let right = &s[dot_pos + 2..];
            // Both sides must look like version strings (not empty operators)
            if !left.is_empty()
                && !right.is_empty()
                && !left.starts_with('>')
                && !left.starts_with('<')
                && !left.starts_with('=')
                && !left.starts_with('!')
                && !left.starts_with('~')
            {
                // "left..right" -> ">=left,<right"
                let new_s = format!(">={},<{}", left.trim(), right.trim());
                return parse_range_str(&new_s);
            }
        }
    }

    // Split on | for OR (union)
    let or_parts: Vec<&str> = s.split('|').collect();
    let mut result = Vec::new();

    for or_part in or_parts {
        let bound_set = parse_conjunction(or_part.trim())?;
        result.push(bound_set);
    }

    Ok(result)
}

/// Parse a conjunction of constraints (AND semantics)
/// Supports: `>=1.0,<2.0` (comma) or `>=1.0 <2.0` (space) or `1.0+<2.0` (rez syntax)
pub(super) fn parse_conjunction(s: &str) -> Result<BoundSet, RezCoreError> {
    if s.is_empty() || s == "*" {
        return Ok(BoundSet::any());
    }

    // Handle rez shorthand: "1.0+" = ">=1.0"
    // Handle rez shorthand: "1.0+<2.0" = ">=1.0,<2.0"
    // The `+` in rez means "this version and above", with optional upper bound after it
    let s = if s.contains('+')
        && !s.starts_with('>')
        && !s.starts_with('<')
        && !s.starts_with('=')
        && !s.starts_with('!')
        && !s.starts_with('~')
    {
        // Find the + and split: "1.0+<2.0" -> prefix="1.0", suffix="<2.0"
        if let Some(plus_pos) = s.find('+') {
            let prefix = &s[..plus_pos];
            let suffix = &s[plus_pos + 1..];
            if suffix.is_empty() {
                // "1.0+" -> ">=1.0"
                format!(">={}", prefix)
            } else {
                // "1.0+<2.0" -> ">=1.0,<2.0"
                format!(">={},{}", prefix, suffix)
            }
        } else {
            s.to_string()
        }
    } else {
        s.to_string()
    };

    let mut bounds = Vec::new();

    // Split on commas first, then spaces that separate constraints
    let parts = split_constraint_parts(&s);

    for part in parts {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let bound = parse_single_constraint(part)?;
        bounds.push(bound);
    }

    if bounds.is_empty() {
        return Ok(BoundSet::any());
    }

    Ok(BoundSet { bounds })
}

/// Split a string into individual constraint parts (handles comma and space separators)
fn split_constraint_parts(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    for ch in s.chars() {
        if ch == ',' {
            if !current.trim().is_empty() {
                parts.push(current.trim().to_string());
            }
            current = String::new();
        } else {
            current.push(ch);
        }
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }

    // Further split space-separated constraints within each part
    let mut final_parts = Vec::new();
    for part in parts {
        let space_parts = split_on_operator_boundaries(&part);
        final_parts.extend(space_parts);
    }

    final_parts
}

/// Split on spaces that are followed by an operator (>=, <=, >, <, ==, !=, ~=)
fn split_on_operator_boundaries(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut current = String::new();

    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let ch = chars[i];
        if ch == ' ' {
            // Check if next non-space char starts an operator
            let mut j = i + 1;
            while j < chars.len() && chars[j] == ' ' {
                j += 1;
            }
            if j < chars.len() {
                let next = chars[j];
                if next == '>' || next == '<' || next == '=' || next == '!' || next == '~' {
                    if !current.trim().is_empty() {
                        parts.push(current.trim().to_string());
                    }
                    current = String::new();
                    i = j;
                    continue;
                }
            }
            current.push(ch);
        } else {
            current.push(ch);
        }
        i += 1;
    }
    if !current.trim().is_empty() {
        parts.push(current.trim().to_string());
    }
    parts
}

/// Parse a single constraint like `>=1.0`, `<2.0`, `==1.5`, `!=1.0`, `~=1.4`
pub(super) fn parse_single_constraint(s: &str) -> Result<Bound, RezCoreError> {
    let s = s.trim();

    if s.is_empty() || s == "*" {
        return Ok(Bound::Any);
    }

    // Try two-char operators first
    if let Some(rest) = s.strip_prefix(">=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Ge(v));
    }
    if let Some(rest) = s.strip_prefix("<=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Le(v));
    }
    if let Some(rest) = s.strip_prefix("==") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Eq(v));
    }
    if let Some(rest) = s.strip_prefix("!=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Ne(v));
    }
    if let Some(rest) = s.strip_prefix("~=") {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Compatible(v));
    }

    // Single-char operators
    if let Some(rest) = s.strip_prefix('>') {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Gt(v));
    }
    if let Some(rest) = s.strip_prefix('<') {
        let v = Version::parse(rest.trim()).map_err(|e| {
            RezCoreError::VersionRange(format!("Invalid version in range '{}': {}", s, e))
        })?;
        return Ok(Bound::Lt(v));
    }

    // No operator - treat as exact version (rez: bare version = "==version")
    let v = Version::parse(s).map_err(|e| {
        RezCoreError::VersionRange(format!("Invalid version constraint '{}': {}", s, e))
    })?;
    Ok(Bound::Eq(v))
}
