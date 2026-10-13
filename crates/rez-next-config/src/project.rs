//! Project-level configuration lookup.
//!
//! `rez-next-config` is published before `rez-next-common`, so the shared
//! lookup lives here and `rez-next-common` calls into it. The reverse edge
//! would be a dependency cycle that `cargo package` cannot resolve.

use std::path::{Path, PathBuf};

/// The file names probed for a project config, in priority order.
pub const PROJECT_CONFIG_NAMES: &[&str] = &[
    "rezconfig.toml",
    "rezconfig.yaml",
    "rezconfig.yml",
    "rezconfig.json",
];

/// Return the project config file inside `dir`, if that directory has one.
#[must_use]
pub fn project_config_in(dir: &Path) -> Option<PathBuf> {
    let rez_dir = dir.join(".rez");
    PROJECT_CONFIG_NAMES
        .iter()
        .map(|name| rez_dir.join(name))
        .find(|candidate| candidate.is_file())
}

/// Compare two paths for equality without requiring both to exist.
///
/// A canonicalized comparison silently fails on Windows for paths under
/// temporary directories, so compare the paths as given first and only fall
/// back to canonicalization when both sides resolve.
#[must_use]
pub fn paths_equal(a: &Path, b: &Path) -> bool {
    if a == b {
        return true;
    }
    match (a.canonicalize(), b.canonicalize()) {
        (Ok(a), Ok(b)) => a == b,
        _ => false,
    }
}

/// Find the project configuration file by walking up from `from`.
///
/// **rez-next extension**: upstream rez has no project-level config file —
/// it reads `REZ_CONFIG_FILE` and `~/.rezconfig` only. This lookup is a
/// rez-next addition and does not change rez's own behaviour.
///
/// The walk is nearest-first and stops at the first of:
///
/// 1. a directory containing `.rez/rezconfig.{toml,yaml,yml,json}` — adopted;
/// 2. the git root (a directory containing `.git`) — checked once, then stop;
/// 3. `$REZ_PROJECT_ROOT` — stop;
/// 4. the filesystem root — stop.
#[must_use]
pub fn find_project_config_file(from: &Path) -> Option<PathBuf> {
    let project_root = std::env::var("REZ_PROJECT_ROOT")
        .ok()
        .filter(|v| !v.is_empty());

    let mut dir = Some(from);

    while let Some(current) = dir {
        if let Some(found) = project_config_in(current) {
            return Some(found);
        }

        // Stop after the git root itself has been checked (rule 2).
        if current.join(".git").exists() {
            break;
        }

        // Stop at $REZ_PROJECT_ROOT (rule 3).
        if let Some(root) = &project_root
            && paths_equal(current, Path::new(root))
        {
            break;
        }

        dir = current.parent();
    }

    // `REZ_PROJECT_ROOT` is also a candidate: when the cwd is not below it,
    // the upward walk never reaches it, so check it explicitly once.
    if let Some(root) = &project_root
        && let Some(found) = project_config_in(Path::new(root))
    {
        return Some(found);
    }

    None
}

/// The configuration files that exist and would actually be read.
///
/// Mirrors `RezCoreConfig::get_search_paths` — built-in, `REZ_CONFIG_FILE`,
/// system, home and project layers, in that precedence order — so
/// `config validate` with no `--file` checks exactly what the binary reads.
/// The list is duplicated rather than shared because `rez-next-common`
/// depends on this crate, not the other way round.
#[must_use]
pub fn sourced_paths() -> Vec<PathBuf> {
    let mut paths = Vec::new();

    if let Ok(exe_path) = std::env::current_exe()
        && let Some(exe_dir) = exe_path.parent()
    {
        paths.push(exe_dir.join("rezconfig.yaml"));
        paths.push(exe_dir.join("rezconfig.json"));
    }

    if let Ok(config_file) = std::env::var("REZ_CONFIG_FILE") {
        let separator = if cfg!(windows) { ';' } else { ':' };
        for path in config_file.split(separator) {
            paths.push(PathBuf::from(path));
        }
    }

    if cfg!(unix) {
        paths.push(PathBuf::from("/etc/rez/config.yaml"));
        paths.push(PathBuf::from("/usr/local/etc/rez/config.yaml"));
    } else if cfg!(windows)
        && let Ok(program_data) = std::env::var("PROGRAMDATA")
    {
        paths.push(PathBuf::from(program_data).join("rez").join("config.yaml"));
    }

    if std::env::var("REZ_DISABLE_HOME_CONFIG")
        .unwrap_or_default()
        .to_lowercase()
        != "1"
    {
        let home = std::env::var("HOME")
            .or_else(|_| std::env::var("USERPROFILE"))
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from);

        if let Some(home) = home {
            paths.push(home.join(".rezconfig"));
            paths.push(home.join(".rezconfig.yaml"));
            paths.push(home.join(".rez").join("config.yaml"));
        }
    }

    if let Ok(cwd) = std::env::current_dir()
        && let Some(project) = find_project_config_file(&cwd)
    {
        paths.push(project);
    }

    paths.into_iter().filter(|path| path.exists()).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_dir_has_no_project_config() {
        let dir = std::env::temp_dir().join("rz_no_project_cfg");
        let _ = std::fs::create_dir_all(&dir);
        assert!(project_config_in(&dir).is_none());
    }
}
