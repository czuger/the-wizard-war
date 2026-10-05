use crate::domain::Cult;
use crate::domain::Fanatic;
use crate::domain::ItemCounts;
use crate::domain::MagicItem;

/// Returns the Dominant Fanatic: the cult with more strength in the great hall.
///
/// # Returns
/// `None` when the hall is empty or both cults are tied, so the fanatics do not raid.
pub fn dominant_cult(hall: &[Fanatic]) -> Option<Cult> {
    let strength_of = |cult: Cult| -> i64 {
        hall.iter()
            .filter(|fanatic| fanatic.cult == cult)
            .map(|fanatic| fanatic.strength)
            .sum()
    };
    let xephis = strength_of(Cult::Xephis);
    let yaboul = strength_of(Cult::Yaboul);
    match xephis.cmp(&yaboul) {
        std::cmp::Ordering::Greater => Some(Cult::Xephis),
        std::cmp::Ordering::Less => Some(Cult::Yaboul),
        std::cmp::Ordering::Equal => None,
    }
}

/// What the dominant fanatics do to one alchemist, judged on this turn's production.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FanaticVerdict {
    /// No cult item was made this turn (Toratsa talismans are never counted).
    Unaffected,
    /// Every cult item made is of the dominant cult: the alchemist gets one free item of it.
    Favoured {
        /// The dominant cult.
        cult: Cult,
    },
    /// Items of both cults were made: those of the opposed cult are burned.
    Purged {
        /// The cult whose items are burned.
        burned: Cult,
    },
    /// Every cult item made is of the opposed cult: they are destroyed with the whole stock.
    Ravaged,
}

impl FanaticVerdict {
    /// Judges an alchemist's production against the dominant cult.
    pub fn judge(dominant: Cult, production: &ItemCounts) -> FanaticVerdict {
        let favoured = production.get(MagicItem::of_cult(dominant));
        let opposed = production.get(MagicItem::of_cult(dominant.opposite()));
        match (favoured > 0, opposed > 0) {
            (false, false) => FanaticVerdict::Unaffected,
            (true, false) => FanaticVerdict::Favoured { cult: dominant },
            (true, true) => FanaticVerdict::Purged {
                burned: dominant.opposite(),
            },
            (false, true) => FanaticVerdict::Ravaged,
        }
    }

    /// Returns the alchemist's stock once this turn's production has gone through the verdict.
    ///
    /// # Arguments
    /// * `bonus_available` - whether the supply still holds an item for a [`FanaticVerdict::Favoured`] alchemist.
    pub fn new_stock(
        &self,
        production: ItemCounts,
        stock: ItemCounts,
        bonus_available: bool,
    ) -> ItemCounts {
        match *self {
            FanaticVerdict::Unaffected => stock.plus(production),
            FanaticVerdict::Favoured { cult } => {
                let bonus = if bonus_available { 1 } else { 0 };
                let item = MagicItem::of_cult(cult);
                let produced = production.with(item, production.get(item) + bonus);
                stock.plus(produced)
            }
            FanaticVerdict::Purged { burned } => {
                stock.plus(production.with(MagicItem::of_cult(burned), 0))
            }
            // Toratsa talismans being made are not counted by the fanatics, so they survive.
            FanaticVerdict::Ravaged => ItemCounts::new(production.toratsa, 0, 0),
        }
    }

    /// Returns the item the supply must give for this verdict, if any.
    pub fn bonus_item(&self) -> Option<MagicItem> {
        match *self {
            FanaticVerdict::Favoured { cult } => Some(MagicItem::of_cult(cult)),
            _ => None,
        }
    }

    /// Returns the French sentence describing the verdict for `player_name`.
    pub fn describe(&self, player_name: &str, bonus_given: bool) -> String {
        match *self {
            FanaticVerdict::Unaffected => {
                format!("Les fanatiques épargnent le laboratoire de {player_name}.")
            }
            FanaticVerdict::Favoured { cult } if bonus_given => format!(
                "Les fanatiques {} aident {player_name}, qui reçoit un objet magique gratuit.",
                cult.label()
            ),
            FanaticVerdict::Favoured { cult } => format!(
                "Les fanatiques {} aident {player_name}, mais la réserve est vide.",
                cult.label()
            ),
            FanaticVerdict::Purged { burned } => format!(
                "Les fanatiques brûlent les objets {} fabriqués par {player_name}.",
                burned.label()
            ),
            FanaticVerdict::Ravaged => format!(
                "Les fanatiques saccagent le laboratoire de {player_name} : fabrication et stock détruits."
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dominant_cult_sums_strengths() {
        let hall = [
            Fanatic::new(Cult::Xephis, 1),
            Fanatic::new(Cult::Xephis, 1),
            Fanatic::new(Cult::Yaboul, 3),
        ];
        assert_eq!(dominant_cult(&hall), Some(Cult::Yaboul));
    }

    #[test]
    fn empty_or_tied_hall_has_no_dominant() {
        assert_eq!(dominant_cult(&[]), None);
        let hall = [Fanatic::new(Cult::Xephis, 2), Fanatic::new(Cult::Yaboul, 2)];
        assert_eq!(dominant_cult(&hall), None);
    }

    #[test]
    fn toratsa_only_production_is_unaffected() {
        let production = ItemCounts::new(2, 0, 0);
        let verdict = FanaticVerdict::judge(Cult::Xephis, &production);
        assert_eq!(verdict, FanaticVerdict::Unaffected);
        let stock = ItemCounts::new(0, 1, 1);
        assert_eq!(
            verdict.new_stock(production, stock, true),
            ItemCounts::new(2, 1, 1)
        );
    }

    #[test]
    fn favoured_alchemist_gets_a_bonus_item() {
        let production = ItemCounts::new(1, 2, 0);
        let verdict = FanaticVerdict::judge(Cult::Xephis, &production);
        assert_eq!(verdict, FanaticVerdict::Favoured { cult: Cult::Xephis });
        assert_eq!(verdict.bonus_item(), Some(MagicItem::Xephis));
        let stock = ItemCounts::default();
        assert_eq!(
            verdict.new_stock(production, stock, true),
            ItemCounts::new(1, 3, 0)
        );
        assert_eq!(
            verdict.new_stock(production, stock, false),
            ItemCounts::new(1, 2, 0)
        );
    }

    #[test]
    fn mixed_production_loses_opposed_items() {
        let production = ItemCounts::new(0, 1, 2);
        let verdict = FanaticVerdict::judge(Cult::Yaboul, &production);
        assert_eq!(
            verdict,
            FanaticVerdict::Purged {
                burned: Cult::Xephis
            }
        );
        let stock = ItemCounts::new(1, 1, 1);
        assert_eq!(
            verdict.new_stock(production, stock, true),
            ItemCounts::new(1, 1, 3)
        );
    }

    #[test]
    fn opposed_production_destroys_the_stock() {
        let production = ItemCounts::new(1, 0, 2);
        let verdict = FanaticVerdict::judge(Cult::Xephis, &production);
        assert_eq!(verdict, FanaticVerdict::Ravaged);
        let stock = ItemCounts::new(3, 3, 3);
        assert_eq!(
            verdict.new_stock(production, stock, true),
            ItemCounts::new(1, 0, 0)
        );
    }
}
