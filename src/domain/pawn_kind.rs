/// The two kinds of pawns that carry magic items out of the city (phase V).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PawnKind {
    /// A lone, slow seller.
    Peddler,
    /// A wagon, faster than a peddler.
    Caravan,
}

impl PawnKind {
    /// Both kinds, in display order.
    pub const ALL: [PawnKind; 2] = [PawnKind::Peddler, PawnKind::Caravan];

    /// Most steps along paths the pawn may take in one turn.
    pub fn speed(self) -> usize {
        match self {
            PawnKind::Peddler => 1,
            PawnKind::Caravan => 2,
        }
    }

    /// Price paid to the bank in phase VII, in écus.
    pub fn price(self) -> i64 {
        match self {
            PawnKind::Peddler => 3_000,
            PawnKind::Caravan => 7_000,
        }
    }

    /// Price the bank pays back for a pawn in the city: half its price.
    pub fn resale_price(self) -> i64 {
        self.price() / 2
    }

    /// Most pawns of this kind one player may own.
    pub fn max_owned(self) -> usize {
        match self {
            PawnKind::Peddler => 4,
            PawnKind::Caravan => 3,
        }
    }

    /// Returns the value stored in the database, also used in image names.
    pub fn key(self) -> &'static str {
        match self {
            PawnKind::Peddler => "peddler",
            PawnKind::Caravan => "caravan",
        }
    }

    /// Parses a value made by [`PawnKind::key`].
    ///
    /// # Returns
    /// `None` when the value is not a known kind.
    pub fn from_key(value: &str) -> Option<PawnKind> {
        PawnKind::ALL.into_iter().find(|kind| kind.key() == value)
    }

    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            PawnKind::Peddler => "colporteur",
            PawnKind::Caravan => "caravane",
        }
    }

    /// Returns the French name with its definite article, e.g. « la caravane ».
    pub fn with_article(self) -> &'static str {
        match self {
            PawnKind::Peddler => "le colporteur",
            PawnKind::Caravan => "la caravane",
        }
    }

    /// Returns the French name after « à », e.g. « au colporteur ».
    pub fn to_article(self) -> &'static str {
        match self {
            PawnKind::Peddler => "au colporteur",
            PawnKind::Caravan => "à la caravane",
        }
    }

    /// Returns the French name of another pawn of the same kind, e.g. « une autre caravane ».
    pub fn another(self) -> &'static str {
        match self {
            PawnKind::Peddler => "un autre colporteur",
            PawnKind::Caravan => "une autre caravane",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_round_trip_and_prices() {
        for kind in PawnKind::ALL {
            assert_eq!(PawnKind::from_key(kind.key()), Some(kind));
        }
        assert_eq!(PawnKind::Caravan.resale_price(), 3_500);
        assert_eq!(PawnKind::from_key("wagon"), None);
    }
}
