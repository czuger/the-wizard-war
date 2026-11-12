//! Plays games between computer-controlled alchemists and prints how each level and each
//! playing style fared, to evaluate changes to the AIs.
//!
//! Usage: `cargo run --release --bin ai_arena -- [games] [turns] [levels]`, for instance
//! `ai_arena 200 8 strong,strong,beginner,beginner` (the defaults). Seats are shuffled for
//! every game; each game runs in its own in-memory database. When `AI_ARENA_LOG` names a
//! file, the detailed AI log of the first game is written there.
//!
//! The report ends with the first level of the list against the others, game by game: its
//! average money minus theirs, and its share of the wins, with their standard errors.

use std::collections::BTreeMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use rand::seq::SliceRandom;
use wizard_war::db::{self, EventRecord, PlayerRecord};
use wizard_war::domain::AiLevel;
use wizard_war::service::{export_ai_log, play_ai_match};

/// One player's result in one game.
struct Outcome {
    level: AiLevel,
    archetype: &'static str,
    money: i64,
    /// Share of the win: 1 for a sole winner, split on ties, 0 otherwise.
    win: f64,
    /// Items sold on the map, and what they brought.
    map_sales: (i64, i64),
    /// Items sold in the city, and what they brought.
    city_sales: (i64, i64),
    /// Items left unsold in the stock at the end.
    stock_left: i64,
    laboratories: i64,
    /// Fanatic verdicts: free items received, productions burned, laboratories ravaged.
    verdicts: (i64, i64, i64),
}

/// Sums of the outcomes of one group of players.
#[derive(Default)]
struct Tally {
    games: u32,
    money: i64,
    wins: f64,
    map_sales: (i64, i64),
    city_sales: (i64, i64),
    stock_left: i64,
    laboratories: i64,
    verdicts: (i64, i64, i64),
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = std::env::args().skip(1);
    let games: usize = args.next().map_or(Ok(200), |value| value.parse())?;
    let turns: i64 = args.next().map_or(Ok(8), |value| value.parse())?;
    let levels = match args.next() {
        Some(list) => list
            .split(',')
            .map(|key| AiLevel::from_key(key.trim()).ok_or(format!("unknown level {key}")))
            .collect::<Result<Vec<_>, _>>()?,
        None => vec![
            AiLevel::Strong,
            AiLevel::Strong,
            AiLevel::Beginner,
            AiLevel::Beginner,
        ],
    };
    let log_path = std::env::var("AI_ARENA_LOG").ok();
    let next_game = Arc::new(AtomicUsize::new(0));
    let workers = std::thread::available_parallelism().map_or(4, usize::from);
    let mut tasks = Vec::new();
    for _ in 0..workers {
        let (next_game, levels, log_path) = (next_game.clone(), levels.clone(), log_path.clone());
        tasks.push(tokio::spawn(async move {
            let mut results = Vec::new();
            loop {
                let index = next_game.fetch_add(1, Ordering::Relaxed);
                if index >= games {
                    return Ok::<_, String>(results);
                }
                let mut seats = levels.clone();
                seats.shuffle(&mut rand::rng());
                let log_path = log_path.clone().filter(|_| index == 0);
                results.push(play_one(turns, seats, log_path).await?);
            }
        }));
    }
    let mut results = Vec::new();
    for task in tasks {
        results.extend(task.await??);
    }
    print_report(&results, levels[0]);
    Ok(())
}

/// Plays one game and returns every player's outcome, writing its AI log to `log_path`.
async fn play_one(
    turns: i64,
    seats: Vec<AiLevel>,
    log_path: Option<String>,
) -> Result<Vec<Outcome>, String> {
    let pool = db::connect("sqlite::memory:")
        .await
        .map_err(|error| error.to_string())?;
    let game_id = play_ai_match(&pool, turns, &seats)
        .await
        .map_err(|error| error.to_string())?;
    if let Some(path) = log_path {
        let log = export_ai_log(&pool, game_id)
            .await
            .map_err(|error| error.to_string())?;
        std::fs::write(path, log).map_err(|error| error.to_string())?;
    }
    let mut conn = pool.acquire().await.map_err(|error| error.to_string())?;
    let players = PlayerRecord::list_for_game(&mut conn, game_id)
        .await
        .map_err(|error| error.to_string())?;
    let events = EventRecord::list_for_game(&mut conn, game_id)
        .await
        .map_err(|error| error.to_string())?;
    let best = players.iter().map(|player| player.money).max().unwrap_or(0);
    let winners = players.iter().filter(|player| player.money == best).count() as f64;
    Ok(players
        .iter()
        .map(|player| Outcome {
            level: player.ai_level.unwrap_or_default(),
            archetype: player.archetype.key(),
            money: player.money,
            win: if player.money == best {
                1.0 / winners
            } else {
                0.0
            },
            map_sales: sales(&events, &player.name, false),
            city_sales: sales(&events, &player.name, true),
            stock_left: player.stock.total(),
            laboratories: player.laboratories,
            verdicts: verdicts(&events, &player.name),
        })
        .collect())
}

/// Counts the sales of `name` in the log, on the map or in the city, and sums their prices.
fn sales(events: &[EventRecord], name: &str, in_city: bool) -> (i64, i64) {
    let prefix = format!("{name} vend ");
    events
        .iter()
        .filter(|event| event.message.starts_with(&prefix))
        .filter(|event| event.message.contains(" en ville pour ") == in_city)
        .filter_map(|event| {
            let price = event.message.rsplit(" pour ").next()?;
            price.trim_end_matches(" écus.").parse::<i64>().ok()
        })
        .fold((0, 0), |(count, total), price| (count + 1, total + price))
}

/// Counts the fanatic verdicts against `name`: helped, burned, ravaged.
fn verdicts(events: &[EventRecord], name: &str) -> (i64, i64, i64) {
    let count = |text: &str| {
        events
            .iter()
            .filter(|event| event.message.contains(text))
            .count() as i64
    };
    (
        count(&format!("aident {name}, qui reçoit")),
        count(&format!("fabriqués par {name}.")),
        count(&format!("saccagent le laboratoire de {name} ")),
    )
}

/// Prints the average money and win share of each level, of each style at each level, then
/// `focus` against the other levels.
fn print_report(results: &[Vec<Outcome>], focus: AiLevel) {
    println!("{} games", results.len());
    let outcomes: Vec<&Outcome> = results.iter().flatten().collect();
    let by_level = tally(&outcomes, |outcome| outcome.level.key().to_string());
    print_tallies("level", &by_level);
    let by_style = tally(&outcomes, |outcome| {
        format!("{} {}", outcome.level.key(), outcome.archetype)
    });
    print_tallies("level and style", &by_style);
    print_head_to_head(results, focus);
}

fn tally(outcomes: &[&Outcome], key: impl Fn(&Outcome) -> String) -> BTreeMap<String, Tally> {
    let mut tallies: BTreeMap<String, Tally> = BTreeMap::new();
    for outcome in outcomes {
        let entry = tallies.entry(key(outcome)).or_default();
        entry.games += 1;
        entry.money += outcome.money;
        entry.wins += outcome.win;
        entry.map_sales.0 += outcome.map_sales.0;
        entry.map_sales.1 += outcome.map_sales.1;
        entry.city_sales.0 += outcome.city_sales.0;
        entry.city_sales.1 += outcome.city_sales.1;
        entry.stock_left += outcome.stock_left;
        entry.laboratories += outcome.laboratories;
        entry.verdicts.0 += outcome.verdicts.0;
        entry.verdicts.1 += outcome.verdicts.1;
        entry.verdicts.2 += outcome.verdicts.2;
    }
    tallies
}

fn print_tallies(title: &str, tallies: &BTreeMap<String, Tally>) {
    println!(
        "\n{title:<28} {:>6} {:>8} {:>7} {:>5} {:>7} {:>5} {:>7} {:>5} {:>5} {:>5} {:>5} {:>5}",
        "seats",
        "money",
        "wins",
        "map",
        "écus",
        "city",
        "écus",
        "left",
        "labs",
        "help",
        "burn",
        "ravag"
    );
    for (name, tally) in tallies {
        let games = f64::from(tally.games.max(1));
        let per_game = |value: i64| value as f64 / games;
        println!(
            "{name:<28} {:>6} {:>8.0} {:>6.1}% {:>5.2} {:>7.0} {:>5.2} {:>7.0} {:>5.2} {:>5.2} {:>5.2} {:>5.2} {:>5.2}",
            tally.games,
            per_game(tally.money),
            100.0 * tally.wins / games,
            per_game(tally.map_sales.0),
            per_game(tally.map_sales.1),
            per_game(tally.city_sales.0),
            per_game(tally.city_sales.1),
            per_game(tally.stock_left),
            per_game(tally.laboratories),
            per_game(tally.verdicts.0),
            per_game(tally.verdicts.1),
            per_game(tally.verdicts.2),
        );
    }
}

/// Prints, game by game, `focus`'s average money minus the others' and its share of the
/// wins per seat, each with its standard error.
fn print_head_to_head(results: &[Vec<Outcome>], focus: AiLevel) {
    let mut gaps = Vec::new();
    let mut wins = Vec::new();
    for game in results {
        let (ours, theirs): (Vec<&Outcome>, Vec<&Outcome>) =
            game.iter().partition(|outcome| outcome.level == focus);
        if ours.is_empty() || theirs.is_empty() {
            continue;
        }
        let mean = |side: &[&Outcome]| {
            side.iter().map(|outcome| outcome.money as f64).sum::<f64>() / side.len() as f64
        };
        gaps.push(mean(&ours) - mean(&theirs));
        let share: f64 = ours.iter().map(|outcome| outcome.win).sum();
        wins.push(100.0 * share * game.len() as f64 / ours.len() as f64);
    }
    let (gap, gap_error) = mean_and_error(&gaps);
    let (win, win_error) = mean_and_error(&wins);
    println!(
        "\n{} against the others: money {gap:+.0} ± {gap_error:.0}, \
         wins {win:.1}% ± {win_error:.1} of a fair share (100%)",
        focus.key()
    );
}

/// The mean of `values` and its standard error.
fn mean_and_error(values: &[f64]) -> (f64, f64) {
    let count = values.len() as f64;
    if count < 2.0 {
        return (values.first().copied().unwrap_or(0.0), 0.0);
    }
    let mean = values.iter().sum::<f64>() / count;
    let variance = values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f64>()
        / (count - 1.0);
    (mean, (variance / count).sqrt())
}
