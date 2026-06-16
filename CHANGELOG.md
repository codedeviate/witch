# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html),
and commit messages follow [Conventional Commits](https://www.conventionalcommits.org/).

## [0.2.0] - 2026-06-16

### Added

- Drop-in `which` compatibility: accepts the union of BSD and GNU `which` flags
- `-s` silent flag (BSD `which -s`), an alias of `--quiet`
- `--strict` mode (also auto-enabled when invoked as `which` via a symlink):
  disables fuzzy matching for byte-for-byte `which` behavior
- GNU directory flags: `--skip-dot`, `--skip-tilde`, `--show-dot`,
  `--show-tilde`, `--tty-only`
- GNU `--read-alias`/`--skip-alias`/`--read-functions`/`--skip-functions`
  accepted as no-ops for shell-wrapper compatibility

### Changed

- Exact matches now resolve through a dedicated `which`-style lookup; `-a` lists
  every instance on `PATH` including duplicates from repeated `PATH` entries

## [0.1.1] - 2026-06-11

### Added

- README, MIT LICENSE, and full crate metadata for crates.io and Homebrew
  distribution
- Available via the `codedeviate/cli` Homebrew tap —
  `brew install codedeviate/cli/witch` (builds from source)

### Changed

- Package renamed to `witch-cli` (the name `witch` is taken on crates.io);
  the installed binary is still `witch`

## [0.1.0] - 2026-06-11

### Added

- Typo-tolerant PATH lookup with `which`-style output (Jaro-Winkler matching
  with a quality-cliff cutoff)
- Exact-match short-circuit identical to `which` behavior
- Single-result mode when stdout is not a TTY, so `$(witch gerp)` and pipes
  get exactly one path
- `-1/--first`, `-a/--all`, `-q/--quiet`, `-i/--pick` flags
- `--examples` flag, plus clap-generated `-h/--help` and `-V/--version`
- Interactive picker rendered on stderr so it works inside command substitution

[0.2.0]: https://github.com/codedeviate/witch/releases/tag/v0.2.0
[0.1.1]: https://github.com/codedeviate/witch/releases/tag/v0.1.1
[0.1.0]: https://github.com/codedeviate/witch/releases/tag/v0.1.0
