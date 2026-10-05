/// How rich a player looks to the others: they only see the thickness of the banknote pile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Wealth {
    coins: usize,
}

impl Wealth {
    /// Estimates the wealth shown for `money` écus.
    pub fn of(money: i64) -> Self {
        let coins = match money {
            ..5_000 => 1,
            5_000..10_000 => 2,
            10_000..20_000 => 3,
            _ => 4,
        };
        Self { coins }
    }

    /// Number of coin icons drawn, from 1 (under 5,000 écus) to 4 (20,000 écus or more).
    pub fn coins(self) -> usize {
        self.coins
    }

    /// French range of écus matching the coins, for tooltips.
    pub fn label(self) -> &'static str {
        match self.coins {
            1 => "moins de 5 000 écus",
            2 => "de 5 000 à 10 000 écus",
            3 => "de 10 000 à 20 000 écus",
            _ => "plus de 20 000 écus",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn brackets_follow_the_banknote_piles() {
        assert_eq!(Wealth::of(0).coins(), 1);
        assert_eq!(Wealth::of(4_900).coins(), 1);
        assert_eq!(Wealth::of(5_000).coins(), 2);
        assert_eq!(Wealth::of(9_900).coins(), 2);
        assert_eq!(Wealth::of(10_000).coins(), 3);
        assert_eq!(Wealth::of(19_900).coins(), 3);
        assert_eq!(Wealth::of(20_000).coins(), 4);
        assert_eq!(Wealth::of(150_000).coins(), 4);
    }
}
