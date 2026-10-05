/// The step of the turn a game is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Phase I.1: each player may send one fanatic to the great hall.
    Announcements,
    /// Phase I.2: each player secretly pays into the collective expense.
    Expenses,
    /// Phase II.1: players convert sites of the map one after another.
    Conversions,
    /// Phase II.2: the turn's first player moves every rebels pawn.
    RebelMovement,
    /// Phase II.2: the turn's first player picks the pawn each arriving rebels pawn attacks.
    RebelFights,
    /// Phase III: players take magic items from the supply one after another.
    Production,
    /// Phase V: players move their peddlers and caravans, then sell, one after another.
    Peddlers,
    /// Phase VI: players sell items from their stock in the city, one at a time, in turns.
    CitySales,
    /// Phase VII: players buy pawns and laboratories from the bank one after another.
    Purchases,
    /// The last turn is over.
    Finished,
}

impl Phase {
    /// Returns the value stored in the database for this phase.
    pub fn as_db_str(self) -> &'static str {
        match self {
            Phase::Announcements => "announcements",
            Phase::Expenses => "expenses",
            Phase::Conversions => "conversions",
            Phase::RebelMovement => "rebel_movement",
            Phase::RebelFights => "rebel_fights",
            Phase::Production => "production",
            Phase::Peddlers => "peddlers",
            Phase::CitySales => "city_sales",
            Phase::Purchases => "purchases",
            Phase::Finished => "finished",
        }
    }

    /// Parses a value stored by [`Phase::as_db_str`].
    ///
    /// # Returns
    /// `None` when the value is not a known phase.
    pub fn from_db_str(value: &str) -> Option<Phase> {
        match value {
            "announcements" => Some(Phase::Announcements),
            "expenses" => Some(Phase::Expenses),
            "conversions" => Some(Phase::Conversions),
            "rebel_movement" => Some(Phase::RebelMovement),
            "rebel_fights" => Some(Phase::RebelFights),
            "production" => Some(Phase::Production),
            "peddlers" => Some(Phase::Peddlers),
            "city_sales" => Some(Phase::CitySales),
            "purchases" => Some(Phase::Purchases),
            "finished" => Some(Phase::Finished),
            _ => None,
        }
    }

    /// Returns the French title shown to players.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Announcements => "Phase I — Annonces",
            Phase::Expenses => "Phase I — Dépenses",
            Phase::Conversions => "Phase II — Conversions",
            Phase::RebelMovement => "Phase II — Déplacement des rebelles",
            Phase::RebelFights => "Phase II — Combats des rebelles",
            Phase::Production => "Phase III — Fabrication d'objets magiques",
            Phase::Peddlers => "Phase V — Colporteurs et caravanes",
            Phase::CitySales => "Phase VI — Ventes intra-muros",
            Phase::Purchases => "Phase VII — Achats",
            Phase::Finished => "Partie terminée",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_round_trip() {
        for phase in [
            Phase::Announcements,
            Phase::Expenses,
            Phase::Conversions,
            Phase::RebelMovement,
            Phase::RebelFights,
            Phase::Production,
            Phase::Peddlers,
            Phase::CitySales,
            Phase::Purchases,
            Phase::Finished,
        ] {
            assert_eq!(Phase::from_db_str(phase.as_db_str()), Some(phase));
        }
        assert_eq!(Phase::from_db_str("selling"), None);
    }
}
