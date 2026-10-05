/// The step of the turn a game is in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Phase I.1: each player may send one fanatic to the great hall.
    Announcements,
    /// Phase I.2: each player secretly pays into the collective expense.
    Expenses,
    /// Phase III: players take magic items from the supply one after another.
    Production,
    /// The last turn is over.
    Finished,
}

impl Phase {
    /// Returns the value stored in the database for this phase.
    pub fn as_db_str(self) -> &'static str {
        match self {
            Phase::Announcements => "announcements",
            Phase::Expenses => "expenses",
            Phase::Production => "production",
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
            "production" => Some(Phase::Production),
            "finished" => Some(Phase::Finished),
            _ => None,
        }
    }

    /// Returns the French title shown to players.
    pub fn label(self) -> &'static str {
        match self {
            Phase::Announcements => "Phase I — Annonces",
            Phase::Expenses => "Phase I — Dépenses",
            Phase::Production => "Phase III — Fabrication d'objets magiques",
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
            Phase::Production,
            Phase::Finished,
        ] {
            assert_eq!(Phase::from_db_str(phase.as_db_str()), Some(phase));
        }
        assert_eq!(Phase::from_db_str("selling"), None);
    }
}
