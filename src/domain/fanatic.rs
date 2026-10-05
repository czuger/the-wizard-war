use crate::domain::Cult;

/// A fanatic pawn: a cult and a strength of 1 to 3 (the number of initials printed on it).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Fanatic {
    /// The cult the fanatics serve.
    pub cult: Cult,
    /// The pawn's strength, from 1 to 3.
    pub strength: i64,
}

impl Fanatic {
    /// Builds a fanatic pawn.
    pub fn new(cult: Cult, strength: i64) -> Fanatic {
        Fanatic { cult, strength }
    }

    /// Returns the French description shown to players, e.g. "Fanatiques Xéphis (force 2)".
    pub fn label(&self) -> String {
        format!("Fanatiques {} (force {})", self.cult.label(), self.strength)
    }

    /// Returns the image sprite index in `img/fanatics.jpg` (Yaboul 1–3 then Xéphis 1–3).
    pub fn sprite_index(&self) -> i64 {
        let cult_offset = match self.cult {
            Cult::Yaboul => 0,
            Cult::Xephis => 3,
        };
        cult_offset + self.strength - 1
    }
}
