use sqlx::SqlitePool;
use wizard_war::db::{self, FanaticRecord, GameRecord, PlayerRecord};
use wizard_war::domain::rules::{AI_NAMES, HUMAN_SEAT, PLAYER_COUNT};
use wizard_war::domain::{Conversion, ConversionError, Cult, ItemCounts, Phase, SiteId};
use wizard_war::error::AppError;
use wizard_war::service::{
    CreatedGame, GameView, RebelMove, announce_fanatic, convert_site, create_game, move_rebels,
    pay_expense, produce_items, required_items,
};

async fn new_game(max_turns: i64) -> (SqlitePool, CreatedGame) {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    let game = create_game(&pool, "Ana", max_turns).await.unwrap();
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

/// Takes the first items in the supply, as many as the human must.
async fn produce_first_items(pool: &SqlitePool, created: &CreatedGame) {
    let game = game_record(pool, created.game_id).await;
    let mut left = required_items(1, &game.supply);
    let mut items = ItemCounts::default();
    for item in wizard_war::domain::MagicItem::ALL {
        let taken = left.min(game.supply.get(item));
        items = items.with(item, taken);
        left -= taken;
    }
    produce_items(pool, created.game_id, &created.human_token, items)
        .await
        .unwrap();
}

/// Makes the first conversion offered to the human, or passes when none is.
async fn convert_first_option(pool: &SqlitePool, created: &CreatedGame) {
    let view = GameView::load(pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let conversion = view
        .board
        .conversion_groups
        .first()
        .and_then(|group| group.options.first())
        .and_then(|option| Conversion::from_form_value(&option.value));
    convert_site(pool, created.game_id, &created.human_token, conversion)
        .await
        .unwrap();
}

/// Moves every rebels pawn to the first village offered.
async fn move_rebels_first_option(pool: &SqlitePool, created: &CreatedGame) {
    let view = GameView::load(pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let moves = view
        .board
        .rebel_choices
        .iter()
        .map(|choice| RebelMove {
            rebel_id: choice.rebel_id,
            to: SiteId(choice.destinations[0].site),
        })
        .collect();
    move_rebels(pool, created.game_id, &created.human_token, moves)
        .await
        .unwrap();
}

/// Plays one whole turn for the human: no fanatic, no expense, the first conversion and
/// rebel moves offered, the first items in the supply.
async fn play_human_turn(pool: &SqlitePool, created: &CreatedGame) {
    let (id, token) = (created.game_id, &created.human_token);
    let turn = game_record(pool, id).await.turn;
    announce_fanatic(pool, id, token, None).await.unwrap();
    pay_expense(pool, id, token, 0).await.unwrap();
    loop {
        let game = game_record(pool, id).await;
        if game.turn != turn || game.phase == Phase::Finished {
            return;
        }
        assert_eq!(
            game.active_seat,
            Some(HUMAN_SEAT),
            "AIs never block the game"
        );
        match game.phase {
            Phase::Conversions => convert_first_option(pool, created).await,
            Phase::RebelMovement => move_rebels_first_option(pool, created).await,
            Phase::Production => produce_first_items(pool, created).await,
            phase => panic!("unexpected phase {phase:?}"),
        }
    }
}

/// Plays the human's turn up to the end of their conversion.
async fn play_until_converted(pool: &SqlitePool, created: &CreatedGame) {
    let (id, token) = (created.game_id, &created.human_token);
    announce_fanatic(pool, id, token, None).await.unwrap();
    pay_expense(pool, id, token, 0).await.unwrap();
    convert_first_option(pool, created).await;
}

#[tokio::test]
async fn setup_seats_the_human_first_against_five_ais() {
    let (pool, created) = new_game(8).await;
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.turn, 1);
    assert_eq!(game.supply, ItemCounts::new(20, 25, 35));
    let players = players(&pool, created.game_id).await;
    assert_eq!(players.len(), PLAYER_COUNT);
    for player in &players {
        let human = player.seat == HUMAN_SEAT;
        assert_eq!(player.is_ai, !human);
        assert_eq!(player.token == created.human_token, human);
        assert_eq!(player.announcement_done, !human, "AIs announce at once");
        let mut conn = pool.acquire().await.unwrap();
        let fanatics = FanaticRecord::list_for_player(&mut conn, player.id)
            .await
            .unwrap();
        assert_eq!(fanatics.len(), 4);
    }
    let names: Vec<&str> = players[1..].iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, AI_NAMES);
}

#[tokio::test]
async fn ais_act_as_soon_as_the_human_has_acted() {
    let (pool, created) = new_game(8).await;
    let (id, token) = (created.game_id, &created.human_token);
    announce_fanatic(&pool, id, token, None).await.unwrap();
    assert!(matches!(
        announce_fanatic(&pool, id, token, None).await,
        Err(AppError::WrongPhase)
    ));
    assert_eq!(game_record(&pool, id).await.phase, Phase::Expenses);
    for player in players(&pool, id).await {
        assert_eq!(player.expense_done, player.is_ai);
    }

    assert!(matches!(
        pay_expense(&pool, id, token, 150).await,
        Err(AppError::InvalidExpense)
    ));
    pay_expense(&pool, id, token, 300).await.unwrap();
    let game = game_record(&pool, id).await;
    assert_eq!(game.phase, Phase::Conversions);
    assert_eq!(
        game.active_seat,
        Some(HUMAN_SEAT),
        "AIs before the human have converted"
    );
    assert_eq!(players(&pool, id).await[0].money, 4_700);
}

#[tokio::test]
async fn human_production_is_checked_then_the_turn_completes() {
    let (pool, created) = new_game(8).await;
    let (id, token) = (created.game_id, &created.human_token);
    let first_seat = game_record(&pool, id).await.first_seat;
    play_until_converted(&pool, &created).await;
    if game_record(&pool, id).await.phase == Phase::RebelMovement {
        move_rebels_first_option(&pool, &created).await;
    }
    assert!(matches!(
        produce_items(&pool, id, token, ItemCounts::new(1, 1, 0)).await,
        Err(AppError::WrongProductionCount { expected: 1 })
    ));
    produce_items(&pool, id, token, ItemCounts::new(1, 0, 0))
        .await
        .unwrap();

    let game = game_record(&pool, id).await;
    assert_eq!(game.turn, 2);
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.first_seat, (first_seat + 1) % PLAYER_COUNT as i64);
    for player in players(&pool, id).await {
        assert!(
            player.stock.total() >= 1 || player.is_ai,
            "the talisman is kept"
        );
        assert_eq!(player.production, ItemCounts::default());
        assert_eq!(player.announcement_done, player.is_ai);
    }
}

#[tokio::test]
async fn a_long_game_runs_to_the_end_with_an_empty_supply() {
    let (pool, created) = new_game(20).await;
    for _ in 0..20 {
        play_human_turn(&pool, &created).await;
    }
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Finished);
    assert_eq!(
        game.supply.total(),
        0,
        "120 items wanted, only 80 in the box"
    );
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, &created.human_token, None).await,
        Err(AppError::WrongPhase)
    ));
}

#[tokio::test]
async fn invalid_names_and_tokens_are_rejected() {
    let (pool, created) = new_game(8).await;
    assert!(matches!(
        create_game(&pool, AI_NAMES[2], 8).await,
        Err(AppError::InvalidPlayerName)
    ));
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, "nope", None).await,
        Err(AppError::PlayerNotFound)
    ));
}

#[tokio::test]
async fn first_turn_conversions_follow_the_rules() {
    let (pool, created) = new_game(8).await;
    let (id, token) = (created.game_id, &created.human_token);
    announce_fanatic(&pool, id, token, None).await.unwrap();
    pay_expense(&pool, id, token, 0).await.unwrap();
    let inland = Conversion::Place {
        site: SiteId(11),
        cult: Cult::Xephis,
    };
    assert!(matches!(
        convert_site(&pool, id, token, Some(inland)).await,
        Err(AppError::Conversion(ConversionError::NotOnEdge))
    ));
    assert!(matches!(
        convert_site(&pool, id, token, None).await,
        Err(AppError::ConversionRequired)
    ));
    convert_first_option(&pool, &created).await;

    let game = game_record(&pool, id).await;
    assert!(matches!(
        game.phase,
        Phase::RebelMovement | Phase::Production
    ));
    let view = GameView::load(&pool, id, token).await.unwrap();
    let marked = view
        .board
        .sites
        .iter()
        .filter(|site| site.marker.is_some())
        .count();
    assert!(marked >= 1, "every player converted something");
    assert!(
        view.board.conversion_groups.is_empty(),
        "the human has converted"
    );
}

#[tokio::test]
async fn ai_conversions_never_block_a_long_game() {
    let (pool, created) = new_game(12).await;
    for _ in 0..12 {
        play_human_turn(&pool, &created).await;
    }
    assert_eq!(
        game_record(&pool, created.game_id).await.phase,
        Phase::Finished
    );
    let view = GameView::load(&pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let marked = view
        .board
        .sites
        .iter()
        .filter(|site| site.marker.is_some())
        .count();
    assert!(marked >= 6, "markers spread over the map ({marked})");
    let rebels: usize = view.board.sites.iter().map(|site| site.rebels).sum();
    assert!(rebels <= 7);
}

#[tokio::test]
async fn rebel_moves_are_checked() {
    let (pool, created) = new_game(30).await;
    let (id, token) = (created.game_id, &created.human_token);
    // Play until the human has to move rebels, which needs a replaced marker and the human
    // as first player.
    for _ in 0..200 {
        let game = game_record(&pool, id).await;
        if game.phase == Phase::RebelMovement && game.active_seat == Some(HUMAN_SEAT) {
            let view = GameView::load(&pool, id, token).await.unwrap();
            assert!(!view.board.rebel_choices.is_empty());
            assert!(matches!(
                move_rebels(&pool, id, token, Vec::new()).await,
                Err(AppError::InvalidRebelMove)
            ));
            let choice = &view.board.rebel_choices[0];
            let far = RebelMove {
                rebel_id: choice.rebel_id,
                to: SiteId(99),
            };
            assert!(matches!(
                move_rebels(&pool, id, token, vec![far]).await,
                Err(AppError::InvalidRebelMove)
            ));
            move_rebels_first_option(&pool, &created).await;
            assert_ne!(game_record(&pool, id).await.phase, Phase::RebelMovement);
            return;
        }
        match game.phase {
            Phase::Announcements => announce_fanatic(&pool, id, token, None).await.unwrap(),
            Phase::Expenses => pay_expense(&pool, id, token, 0).await.unwrap(),
            Phase::Conversions => convert_first_option(&pool, &created).await,
            Phase::Production => produce_first_items(&pool, &created).await,
            phase => panic!("unexpected phase {phase:?}"),
        }
    }
    panic!("the human never had to move rebels");
}
