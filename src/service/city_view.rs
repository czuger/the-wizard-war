use crate::db::{FanaticRecord, PlayerRecord};
use crate::domain::board::{BOARD_HEIGHT, BOARD_WIDTH, HALL_POSITION};

const HIDDEN_FANATIC_IMAGE: &str = "/img/token-fanatic-hidden.png";

/// One district of the city, with its owner's laboratories.
#[derive(Debug, Clone)]
pub struct DistrictView {
    /// Horizontal position, in percent of the map's width.
    pub left: String,
    /// Vertical position, in percent of the map's height.
    pub top: String,
    /// Number of laboratory tokens to draw.
    pub laboratories: usize,
    /// French description shown on hover, e.g. « Laboratoires de Morgane (3) ».
    pub title: String,
}

/// A fanatic pawn in the great hall: face down until phase IV, then face up.
#[derive(Debug, Clone)]
pub struct HallToken {
    /// Path of the pawn's image, or of the hidden face.
    pub image: String,
    /// French description shown on hover.
    pub title: String,
}

/// The city of Argh as drawn on the map: laboratories and the great hall.
#[derive(Debug, Clone, Default)]
pub struct CityView {
    /// One entry per player, in seat order.
    pub districts: Vec<DistrictView>,
    /// Horizontal position of the hall, in percent of the map's width.
    pub hall_left: String,
    /// Vertical position of the hall, in percent of the map's height.
    pub hall_top: String,
    /// Pawns in the hall, in the order they were sent.
    pub hall: Vec<HallToken>,
}

impl CityView {
    /// Builds the view of the city.
    ///
    /// # Arguments
    /// * `hall` - the fanatics in the great hall; only revealed ones show their face.
    pub fn build(players: &[PlayerRecord], hall: &[FanaticRecord]) -> CityView {
        CityView {
            districts: players.iter().map(district_view).collect(),
            hall_left: percent(HALL_POSITION.0, BOARD_WIDTH),
            hall_top: percent(HALL_POSITION.1, BOARD_HEIGHT),
            hall: hall.iter().map(hall_token).collect(),
        }
    }
}

/// Formats a pixel coordinate of `img/board.jpg` as a CSS percentage.
pub fn percent(value: u16, total: u16) -> String {
    format!("{:.2}", f64::from(value) * 100.0 / f64::from(total))
}

fn district_view(player: &PlayerRecord) -> DistrictView {
    let (x, y) = player.district.board_position();
    DistrictView {
        left: percent(x, BOARD_WIDTH),
        top: percent(y, BOARD_HEIGHT),
        laboratories: usize::try_from(player.laboratories).unwrap_or(0),
        title: format!(
            "Laboratoires de {} ({}) — {}",
            player.name,
            player.laboratories,
            player.district.label()
        ),
    }
}

fn hall_token(record: &FanaticRecord) -> HallToken {
    if record.revealed {
        HallToken {
            image: record.fanatic.image(),
            title: record.fanatic.label(),
        }
    } else {
        HallToken {
            image: HIDDEN_FANATIC_IMAGE.to_string(),
            title: "Fanatiques face cachée".to_string(),
        }
    }
}
