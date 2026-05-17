//! String utility functions

use std::fmt::Display;

/// Normalize a package name (convert to lowercase, replace spaces/hyphens with underscores)
pub fn normalize_name(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| if c == ' ' || c == '-' { '_' } else { c })
        .collect()
}

/// Check if a string is a valid identifier (alphanumeric and underscores, starting with a letter or underscore)
pub fn is_valid_identifier(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let mut chars = s.chars();
    let first = chars.next().unwrap();

    if !first.is_alphabetic() && first != '_' {
        return false;
    }

    chars.all(|c| c.is_alphanumeric() || c == '_')
}

/// Truncate a string to a maximum length, adding "..." if truncated
pub fn truncate(s: &str, max_len: usize) -> String {
    if max_len <= 3 {
        "...".to_string()
    } else if s.len() <= max_len - 3 {
        s.to_string()
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Indent each line of a string by a given number of spaces
pub fn indent(s: &str, spaces: usize) -> String {
    let indent_str = " ".repeat(spaces);
    s.lines()
        .map(|line| format!("{}{}", indent_str, line))
        .collect::<Vec<_>>()
        .join("\n")
}

/// Convert a string to a Python-compatible identifier (PEP 8 style)
pub fn to_python_identifier(s: &str) -> String {
    // Replace hyphens and spaces with underscores
    let s = s.replace(['-', ' '], "_");

    // Remove any characters that are not alphanumeric or underscore
    let s: String = s
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect();

    // Ensure it starts with a letter or underscore
    if s.is_empty() || !s.chars().next().unwrap().is_alphabetic() {
        format!("_{}", s)
    } else {
        s
    }
}

/// Format a list of items as a human-readable string (e.g., "a, b, and c")
pub fn format_list<T: Display>(items: &[T], conjunction: &str) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].to_string(),
        2 => format!("{} {} {}", items[0], conjunction, items[1]),
        _ => {
            let mut result = String::new();
            let last_idx = items.len() - 1;

            for (i, item) in items.iter().enumerate() {
                if i == 0 {
                    result.push_str(&item.to_string());
                } else if i == last_idx {
                    result.push_str(&format!(", {} {}", conjunction, item));
                } else {
                    result.push_str(", ");
                    result.push_str(&item.to_string());
                }
            }

            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_name() {
        assert_eq!(normalize_name("Hello World"), "hello_world");
        assert_eq!(normalize_name("maya-2024"), "maya_2024");
        assert_eq!(normalize_name("Python3"), "python3");
    }

    #[test]
    fn test_is_valid_identifier() {
        assert!(is_valid_identifier("hello"));
        assert!(is_valid_identifier("_hello"));
        assert!(is_valid_identifier("hello123"));
        assert!(!is_valid_identifier("123hello"));
        assert!(!is_valid_identifier(""));
        assert!(!is_valid_identifier("hello world"));
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        assert_eq!(truncate("hi", 3), "...");
    }

    #[test]
    fn test_indent() {
        assert_eq!(indent("hello", 2), "  hello");
        assert_eq!(indent("hello\nworld", 2), "  hello\n  world");
    }

    #[test]
    fn test_to_python_identifier() {
        assert_eq!(to_python_identifier("hello-world"), "hello_world");
        assert_eq!(to_python_identifier("123hello"), "_123hello");
        assert_eq!(to_python_identifier("hello world"), "hello_world");
    }

    #[test]
    fn test_format_list() {
        assert_eq!(format_list::<String>(&[], "and"), "");
        assert_eq!(format_list(&["a".to_string()], "and"), "a");
        assert_eq!(
            format_list(&["a".to_string(), "b".to_string()], "and"),
            "a and b"
        );
        assert_eq!(
            format_list(&["a".to_string(), "b".to_string(), "c".to_string()], "and"),
            "a, b, and c"
        );
    }
}
