use askama::Template;

use crate::domain::{ItemCounts, MagicItem, Zone, market};
use crate::service::{ConversionOption, FightView, GameView, MoveChoice, RebelChoice, SaleChoice};
use crate::web::screen::Screen;

/// One kind of magic item with a count, as shown on the page.
pub struct ItemLine {
    /// Identifier used in form fields.
    pub key: &'static str,
    /// Path of the item's token image.
    pub image: &'static str,
    /// French name.
    pub label: &'static str,
    /// Number of items.
    pub count: i64,
}

/// One row of the production table: an item kind with its supply, stock and buying prices.
pub struct ProductionLine {
    /// The item and how many are left in the supply.
    pub supply: ItemLine,
    /// How many the viewing player has in stock.
    pub stock: i64,
    /// Table 1 price paid by a buying site of each zone, in [`Zone::ALL`] order, `—` where
    /// no site of the zone buys it.
    pub site_prices: Vec<String>,
    /// Table 1 price paid by the city in phase VI.
    pub city_price: i64,
}

/// The page a player plays from.
#[derive(Template)]
#[template(path = "game.html")]
pub struct GamePage {
    /// The game as the player sees it.
    pub view: GameView,
    /// The screen drawn for the current phase.
    pub screen: Screen,
    /// Error from the player's last action, if it failed.
    pub error: Option<String>,
}

impl GamePage {
    /// Builds the page, with an optional error message from a rejected action.
    pub fn new(view: GameView, error: Option<String>) -> GamePage {
        let screen = Screen::of(&view);
        GamePage {
            view,
            screen,
            error,
        }
    }

    fn lines(counts: &ItemCounts) -> Vec<ItemLine> {
        MagicItem::ALL
            .iter()
            .map(|&item| ItemLine {
                key: item.key(),
                image: item.image(),
                label: item.label(),
                count: counts.get(item),
            })
            .collect()
    }

    /// The viewing player's stock, by kind.
    pub fn stock_lines(&self) -> Vec<ItemLine> {
        GamePage::lines(&self.view.me.stock)
    }

    /// Items left in the supply, by kind.
    pub fn supply_lines(&self) -> Vec<ItemLine> {
        GamePage::lines(&self.view.supply)
    }

    /// Rows of the production table, one per item kind.
    pub fn production_lines(&self) -> Vec<ProductionLine> {
        MagicItem::ALL
            .iter()
            .zip(self.supply_lines())
            .map(|(&item, supply)| ProductionLine {
                supply,
                stock: self.view.me.stock.get(item),
                site_prices: Zone::ALL
                    .iter()
                    .map(|&zone| {
                        market::zone_price(item, zone)
                            .map_or_else(|| "—".to_string(), |price| price.to_string())
                    })
                    .collect(),
                city_price: market::city_price(item),
            })
            .collect()
    }

    /// French name of the viewing player's district.
    pub fn my_district(&self) -> &'static str {
        self.view.me.district.label()
    }

    /// Path of the player's page, which actions are posted under.
    pub fn player_path(&self) -> String {
        format!(
            "/games/{}/players/{}",
            self.view.game_id, self.view.me.token
        )
    }

    /// Whether the screen draws the map.
    pub fn shows_map(&self) -> bool {
        self.screen.shows_map(self.view.phase)
    }

    /// The conversions offered on a site of the map.
    pub fn conversions_at(&self, site: &u8) -> Vec<&ConversionOption> {
        self.view
            .board
            .conversions
            .iter()
            .filter(|option| option.site == *site)
            .collect()
    }

    /// The rebels pawns to move from a site of the map.
    pub fn rebel_movers_at(&self, site: &u8) -> Vec<&RebelChoice> {
        self.view
            .board
            .rebel_choices
            .iter()
            .filter(|choice| choice.from_site == *site)
            .collect()
    }

    /// The fight to settle on a site of the map, if it is there.
    pub fn fight_at(&self, site: &u8) -> Option<&FightView> {
        self.view
            .trade
            .fight
            .as_ref()
            .filter(|fight| fight.site_id == *site)
    }

    /// How the viewing player may move one of their pawns, if they may.
    pub fn move_for(&self, pawn_id: &i64) -> Option<&MoveChoice> {
        self.view
            .trade
            .move_choices
            .iter()
            .find(|choice| choice.pawn_id == *pawn_id)
    }

    /// What one of the viewing player's pawns may sell where it stands, if anything.
    pub fn sale_for(&self, pawn_id: &i64) -> Option<&SaleChoice> {
        self.view
            .trade
            .sale_choices
            .iter()
            .find(|choice| choice.pawn_id == *pawn_id)
    }
}
