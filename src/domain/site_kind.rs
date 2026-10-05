/// What stands on a site of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SiteKind {
    /// A village: its cult may change during the game.
    Village,
    /// A castle, marked by a banner: once converted, it never changes cult.
    Castle,
    /// A monastery, marked by a large cross: once converted, it never changes cult.
    Monastery,
}

impl SiteKind {
    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            SiteKind::Village => "village",
            SiteKind::Castle => "château",
            SiteKind::Monastery => "monastère",
        }
    }

    /// Whether a cult marker placed here stays for the rest of the game (restriction II.1 c).
    pub fn is_permanent(self) -> bool {
        self != SiteKind::Village
    }
}
