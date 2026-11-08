//! The reasons behind an AI decision, kept for the per-game AI log.

use std::fmt::Debug;

/// Options listed by [`DecisionTrace::scores`], best first; the rest are only counted.
const LISTED_OPTIONS: usize = 8;

/// Lines explaining one AI decision: the figures it weighed and the options it scored.
///
/// Written for developers evaluating the AI, so in English, and never shown to players.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct DecisionTrace {
    lines: Vec<String>,
}

impl DecisionTrace {
    /// Adds a free-form line.
    pub fn note(&mut self, line: String) {
        self.lines.push(line);
    }

    /// Lists the best scored options under `title`, best first, with their scores.
    pub fn scores<T: Debug>(&mut self, title: &str, scored: &[(T, f64)]) {
        let mut ranked: Vec<&(T, f64)> = scored.iter().collect();
        ranked.sort_by(|a, b| b.1.total_cmp(&a.1));
        self.lines
            .push(format!("{title}: {} option(s), best first", ranked.len()));
        for (option, score) in ranked.iter().take(LISTED_OPTIONS) {
            self.lines.push(format!("  {score:>10.1}  {option:?}"));
        }
        if ranked.len() > LISTED_OPTIONS {
            self.lines
                .push(format!("  … {} more", ranked.len() - LISTED_OPTIONS));
        }
    }

    /// The lines recorded so far, in order.
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scores_are_listed_best_first_and_capped() {
        let mut trace = DecisionTrace::default();
        trace.note("context".to_string());
        let scored: Vec<(usize, f64)> = (0..10).map(|n| (n, n as f64)).collect();
        trace.scores("options", &scored);
        let lines = trace.lines();
        assert_eq!(lines[0], "context");
        assert_eq!(lines[1], "options: 10 option(s), best first");
        assert!(lines[2].ends_with("  9"));
        assert_eq!(lines.len(), 2 + LISTED_OPTIONS + 1);
        assert_eq!(lines.last().unwrap(), "  … 2 more");
    }
}
