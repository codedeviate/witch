use std::collections::HashSet;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Candidate {
    pub name: String,
    pub path: PathBuf,
}

/// Scan `dirs` in order, returning executables deduped by name —
/// first occurrence in PATH order wins, the same shadowing rule `which` uses.
/// Unreadable or missing dirs are silently skipped.
pub fn scan(dirs: &[PathBuf]) -> Vec<Candidate> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for dir in dirs {
        let Ok(entries) = fs::read_dir(dir) else { continue };
        let mut found: Vec<Candidate> = entries
            .flatten()
            .filter_map(|entry| {
                let name = entry.file_name().into_string().ok()?;
                let path = entry.path();
                is_executable(&path).then_some(Candidate { name, path })
            })
            .collect();
        found.sort_by(|a, b| a.name.cmp(&b.name));
        for c in found {
            if seen.insert(c.name.clone()) {
                out.push(c);
            }
        }
    }
    out
}

fn is_executable(path: &Path) -> bool {
    // fs::metadata follows symlinks, so a symlink to an executable counts.
    fs::metadata(path)
        .map(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::os::unix::fs::PermissionsExt;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn fake_bin(dir: &std::path::Path, name: &str) {
        let p = dir.join(name);
        fs::write(&p, "#!/bin/sh\n").unwrap();
        fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
    }

    fn plain_file(dir: &std::path::Path, name: &str) {
        fs::write(dir.join(name), "data").unwrap();
    }

    #[test]
    fn finds_executables_and_skips_plain_files() {
        let tmp = TempDir::new().unwrap();
        fake_bin(tmp.path(), "grep");
        plain_file(tmp.path(), "README");
        let got = scan(&[tmp.path().to_path_buf()]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "grep");
        assert_eq!(got[0].path, tmp.path().join("grep"));
    }

    #[test]
    fn first_dir_in_path_shadows_later_dirs() {
        let a = TempDir::new().unwrap();
        let b = TempDir::new().unwrap();
        fake_bin(a.path(), "grep");
        fake_bin(b.path(), "grep");
        let got = scan(&[a.path().to_path_buf(), b.path().to_path_buf()]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].path, a.path().join("grep"));
    }

    #[test]
    fn missing_dirs_are_skipped() {
        let tmp = TempDir::new().unwrap();
        fake_bin(tmp.path(), "ls");
        let got = scan(&[
            PathBuf::from("/nonexistent-witch-test-dir"),
            tmp.path().to_path_buf(),
        ]);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].name, "ls");
    }

    #[test]
    fn names_within_a_dir_are_sorted() {
        let tmp = TempDir::new().unwrap();
        fake_bin(tmp.path(), "zsh");
        fake_bin(tmp.path(), "bash");
        let got = scan(&[tmp.path().to_path_buf()]);
        let names: Vec<_> = got.iter().map(|c| c.name.as_str()).collect();
        assert_eq!(names, vec!["bash", "zsh"]);
    }
}
