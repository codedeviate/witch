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

    /// Silent; no output, exit code only (BSD `which -s`, alias of --quiet)
    #[arg(short = 's')]
    silent: bool,

    /// Print usage examples and exit
    #[arg(long)]
    examples: bool,

    /// Command name(s) to look up
    #[arg(required_unless_present = "examples")]
    commands: Vec<String>,
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

    // Neutral until Task 6 wires the GNU directory flags.
    let skip_dot = false;
    let skip_tilde = false;
    let show_dot = false;
    let show_tilde = false;
    let skip_home: Option<PathBuf> = None;
    let display_home: Option<PathBuf> = None;
    let strict = false;

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
        if cli.pick && ranked.len() > 1 {
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
