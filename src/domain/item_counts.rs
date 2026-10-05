use crate::domain::MagicItem;

/// A number of items of each kind: a supply, a stock or a production.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ItemCounts {
    /// Number of Toratsa talismans.
    pub toratsa: i64,
    /// Number of Xéphis grimoires.
    pub xephis: i64,
    /// Number of Yaboul pentacles.
    pub yaboul: i64,
}

impl ItemCounts {
    /// Builds counts from one value per item kind.
    pub fn new(toratsa: i64, xephis: i64, yaboul: i64) -> ItemCounts {
        ItemCounts {
            toratsa,
            xephis,
            yaboul,
        }
    }

    /// Returns the full box of magic items available at the start of the game.
    pub fn initial_supply() -> ItemCounts {
        ItemCounts::new(
            MagicItem::Toratsa.initial_supply(),
            MagicItem::Xephis.initial_supply(),
            MagicItem::Yaboul.initial_supply(),
        )
    }

    /// Returns the count for one item kind.
    pub fn get(&self, item: MagicItem) -> i64 {
        match item {
            MagicItem::Toratsa => self.toratsa,
            MagicItem::Xephis => self.xephis,
            MagicItem::Yaboul => self.yaboul,
        }
    }

    /// Returns a copy with the count for one item kind replaced.
    pub fn with(self, item: MagicItem, count: i64) -> ItemCounts {
        let mut counts = self;
        match item {
            MagicItem::Toratsa => counts.toratsa = count,
            MagicItem::Xephis => counts.xephis = count,
            MagicItem::Yaboul => counts.yaboul = count,
        }
        counts
    }

    /// Returns the number of items of all kinds.
    pub fn total(&self) -> i64 {
        self.toratsa + self.xephis + self.yaboul
    }

    /// Adds two counts kind by kind.
    pub fn plus(self, other: ItemCounts) -> ItemCounts {
        ItemCounts::new(
            self.toratsa + other.toratsa,
            self.xephis + other.xephis,
            self.yaboul + other.yaboul,
        )
    }

    /// Subtracts `other` kind by kind.
    pub fn minus(self, other: ItemCounts) -> ItemCounts {
        ItemCounts::new(
            self.toratsa - other.toratsa,
            self.xephis - other.xephis,
            self.yaboul - other.yaboul,
        )
    }

    /// Returns `true` when every count is zero or more.
    pub fn is_non_negative(&self) -> bool {
        self.toratsa >= 0 && self.xephis >= 0 && self.yaboul >= 0
    }

    /// Returns `true` when every count of `self` is at most the matching count of `other`.
    pub fn fits_within(&self, other: &ItemCounts) -> bool {
        other.minus(*self).is_non_negative()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn arithmetic() {
        let a = ItemCounts::new(1, 2, 3);
        let b = ItemCounts::new(1, 1, 1);
        assert_eq!(a.plus(b), ItemCounts::new(2, 3, 4));
        assert_eq!(a.minus(b), ItemCounts::new(0, 1, 2));
        assert_eq!(a.total(), 6);
        assert_eq!(a.with(MagicItem::Xephis, 0).get(MagicItem::Xephis), 0);
    }

    #[test]
    fn fits_within_checks_each_kind() {
        let supply = ItemCounts::new(1, 0, 5);
        assert!(ItemCounts::new(1, 0, 2).fits_within(&supply));
        assert!(!ItemCounts::new(0, 1, 0).fits_within(&supply));
    }
}
