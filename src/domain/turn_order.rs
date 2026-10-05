/// Clockwise seat order for one turn, starting from the turn's first player.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TurnOrder {
    player_count: i64,
    first_seat: i64,
}

impl TurnOrder {
    /// Builds the order of a turn. Seats are numbered from 0 to `player_count - 1`.
    pub fn new(player_count: i64, first_seat: i64) -> TurnOrder {
        TurnOrder {
            player_count,
            first_seat,
        }
    }

    /// Returns the first seat of this turn.
    pub fn first_seat(&self) -> i64 {
        self.first_seat
    }

    /// Returns the seat playing after `seat`, or `None` once every player has played.
    pub fn next_after(&self, seat: i64) -> Option<i64> {
        let next = self.left_of(seat);
        (next != self.first_seat).then_some(next)
    }

    /// Returns the order of the next turn: its first player sits to the left of this turn's.
    pub fn next_turn(&self) -> TurnOrder {
        TurnOrder::new(self.player_count, self.left_of(self.first_seat))
    }

    fn left_of(&self, seat: i64) -> i64 {
        (seat + 1) % self.player_count
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn walks_every_seat_once_from_first() {
        let order = TurnOrder::new(3, 1);
        assert_eq!(order.next_after(1), Some(2));
        assert_eq!(order.next_after(2), Some(0));
        assert_eq!(order.next_after(0), None);
    }

    #[test]
    fn next_turn_rotates_left() {
        assert_eq!(TurnOrder::new(3, 2).next_turn().first_seat(), 0);
        assert_eq!(TurnOrder::new(4, 0).next_turn().first_seat(), 1);
    }
}
