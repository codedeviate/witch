use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;
use std::os::unix::fs::PermissionsExt;
use std::path::Path;
use tempfile::TempDir;

fn fake_bin(dir: &Path, name: &str) {
    let p = dir.join(name);
    fs::write(&p, "#!/bin/sh\n").unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o755)).unwrap();
}

/// witch with PATH pointing at exactly one temp dir.
/// assert_cmd pipes stdout, so the binary always sees a non-TTY stdout here:
/// default mode in every test below is single-result mode.
fn witch(path_dir: &Path) -> Command {
    let mut cmd = Command::cargo_bin("witch").unwrap();
    cmd.env("PATH", path_dir);
    cmd
}

#[test]
fn exact_match_prints_single_path() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    fake_bin(tmp.path(), "grepx");
    witch(tmp.path())
        .arg("grep")
        .assert()
        .success()
        .stdout(format!("{}\n", tmp.path().join("grep").display()));
}

#[test]
fn typo_prints_only_best_match_when_stdout_is_piped() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    fake_bin(tmp.path(), "cat");
    witch(tmp.path())
        .arg("grpe")
        .assert()
        .success()
        .stdout(format!("{}\n", tmp.path().join("grep").display()));
}

#[test]
fn all_flag_lists_multiple_candidates_best_first() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    fake_bin(tmp.path(), "grip");
    // "grap" scores grep and grip identically; tie-break is alphabetical.
    witch(tmp.path())
        .args(["-a", "grap"])
        .assert()
        .success()
        .stdout(format!(
            "{}\n{}\n",
            tmp.path().join("grep").display(),
            tmp.path().join("grip").display()
        ));
}

#[test]
fn first_flag_forces_single_result() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    fake_bin(tmp.path(), "grip");
    witch(tmp.path())
        .args(["-1", "grap"])
        .assert()
        .success()
        .stdout(format!("{}\n", tmp.path().join("grep").display()));
}

#[test]
fn no_match_prints_stderr_error_and_exits_1() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "ls");
    witch(tmp.path())
        .arg("doesnotexist")
        .assert()
        .code(1)
        .stdout("")
        .stderr(predicate::str::contains("no match for 'doesnotexist'"));
}

#[test]
fn first_and_all_flags_conflict() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path()).args(["-1", "-a", "ls"]).assert().code(2);
}

#[test]
fn multiple_queries_resolve_in_argument_order() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    fake_bin(tmp.path(), "cat");
    witch(tmp.path())
        .args(["gerp", "caat"])
        .assert()
        .success()
        .stdout(format!(
            "{}\n{}\n",
            tmp.path().join("grep").display(),
            tmp.path().join("cat").display()
        ));
}

#[test]
fn partial_failure_still_exits_1_but_prints_found_paths() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    witch(tmp.path())
        .args(["gerp", "doesnotexist"])
        .assert()
        .code(1)
        .stdout(format!("{}\n", tmp.path().join("grep").display()))
        .stderr(predicate::str::contains("no match for 'doesnotexist'"));
}

#[test]
fn quiet_suppresses_all_output() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    witch(tmp.path())
        .args(["-q", "gerp"])
        .assert()
        .success()
        .stdout("");
    witch(tmp.path())
        .args(["-q", "doesnotexist"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("");
}

#[test]
fn no_arguments_is_a_usage_error() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path()).assert().code(2);
}

#[test]
fn version_flag_prints_name_and_version() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path())
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::starts_with("witch "));
}

#[test]
fn examples_flag_prints_examples_without_command_argument() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path())
        .arg("--examples")
        .assert()
        .success()
        .stdout(predicate::str::contains("witch gerp"));
}

#[test]
fn pick_flag_conflicts_with_all_first_and_quiet() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path()).args(["-i", "-a", "ls"]).assert().code(2);
    witch(tmp.path()).args(["-i", "-1", "ls"]).assert().code(2);
    witch(tmp.path()).args(["-i", "-q", "ls"]).assert().code(2);
}

#[test]
fn pick_with_single_match_skips_menu_and_prints_path() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    // Exact match resolves via find_exact and bypasses the picker entirely;
    // -i must not attempt a menu.
    witch(tmp.path())
        .args(["-i", "grep"])
        .assert()
        .success()
        .stdout(format!("{}\n", tmp.path().join("grep").display()));
}

#[test]
fn all_flag_shows_duplicate_instances_for_exact_match() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "ls");
    // Same dir twice in PATH -> BSD `which -a` prints the path twice.
    let dup = format!("{p}:{p}", p = tmp.path().display());
    Command::cargo_bin("witch")
        .unwrap()
        .env("PATH", dup)
        .args(["-a", "ls"])
        .assert()
        .success()
        .stdout(format!(
            "{p}\n{p}\n",
            p = tmp.path().join("ls").display()
        ));
}

#[test]
fn silent_flag_suppresses_output_like_quiet() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    witch(tmp.path()).args(["-s", "grep"]).assert().success().stdout("");
    witch(tmp.path())
        .args(["-s", "doesnotexist"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("");
}

#[test]
fn silent_flag_conflicts_with_pick() {
    let tmp = TempDir::new().unwrap();
    witch(tmp.path()).args(["-i", "-s", "ls"]).assert().code(2);
}

#[test]
fn strict_flag_disables_fuzzy_and_fails_silently() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    // A typo that fuzzy would resolve must fail silently under --strict.
    witch(tmp.path())
        .args(["--strict", "gerp"])
        .assert()
        .code(1)
        .stdout("")
        .stderr("");
}

#[test]
fn strict_flag_still_finds_exact_matches() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    witch(tmp.path())
        .args(["--strict", "grep"])
        .assert()
        .success()
        .stdout(format!("{}\n", tmp.path().join("grep").display()));
}

#[test]
fn invoked_as_which_is_strict() {
    let tmp = TempDir::new().unwrap();
    fake_bin(tmp.path(), "grep");
    let bin = assert_cmd::cargo::cargo_bin("witch");
    let linkdir = TempDir::new().unwrap();
    let link = linkdir.path().join("which");
    std::os::unix::fs::symlink(&bin, &link).unwrap();
    // Invoked as `which`, a typo must not fuzzy-match.
    let out = std::process::Command::new(&link)
        .arg("gerp")
        .env("PATH", tmp.path())
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
}
