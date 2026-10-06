/// How well the computer-controlled alchemists of a game play. Chosen when the game is
/// created, the same for every computer-controlled alchemist.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AiLevel {
    /// The basic strategy of [`crate::domain::ai_strategy`] and [`crate::domain::ai_trading`].
    #[default]
    Beginner,
    /// The evaluating strategy of [`crate::domain::strong_ai`].
    Strong,
}

impl AiLevel {
    /// Every level, in the order the creation form offers them.
    pub const ALL: [AiLevel; 2] = [AiLevel::Beginner, AiLevel::Strong];

    /// Returns the value stored in the database and used in forms.
    pub fn key(self) -> &'static str {
        match self {
            AiLevel::Beginner => "beginner",
            AiLevel::Strong => "strong",
        }
    }

    /// Parses a value made by [`AiLevel::key`].
    ///
    /// # Returns
    /// `None` when the value is not a known level.
    pub fn from_key(value: &str) -> Option<AiLevel> {
        AiLevel::ALL.into_iter().find(|level| level.key() == value)
    }

    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            AiLevel::Beginner => "Débutant",
            AiLevel::Strong => "Fort",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_round_trip() {
        for level in AiLevel::ALL {
            assert_eq!(AiLevel::from_key(level.key()), Some(level));
        }
        assert_eq!(AiLevel::from_key("expert"), None);
    }
}
