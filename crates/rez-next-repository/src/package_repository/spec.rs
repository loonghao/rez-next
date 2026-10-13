//! Parsing of upstream's `type@location` repository specifications.
//!
//! rez allows `packages_path` entries to name their repository type
//! explicitly, e.g. `filesystem@/packages/int` or `memory@foo`. A bare
//! path is still accepted and means a filesystem repository.
//!
//! Parsing resolves an entry into one of three buckets, exposed as
//! [`SpecParse`]:
//!
//! * a **known type** (`filesystem`, `memory`) - a typed spec;
//! * a **path** - the entry names no known type but exists on disk, so it is
//!   a filesystem repository whose name merely contains an `@`;
//! * an **unknown type** - the entry names no known type and is not an
//!   existing path either. Upstream rez raises `RezPluginError` here
//!   (`src/rez/package_repository.py`), so this is the bucket callers must
//!   report rather than silently coerce into a path.

use std::path::{Path, PathBuf};

use super::FILESYSTEM_REPO_TYPE;

/// A `packages_path` entry resolved into a repository type and location.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepositorySpec {
    /// Repository type name, matching `PackageRepository::repository_type`.
    pub repo_type: String,
    /// Repository location: a path for `filesystem`, a name otherwise.
    pub location: String,
}

/// The outcome of parsing a `packages_path` entry.
///
/// Parsing distinguishes "this entry names a type we know" from "this entry
/// does not resolve to anything at all". Collapsing the two would either
/// misread a real `pkgs@v1` directory as an unknown type, or silently turn a
/// genuine typo such as `server@host/path` into a path that does not exist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SpecParse {
    /// The entry names a repository type this build knows about.
    Typed(RepositorySpec),
    /// The entry names no known type, but the whole entry is an existing
    /// path. Treated as a filesystem repository.
    Path(RepositorySpec),
    /// The entry names no known type and is not an existing path.
    UnknownType {
        /// The prefix that was read as a type name.
        repo_type: String,
        /// The whole entry, verbatim.
        entry: String,
    },
}

/// The type name a `packages_path` entry asked for but that is not available.
///
/// Upstream rez raises `RezPluginError` for an unregistered repository type;
/// this carries the same information so a caller can decide whether to fail
/// or to warn, per the surface it is serving.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownRepoType {
    /// The prefix that was read as a repository type name.
    pub repo_type: String,
    /// The whole `packages_path` entry, verbatim.
    pub entry: String,
}

impl std::fmt::Display for UnknownRepoType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unrecognized repository type {:?} in packages_path entry {:?}",
            self.repo_type, self.entry
        )
    }
}

impl std::error::Error for UnknownRepoType {}

impl SpecParse {
    /// The resolved filesystem spec, for the [`Typed`](Self::Typed) and
    /// [`Path`](Self::Path) buckets.
    pub fn spec(&self) -> Option<&RepositorySpec> {
        match self {
            Self::Typed(spec) | Self::Path(spec) => Some(spec),
            Self::UnknownType { .. } => None,
        }
    }

    /// Whether this entry resolved to a filesystem repository.
    pub fn is_filesystem(&self) -> bool {
        self.spec().is_some_and(|spec| spec.is_filesystem())
    }

    /// Whether this entry named a type that is not available.
    pub fn is_unknown_type(&self) -> bool {
        matches!(self, Self::UnknownType { .. })
    }

    /// The unknown-type detail, when this entry failed to resolve.
    pub fn unknown_type(&self) -> Option<UnknownRepoType> {
        match self {
            Self::UnknownType { repo_type, entry } => Some(UnknownRepoType {
                repo_type: repo_type.clone(),
                entry: entry.clone(),
            }),
            _ => None,
        }
    }
}

impl RepositorySpec {
    /// Parse a `packages_path` entry.
    ///
    /// `filesystem` is the default, so a bare path keeps working. A leading
    /// `type@` prefix selects another repository type; the remainder is the
    /// location and is kept verbatim so that types such as `memory` (whose
    /// "location" is just a name) are not mangled by path normalization.
    ///
    /// Windows drive letters contain a colon rather than an `@`, so a path
    /// like `C:\packages` never looks like a type prefix.
    ///
    /// This never reports an unknown type. It is the non-failing shape kept
    /// for callers that already decide their own diagnostics; new code should
    /// prefer [`parse_spec`](Self::parse_spec), which distinguishes an
    /// unknown type from a plain path.
    pub fn parse(entry: &str) -> Self {
        Self::parse_spec(entry)
            .spec()
            .cloned()
            .unwrap_or_else(|| Self {
                repo_type: FILESYSTEM_REPO_TYPE.to_string(),
                location: entry.to_string(),
            })
    }

    /// Parse a `packages_path` entry, distinguishing an unknown type.
    ///
    /// Resolution is three-way:
    ///
    /// * the prefix names a type upstream defines - a typed spec, whether or
    ///   not this build implements it. `memory` is upstream's example and is
    ///   preserved as a type so callers see the requested type rather than a
    ///   silently rewritten path;
    /// * the prefix names no known type but the whole entry is an existing
    ///   path - a filesystem repository. A directory may legally be named
    ///   `pkgs@v1` on both Unix and Windows, so an entry that resolves on
    ///   disk must keep resolving;
    /// * neither - [`SpecParse::UnknownType`], which the caller reports.
    ///   Upstream rez raises `RezPluginError` here rather than resolving to a
    ///   path that does not exist.
    pub fn parse_spec(entry: &str) -> SpecParse {
        let Some((repo_type, location)) = entry.split_once('@') else {
            return SpecParse::Path(Self::filesystem(entry));
        };

        if !is_type_prefix(repo_type) || location.is_empty() {
            // The prefix holds path syntax, or there is no location at all:
            // this is a path that happens to contain an `@`.
            return SpecParse::Path(Self::filesystem(entry));
        }

        if is_known_type(repo_type) {
            return SpecParse::Typed(Self {
                repo_type: repo_type.to_string(),
                location: location.to_string(),
            });
        }

        // Identifier-shaped prefix naming no known type. Either it is a real
        // directory whose name contains an `@`, or the entry asks for a
        // repository type that does not exist - the latter is a configuration
        // error and must not be coerced into a path.
        if Path::new(entry).exists() {
            SpecParse::Path(Self::filesystem(entry))
        } else {
            SpecParse::UnknownType {
                repo_type: repo_type.to_string(),
                entry: entry.to_string(),
            }
        }
    }

    /// A filesystem repository at `location`.
    fn filesystem(location: &str) -> Self {
        Self {
            repo_type: FILESYSTEM_REPO_TYPE.to_string(),
            location: location.to_string(),
        }
    }

    /// Whether this spec names the filesystem repository type.
    pub fn is_filesystem(&self) -> bool {
        self.repo_type == FILESYSTEM_REPO_TYPE
    }

    /// The location as a path, for filesystem repositories.
    pub fn path(&self) -> PathBuf {
        Path::new(&self.location).to_path_buf()
    }
}

/// Whether `s` could be a repository type name.
///
/// Type names are short identifiers, so this rejects anything containing path
/// syntax: separators, drive letters, dots and whitespace. That is what keeps
/// a bare path holding an `@` (a ZFS/btrfs subvolume such as `@pkgs`, or any
/// directory simply named with an `@`) from being read as a type prefix.
///
/// This is the shape half of the [`is_known_type`] check. Every name on the
/// allowlist is identifier-shaped, so the shape check currently never changes
/// a branch result on its own; it is kept as a guardrail so that adding a
/// type name containing path syntax to the allowlist cannot silently push
/// real paths into the typed branch.
fn is_type_prefix(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Whether `s` names a repository type that upstream rez defines.
///
/// The allowlist names the types upstream rez ships, so an entry is read as a
/// typed spec only when the prefix is one of them. Types listed here need not
/// be implemented by this build - `memory` is upstream's example and is
/// preserved as a type so callers see the requested type rather than a
/// silently rewritten path.
const KNOWN_REPO_TYPES: &[&str] = &[FILESYSTEM_REPO_TYPE, "memory"];

fn is_known_type(s: &str) -> bool {
    KNOWN_REPO_TYPES.contains(&s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bare_path_defaults_to_filesystem() {
        let spec = RepositorySpec::parse("/packages/int");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "/packages/int");
        assert!(spec.is_filesystem());
    }

    #[test]
    fn explicit_filesystem_prefix_is_accepted() {
        let spec = RepositorySpec::parse("filesystem@/packages/int");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "/packages/int");
    }

    #[test]
    fn non_filesystem_type_is_preserved() {
        let spec = RepositorySpec::parse("memory@local");
        assert_eq!(spec.repo_type, "memory");
        assert_eq!(spec.location, "local");
        assert!(!spec.is_filesystem());
    }

    #[test]
    fn empty_halves_fall_back_to_filesystem() {
        // A trailing or leading `@` is not a type prefix: treat the whole
        // entry as a path so a stray separator cannot silently produce a
        // repository with an empty location.
        assert_eq!(RepositorySpec::parse("@/pkgs").repo_type, "filesystem");
        assert_eq!(RepositorySpec::parse("/pkgs").location, "/pkgs");
        let trailing = RepositorySpec::parse("filesystem@");
        assert_eq!(trailing.repo_type, "filesystem");
        assert_eq!(trailing.location, "filesystem@");
    }

    #[test]
    fn windows_drive_letter_is_not_a_type_prefix() {
        let spec = RepositorySpec::parse("C:\\packages");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "C:\\packages");
    }

    #[test]
    fn bare_path_with_at_is_filesystem() {
        // A path may contain an `@` without naming a repository type: ZFS and
        // btrfs subvolumes are conventionally named `@something`, and any
        // directory can simply have an `@` in it. Only a well-formed type
        // name is a prefix, so these stay usable paths.
        for entry in [
            "/packages/@studio/tools",
            "/mnt/@pkgs/rez",
            r"C:\my@pkgs",
            "/pkgs/@",
        ] {
            let spec = RepositorySpec::parse(entry);
            assert_eq!(spec.repo_type, "filesystem", "entry: {entry}");
            assert_eq!(spec.location, entry, "entry: {entry}");
            assert!(spec.is_filesystem(), "entry: {entry}");
        }
    }

    #[test]
    fn type_prefix_must_look_like_an_identifier() {
        // Path syntax in the prefix means it is not a type name.
        let spec = RepositorySpec::parse("sub/dir@/packages");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "sub/dir@/packages");
    }

    #[test]
    fn identifier_shaped_prefix_that_is_not_a_registered_type_is_a_path() {
        // A real directory named `pkgs@v1` is legal on Unix and Windows. The
        // prefix is identifier-shaped, but `pkgs` is not a repository type
        // this build provides, so the entry must stay a filesystem path
        // rather than being read as a typed spec and silently dropped.
        let spec = RepositorySpec::parse("pkgs@v1");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "pkgs@v1");
        assert!(spec.is_filesystem());
    }

    #[test]
    fn path_helper_round_trips_the_location() {
        let spec = RepositorySpec::parse("filesystem@/packages/int");
        assert_eq!(spec.path(), PathBuf::from("/packages/int"));
    }

    #[test]
    fn unknown_type_is_reported_instead_of_a_path() {
        // `server` is not a repository type upstream rez defines, and
        // `server@host/path` is not an existing path either. Upstream raises
        // RezPluginError here; coercing the entry into a path would leave a
        // configuration error to surface as a silently missing repository.
        let parsed = RepositorySpec::parse_spec("server@host/path");
        assert_eq!(
            parsed.unknown_type(),
            Some(UnknownRepoType {
                repo_type: "server".to_string(),
                entry: "server@host/path".to_string(),
            })
        );
        assert!(parsed.is_unknown_type());
        assert!(parsed.spec().is_none());
        assert!(!parsed.is_filesystem());
    }

    #[test]
    fn unknown_type_message_names_the_type_and_the_entry() {
        let message = RepositorySpec::parse_spec("server@host/path")
            .unknown_type()
            .unwrap()
            .to_string();
        assert!(message.contains("server"), "message: {message}");
        assert!(message.contains("server@host/path"), "message: {message}");
    }

    #[test]
    fn known_and_path_buckets_are_not_unknown() {
        assert!(RepositorySpec::parse_spec("memory@local").spec().is_some());
        assert!(RepositorySpec::parse_spec("/packages/int").spec().is_some());
        assert!(!RepositorySpec::parse_spec("memory@local").is_unknown_type());
        assert!(!RepositorySpec::parse_spec("/packages/int").is_unknown_type());
    }

    #[test]
    fn existing_directory_with_at_is_a_path_not_an_unknown_type() {
        // The regression that motivated the allowlist: a directory whose name
        // contains an `@` must keep resolving. It is reported as a path
        // rather than an unknown type only because it exists on disk.
        let root = tempfile::TempDir::new().unwrap();
        let dir = root.path().join("pkgs@v1");
        std::fs::create_dir_all(&dir).unwrap();
        let entry = dir.to_string_lossy().to_string();

        // The entry contains path separators, so `is_type_prefix` already
        // rejects a typed reading; assert the bucket as well so the three-way
        // split is what is being depended on.
        let parsed = RepositorySpec::parse_spec(&entry);
        assert!(!parsed.is_unknown_type());
        assert!(parsed.is_filesystem());
    }

    #[test]
    fn relative_identifier_shaped_directory_is_a_path_not_an_unknown_type() {
        // The tight case: no path syntax in the prefix, no known type, and
        // the entry exists. Resolved through the current directory, which the
        // test process owns, so this stays hermetic.
        let root = tempfile::TempDir::new().unwrap();
        let previous = std::env::current_dir().unwrap();
        std::env::set_current_dir(root.path()).unwrap();
        std::fs::create_dir_all("pkgs@v1").unwrap();

        let parsed = RepositorySpec::parse_spec("pkgs@v1");

        std::env::set_current_dir(previous).unwrap();
        assert!(!parsed.is_unknown_type());
        assert!(parsed.is_filesystem());
        assert_eq!(parsed.spec().unwrap().location, "pkgs@v1");
    }

    #[test]
    fn legacy_parse_never_reports_unknown_type() {
        // `parse` keeps its infallible signature for callers that decide
        // their own diagnostics, so an unknown type must fall back to the
        // whole entry rather than changing the returned location.
        let spec = RepositorySpec::parse("server@host/path");
        assert_eq!(spec.repo_type, "filesystem");
        assert_eq!(spec.location, "server@host/path");
    }
}
