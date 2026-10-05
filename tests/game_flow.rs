use sqlx::SqlitePool;
use wizard_war::db::{self, FanaticRecord, GameRecord, PlayerRecord};
use wizard_war::domain::{ItemCounts, Phase};
use wizard_war::error::AppError;
use wizard_war::service::{CreatedGame, announce_fanatic, create_game, pay_expense, produce_items};

async fn new_game(max_turns: i64) -> (SqlitePool, CreatedGame) {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    let names = vec!["Ana".to_string(), "Bob".to_string()];
    let game = create_game(&pool, &names, max_turns).await.unwrap();
    (pool, game)
}

async fn game_record(pool: &SqlitePool, id: i64) -> GameRecord {
    GameRecord::find(&mut pool.acquire().await.unwrap(), id)
        .await
        .unwrap()
}

async fn players(pool: &SqlitePool, id: i64) -> Vec<PlayerRecord> {
    PlayerRecord::list_for_game(&mut pool.acquire().await.unwrap(), id)
        .await
        .unwrap()
}

/// Tokens in production order for the current turn.
async fn tokens_in_turn_order(pool: &SqlitePool, created: &CreatedGame) -> Vec<String> {
    let game = game_record(pool, created.game_id).await;
    let mut seats: Vec<PlayerRecord> = players(pool, created.game_id).await;
    seats.sort_by_key(|player| (player.seat - game.first_seat).rem_euclid(2));
    seats.into_iter().map(|player| player.token).collect()
}

async fn play_announcements_and_expenses(pool: &SqlitePool, created: &CreatedGame) {
    for player in &created.players {
        announce_fanatic(pool, created.game_id, &player.token, None)
            .await
            .unwrap();
    }
    for player in &created.players {
        pay_expense(pool, created.game_id, &player.token, 100)
            .await
            .unwrap();
    }
}

#[tokio::test]
async fn setup_follows_the_rules() {
    let (pool, created) = new_game(8).await;
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.turn, 1);
    assert_eq!(game.supply, ItemCounts::new(20, 25, 35));
    for player in players(&pool, created.game_id).await {
        assert_eq!(player.money, 5_000);
        assert_eq!(player.laboratories, 1);
        let mut conn = pool.acquire().await.unwrap();
        let fanatics = FanaticRecord::list_for_player(&mut conn, player.id)
            .await
            .unwrap();
        assert_eq!(fanatics.len(), 6);
    }
}

#[tokio::test]
async fn phases_advance_once_everyone_has_acted() {
    let (pool, created) = new_game(8).await;
    let ana = &created.players[0].token;
    announce_fanatic(&pool, created.game_id, ana, None)
        .await
        .unwrap();
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, ana, None).await,
        Err(AppError::AlreadyDone)
    ));
    assert!(matches!(
        pay_expense(&pool, created.game_id, ana, 100).await,
        Err(AppError::WrongPhase)
    ));
    let bob = &created.players[1].token;
    announce_fanatic(&pool, created.game_id, bob, None)
        .await
        .unwrap();
    assert_eq!(
        game_record(&pool, created.game_id).await.phase,
        Phase::Expenses
    );

    assert!(matches!(
        pay_expense(&pool, created.game_id, ana, 150).await,
        Err(AppError::InvalidExpense)
    ));
    assert!(matches!(
        pay_expense(&pool, created.game_id, ana, 5_100).await,
        Err(AppError::InsufficientFunds)
    ));
    pay_expense(&pool, created.game_id, ana, 300).await.unwrap();
    pay_expense(&pool, created.game_id, bob, 0).await.unwrap();
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Production);
    assert_eq!(game.active_seat, Some(game.first_seat));
    assert_eq!(game.collective_expense, 300);
    assert_eq!(players(&pool, created.game_id).await[0].money, 4_700);
}

#[tokio::test]
async fn production_is_sequential_and_limited_to_laboratories() {
    let (pool, created) = new_game(8).await;
    play_announcements_and_expenses(&pool, &created).await;
    let order = tokens_in_turn_order(&pool, &created).await;

    assert!(matches!(
        produce_items(&pool, created.game_id, &order[1], ItemCounts::new(1, 0, 0)).await,
        Err(AppError::NotYourTurn)
    ));
    assert!(matches!(
        produce_items(&pool, created.game_id, &order[0], ItemCounts::new(1, 1, 0)).await,
        Err(AppError::WrongProductionCount { expected: 1 })
    ));
    produce_items(&pool, created.game_id, &order[0], ItemCounts::new(0, 1, 0))
        .await
        .unwrap();
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.supply, ItemCounts::new(20, 24, 35));
}

#[tokio::test]
async fn fanatics_resolve_and_next_turn_rotates_first_player() {
    let (pool, created) = new_game(8).await;
    let first_seat = game_record(&pool, created.game_id).await.first_seat;
    let ana_id = players(&pool, created.game_id).await[0].id;
    let ana_fanatic = {
        let mut conn = pool.acquire().await.unwrap();
        FanaticRecord::list_for_player(&mut conn, ana_id)
            .await
            .unwrap()[0]
            .clone()
    };
    let dominant_item = ana_fanatic.fanatic.cult;

    announce_fanatic(
        &pool,
        created.game_id,
        &created.players[0].token,
        Some(ana_fanatic.id),
    )
    .await
    .unwrap();
    announce_fanatic(&pool, created.game_id, &created.players[1].token, None)
        .await
        .unwrap();
    for player in &created.players {
        pay_expense(&pool, created.game_id, &player.token, 0)
            .await
            .unwrap();
    }
    let favoured = match dominant_item {
        wizard_war::domain::Cult::Xephis => ItemCounts::new(0, 1, 0),
        wizard_war::domain::Cult::Yaboul => ItemCounts::new(0, 0, 1),
    };
    for token in tokens_in_turn_order(&pool, &created).await {
        produce_items(&pool, created.game_id, &token, favoured)
            .await
            .unwrap();
    }

    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.turn, 2);
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.first_seat, (first_seat + 1) % 2);
    for player in players(&pool, created.game_id).await {
        assert_eq!(player.stock.total(), 2, "one made plus one fanatic bonus");
        assert_eq!(player.production, ItemCounts::default());
        assert!(!player.announcement_done && !player.expense_done);
    }
    let mut conn = pool.acquire().await.unwrap();
    assert!(
        FanaticRecord::list_in_hall(&mut conn, created.game_id)
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn game_finishes_after_the_last_turn() {
    let (pool, created) = new_game(1).await;
    play_announcements_and_expenses(&pool, &created).await;
    for token in tokens_in_turn_order(&pool, &created).await {
        produce_items(&pool, created.game_id, &token, ItemCounts::new(1, 0, 0))
            .await
            .unwrap();
    }
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Finished);
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, &created.players[0].token, None).await,
        Err(AppError::WrongPhase)
    ));
}

#[tokio::test]
async fn unknown_token_is_rejected() {
    let (pool, created) = new_game(8).await;
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, "nope", None).await,
        Err(AppError::PlayerNotFound)
    ));
}
