/// The result of a fight between a rebels pawn and a peddler or caravan (phase II.2 table).
///
/// Whatever the result, the rebels pawn leaves the game afterwards.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FightOutcome {
    /// 1–2: the brawl calms down.
    CalmedDown,
    /// 3–4: the pawn loses its item, back to the supply; an empty pawn stops for one turn.
    Robbed,
    /// 5–6: the pawn and its item are removed from the game.
    Destroyed,
}

impl FightOutcome {
    /// Reads the die.
    ///
    /// # Arguments
    /// * `die` - a roll from 1 to 6; higher values count as 6.
    pub fn from_die(die: u8) -> FightOutcome {
        match die {
            ..=2 => FightOutcome::CalmedDown,
            3 | 4 => FightOutcome::Robbed,
            _ => FightOutcome::Destroyed,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_die() {
        assert_eq!(FightOutcome::from_die(2), FightOutcome::CalmedDown);
        assert_eq!(FightOutcome::from_die(3), FightOutcome::Robbed);
        assert_eq!(FightOutcome::from_die(6), FightOutcome::Destroyed);
    }
}
