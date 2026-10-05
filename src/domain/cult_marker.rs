use crate::domain::Cult;

/// A cult marker lying on a site of the map.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CultMarker {
    /// The cult the site follows.
    pub cult: Cult,
    /// Whether the marker shows its emblem; flipping it hides it, and a hidden marker can be
    /// replaced by the rival cult.
    pub face_up: bool,
}

impl CultMarker {
    /// A marker freshly placed, emblem up.
    pub fn face_up(cult: Cult) -> CultMarker {
        CultMarker {
            cult,
            face_up: true,
        }
    }

    /// The same marker turned over.
    pub fn flipped(self) -> CultMarker {
        CultMarker {
            face_up: !self.face_up,
            ..self
        }
    }

    /// Describes the marker for players, e.g. « Xéphis, face cachée ».
    pub fn describe(self) -> String {
        let side = if self.face_up {
            "face visible"
        } else {
            "face cachée"
        };
        format!("{}, {side}", self.cult.label())
    }
}
