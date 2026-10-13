//! Parsing of upstream's `type@location` repository specifications.
//!
//! rez allows `packages_path` entries to name their repository type
//! explicitly, e.g. `filesystem@/packages/int` or `memory@foo`. A bare
//! path is still accepted and means a filesystem repository.

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
    /// A path may still legitimately contain an `@` (`/packages/@studio/tools`,
    /// `C:\my@pkgs`), so the prefix is only honoured when it looks like a
    /// repository type name. Otherwise the entry would be misread as an
    /// unknown type and dropped, silently breaking a path that resolved
    /// before.
    pub fn parse(entry: &str) -> Self {
        match entry.split_once('@') {
            Some((repo_type, location))
                if is_type_prefix(repo_type)
                    && is_known_type(repo_type)
                    && !location.is_empty() =>
            {
                Self {
                    repo_type: repo_type.to_string(),
                    location: location.to_string(),
                }
            }
            _ => Self {
                repo_type: FILESYSTEM_REPO_TYPE.to_string(),
                location: entry.to_string(),
            },
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
fn is_type_prefix(s: &str) -> bool {
    !s.is_empty()
        && s.len() <= 32
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// Whether `s` names a repository type that upstream rez defines.
///
/// `is_type_prefix` only checks the shape of the name, so on its own it would
/// accept a real directory such as `pkgs@v1`: the entry would be read as a
/// typed spec, found to have no implementation, and dropped — silently
/// breaking a path that used to resolve.
///
/// The allowlist names the types upstream defines, so an entry is read as a
/// typed spec only when the prefix is one of them; anything else is a path
/// that happens to contain an `@`. Types listed here need not be implemented
/// by this build — `memory` is upstream's example and is preserved as a type
/// so callers see the requested type rather than a silently rewritten path.
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
}
