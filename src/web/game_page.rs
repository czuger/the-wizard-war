use askama::Template;

use crate::domain::{ItemCounts, MagicItem};
use crate::service::GameView;

/// One kind of magic item with a count, as shown on the page.
pub struct ItemLine {
    /// Identifier used in form fields and image names.
    pub key: &'static str,
    /// French name.
    pub label: &'static str,
    /// Number of items.
    pub count: i64,
}

/// The page a player plays from.
#[derive(Template)]
#[template(path = "game.html")]
pub struct GamePage {
    /// The game as the player sees it.
    pub view: GameView,
    /// Error from the player's last action, if it failed.
    pub error: Option<String>,
}

impl GamePage {
    /// Builds the page, with an optional error message from a rejected action.
    pub fn new(view: GameView, error: Option<String>) -> GamePage {
        GamePage { view, error }
    }

    fn lines(counts: &ItemCounts) -> Vec<ItemLine> {
        MagicItem::ALL
            .iter()
            .map(|&item| ItemLine {
                key: item.key(),
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

    /// Path of the player's page, which actions are posted under.
    pub fn player_path(&self) -> String {
        format!(
            "/games/{}/players/{}",
            self.view.game_id, self.view.me.token
        )
    }
}
