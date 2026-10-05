/// One of the three zones around the city, which set sale prices in phase V.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Zone {
    /// The outskirts (*faubourgs*), just outside the walls.
    Outskirts,
    /// The countryside (*campagne*).
    Countryside,
    /// The mountain (*montagne*).
    Mountain,
}

impl Zone {
    /// Returns the French name shown to players.
    pub fn label(self) -> &'static str {
        match self {
            Zone::Outskirts => "faubourgs",
            Zone::Countryside => "campagne",
            Zone::Mountain => "montagne",
        }
    }
}
