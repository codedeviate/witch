mod exact;
mod matcher;
mod path_scan;
mod picker;

use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;
use crate::path_scan::Candidate;

/// A fuzzy `which`: finds commands on PATH even when you misspell them.
#[derive(Parser, Debug)]
#[command(name = "witch", version, about)]
struct Cli {
    /// Print only the best match
    #[arg(short = '1', long = "first", conflicts_with = "all")]
    first: bool,

    /// Print all candidates even when stdout is not a TTY
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Interactively pick from the candidates
    #[arg(short = 'i', long = "pick", conflicts_with_all = ["all", "first", "quiet", "silent"])]
    pick: bool,

    /// No output, exit code only
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

    /// No output, exit code only (BSD `which -s`)
    #[arg(short = 's')]
    silent: bool,

    /// Print usage examples and exit
    #[arg(long)]
    examples: bool,

    /// Disable fuzzy matching; behave byte-for-byte like which.
    /// Auto-enabled when the binary is invoked as `which`.
    #[arg(long)]
    strict: bool,

    /// Skip PATH entries that start with `.`
    #[arg(long = "skip-dot")]
    skip_dot: bool,

    /// Skip PATH entries that start with `~` and entries under $HOME
    #[arg(long = "skip-tilde")]
    skip_tilde: bool,

    /// Print `./prog` for dot-relative PATH entries instead of absolutizing
    #[arg(long = "show-dot")]
    show_dot: bool,

    /// Print `~/...` for matches under $HOME (ignored when run as root)
    #[arg(long = "show-tilde")]
    show_tilde: bool,

    /// Honor display flags only when stdout is a TTY
    #[arg(long = "tty-only")]
    tty_only: bool,

    /// Command name(s) to look up
    #[arg(required_unless_present = "examples")]
    commands: Vec<String>,
}

/// True when argv[0]'s file name is exactly `which` (e.g. via a symlink),
/// so a `which`-named binary defaults to strict, real-which behavior.
fn invoked_as_which() -> bool {
    std::env::args_os()
        .next()
        .map(PathBuf::from)
        .and_then(|p| p.file_name().map(|n| n == "which"))
        .unwrap_or(false)
}

const EXAMPLES: &str = "\
EXAMPLES:
    witch grep         exact lookup, same as which
    witch gerp         typo-tolerant lookup, prints best matches
    witch -a pyhton    print all candidates even when piped
    witch -1 grap      print only the best match
    $(witch gerp)      command substitution gets exactly one path
    witch -i grap      interactive picker (menu on stderr)
    witch -q gerp      no output, exit code only
";

fn main() -> ExitCode {
    let cli = Cli::parse();
    if cli.examples {
        print!("{EXAMPLES}");
        return ExitCode::SUCCESS;
    }
    let quiet = cli.quiet || cli.silent;
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let single = cli.first || (!cli.all && !std::io::stdout().is_terminal());

    let is_tty = std::io::stdout().is_terminal();
    // `--tty-only`: when stdout is not a TTY, drop display niceties.
    let display_active = !(cli.tty_only && !is_tty);
    let show_dot = cli.show_dot && display_active;
    let show_tilde = cli.show_tilde && display_active;
    let skip_dot = cli.skip_dot;
    let skip_tilde = cli.skip_tilde;
    let pick = cli.pick && display_active;

    let home_env = std::env::var_os("HOME").map(PathBuf::from);
    // $HOME drives --skip-tilde regardless of user.
    let skip_home = home_env.clone();
    // --show-tilde is ignored when running as root (euid == 0).
    let euid = unsafe { libc::geteuid() };
    let display_home = if euid == 0 { None } else { home_env };
    let strict = cli.strict || invoked_as_which();

    // Fuzzy candidate list is only scanned if an exact lookup misses.
    let mut fuzzy: Option<Vec<Candidate>> = None;

    let mut all_found = true;
    for query in &cli.commands {
        let matches = exact::find_exact(
            &dirs,
            query,
            cli.all,
            skip_dot,
            skip_tilde,
            skip_home.as_deref(),
        );

        if !matches.is_empty() {
            if !quiet {
                // find_exact returns at most one match when !all, so this also
                // satisfies -1/--first; no separate `single` guard is needed here.
                let to_print = if cli.all { &matches[..] } else { &matches[..1] };
                for m in to_print {
                    println!(
                        "{}",
                        exact::display_path(m, show_dot, show_tilde, display_home.as_deref())
                    );
                }
            }
            continue;
        }

        // No exact match.
        if strict {
            all_found = false; // silent, BSD-style
            continue;
        }

        // Fuzzy fallback (non-strict only).
        let candidates = fuzzy.get_or_insert_with(|| path_scan::scan(&dirs));
        let ranked = matcher::rank(query, candidates);
        if ranked.is_empty() {
            if !quiet {
                eprintln!("witch: no match for '{query}'");
            }
            all_found = false;
            continue;
        }
        if quiet {
            continue;
        }
        if pick && ranked.len() > 1 {
            match picker::pick(ranked) {
                Some(r) => println!("{}", r.candidate.path.display()),
                None => all_found = false,
            }
        } else if single {
            println!("{}", ranked[0].candidate.path.display());
        } else {
            for r in &ranked {
                println!("{}", r.candidate.path.display());
            }
        }
    }
    if all_found {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
