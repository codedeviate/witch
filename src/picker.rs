use crate::matcher::Ranked;
use dialoguer::{theme::ColorfulTheme, Select};

/// Menu labels: one absolute path per candidate, best first.
pub fn labels(ranked: &[Ranked]) -> Vec<String> {
    ranked
        .iter()
        .map(|r| r.candidate.path.display().to_string())
        .collect()
}

/// Show a select menu (rendered on stderr, so stdout stays clean for
/// command substitution). Returns None if the user cancels (Esc/q/Ctrl-C).
pub fn pick(ranked: Vec<Ranked>) -> Option<Ranked> {
    let items = labels(&ranked);
    let choice = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("witch: pick a command")
        .items(&items)
        .default(0)
        .interact_opt()
        .ok()
        .flatten()?;
    ranked.into_iter().nth(choice)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::matcher::Ranked;
    use crate::path_scan::Candidate;
    use std::path::PathBuf;

    #[test]
    fn labels_are_the_candidate_paths() {
        let ranked = vec![
            Ranked {
                candidate: Candidate {
                    name: "grep".into(),
                    path: PathBuf::from("/usr/bin/grep"),
                },
                score: 0.93,
            },
            Ranked {
                candidate: Candidate {
                    name: "grip".into(),
                    path: PathBuf::from("/usr/local/bin/grip"),
                },
                score: 0.87,
            },
        ];
        assert_eq!(labels(&ranked), vec!["/usr/bin/grep", "/usr/local/bin/grip"]);
    }
}
