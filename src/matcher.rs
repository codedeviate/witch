use crate::path_scan::Candidate;
use strsim::jaro_winkler;

pub const SCORE_FLOOR: f64 = 0.7;
pub const TOP_BAND: f64 = 0.10;
pub const CLIFF_DROP: f64 = 0.05;
pub const MAX_RESULTS: usize = 10;

#[derive(Debug, Clone, PartialEq)]
pub struct Ranked {
    pub candidate: Candidate,
    pub score: f64,
}

/// Rank `candidates` against `query`. An exact name match (byte-for-byte)
/// returns just that candidate; otherwise candidates are scored with
/// case-insensitive Jaro-Winkler and trimmed by `apply_cliff`.
pub fn rank(query: &str, candidates: &[Candidate]) -> Vec<Ranked> {
    if let Some(c) = candidates.iter().find(|c| c.name == query) {
        return vec![Ranked {
            candidate: c.clone(),
            score: 1.0,
        }];
    }
    let q = query.to_lowercase();
    let mut scored: Vec<Ranked> = candidates
        .iter()
        .map(|c| Ranked {
            score: jaro_winkler(&q, &c.name.to_lowercase()),
            candidate: c.clone(),
        })
        .filter(|r| r.score >= SCORE_FLOOR)
        .collect();
    scored.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .expect("jaro_winkler never returns NaN")
            .then_with(|| a.candidate.name.cmp(&b.candidate.name))
    });
    apply_cliff(scored)
}

/// Keep results within TOP_BAND of the best score, stop at the first drop
/// larger than CLIFF_DROP between consecutive results, cap at MAX_RESULTS.
/// Input must already be sorted descending by score.
fn apply_cliff(sorted: Vec<Ranked>) -> Vec<Ranked> {
    let mut out: Vec<Ranked> = Vec::new();
    for r in sorted {
        if out.len() >= MAX_RESULTS {
            break;
        }
        if let Some(first) = out.first() {
            if first.score - r.score > TOP_BAND {
                break;
            }
        }
        if let Some(last) = out.last() {
            if last.score - r.score > CLIFF_DROP {
                break;
            }
        }
        out.push(r);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::path_scan::Candidate;
    use std::path::PathBuf;

    fn cands(names: &[&str]) -> Vec<Candidate> {
        names
            .iter()
            .map(|n| Candidate {
                name: n.to_string(),
                path: PathBuf::from(format!("/bin/{n}")),
            })
            .collect()
    }

    /// Synthetic Ranked list with the given scores, names c0, c1, ...
    fn synthetic(scores: &[f64]) -> Vec<Ranked> {
        scores
            .iter()
            .enumerate()
            .map(|(i, s)| Ranked {
                candidate: Candidate {
                    name: format!("c{i}"),
                    path: PathBuf::from(format!("/bin/c{i}")),
                },
                score: *s,
            })
            .collect()
    }

    #[test]
    fn exact_match_returns_only_that_candidate() {
        let c = cands(&["grep", "grepx", "rg"]);
        let got = rank("grep", &c);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].candidate.name, "grep");
        assert_eq!(got[0].score, 1.0);
    }

    #[test]
    fn transposed_typo_ranks_intended_command_first() {
        let c = cands(&["grep", "cat", "ls", "git"]);
        let got = rank("grpe", &c);
        assert!(!got.is_empty());
        assert_eq!(got[0].candidate.name, "grep");
    }

    #[test]
    fn unrelated_names_fall_below_floor() {
        let c = cands(&["ls", "cat"]);
        let got = rank("kubernetes", &c);
        assert!(got.is_empty());
    }

    #[test]
    fn matching_is_case_insensitive() {
        let c = cands(&["grep"]);
        let got = rank("GERP", &c);
        assert_eq!(got.len(), 1);
        assert_eq!(got[0].candidate.name, "grep");
    }

    #[test]
    fn equal_scores_tie_break_alphabetically() {
        let c = cands(&["grip", "grep"]);
        let got = rank("grap", &c);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].candidate.name, "grep");
        assert_eq!(got[1].candidate.name, "grip");
    }

    #[test]
    fn cliff_stops_at_sharp_drop_between_neighbors() {
        // 0.86 is within 0.10 of the top (0.95) but drops 0.07 from 0.93 — cut.
        let got = apply_cliff(synthetic(&[0.95, 0.93, 0.86]));
        assert_eq!(got.len(), 2);
    }

    #[test]
    fn band_excludes_results_far_from_top_score() {
        // 0.84 is 0.11 below the top score — outside TOP_BAND — even though
        // each neighbor drop is <= 0.05.
        let got = apply_cliff(synthetic(&[0.95, 0.91, 0.87, 0.84]));
        assert_eq!(got.len(), 3);
    }

    #[test]
    fn hard_cap_limits_results() {
        let scores = vec![0.9; 15];
        let got = apply_cliff(synthetic(&scores));
        assert_eq!(got.len(), MAX_RESULTS);
    }
}
