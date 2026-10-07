use crate::domain::Phase;

/// The part of the turn a log line belongs to, following the rules' phases I to VII.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogSection {
    /// Start of the game or of a turn, and end of the game.
    Turn,
    /// Phase I: announcements and expenses.
    Announcements,
    /// Phase II: conversions, rebel moves and fights.
    Conversions,
    /// Phase III: production of magic items.
    Production,
    /// Phase IV: the great hall's fanatics.
    Fanatics,
    /// Phase V: peddlers and caravans.
    Peddlers,
    /// Phase VI: sales in the city.
    CitySales,
    /// Phase VII: purchases.
    Purchases,
}

impl LogSection {
    /// Returns the section of the lines logged during a game phase.
    pub fn of(phase: Phase) -> LogSection {
        match phase {
            Phase::Announcements | Phase::Expenses => LogSection::Announcements,
            Phase::Conversions | Phase::RebelMovement | Phase::RebelFights => {
                LogSection::Conversions
            }
            Phase::Production => LogSection::Production,
            Phase::Peddlers => LogSection::Peddlers,
            Phase::CitySales => LogSection::CitySales,
            Phase::Purchases => LogSection::Purchases,
            Phase::Finished => LogSection::Turn,
        }
    }

    /// Returns the value stored in the database for this section.
    pub fn as_db_str(self) -> &'static str {
        match self {
            LogSection::Turn => "turn",
            LogSection::Announcements => "announcements",
            LogSection::Conversions => "conversions",
            LogSection::Production => "production",
            LogSection::Fanatics => "fanatics",
            LogSection::Peddlers => "peddlers",
            LogSection::CitySales => "city_sales",
            LogSection::Purchases => "purchases",
        }
    }

    /// Parses a value stored by [`LogSection::as_db_str`].
    ///
    /// # Returns
    /// `None` when the value is not a known section.
    pub fn from_db_str(value: &str) -> Option<LogSection> {
        match value {
            "turn" => Some(LogSection::Turn),
            "announcements" => Some(LogSection::Announcements),
            "conversions" => Some(LogSection::Conversions),
            "production" => Some(LogSection::Production),
            "fanatics" => Some(LogSection::Fanatics),
            "peddlers" => Some(LogSection::Peddlers),
            "city_sales" => Some(LogSection::CitySales),
            "purchases" => Some(LogSection::Purchases),
            _ => None,
        }
    }

    /// Returns the roman numeral of the rules' phase, empty for [`LogSection::Turn`].
    pub fn numeral(self) -> &'static str {
        match self {
            LogSection::Turn => "",
            LogSection::Announcements => "I",
            LogSection::Conversions => "II",
            LogSection::Production => "III",
            LogSection::Fanatics => "IV",
            LogSection::Peddlers => "V",
            LogSection::CitySales => "VI",
            LogSection::Purchases => "VII",
        }
    }

    /// Returns the French name of the section, shown as a tooltip.
    pub fn label(self) -> &'static str {
        match self {
            LogSection::Turn => "Déroulement de la partie",
            LogSection::Announcements => "Phase I — Annonces et dépenses",
            LogSection::Conversions => "Phase II — Conversions et rebelles",
            LogSection::Production => "Phase III — Fabrication",
            LogSection::Fanatics => "Phase IV — Fanatiques",
            LogSection::Peddlers => "Phase V — Colporteurs et caravanes",
            LogSection::CitySales => "Phase VI — Ventes intra-muros",
            LogSection::Purchases => "Phase VII — Achats",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL: [LogSection; 8] = [
        LogSection::Turn,
        LogSection::Announcements,
        LogSection::Conversions,
        LogSection::Production,
        LogSection::Fanatics,
        LogSection::Peddlers,
        LogSection::CitySales,
        LogSection::Purchases,
    ];

    #[test]
    fn db_round_trip() {
        for section in ALL {
            assert_eq!(LogSection::from_db_str(section.as_db_str()), Some(section));
        }
        assert_eq!(LogSection::from_db_str("selling"), None);
    }

    #[test]
    fn both_halves_of_phase_one_share_a_section() {
        assert_eq!(
            LogSection::of(Phase::Announcements),
            LogSection::of(Phase::Expenses)
        );
        assert_eq!(LogSection::of(Phase::RebelFights), LogSection::Conversions);
    }
}
