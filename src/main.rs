mod matcher;
mod path_scan;
mod picker;

use clap::Parser;
use std::io::IsTerminal;
use std::path::PathBuf;
use std::process::ExitCode;

/// A fuzzy `which`: finds commands on PATH even when you misspell them.
#[derive(Parser, Debug)]
#[command(version, about)]
struct Cli {
    /// Print only the best match
    #[arg(short = '1', long = "first", conflicts_with = "all")]
    first: bool,

    /// Print all candidates even when stdout is not a TTY
    #[arg(short = 'a', long = "all")]
    all: bool,

    /// Interactively pick from the candidates
    #[arg(short = 'i', long = "pick", conflicts_with_all = ["all", "first"])]
    pick: bool,

    /// No output, exit code only
    #[arg(short = 'q', long = "quiet")]
    quiet: bool,

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
    let dirs: Vec<PathBuf> = std::env::var_os("PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let candidates = path_scan::scan(&dirs);
    let single = cli.first || (!cli.all && !std::io::stdout().is_terminal());

    let mut all_found = true;
    for query in &cli.commands {
        let ranked = matcher::rank(query, &candidates);
        if ranked.is_empty() {
            if !cli.quiet {
                eprintln!("witch: no match for '{query}'");
            }
            all_found = false;
            continue;
        }
        if cli.quiet {
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
