use crate::domain::{SiteKind, Zone};

/// Identifier of a site of the map, from 1 to 32.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SiteId(pub u8);

impl SiteId {
    /// Returns the value stored in the database.
    pub fn as_db(self) -> i64 {
        i64::from(self.0)
    }

    /// Reads a value stored by [`SiteId::as_db`].
    ///
    /// # Returns
    /// `None` when the value is out of range; it may still not be a site of the board.
    pub fn from_db(value: i64) -> Option<SiteId> {
        u8::try_from(value).ok().map(SiteId)
    }
}

/// A village, castle or monastery of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Site {
    /// Identifier, also shown on the map.
    pub id: SiteId,
    /// French name invented for the site (the original board has none).
    pub name: &'static str,
    /// Village, castle or monastery.
    pub kind: SiteKind,
    /// Zone the site lies in.
    pub zone: Zone,
    /// Horizontal position on `img/board.jpg`, in pixels.
    pub x: u16,
    /// Vertical position on `img/board.jpg`, in pixels.
    pub y: u16,
    /// Whether a path joins the site to an edge of the board (restriction II.1 a).
    pub on_edge: bool,
}

impl Site {
    /// Describes a site for players, e.g. « Valbrune (village, campagne) ».
    pub fn describe(&self) -> String {
        format!(
            "{} ({}, {})",
            self.name,
            self.kind.label(),
            self.zone.label()
        )
    }
}
