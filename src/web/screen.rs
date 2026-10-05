use crate::domain::Phase;
use crate::service::GameView;

/// The screen a player's page shows: one per thing the player has to do, plus waiting and the
/// end of the game.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Screen {
    /// Phase I: send a fanatic to the great hall, or none.
    Announcement,
    /// Phase I: the secret collective expense.
    Expense,
    /// Phase II.1: convert a site, on the map.
    Conversion,
    /// Phase II.2: move every rebels pawn, on the map.
    RebelMovement,
    /// Phase II.3: choose the pawn the rebels attack, on the map.
    RebelFight,
    /// Phase III: take items from the supply.
    Production,
    /// Phase V.1: move peddlers and caravans, on the map.
    PawnMoves,
    /// Phase V.2: sell where the pawns stand, on the map.
    Sales,
    /// Phase VI: sell one item in the city.
    CitySale,
    /// Phase VII: buy from the bank.
    Purchase,
    /// Another alchemist is playing.
    Waiting,
    /// The game is over.
    Finished,
}

impl Screen {
    /// The screen matching what the viewing player has to do now.
    pub fn of(view: &GameView) -> Screen {
        if view.can_announce() {
            Screen::Announcement
        } else if view.can_pay() {
            Screen::Expense
        } else if view.can_convert() {
            Screen::Conversion
        } else if view.can_move_rebels() {
            Screen::RebelMovement
        } else if view.can_attack() {
            Screen::RebelFight
        } else if view.can_produce() {
            Screen::Production
        } else if view.can_move_pawns() {
            Screen::PawnMoves
        } else if view.can_sell_items() {
            Screen::Sales
        } else if view.can_sell_in_city() {
            Screen::CitySale
        } else if view.can_buy() {
            Screen::Purchase
        } else if view.is_finished() {
            Screen::Finished
        } else {
            Screen::Waiting
        }
    }

    /// Whether the screen draws the map: only the phases played on it do.
    ///
    /// # Arguments
    /// * `phase` - the game's phase, which decides for the waiting screen.
    pub fn shows_map(self, phase: Phase) -> bool {
        match self {
            Screen::Conversion
            | Screen::RebelMovement
            | Screen::RebelFight
            | Screen::PawnMoves
            | Screen::Sales => true,
            Screen::Waiting => matches!(
                phase,
                Phase::Conversions | Phase::RebelMovement | Phase::RebelFights | Phase::Peddlers
            ),
            Screen::Announcement
            | Screen::Expense
            | Screen::Production
            | Screen::CitySale
            | Screen::Purchase
            | Screen::Finished => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_phases_draw_the_map() {
        assert!(Screen::Conversion.shows_map(Phase::Conversions));
        assert!(Screen::PawnMoves.shows_map(Phase::Peddlers));
        assert!(Screen::Waiting.shows_map(Phase::RebelMovement));
    }

    #[test]
    fn other_phases_hide_the_map() {
        assert!(!Screen::Announcement.shows_map(Phase::Announcements));
        assert!(!Screen::Purchase.shows_map(Phase::Purchases));
        assert!(!Screen::Waiting.shows_map(Phase::Production));
        assert!(!Screen::Finished.shows_map(Phase::Finished));
    }
}
