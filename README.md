# witch

**A typo-tolerant `which`.** You meant `grep`, you typed `gerp` — `witch`
finds it anyway and prints the same absolute paths `which` would.

```
$ witch gerp
/usr/bin/grep

$ which gerp
gerp not found
```

Exact names behave exactly like `which`; misspelled names are fuzzy-matched
against every executable on your `PATH` and ranked by similarity. When the
output is captured — command substitution or a pipe — `witch` prints only the
best match, so `$(witch gerp)` always expands to exactly one path.

## Features

- **Typo-tolerant lookup** — Jaro-Winkler scoring handles transpositions
  (`grpe` → `grep`), wrong letters (`pyhton3` → `python3`), and case slips
- **Exact-match short-circuit** — a correctly spelled command is resolved
  byte-for-byte like `which`, fuzzy matching never engages
- **Command-substitution safe** — non-TTY stdout automatically collapses to
  the single best match; `$(witch gerp)` just works
- **Quality-cliff ranking** — shows the few plausible candidates, not a wall
  of noise: results stay close to the top score and the list ends at the
  first sharp drop (hard cap 10)
- **Interactive picker** — `witch -i grap` renders a menu on **stderr** and
  prints the chosen path to stdout, so it even works inside `$( )`
- **`which`-compatible behavior** — first-in-`PATH` shadowing, one path per
  line, exit codes `0`/`1`, multiple command arguments resolved in order

## Install

### Homebrew (macOS / Linuxbrew)

```bash
brew install codedeviate/cli/witch
```

### crates.io

```bash
cargo install witch-cli
```

(The crate is `witch-cli`; the installed binary is `witch`.)

### From source

```bash
git clone https://github.com/codedeviate/witch.git
cd witch
cargo install --path .
```

Requires Rust 1.85+. Unix-only (macOS, Linux) — executable detection uses
Unix permission bits.

## Quick start

```bash
witch grep          # exact lookup, same as which
witch gerp          # typo-tolerant lookup, best matches first
witch -a pyhton     # all candidates, even when piped
witch -1 grap       # only the best match
$(witch gerp) -V    # command substitution: exactly one path
witch -i grap       # interactive picker (menu on stderr)
witch -q gerp       # no output, exit code only
```

## Usage

```
witch [OPTIONS] <COMMANDS>...

Arguments:
  <COMMANDS>...  Command name(s) to look up

Options:
  -1, --first     Print only the best match
  -a, --all       Print all candidates even when stdout is not a TTY
  -q, --quiet     No output, exit code only
  -i, --pick      Interactively pick from the candidates
      --examples  Print usage examples and exit
  -h, --help      Print help
  -V, --version   Print version
```

`-1`/`-a`/`-i`/`-q` with contradictory intents conflict (exit 2) instead of
silently overriding each other: `-1` vs `-a`, and `-i` vs any of `-1`/`-a`/`-q`.

### Output modes

| Situation | Output |
|---|---|
| Exact name match | that single path, always |
| stdout is a TTY | ranked candidates, one path per line, best first |
| stdout is not a TTY (pipe, `$( )`) | best match only |
| `-1` | best match only, regardless of TTY |
| `-a` | full ranked list, regardless of TTY |
| `-i` with 2+ candidates | menu on stderr, chosen path on stdout |

### How matching works

Candidates are every executable on `PATH` (deduped by name, first occurrence
wins — the same shadowing rule `which` uses). Each name is scored against
your query with case-insensitive Jaro-Winkler similarity:

1. Scores below **0.8** are discarded (keeps junk from matching short names
   like `xz`)
2. Survivors are ranked best-first, ties broken alphabetically
3. The list keeps only results within **0.10** of the top score, ends at the
   first drop greater than **0.05** between neighbors, and caps at **10**

### Exit codes

| Code | Meaning |
|---|---|
| `0` | every query produced at least one match |
| `1` | at least one query found nothing (or the picker was cancelled) |
| `2` | usage error (unknown/conflicting flags, no arguments) |

A query with no match prints `witch: no match for 'xyz'` to stderr.

## License

MIT — see [LICENSE](LICENSE).
