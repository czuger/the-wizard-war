use crate::domain::market::{CITY_QUOTA_LABORATORY_BANDS, CITY_QUOTA_TABLE, city_quota_row};

/// One row of Table 2, shown to players choosing their collective expense.
#[derive(Debug, Clone)]
pub struct CityQuotaRow {
    /// Fewest laboratories on the board for this row.
    pub first: i64,
    /// Most laboratories on the board for this row.
    pub last: i64,
    /// Items the city buys, for each collective expense column.
    pub quotas: [i64; 6],
    /// Whether the laboratories now on the board fall in this row.
    pub current: bool,
}

impl CityQuotaRow {
    /// Every row of Table 2, flagging the one matching `laboratories` on the board.
    pub fn table(laboratories: i64) -> Vec<CityQuotaRow> {
        let current = city_quota_row(laboratories);
        CITY_QUOTA_LABORATORY_BANDS
            .iter()
            .zip(CITY_QUOTA_TABLE)
            .enumerate()
            .map(|(index, (&(first, last), quotas))| CityQuotaRow {
                first,
                last,
                quotas,
                current: index == current,
            })
            .collect()
    }
}
