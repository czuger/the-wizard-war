use crate::domain::Cult;

/// A kind of magic item an alchemist can make.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MagicItem {
    /// Talisman of the cult-free abbot Toratsa.
    Toratsa,
    /// Grimoire of the Xéphis cult.
    Xephis,
    /// Pentacle of the Yaboul cult.
    Yaboul,
}

impl MagicItem {
    /// Every kind of magic item, in display order.
    pub const ALL: [MagicItem; 3] = [MagicItem::Toratsa, MagicItem::Xephis, MagicItem::Yaboul];

    /// Returns the cult the item belongs to, or `None` for Toratsa talismans.
    pub fn cult(self) -> Option<Cult> {
        match self {
            MagicItem::Toratsa => None,
            MagicItem::Xephis => Some(Cult::Xephis),
            MagicItem::Yaboul => Some(Cult::Yaboul),
        }
    }

    /// Returns the item made for a cult.
    pub fn of_cult(cult: Cult) -> MagicItem {
        match cult {
            Cult::Xephis => MagicItem::Xephis,
            Cult::Yaboul => MagicItem::Yaboul,
        }
    }

    /// Returns how many of these items are in the box at the start of the game.
    pub fn initial_supply(self) -> i64 {
        match self {
            MagicItem::Toratsa => 20,
            MagicItem::Xephis => 25,
            MagicItem::Yaboul => 35,
        }
    }

    /// Returns a stable identifier used in form fields.
    pub fn key(self) -> &'static str {
        match self {
            MagicItem::Toratsa => "toratsa",
            MagicItem::Xephis => "xephis",
            MagicItem::Yaboul => "yaboul",
        }
    }

    /// Returns the path of the item's token image.
    pub fn image(self) -> &'static str {
        match self {
            MagicItem::Toratsa => "/img/token-talisman-toratsa.png",
            MagicItem::Xephis => "/img/token-grimoire-xephis.png",
            MagicItem::Yaboul => "/img/token-pentacle-yaboul.png",
        }
    }

    /// Parses a value made by [`MagicItem::key`].
    ///
    /// # Returns
    /// `None` when the value is not a known item.
    pub fn from_key(value: &str) -> Option<MagicItem> {
        MagicItem::ALL.into_iter().find(|item| item.key() == value)
    }

    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            MagicItem::Toratsa => "Talisman Toratsa",
            MagicItem::Xephis => "Grimoire Xéphis",
            MagicItem::Yaboul => "Pentacle Yaboul",
        }
    }

    /// Returns the French name with an indefinite article, e.g. « un grimoire Xéphis ».
    pub fn with_article(self) -> &'static str {
        match self {
            MagicItem::Toratsa => "un talisman Toratsa",
            MagicItem::Xephis => "un grimoire Xéphis",
            MagicItem::Yaboul => "un pentacle Yaboul",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initial_supply_matches_rules() {
        let total: i64 = MagicItem::ALL
            .iter()
            .map(|item| item.initial_supply())
            .sum();
        assert_eq!(total, 80);
    }

    #[test]
    fn cult_items_round_trip() {
        for cult in [Cult::Xephis, Cult::Yaboul] {
            assert_eq!(MagicItem::of_cult(cult).cult(), Some(cult));
        }
        assert_eq!(MagicItem::Toratsa.cult(), None);
        assert_eq!(MagicItem::from_key("yaboul"), Some(MagicItem::Yaboul));
    }
}
