/// One of the two wizard cults fighting over the county of Arghtal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cult {
    /// The cult of the wizard Xéphis (grimoires).
    Xephis,
    /// The cult of the wizard Yaboul (pentacles).
    Yaboul,
}

impl Cult {
    /// Returns the rival cult.
    pub fn opposite(self) -> Cult {
        match self {
            Cult::Xephis => Cult::Yaboul,
            Cult::Yaboul => Cult::Xephis,
        }
    }

    /// Returns the value stored in the database for this cult.
    pub fn as_db_str(self) -> &'static str {
        match self {
            Cult::Xephis => "xephis",
            Cult::Yaboul => "yaboul",
        }
    }

    /// Parses a value stored by [`Cult::as_db_str`].
    ///
    /// # Returns
    /// `None` when the value is not a known cult.
    pub fn from_db_str(value: &str) -> Option<Cult> {
        match value {
            "xephis" => Some(Cult::Xephis),
            "yaboul" => Some(Cult::Yaboul),
            _ => None,
        }
    }

    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            Cult::Xephis => "Xéphis",
            Cult::Yaboul => "Yaboul",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_round_trip() {
        for cult in [Cult::Xephis, Cult::Yaboul] {
            assert_eq!(Cult::from_db_str(cult.as_db_str()), Some(cult));
        }
        assert_eq!(Cult::from_db_str("toratsa"), None);
    }

    #[test]
    fn opposite_is_involutive() {
        assert_eq!(Cult::Xephis.opposite(), Cult::Yaboul);
        assert_eq!(Cult::Yaboul.opposite().opposite(), Cult::Yaboul);
    }
}
