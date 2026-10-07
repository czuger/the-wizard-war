use rand::Rng;
use rand::seq::SliceRandom;

use crate::domain::Cult;
use crate::domain::Fanatic;

/// Strength distribution of the 24 fanatic pawns: (cult, strength, number of pawns).
///
/// The rules give 14 Xéphis and 10 Yaboul pawns but not their strengths; this split is an assumption.
const POOL_COMPOSITION: [(Cult, i64, usize); 6] = [
    (Cult::Xephis, 1, 4),
    (Cult::Xephis, 2, 5),
    (Cult::Xephis, 3, 5),
    (Cult::Yaboul, 1, 4),
    (Cult::Yaboul, 2, 3),
    (Cult::Yaboul, 3, 3),
];

/// The hands dealt at setup, and the pawns left in the box.
#[derive(Debug, Clone)]
pub struct FanaticDeal {
    /// One hand per player, in seat order.
    pub hands: Vec<Vec<Fanatic>>,
    /// Pawns dealt to nobody: there are some with fewer than 6 players.
    pub spare: Vec<Fanatic>,
}

/// The fanatic pawns dealt to the players at setup.
#[derive(Debug, Clone)]
pub struct FanaticPool {
    tokens: Vec<Fanatic>,
}

impl FanaticPool {
    /// Returns the full, unshuffled set of 24 fanatic pawns.
    pub fn full() -> FanaticPool {
        let tokens = POOL_COMPOSITION
            .iter()
            .flat_map(|&(cult, strength, count)| {
                std::iter::repeat_n(Fanatic::new(cult, strength), count)
            })
            .collect();
        FanaticPool { tokens }
    }

    /// Returns the pawns in the pool.
    pub fn tokens(&self) -> &[Fanatic] {
        &self.tokens
    }

    /// Returns how many pawns each player receives, or `None` for an unsupported player count.
    pub fn tokens_per_player(player_count: usize) -> Option<usize> {
        match player_count {
            2 => Some(6),
            3 | 4 => Some(5),
            5 | 6 => Some(4),
            _ => None,
        }
    }

    /// Shuffles the pool and deals one hand per player.
    ///
    /// # Returns
    /// `None` for an unsupported player count.
    pub fn deal<R: Rng>(mut self, player_count: usize, rng: &mut R) -> Option<FanaticDeal> {
        let per_player = FanaticPool::tokens_per_player(player_count)?;
        self.tokens.shuffle(rng);
        let spare = self.tokens.split_off(per_player * player_count);
        let hands = self
            .tokens
            .chunks(per_player)
            .map(<[Fanatic]>::to_vec)
            .collect();
        Some(FanaticDeal { hands, spare })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_pool_has_fourteen_xephis_and_ten_yaboul() {
        let pool = FanaticPool::full();
        let count = |cult| pool.tokens().iter().filter(|f| f.cult == cult).count();
        assert_eq!(pool.tokens().len(), 24);
        assert_eq!(count(Cult::Xephis), 14);
        assert_eq!(count(Cult::Yaboul), 10);
    }

    #[test]
    fn deal_sizes_follow_player_count() {
        let mut rng = rand::rng();
        for (players, per_player) in [(2, 6), (3, 5), (4, 5), (5, 4), (6, 4)] {
            let deal = FanaticPool::full().deal(players, &mut rng).unwrap();
            assert_eq!(deal.hands.len(), players);
            assert!(deal.hands.iter().all(|hand| hand.len() == per_player));
            assert_eq!(deal.spare.len(), 24 - players * per_player);
        }
        assert!(FanaticPool::full().deal(7, &mut rng).is_none());
    }
}
