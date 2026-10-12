//! Configuration validation with source locations.
//!
//! **rez-next extension**: upstream rez validates its config in Python while
//! loading it and reports a Python traceback. This module validates a config
//! file *before* it is loaded and reports each problem as key, expected type,
//! file and line, so an agent can fix the file without reading rez's internals.
//!
//! The scanner here is deliberately separate from the loader: the loader needs
//! full nested semantics, while validation only needs to know where a key was
//! written. Reading the file line by line is what preserves the line number.

use crate::keys::{KeyType, find};
use serde_json::Value as JsonValue;
use std::fmt;
use std::path::{Path, PathBuf};

/// A single configuration problem, with everything needed to fix it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationIssue {
    /// File the offending key was written in.
    pub file: PathBuf,
    /// 1-based line number of the key.
    pub line: usize,
    /// The key that failed validation.
    pub key: String,
    /// The type the registry declares for this key.
    pub expected: String,
    /// The type actually found.
    pub actual: String,
}

impl fmt::Display for ValidationIssue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}:{}: {}: expected {}, got {}",
            self.file.display(),
            self.line,
            self.key,
            self.expected,
            self.actual
        )
    }
}

/// Validate every configuration file that would be loaded.
///
/// Falls back to the CLI's own search path when `paths` is empty, so
/// `config validate` with no `--file` checks exactly what rez-next reads.
pub fn validate_all(paths: &[PathBuf]) -> Vec<ValidationIssue> {
    let owned: Vec<PathBuf>;
    let targets = if paths.is_empty() {
        owned = rez_next_common::config::RezCoreConfig::get_sourced_paths();
        &owned
    } else {
        paths
    };

    targets
        .iter()
        .flat_map(|path| validate_file(path))
        .collect()
}

/// Validate a single configuration file.
///
/// A file that cannot be read or parsed is reported as one issue at line 1
/// rather than silently skipped: a config that never parses hides every other
/// problem in it.
#[must_use]
pub fn validate_file(path: &Path) -> Vec<ValidationIssue> {
    let content = match std::fs::read_to_string(path) {
        Ok(content) => content,
        Err(e) => {
            return vec![ValidationIssue {
                file: path.to_path_buf(),
                line: 1,
                key: "<file>".to_string(),
                expected: "a readable file".to_string(),
                actual: e.to_string(),
            }];
        }
    };

    if content.trim().is_empty() {
        return Vec::new();
    }

    let entries = scan_key_positions(&content);

    let mut issues = Vec::new();
    for (key, line, value_text) in entries {
        let Some(def) = find(&key) else {
            // Unknown keys are allowed: rez resolves plugin settings and
            // package-defined keys that this registry cannot enumerate.
            continue;
        };
        if let Some(actual) = mismatched_type(def.ty, &value_text) {
            issues.push(ValidationIssue {
                file: path.to_path_buf(),
                line,
                key,
                expected: def.ty.to_string(),
                actual,
            });
        }
    }

    issues
}

/// Describe a key type the way an error message should.
impl fmt::Display for KeyType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::PathList => "a list of paths",
            Self::StrList | Self::OptionalStrList => "a list of strings",
            Self::Bool | Self::OptionalBool => "a boolean",
            Self::ForceOrBool => "a boolean or one of force/always/never",
            Self::Int => "an integer",
            Self::Float => "a number",
            Self::Str | Self::OptionalStr => "a string",
            Self::Dict | Self::OptionalDict => "a mapping",
            Self::OptionalDictOrDictList => "a mapping or list of mappings",
            Self::Char => "a single character",
        };
        f.write_str(name)
    }
}

/// Return the observed type name when `value_text` cannot satisfy `ty`.
///
/// `None` means the value is acceptable. Interpretation is intentionally
/// conservative: an unrecognised scalar shape passes, because rejecting a
/// valid config is worse than missing an exotic invalid one.
fn mismatched_type(ty: KeyType, value_text: &str) -> Option<String> {
    let trimmed = value_text.trim();

    if trimmed.is_empty() || trimmed == "null" || trimmed == "~" {
        // An explicit null is a meaningful value for every Optional* key, and
        // for non-optional keys the loader's own defaults cover it.
        return None;
    }

    let observed = observed_kind(trimmed);

    let accepted = accepted_kinds(ty);
    if accepted.contains(&observed) {
        return None;
    }

    Some(observed.to_string())
}

/// The coarse kind of a scalar or collection literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Kind {
    Bool,
    Int,
    Float,
    Str,
    Seq,
    Map,
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Self::Bool => "a boolean",
            Self::Int => "an integer",
            Self::Float => "a number",
            Self::Str => "a string",
            Self::Seq => "a list",
            Self::Map => "a mapping",
        };
        f.write_str(name)
    }
}

/// Which kinds a key type accepts.
fn accepted_kinds(ty: KeyType) -> Vec<Kind> {
    match ty {
        KeyType::PathList | KeyType::StrList => vec![Kind::Seq],
        KeyType::OptionalStrList => vec![Kind::Seq],
        KeyType::Bool => vec![Kind::Bool],
        KeyType::OptionalBool => vec![Kind::Bool],
        KeyType::ForceOrBool => vec![Kind::Bool, Kind::Str],
        KeyType::Int => vec![Kind::Int],
        KeyType::Float => vec![Kind::Int, Kind::Float],
        KeyType::Str => vec![Kind::Str],
        KeyType::OptionalStr => vec![Kind::Str],
        KeyType::Dict => vec![Kind::Map],
        KeyType::OptionalDict => vec![Kind::Map],
        KeyType::OptionalDictOrDictList => vec![Kind::Map, Kind::Seq],
        KeyType::Char => vec![Kind::Str],
    }
}

/// Classify a value literal by its shape.
fn observed_kind(text: &str) -> Kind {
    // `[]` / `{}` are the markers `scan_key_positions` leaves for a key that
    // opened a block, so they are matched before the general bracket rule.
    if text == "[]" {
        return Kind::Seq;
    }
    if text == "{}" {
        return Kind::Map;
    }

    // A bracketed or braced literal is a collection regardless of contents.
    if text.starts_with('[') && text.ends_with(']') {
        return Kind::Seq;
    }
    if text.starts_with('{') && text.ends_with('}') {
        return Kind::Map;
    }

    if matches!(
        text.to_ascii_lowercase().as_str(),
        "true" | "false" | "yes" | "no" | "on" | "off"
    ) {
        return Kind::Bool;
    }

    if text.parse::<i64>().is_ok() {
        return Kind::Int;
    }
    if text.parse::<f64>().is_ok() {
        return Kind::Float;
    }

    Kind::Str
}

/// Flatten a config file into `(key, line, value_text)` triples.
///
/// Handles the two shapes rez config files actually use: YAML block mappings
/// (including one level of nesting) and JSON objects. The line number is the
/// line the key was written on, which is what a validation message needs.
///
/// A key that opens a block (`cache:` followed by indented keys) is recorded
/// with the marker value `"{}"` so a scalar-typed key written as a mapping is
/// still caught, while a key that opens a sequence is recorded as `"[]"`.
fn scan_key_positions(content: &str) -> Vec<(String, usize, String)> {
    let lines: Vec<&str> = content.lines().collect();

    if content.trim_start().starts_with('{') {
        return scan_json_keys(content);
    }

    let mut entries = Vec::new();
    // One level of nesting: the stack holds (indent, key prefix).
    let mut stack: Vec<(usize, String)> = Vec::new();

    for (index, raw) in lines.iter().enumerate() {
        let line = strip_comment(raw);
        if line.trim().is_empty() || line.trim() == "---" {
            continue;
        }

        let indent = leading_spaces(line);
        let trimmed = line.trim();

        // Sequence items belong to the key that introduced them; they carry no
        // key of their own, so they are skipped.
        if trimmed.starts_with("- ") || trimmed == "-" {
            continue;
        }

        let Some((key, value)) = split_key_value(line) else {
            continue;
        };

        // Pop back to the enclosing level.
        while let Some((parent_indent, _)) = stack.last()
            && indent <= *parent_indent
        {
            stack.pop();
        }

        let full_key = match stack.last() {
            Some((_, prefix)) => format!("{prefix}.{key}"),
            None => key.clone(),
        };

        match value {
            // An inline value is validated as written.
            Some(text) => entries.push((full_key, index + 1, text)),
            // No inline value: this key opens either a sequence or a mapping,
            // or is simply empty. Decide which by looking ahead.
            None => {
                let opened = block_kind(&lines, index, indent);
                entries.push((full_key.clone(), index + 1, opened.to_string()));
                // Only a mapping introduces a key prefix; a sequence's items
                // are values, not keys.
                if opened == "{}" {
                    stack.push((indent, full_key));
                }
            }
        }
    }

    entries
}

/// Whether a key with no inline value opens a sequence (`[]`) or a mapping (`{}`).
///
/// Returns `""` when the key is the last one in the file or nothing follows it,
/// which is the YAML spelling of an empty value rather than an empty block.
fn block_kind(lines: &[&str], from: usize, parent_indent: usize) -> &'static str {
    for line in lines.iter().skip(from + 1) {
        let stripped = strip_comment(line);
        if stripped.trim().is_empty() {
            continue;
        }
        let indent = leading_spaces(stripped);
        if indent <= parent_indent {
            // The block is empty or already closed.
            break;
        }
        return if stripped.trim().starts_with('-') {
            "[]"
        } else {
            "{}"
        };
    }

    ""
}

/// Extract top-level keys from a JSON object, with line numbers.
fn scan_json_keys(content: &str) -> Vec<(String, usize, String)> {
    let Ok(value) = serde_json::from_str::<JsonValue>(content) else {
        return Vec::new();
    };

    let Some(object) = value.as_object() else {
        return Vec::new();
    };

    let lines: Vec<&str> = content.lines().collect();

    object
        .keys()
        .filter_map(|key| {
            let line = lines
                .iter()
                .position(|l| l.trim().starts_with(&format!("\"{key}\"")))
                .map(|i| i + 1)?;
            let rendered = serde_json::to_string(&object[key]).unwrap_or_default();
            Some((key.clone(), line, rendered))
        })
        .collect()
}

/// Strip a trailing `#` comment that is not inside a quoted string.
fn strip_comment(line: &str) -> &str {
    let mut in_single = false;
    let mut in_double = false;

    for (index, ch) in line.char_indices() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            '#' if !in_single && !in_double => return &line[..index],
            _ => {}
        }
    }

    line
}

/// Count leading spaces; tabs count as one level.
fn leading_spaces(line: &str) -> usize {
    line.chars().take_while(|c| *c == ' ' || *c == '\t').count()
}

/// Split `  key: value` into the key and the optional inline value.
fn split_key_value(line: &str) -> Option<(String, Option<String>)> {
    let trimmed = line.trim();
    let (key, rest) = trimmed.split_once(':')?;

    let key = key.trim().trim_matches('"').trim_matches('\'').to_string();
    if key.is_empty() {
        return None;
    }

    let rest = rest.trim();
    let value = if rest.is_empty() {
        None
    } else {
        Some(rest.to_string())
    };

    Some((key, value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn write_fixture(name: &str, body: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join(name);
        let mut file = std::fs::File::create(&path).expect("create fixture");
        file.write_all(body.as_bytes()).expect("write fixture");
        (dir, path)
    }

    #[test]
    fn test_valid_config_has_no_issues() {
        let (_dir, path) = write_fixture(
            "ok.yaml",
            "packages_path:\n  - /custom/pkgs\neditor: /usr/bin/vim\n",
        );
        let issues = validate_file(&path);
        assert!(
            issues.is_empty(),
            "a valid config must produce no issues: {issues:?}"
        );
    }

    #[test]
    fn test_wrong_type_reports_key_file_line_and_expected_type() {
        // Regression target: the message must be actionable without reading
        // rez's internals — key, expected type, file and line.
        let (_dir, path) = write_fixture("bad.yaml", "packages_path: /not/a/list\n");

        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "expected exactly one issue: {issues:?}");

        let issue = &issues[0];
        assert_eq!(issue.key, "packages_path");
        assert_eq!(issue.line, 1);
        assert_eq!(issue.file, path);
        assert!(
            issue.expected.contains("list"),
            "expected type must name a list, got: {}",
            issue.expected
        );
        assert!(
            issue.actual.contains("string"),
            "actual type must name a string, got: {}",
            issue.actual
        );
    }

    #[test]
    fn test_issue_display_is_file_line_key_expected_actual() {
        let (_dir, path) = write_fixture("bad.yaml", "packages_path: /not/a/list\n");
        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "{issues:?}");

        let rendered = issues[0].to_string();
        assert!(
            rendered.contains("bad.yaml"),
            "must name the file: {rendered}"
        );
        assert!(rendered.contains(":1:"), "must name the line: {rendered}");
        assert!(
            rendered.contains("packages_path"),
            "must name the key: {rendered}"
        );
        assert!(rendered.contains("expected"), "must name expectation");
    }

    #[test]
    fn test_line_number_points_at_the_offending_key() {
        // The line number is the whole point of this module: a config with a
        // problem on line 4 must say line 4, not line 1.
        let (_dir, path) = write_fixture(
            "multi.yaml",
            "packages_path:\n  - /ok\neditor: /usr/bin/vim\nuse_rust_solver: not-a-bool\n",
        );
        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].line, 4, "must report line 4: {issues:?}");
        assert_eq!(issues[0].key, "use_rust_solver");
    }

    #[test]
    fn test_boolean_key_accepts_rez_forms() {
        for literal in ["true", "false", "yes", "no", "on", "off"] {
            let (_dir, path) = write_fixture("b.yaml", &format!("use_rust_solver: {literal}\n"));
            assert!(
                validate_file(&path).is_empty(),
                "{literal} must be accepted as a boolean"
            );
        }
    }

    #[test]
    fn test_nested_key_is_validated_under_its_parent() {
        // A nested key is reported under its dotted name, so the message points
        // at the inner setting rather than blaming the whole block.
        let (_dir, path) = write_fixture("nested.yaml", "cache:\n  enable_memory_cache: maybe\n");
        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].key, "cache.enable_memory_cache");
        assert_eq!(issues[0].line, 2);
    }

    #[test]
    fn test_scalar_key_written_as_a_mapping_is_reported() {
        // `editor` is a string, but writing it as a nested mapping is a real
        // mistake. The block marker is what makes this detectable: without it
        // the key would look empty and be skipped.
        let (_dir, path) = write_fixture("nested.yaml", "editor:\n  nested: true\n");
        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].key, "editor");
        assert_eq!(issues[0].line, 1);
        assert!(issues[0].expected.contains("string"), "{}", issues[0]);
    }

    #[test]
    fn test_unknown_key_is_not_reported() {
        // rez resolves plugin settings that this registry cannot enumerate, so
        // an unknown key must not be an error.
        let (_dir, path) = write_fixture("unknown.yaml", "some_future_key: 42\n");
        assert!(validate_file(&path).is_empty());
    }

    #[test]
    fn test_empty_file_has_no_issues() {
        let (_dir, path) = write_fixture("empty.yaml", "");
        assert!(validate_file(&path).is_empty());

        let (_dir, path) = write_fixture("blank.yaml", "\n\n# just a comment\n");
        assert!(validate_file(&path).is_empty());
    }

    #[test]
    fn test_comments_are_not_treated_as_values() {
        let (_dir, path) = write_fixture("cmt.yaml", "editor: /usr/bin/vim  # inline comment\n");
        assert!(
            validate_file(&path).is_empty(),
            "trailing comment must not corrupt the value"
        );
    }

    #[test]
    fn test_missing_file_is_reported_not_skipped() {
        let issues = validate_file(Path::new("/definitely/not/here.yaml"));
        assert_eq!(issues.len(), 1);
        assert_eq!(issues[0].line, 1);
        assert_eq!(issues[0].key, "<file>");
    }

    #[test]
    fn test_json_config_is_validated_with_line_numbers() {
        let (_dir, path) =
            write_fixture("cfg.json", "{\n  \"packages_path\": \"/not/a/list\"\n}\n");
        let issues = validate_file(&path);
        assert_eq!(issues.len(), 1, "{issues:?}");
        assert_eq!(issues[0].key, "packages_path");
        assert_eq!(issues[0].line, 2, "must report the JSON line: {issues:?}");
    }

    #[test]
    fn test_explicit_null_is_accepted() {
        for literal in ["null", "~", ""] {
            let (_dir, path) = write_fixture("n.yaml", &format!("editor: {literal}\n"));
            assert!(
                validate_file(&path).is_empty(),
                "{literal} must be accepted for an optional key"
            );
        }
    }

    #[test]
    fn test_validate_all_aggregates_across_files() {
        let (_d1, bad1) = write_fixture("a.yaml", "packages_path: /nope\n");
        let (_d2, bad2) = write_fixture("b.yaml", "editor:\n  x: 1\n");

        let issues = validate_all(&[bad1, bad2]);
        assert_eq!(issues.len(), 2, "both files must be checked: {issues:?}");
    }

    #[test]
    fn test_strip_comment_respects_quotes() {
        assert_eq!(strip_comment("a: b # real"), "a: b ");
        assert_eq!(
            strip_comment("a: \"b # not a comment\""),
            "a: \"b # not a comment\""
        );
        assert_eq!(strip_comment("a: 'b # nope'"), "a: 'b # nope'");
    }
}
