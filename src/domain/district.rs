/// One of the six coloured districts of the city of Argh. Each alchemist's laboratories
/// stand in one district, and their peddlers and caravans share its colour.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum District {
    /// The blue district.
    Blue,
    /// The white district.
    White,
    /// The yellow district.
    Yellow,
    /// The pink (red) district.
    Pink,
    /// The black district.
    Black,
    /// The green district.
    Green,
}

impl District {
    /// Every district, in the order AIs are given the ones left.
    pub const ALL: [District; 6] = [
        District::Blue,
        District::White,
        District::Yellow,
        District::Pink,
        District::Black,
        District::Green,
    ];

    /// Returns the value stored in the database, also used in forms and image names.
    pub fn key(self) -> &'static str {
        match self {
            District::Blue => "blue",
            District::White => "white",
            District::Yellow => "yellow",
            District::Pink => "pink",
            District::Black => "black",
            District::Green => "green",
        }
    }

    /// Parses a value made by [`District::key`].
    ///
    /// # Returns
    /// `None` when the value is not a known district.
    pub fn from_key(value: &str) -> Option<District> {
        District::ALL
            .into_iter()
            .find(|district| district.key() == value)
    }

    /// Returns the French name shown to players, e.g. « quartier bleu ».
    pub fn label(self) -> &'static str {
        match self {
            District::Blue => "quartier bleu",
            District::White => "quartier blanc",
            District::Yellow => "quartier jaune",
            District::Pink => "quartier rose",
            District::Black => "quartier noir",
            District::Green => "quartier vert",
        }
    }

    /// Returns the centre of the district's houses, in pixels of `img/board.jpg`.
    pub fn board_position(self) -> (u16, u16) {
        match self {
            District::Blue => (790, 520),
            District::White => (890, 522),
            District::Yellow => (750, 572),
            District::Pink => (795, 635),
            District::Black => (920, 590),
            District::Green => (875, 637),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_round_trip() {
        for district in District::ALL {
            assert_eq!(District::from_key(district.key()), Some(district));
        }
        assert_eq!(District::from_key("red"), None);
    }
}
