//! Prints the detailed AI log of one game, to evaluate the computer-controlled alchemists.
//!
//! Usage: `cargo run --bin ai_log -- <game id> > game.log` (reads `DATABASE_URL`).

use wizard_war::config::AppConfig;
use wizard_war::db;
use wizard_war::service::export_ai_log;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let game_id: i64 = std::env::args()
        .nth(1)
        .ok_or("usage: ai_log <game id>")?
        .parse()?;
    let pool = db::connect(&AppConfig::from_env().database_url).await?;
    print!("{}", export_ai_log(&pool, game_id).await?);
    Ok(())
}
