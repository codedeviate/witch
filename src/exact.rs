use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Match {
    /// The executable path as found: `dir.join(name)`.
    pub path: PathBuf,
    /// The PATH entry the match came from (used for display transforms).
    pub dir: PathBuf,
}

/// True if `path` is a regular file with any execute bit set.
/// `fs::metadata` follows symlinks, so a symlink to an executable counts.
pub fn is_executable(path: &Path) -> bool {
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

/// Should this PATH entry be skipped given the skip flags?
/// `skip_dot` drops entries whose textual form starts with `.` (i.e.
/// relative paths beginning with a dot; internal path components are not
/// inspected).
/// `skip_tilde` drops entries that start with `~` and entries equal to or
/// under `home`.
fn skip_dir(dir: &Path, skip_dot: bool, skip_tilde: bool, home: Option<&Path>) -> bool {
    let s = dir.to_string_lossy();
    if skip_dot && s.starts_with('.') {
        return true;
    }
    if skip_tilde {
        if s.starts_with('~') {
            return true;
        }
        if let Some(h) = home
            && (dir == h || dir.starts_with(h))
        {
            return true;
        }
    }
    false
}

/// Walk `dirs` in PATH order looking for an executable named `name`.
/// No dedup: a directory repeated in PATH yields a repeated match, matching
/// BSD `which -a` (`PATH=/bin:/bin which -a ls` prints `/bin/ls` twice).
/// With `all == false`, returns at most the first match.
pub fn find_exact(
    dirs: &[PathBuf],
    name: &str,
    all: bool,
    skip_dot: bool,
    skip_tilde: bool,
    home: Option<&Path>,
) -> Vec<Match> {
    let mut out = Vec::new();
    for dir in dirs {
        if skip_dir(dir, skip_dot, skip_tilde, home) {
            continue;
        }
        let path = dir.join(name);
        if is_executable(&path) {
            out.push(Match {
                path,
                dir: dir.clone(),
            });
            if !all {
                return out;
            }
        }
    }
    out
}

/// Resolve `p` against the current directory, dropping `.` components and
/// applying `..` so the result has no `/./` noise. Used to turn a
/// dot-relative PATH match into a clean absolute path.
fn absolutize(p: &Path) -> PathBuf {
    let mut out = std::env::current_dir().unwrap_or_default();
    for comp in p.components() {
        match comp {
            std::path::Component::CurDir => {}
            std::path::Component::ParentDir => {
                out.pop();
            }
            other => out.push(other.as_os_str()),
        }
    }
    out
}

/// Render a match for printing.
/// - default: a dot-relative entry (`./bin`) is absolutized against the
///   current directory; absolute entries are printed as-is.
/// - `show_dot`: keep the dot-relative form (`./bin/grep`).
/// - `show_tilde`: if the shown path is under `home`, rewrite the prefix to
///   `~`. Pass `home = None` (e.g. when euid == 0) to disable.
pub fn display_path(m: &Match, show_dot: bool, show_tilde: bool, home: Option<&Path>) -> String {
    let is_dot = m.dir.to_string_lossy().starts_with('.');
    let shown: PathBuf = if is_dot && !show_dot {
        absolutize(&m.path)
    } else {
        m.path.clone()
    };

    if show_tilde
        && let Some(h) = home
        && let Ok(rest) = shown.strip_prefix(h)
    {
        return format!("~/{}", rest.display());
    }
    shown.display().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use tempfile::TempDir;

    fn fake_bin(dir: &Path, name: &str) {
        let p = dir.join(name);
        fs::write(&p, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }

    #[test]
    fn returns_first_match_when_not_all() {
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        fake_bin(a.path(), "grep");
        fake_bin(b.path(), "grep");
        let got = find_exact(
            &[a.path().to_path_buf(), b.path().to_path_buf()],
            "grep",
            false,
            false,
            false,
            None,
        );
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, a.path().join("grep"));
    }

    #[test]
    fn all_returns_every_instance_including_duplicate_path_entries() {
        let a = TempDir::new().unwrap();
        fake_bin(a.path(), "ls");
        // Same dir twice in PATH -> two matches, like BSD `which -a`.
        let got = find_exact(
            &[a.path().to_path_buf(), a.path().to_path_buf()],
            "ls",
            true,
            false,
            false,
            None,
        );
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].path, a.path().join("ls"));
        assert_eq!(got[1].path, a.path().join("ls"));
    }

    #[test]
    fn no_match_returns_empty() {
        let a = TempDir::new().unwrap();
        fake_bin(a.path(), "ls");
        let got = find_exact(&[a.path().to_path_buf()], "grep", true, false, false, None);
        assert!(got.is_empty());
    }

    #[test]
    fn skip_dot_drops_dot_relative_entries() {
        let got = find_exact(
            &[PathBuf::from("./bin")],
            "grep",
            true,
            true, // skip_dot
            false,
            None,
        );
        assert!(got.is_empty());
    }

    #[test]
    fn skip_tilde_drops_entries_under_home() {
        let home = TempDir::new().unwrap();
        let bin = home.path().join("bin");
        fs::create_dir(&bin).unwrap();
        fake_bin(&bin, "grep");
        let got = find_exact(
            &[bin.clone()],
            "grep",
            true,
            false,
            true, // skip_tilde
            Some(home.path()),
        );
        assert!(got.is_empty());
    }

    #[test]
    fn display_default_absolutizes_dot_relative_paths() {
        let m = Match {
            path: PathBuf::from("./bin/grep"),
            dir: PathBuf::from("./bin"),
        };
        let got = display_path(&m, false, false, None);
        assert!(got.starts_with('/'), "expected absolute path, got {got}");
        assert!(got.ends_with("/bin/grep"), "got {got}");
        assert!(!got.contains("/./"), "should be normalized, got {got}");
    }

    #[test]
    fn show_dot_keeps_relative_form() {
        let m = Match {
            path: PathBuf::from("./bin/grep"),
            dir: PathBuf::from("./bin"),
        };
        let got = display_path(&m, true, false, None);
        assert_eq!(got, "./bin/grep");
    }

    #[test]
    fn show_tilde_rewrites_home_prefix() {
        let home = PathBuf::from("/home/u");
        let m = Match {
            path: PathBuf::from("/home/u/bin/grep"),
            dir: PathBuf::from("/home/u/bin"),
        };
        let got = display_path(&m, false, true, Some(&home));
        assert_eq!(got, "~/bin/grep");
    }

    #[test]
    fn show_tilde_ignored_when_home_is_none_root() {
        let m = Match {
            path: PathBuf::from("/home/u/bin/grep"),
            dir: PathBuf::from("/home/u/bin"),
        };
        let got = display_path(&m, false, true, None);
        assert_eq!(got, "/home/u/bin/grep");
    }
}
