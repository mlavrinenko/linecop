//! Finds the config file: the nearest one walking up from the scanned path.

use std::path::{Path, PathBuf};

/// Directory entries that mark a repository root.
const REPO_MARKERS: [&str; 4] = [".git", ".jj", ".hg", ".svn"];

/// How far up config discovery walks.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, clap::ValueEnum)]
pub enum ConfigSearch {
    /// Stop after the repository root; with no repository, at the working directory.
    #[default]
    Repo,
    /// Walk to the filesystem root, ignoring repository markers.
    Root,
}

fn is_repo_root(dir: &Path) -> bool {
    REPO_MARKERS.iter().any(|marker| dir.join(marker).exists())
}

/// Searches for the nearest `.linecop.yaml` from `start` (canonicalized, so
/// `..` and symlinks resolve) upwards. With [`ConfigSearch::Repo`] the walk
/// ends at the first directory holding a repository marker, or at `cwd` when
/// no marker exists up the chain; with [`ConfigSearch::Root`] it ends at the
/// filesystem root. Returns the first path found, or `None`.
#[must_use]
pub fn find_config(start: &Path, cwd: &Path, search: ConfigSearch) -> Option<PathBuf> {
    let start = std::fs::canonicalize(start).unwrap_or_else(|_| start.to_path_buf());
    let cwd = std::fs::canonicalize(cwd).unwrap_or_else(|_| cwd.to_path_buf());
    let mut dirs: Vec<&Path> = start.ancestors().collect();
    if start.is_file() {
        dirs.remove(0);
    }
    let limit = match search {
        ConfigSearch::Root => dirs.len(),
        ConfigSearch::Repo => dirs
            .iter()
            .position(|dir| is_repo_root(dir))
            .or_else(|| dirs.iter().position(|dir| *dir == cwd))
            .map_or(dirs.len(), |idx| idx + 1),
    };
    dirs.iter()
        .take(limit)
        .map(|dir| dir.join(".linecop.yaml"))
        .find(|candidate| candidate.is_file())
}

#[cfg(test)]
mod tests {
    use super::{ConfigSearch, find_config};

    #[test]
    fn find_config_in_same_dir() {
        let dir = tempfile::tempdir().expect("tempdir");
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        let found = find_config(dir.path(), dir.path(), ConfigSearch::Repo);
        assert_eq!(found, Some(cfg));
    }

    #[test]
    fn find_config_traverses_up() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("a").join("b");
        std::fs::create_dir_all(&sub).expect("mkdir");
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        let found = find_config(&sub, dir.path(), ConfigSearch::Repo);
        assert_eq!(found, Some(cfg));
    }

    #[test]
    fn find_config_stops_at_boundary() {
        let dir = tempfile::tempdir().expect("tempdir");
        let parent = dir.path().join("parent");
        let child = parent.join("child");
        std::fs::create_dir_all(&child).expect("mkdir");
        // Place config above the stop boundary
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 500\n").expect("write");
        // Search from child, stop at parent — should NOT find config in dir
        let found = find_config(&child, &parent, ConfigSearch::Repo);
        assert!(found.is_none());
    }

    #[test]
    fn find_config_returns_none_when_missing() {
        let dir = tempfile::tempdir().expect("tempdir");
        let found = find_config(dir.path(), dir.path(), ConfigSearch::Repo);
        assert!(found.is_none());
    }

    #[test]
    fn find_config_stops_after_repo_root() {
        let dir = tempfile::tempdir().expect("tempdir");
        let repo = dir.path().join("repo");
        let sub = repo.join("sub");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::create_dir(repo.join(".git")).expect("mkdir");
        std::fs::write(dir.path().join(".linecop.yaml"), "limits:\n  Rust: 5\n").expect("write");
        assert!(find_config(&sub, dir.path(), ConfigSearch::Repo).is_none());
        assert!(find_config(&sub, &sub, ConfigSearch::Root).is_some());
    }

    #[test]
    fn find_config_repo_root_beyond_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let sub = dir.path().join("sub");
        std::fs::create_dir_all(&sub).expect("mkdir");
        std::fs::create_dir(dir.path().join(".git")).expect("mkdir");
        let cfg = dir.path().join(".linecop.yaml");
        std::fs::write(&cfg, "limits:\n  Rust: 5\n").expect("write");
        let found = find_config(&sub, &sub, ConfigSearch::Repo);
        assert_eq!(found, Some(std::fs::canonicalize(cfg).expect("canon")));
    }
}
