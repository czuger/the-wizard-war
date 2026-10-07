//! A choice among the options almost as good as the best one, so that an AI does not
//! always play the same move from the same position.

use rand::{Rng, RngExt};

/// Picks one of the options scoring within `tolerance` of the best.
///
/// Each option is weighted by how far it stands above the threshold, plus one, so options
/// tied with the best are equally likely and a tolerance of 0 only picks among those.
///
/// # Returns
/// `None` when `scored` is empty.
pub fn pick_near_best<T, R: Rng>(scored: Vec<(T, f64)>, tolerance: f64, rng: &mut R) -> Option<T> {
    let best = scored
        .iter()
        .map(|(_, score)| *score)
        .max_by(f64::total_cmp)?;
    let threshold = best - tolerance.max(0.0);
    let candidates: Vec<(T, f64)> = scored
        .into_iter()
        .filter(|(_, score)| *score >= threshold)
        .map(|(option, score)| (option, score - threshold + 1.0))
        .collect();
    let total: f64 = candidates.iter().map(|(_, weight)| weight).sum();
    let mut drawn = rng.random_range(0.0..total);
    let last = candidates.len() - 1;
    for (index, (option, weight)) in candidates.into_iter().enumerate() {
        if drawn < weight || index == last {
            return Some(option);
        }
        drawn -= weight;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn no_tolerance_picks_the_best() {
        let mut rng = rand::rng();
        for _ in 0..50 {
            let scored = vec![("a", 10.0), ("b", 30.0), ("c", 29.0)];
            assert_eq!(pick_near_best(scored, 0.0, &mut rng), Some("b"));
        }
        assert_eq!(pick_near_best(Vec::<((), f64)>::new(), 5.0, &mut rng), None);
    }

    #[test]
    fn never_picks_outside_the_tolerance() {
        let mut rng = rand::rng();
        let mut seen = Vec::new();
        for _ in 0..200 {
            let scored = vec![("far", 0.0), ("best", 1_000.0), ("near", 900.0)];
            let picked = pick_near_best(scored, 200.0, &mut rng).unwrap();
            assert_ne!(picked, "far");
            if !seen.contains(&picked) {
                seen.push(picked);
            }
        }
        assert_eq!(seen.len(), 2, "both near-best options come up");
    }
}
