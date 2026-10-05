use thiserror::Error;

use crate::domain::PawnKind;
use crate::domain::rules::LABORATORY_PRICE;

/// What a player buys from and sells back to the bank in phase VII.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Purchase {
    /// Peddlers bought.
    pub peddlers: i64,
    /// Caravans bought.
    pub caravans: i64,
    /// Laboratories bought.
    pub laboratories: i64,
    /// Peddlers in the city sold back at half price.
    pub sold_peddlers: i64,
    /// Caravans in the city sold back at half price.
    pub sold_caravans: i64,
}

/// What a player owns, as far as phase VII is concerned.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Holdings {
    /// Écus owned.
    pub money: i64,
    /// Peddlers owned, wherever they are.
    pub peddlers: i64,
    /// Caravans owned, wherever they are.
    pub caravans: i64,
    /// Peddlers inside the city, the only ones the bank buys back.
    pub peddlers_in_city: i64,
    /// Caravans inside the city.
    pub caravans_in_city: i64,
    /// Laboratories still in the box.
    pub laboratories_left: i64,
    /// Whether the bank still buys pawns back (closed during the last turns).
    pub bank_open: bool,
}

/// Why a purchase is refused. The texts are shown to players.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
pub enum PurchaseError {
    /// A count is negative.
    #[error("Les quantités ne peuvent pas être négatives.")]
    Negative,
    /// The player would own more pawns of a kind than allowed.
    #[error("Vous ne pouvez pas posséder plus de 4 colporteurs et 3 caravanes.")]
    TooManyPawns,
    /// Only pawns inside the city can be sold back.
    #[error("Seuls les colporteurs et caravanes rentrés en ville peuvent être revendus.")]
    NotInCity,
    /// The bank is closed for buy-backs.
    #[error("La banque ne rachète plus de pions pendant les deux derniers tours.")]
    BankClosed,
    /// Every laboratory is already on the board.
    #[error("Il ne reste plus de laboratoire à acheter.")]
    NoLaboratoryLeft,
    /// The player cannot pay.
    #[error("Vous n'avez pas assez d'écus.")]
    InsufficientFunds,
}

impl Purchase {
    /// Whether the player buys and sells nothing.
    pub fn is_empty(&self) -> bool {
        *self == Purchase::default()
    }

    /// Écus paid to the bank.
    pub fn cost(&self) -> i64 {
        self.peddlers * PawnKind::Peddler.price()
            + self.caravans * PawnKind::Caravan.price()
            + self.laboratories * LABORATORY_PRICE
    }

    /// Écus the bank pays back for the pawns sold.
    pub fn refund(&self) -> i64 {
        self.sold_peddlers * PawnKind::Peddler.resale_price()
            + self.sold_caravans * PawnKind::Caravan.resale_price()
    }

    /// Checks the purchase against the rules of phase VII; pawns are sold before buying.
    ///
    /// # Errors
    /// The [`PurchaseError`] naming the broken rule.
    pub fn check(&self, holdings: &Holdings) -> Result<(), PurchaseError> {
        let counts = [
            self.peddlers,
            self.caravans,
            self.laboratories,
            self.sold_peddlers,
            self.sold_caravans,
        ];
        if counts.iter().any(|&count| count < 0) {
            return Err(PurchaseError::Negative);
        }
        let sells = self.sold_peddlers + self.sold_caravans > 0;
        if sells && !holdings.bank_open {
            return Err(PurchaseError::BankClosed);
        }
        if self.sold_peddlers > holdings.peddlers_in_city
            || self.sold_caravans > holdings.caravans_in_city
        {
            return Err(PurchaseError::NotInCity);
        }
        let peddlers = holdings.peddlers - self.sold_peddlers + self.peddlers;
        let caravans = holdings.caravans - self.sold_caravans + self.caravans;
        if peddlers > PawnKind::Peddler.max_owned() as i64
            || caravans > PawnKind::Caravan.max_owned() as i64
        {
            return Err(PurchaseError::TooManyPawns);
        }
        if self.laboratories > holdings.laboratories_left {
            return Err(PurchaseError::NoLaboratoryLeft);
        }
        if holdings.money + self.refund() < self.cost() {
            return Err(PurchaseError::InsufficientFunds);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn holdings() -> Holdings {
        Holdings {
            money: 10_000,
            peddlers: 3,
            caravans: 1,
            peddlers_in_city: 1,
            caravans_in_city: 0,
            laboratories_left: 15,
            bank_open: true,
        }
    }

    #[test]
    fn checks_limits_and_money() {
        let buy = |peddlers, caravans, laboratories| Purchase {
            peddlers,
            caravans,
            laboratories,
            ..Purchase::default()
        };
        assert_eq!(buy(1, 1, 0).check(&holdings()), Ok(()));
        assert_eq!(
            buy(2, 0, 0).check(&holdings()),
            Err(PurchaseError::TooManyPawns)
        );
        assert_eq!(
            buy(0, 0, 1).check(&holdings()),
            Err(PurchaseError::InsufficientFunds)
        );
        assert_eq!(
            buy(-1, 0, 0).check(&holdings()),
            Err(PurchaseError::Negative)
        );
        let no_lab = Holdings {
            laboratories_left: 0,
            money: 50_000,
            ..holdings()
        };
        assert_eq!(
            buy(0, 0, 1).check(&no_lab),
            Err(PurchaseError::NoLaboratoryLeft)
        );
    }

    #[test]
    fn sells_back_pawns_in_the_city_while_the_bank_is_open() {
        let sale = Purchase {
            sold_peddlers: 1,
            laboratories: 1,
            ..Purchase::default()
        };
        let rich = Holdings {
            money: 9_500,
            ..holdings()
        };
        assert_eq!(sale.check(&rich), Ok(()), "1,500 back, 11,000 paid");
        assert_eq!(sale.refund(), 1_500);
        let closed = Holdings {
            bank_open: false,
            ..rich
        };
        assert_eq!(sale.check(&closed), Err(PurchaseError::BankClosed));
        let caravan = Purchase {
            sold_caravans: 1,
            ..Purchase::default()
        };
        assert_eq!(caravan.check(&holdings()), Err(PurchaseError::NotInCity));
    }
}
