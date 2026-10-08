use askama::Template;
use sqlx::SqlitePool;
use wizard_war::db::{
    self, FanaticRecord, GameRecord, PawnRecord, PlayerRecord, SpareFanaticRecord,
};
use wizard_war::domain::rules::{AI_NAMES, HUMAN_SEAT, MAX_PLAYER_COUNT};
use wizard_war::domain::{
    AiLevel, Archetype, Conversion, ConversionError, Cult, District, ItemCounts, Location,
    LogSection, MagicItem, PawnKind, PawnOrder, Phase, Purchase, PurchaseError, SiteId,
};
use wizard_war::error::AppError;
use wizard_war::service::{
    CreatedGame, GameSettings, GameView, RebelMove, announce_fanatic, attack_pawn, buy_from_bank,
    convert_site, create_game, move_pawns, move_rebels, pay_expense, produce_items, required_items,
    sell_in_city, sell_items, transfer_item,
};

async fn new_game(max_turns: i64) -> (SqlitePool, CreatedGame) {
    new_game_against(max_turns, AiLevel::Beginner).await
}

async fn new_game_against(max_turns: i64, ai_level: AiLevel) -> (SqlitePool, CreatedGame) {
    new_game_with(max_turns, ai_level, MAX_PLAYER_COUNT).await
}

async fn new_game_with(
    max_turns: i64,
    ai_level: AiLevel,
    player_count: usize,
) -> (SqlitePool, CreatedGame) {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    let settings = GameSettings {
        human_name: "Ana",
        max_turns,
        district: District::Green,
        ai_level,
        player_count,
    };
    let game = create_game(&pool, &settings).await.unwrap();
    (pool, game)
}

async fn pawns(pool: &SqlitePool, id: i64) -> Vec<PawnRecord> {
    PawnRecord::list_for_game(&mut pool.acquire().await.unwrap(), id)
        .await
        .unwrap()
}

/// Settles the pending rebel fight against the first pawn offered.
async fn attack_first_option(pool: &SqlitePool, created: &CreatedGame) {
    let view = GameView::load(pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let target = view.trade.fight.expect("a fight is pending").targets[0].pawn_id;
    attack_pawn(pool, created.game_id, &created.human_token, target)
        .await
        .unwrap();
}

/// Sends every movable pawn to its first destination, loading the first stock item, then
/// sells everything that can be sold.
async fn move_and_sell_first_options(pool: &SqlitePool, created: &CreatedGame) {
    let (id, token) = (created.game_id, &created.human_token);
    let view = GameView::load(pool, id, token).await.unwrap();
    if view.can_move_pawns() {
        let mut stock = view.me.stock;
        let mut orders = Vec::new();
        for choice in &view.trade.move_choices {
            let item = MagicItem::ALL.into_iter().find(|&item| stock.get(item) > 0);
            if choice.in_city && item.is_none() {
                continue;
            }
            let item = if choice.in_city { item } else { None };
            if let Some(item) = item {
                stock = stock.with(item, stock.get(item) - 1);
            }
            orders.push(PawnOrder {
                pawn_id: choice.pawn_id,
                to: Location::from_form_value(&choice.destinations[0].value).unwrap(),
                item,
            });
        }
        move_pawns(pool, id, token, orders).await.unwrap();
    } else {
        let sellers = view
            .trade
            .sale_choices
            .iter()
            .filter(|choice| choice.checked)
            .map(|choice| choice.pawn_id)
            .collect();
        sell_items(pool, id, token, sellers).await.unwrap();
    }
}

/// Sells the first stock item in the city.
async fn sell_first_in_city(pool: &SqlitePool, created: &CreatedGame) {
    let me = &players(pool, created.game_id).await[HUMAN_SEAT as usize];
    let item = MagicItem::ALL
        .into_iter()
        .find(|&item| me.stock.get(item) > 0);
    sell_in_city(pool, created.game_id, &created.human_token, item)
        .await
        .unwrap();
}

/// Buys one peddler when affordable and allowed, otherwise nothing.
async fn buy_a_peddler(pool: &SqlitePool, created: &CreatedGame) {
    let (id, token) = (created.game_id, &created.human_token);
    let one = Purchase {
        peddlers: 1,
        ..Purchase::default()
    };
    if buy_from_bank(pool, id, token, one).await.is_err() {
        buy_from_bank(pool, id, token, Purchase::default())
            .await
            .unwrap();
    }
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
        .conversions
        .first()
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

/// Plays one whole turn for the human: no fanatic, no expense, the first conversion, rebel
/// moves, target and pawn moves offered, the first items in the supply, every sale possible
/// and a peddler whenever possible.
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
            Phase::RebelFights => attack_first_option(pool, created).await,
            Phase::Production => produce_first_items(pool, created).await,
            Phase::Peddlers => move_and_sell_first_options(pool, created).await,
            Phase::CitySales => sell_first_in_city(pool, created).await,
            Phase::Purchases => buy_a_peddler(pool, created).await,
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
async fn the_ai_level_is_stored_with_the_game() {
    let (pool, created) = new_game_against(8, AiLevel::Strong).await;
    assert_eq!(
        game_record(&pool, created.game_id).await.ai_level,
        AiLevel::Strong
    );
    let view = GameView::load(&pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    assert_eq!(view.ai_level, AiLevel::Strong);
}

#[tokio::test]
async fn every_ai_gets_its_own_hidden_temperament() {
    let (pool, created) = new_game_against(8, AiLevel::Strong).await;
    let players = players(&pool, created.game_id).await;
    assert_eq!(players[HUMAN_SEAT as usize].archetype, Archetype::Balanced);
    let mut drawn: Vec<&str> = players
        .iter()
        .filter(|player| player.is_ai)
        .map(|player| player.archetype.key())
        .collect();
    drawn.sort_unstable();
    let mut expected = Archetype::DRAWN.map(Archetype::key);
    expected.sort_unstable();
    assert_eq!(drawn, expected);
}

#[tokio::test]
async fn players_stored_before_temperaments_are_balanced() {
    let (pool, created) = new_game(8).await;
    let mut conn = pool.acquire().await.unwrap();
    sqlx::query(
        "INSERT INTO players (game_id, seat, name, token, money, laboratories, district)
         VALUES (?, 9, 'Ancien', 'old-token', 0, 0, 'blue')",
    )
    .bind(created.game_id)
    .execute(&mut *conn)
    .await
    .unwrap();
    let old = PlayerRecord::list_for_game(&mut conn, created.game_id)
        .await
        .unwrap()
        .into_iter()
        .find(|player| player.seat == 9)
        .unwrap();
    assert_eq!(old.archetype, Archetype::Balanced);
    assert_eq!(old.temperament().home_cult, Cult::Xephis);
}

#[tokio::test]
async fn setup_seats_the_human_first_against_five_ais() {
    let (pool, created) = new_game(8).await;
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.turn, 1);
    assert_eq!(game.supply, ItemCounts::new(20, 25, 35));
    let players = players(&pool, created.game_id).await;
    assert_eq!(players.len(), MAX_PLAYER_COUNT);
    assert_eq!(game.player_count, MAX_PLAYER_COUNT as i64);
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
    let mut conn = pool.acquire().await.unwrap();
    let spare = SpareFanaticRecord::list_for_game(&mut conn, created.game_id)
        .await
        .unwrap();
    assert!(spare.is_empty());
}

#[tokio::test]
async fn a_two_player_game_deals_six_fanatics_each_and_leaves_twelve_in_the_box() {
    let (pool, created) = new_game_with(8, AiLevel::Strong, 2).await;
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.player_count, 2);
    assert!((0..2).contains(&game.first_seat));
    let players = players(&pool, created.game_id).await;
    let names: Vec<&str> = players.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["Ana", AI_NAMES[0]]);
    assert_ne!(players[0].district, players[1].district);
    let mut conn = pool.acquire().await.unwrap();
    for player in &players {
        let fanatics = FanaticRecord::list_for_player(&mut conn, player.id)
            .await
            .unwrap();
        assert_eq!(fanatics.len(), 6);
    }
    let spare = SpareFanaticRecord::list_for_game(&mut conn, created.game_id)
        .await
        .unwrap();
    assert_eq!(spare.len(), 12);
}

#[tokio::test]
async fn games_of_two_to_five_players_run_to_the_end() {
    for player_count in 2..MAX_PLAYER_COUNT {
        for ai_level in AiLevel::ALL {
            let (pool, created) = new_game_with(6, ai_level, player_count).await;
            for _ in 0..6 {
                play_human_turn(&pool, &created).await;
            }
            let game = game_record(&pool, created.game_id).await;
            assert_eq!(game.phase, Phase::Finished, "{player_count} players");
            assert!((0..player_count as i64).contains(&game.first_seat));
        }
    }
}

#[tokio::test]
async fn player_counts_outside_two_to_six_are_rejected() {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    for player_count in [0, 1, 7] {
        let settings = GameSettings {
            human_name: "Ana",
            max_turns: 8,
            district: District::Green,
            ai_level: AiLevel::Beginner,
            player_count,
        };
        assert!(matches!(
            create_game(&pool, &settings).await,
            Err(AppError::InvalidPlayerCount)
        ));
    }
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

    // Nobody owns a pawn yet: the city sales, if any, then the purchases follow.
    while game_record(&pool, id).await.phase == Phase::CitySales {
        sell_in_city(&pool, id, token, None).await.unwrap();
    }
    let game = game_record(&pool, id).await;
    assert_eq!(game.phase, Phase::Purchases);
    assert_eq!(game.active_seat, Some(HUMAN_SEAT));
    let buy = |purchase: Purchase| buy_from_bank(&pool, id, token, purchase);
    assert!(matches!(
        buy(Purchase {
            caravans: 1,
            peddlers: 1,
            ..Purchase::default()
        })
        .await,
        Err(AppError::Purchase(PurchaseError::InsufficientFunds))
    ));
    assert!(matches!(
        buy(Purchase {
            sold_peddlers: 1,
            ..Purchase::default()
        })
        .await,
        Err(AppError::Purchase(PurchaseError::NotInCity))
    ));
    buy(Purchase {
        peddlers: 1,
        ..Purchase::default()
    })
    .await
    .unwrap();
    let human = &players(&pool, id).await[HUMAN_SEAT as usize];
    assert_eq!(human.money, 2_000);
    let mine: Vec<PawnRecord> = pawns(&pool, id)
        .await
        .into_iter()
        .filter(|pawn| pawn.player_id == human.id)
        .collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].kind, PawnKind::Peddler);
    assert_eq!(mine[0].location, Location::City);

    let game = game_record(&pool, id).await;
    assert_eq!(game.turn, 2);
    assert_eq!(game.phase, Phase::Announcements);
    assert_eq!(game.first_seat, (first_seat + 1) % MAX_PLAYER_COUNT as i64);
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
    for ai_level in AiLevel::ALL {
        long_game_runs_to_the_end(ai_level).await;
    }
}

async fn long_game_runs_to_the_end(ai_level: AiLevel) {
    let (pool, created) = new_game_against(20, ai_level).await;
    for _ in 0..20 {
        play_human_turn(&pool, &created).await;
    }
    let game = game_record(&pool, created.game_id).await;
    assert_eq!(game.phase, Phase::Finished);
    let stocks: i64 = players(&pool, created.game_id)
        .await
        .iter()
        .map(|player| player.stock.total() + player.production.total())
        .sum();
    let carried = pawns(&pool, created.game_id)
        .await
        .iter()
        .filter(|pawn| pawn.item.is_some())
        .count() as i64;
    assert!(
        game.supply.total() + stocks + carried <= 80,
        "items only leave the box or come back to it"
    );
    assert!(matches!(
        announce_fanatic(&pool, created.game_id, &created.human_token, None).await,
        Err(AppError::WrongPhase)
    ));
}

#[tokio::test]
async fn the_whole_log_is_shown_with_the_phase_of_each_line() {
    let (pool, created) = new_game(20).await;
    for _ in 0..20 {
        play_human_turn(&pool, &created).await;
    }
    let view = GameView::load(&pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    assert!(view.events.len() > 120, "the log is not truncated");
    let first = view.events.last().unwrap();
    assert_eq!((first.turn, first.section), (1, Some(LogSection::Turn)));
    assert!(first.message.starts_with("La partie commence"));
    let expense = view
        .events
        .iter()
        .find(|event| event.message.starts_with("Dépense collective du tour"))
        .unwrap();
    assert_eq!(expense.section, Some(LogSection::CitySales));
    for section in [
        LogSection::Announcements,
        LogSection::Conversions,
        LogSection::Production,
        LogSection::Fanatics,
        LogSection::Peddlers,
        LogSection::Purchases,
    ] {
        assert!(
            view.events
                .iter()
                .any(|event| event.section == Some(section)),
            "{section:?} lines are logged"
        );
    }
}

#[tokio::test]
async fn invalid_names_and_tokens_are_rejected() {
    let (pool, created) = new_game(8).await;
    assert!(matches!(
        create_game(
            &pool,
            &GameSettings {
                human_name: AI_NAMES[2],
                max_turns: 8,
                district: District::Blue,
                ai_level: AiLevel::Strong,
                player_count: MAX_PLAYER_COUNT,
            }
        )
        .await,
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
    assert!(view.board.conversions.is_empty(), "the human has converted");
}

#[tokio::test]
async fn ai_conversions_never_block_a_long_game() {
    for ai_level in AiLevel::ALL {
        conversions_never_block(ai_level).await;
    }
}

async fn conversions_never_block(ai_level: AiLevel) {
    let (pool, created) = new_game_against(12, ai_level).await;
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
    assert!(
        marked >= 6,
        "markers spread over the map ({marked}, {ai_level:?})"
    );
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
            Phase::RebelFights => attack_first_option(&pool, &created).await,
            Phase::Production => produce_first_items(&pool, &created).await,
            Phase::Peddlers => move_and_sell_first_options(&pool, &created).await,
            Phase::CitySales => sell_first_in_city(&pool, &created).await,
            Phase::Purchases => buy_a_peddler(&pool, &created).await,
            phase => panic!("unexpected phase {phase:?}"),
        }
    }
    panic!("the human never had to move rebels");
}

/// Plays the human's actions, passing in the city and buying a peddler, until `stop` holds.
async fn play_until(
    pool: &SqlitePool,
    created: &CreatedGame,
    stop: impl Fn(&GameRecord, &PlayerRecord) -> bool,
) -> bool {
    let (id, token) = (created.game_id, &created.human_token);
    for _ in 0..300 {
        let game = game_record(pool, id).await;
        let human = players(pool, id).await.remove(HUMAN_SEAT as usize);
        if stop(&game, &human) {
            return true;
        }
        // Paying into the envelope opens the city to sales.
        let expense = human.money.min(2_000) / 100 * 100;
        match game.phase {
            Phase::Announcements => announce_fanatic(pool, id, token, None).await.unwrap(),
            Phase::Expenses => pay_expense(pool, id, token, expense).await.unwrap(),
            Phase::Conversions => convert_first_option(pool, created).await,
            Phase::RebelMovement => move_rebels_first_option(pool, created).await,
            Phase::RebelFights => attack_first_option(pool, created).await,
            Phase::Production => produce_first_items(pool, created).await,
            Phase::Peddlers => move_and_sell_first_options(pool, created).await,
            Phase::CitySales => sell_in_city(pool, id, token, None).await.unwrap(),
            Phase::Purchases => buy_a_peddler(pool, created).await,
            Phase::Finished => return false,
        }
    }
    false
}

#[tokio::test]
async fn peddlers_carry_items_out_and_sell_them() {
    let (pool, created) = new_game(30).await;
    let (id, token) = (created.game_id, &created.human_token);
    let ready = play_until(&pool, &created, |game, human| {
        game.phase == Phase::Peddlers
            && game.active_seat == Some(HUMAN_SEAT)
            && !human.moves_done
            && human.stock.total() > 0
    })
    .await;
    assert!(ready, "the human never had a peddler and a stock");
    let human = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    let mut peddler = pawns(&pool, id)
        .await
        .into_iter()
        .find(|pawn| pawn.player_id == human.id && pawn.location == Location::City)
        .expect("a peddler waits in the city");
    let order = |to: Location, item: Option<MagicItem>| PawnOrder {
        pawn_id: peddler.id,
        to,
        item,
    };
    let gate = Location::Site(SiteId(27));
    assert!(matches!(
        move_pawns(&pool, id, token, vec![order(gate, None)]).await,
        Err(AppError::CargoRequired)
    ));
    let far = Location::Site(SiteId(1));
    assert!(matches!(
        move_pawns(&pool, id, token, vec![order(far, Some(MagicItem::Toratsa))]).await,
        Err(AppError::InvalidPawnMove)
    ));

    // Put the peddler, carrying a pentacle, in a Yaboul monastery: it buys without limit.
    let monastery = SiteId(5);
    peddler.location = Location::Site(monastery);
    peddler.item = Some(MagicItem::Yaboul);
    let mut conn = pool.acquire().await.unwrap();
    peddler.save(&mut conn).await.unwrap();
    let marker = wizard_war::domain::CultMarker::face_up(Cult::Yaboul);
    wizard_war::db::CultMarkerRecord {
        site: monastery,
        marker,
    }
    .save(&mut conn, id)
    .await
    .unwrap();
    drop(conn);

    move_pawns(&pool, id, token, Vec::new()).await.unwrap();
    let view = GameView::load(&pool, id, token).await.unwrap();
    assert!(
        view.can_sell_items(),
        "the peddler may sell where it stands"
    );
    assert_eq!(view.trade.sale_choices.len(), 1);
    let supply = game_record(&pool, id).await.supply;
    sell_items(&pool, id, token, vec![peddler.id])
        .await
        .unwrap();

    let after = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    assert!(
        after.money >= human.money + 16_000,
        "mountain price for a pentacle"
    );
    assert!(game_record(&pool, id).await.supply.yaboul > supply.yaboul || after.money > 0);
    let sold = pawns(&pool, id)
        .await
        .into_iter()
        .find(|pawn| pawn.id == peddler.id)
        .unwrap();
    assert_eq!(sold.item, None);
}

#[tokio::test]
async fn pawns_on_the_same_site_pass_or_swap_their_items() {
    let (pool, created) = new_game(30).await;
    let (id, token) = (created.game_id, &created.human_token);
    let ready = play_until(&pool, &created, |game, human| {
        game.phase == Phase::Peddlers && game.active_seat == Some(HUMAN_SEAT) && !human.moves_done
    })
    .await;
    assert!(ready, "the human never had to move pawns");
    let human = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    let mut conn = pool.acquire().await.unwrap();
    PawnRecord::insert(&mut conn, id, human.id, PawnKind::Caravan)
        .await
        .unwrap();
    drop(conn);
    let mine: Vec<PawnRecord> = pawns(&pool, id)
        .await
        .into_iter()
        .filter(|pawn| pawn.player_id == human.id)
        .collect();
    let rival = pawns(&pool, id)
        .await
        .into_iter()
        .find(|pawn| pawn.player_id != human.id);
    // A talisman never sells in an outskirts village: only the transfer keeps the step open.
    let village = Location::Site(SiteId(27));
    let mut peddler = *mine
        .iter()
        .find(|pawn| pawn.kind == PawnKind::Peddler)
        .unwrap();
    let mut caravan = *mine.last().unwrap();
    let mut conn = pool.acquire().await.unwrap();
    for pawn in &mine {
        let mut pawn = *pawn;
        pawn.location = Location::City;
        pawn.item = None;
        pawn.save(&mut conn).await.unwrap();
    }
    peddler.location = village;
    peddler.item = Some(MagicItem::Toratsa);
    peddler.save(&mut conn).await.unwrap();
    caravan.location = village;
    caravan.save(&mut conn).await.unwrap();
    drop(conn);
    assert!(matches!(
        transfer_item(&pool, id, token, peddler.id, caravan.id).await,
        Err(AppError::WrongPhase)
    ));

    move_pawns(&pool, id, token, Vec::new()).await.unwrap();
    let view = GameView::load(&pool, id, token).await.unwrap();
    assert!(view.can_sell_items(), "the transfer keeps the sales step");
    assert!(view.trade.sale_choices.is_empty());
    let choice = &view.trade.transfer_choices[0];
    assert_eq!(view.trade.transfer_choices.len(), 1);
    assert_eq!(
        (choice.from_pawn_id, choice.to_pawn_id),
        (peddler.id, caravan.id)
    );
    assert_eq!(
        choice.label,
        "Passer un talisman Toratsa à la caravane vide"
    );
    let html = wizard_war::web::GamePage::new(view, None).render().unwrap();
    assert!(html.contains(&format!(
        r#"formaction="/games/{id}/players/{token}/transfer" name="transfer" value="{}-{}""#,
        peddler.id, caravan.id
    )));

    for (from, to) in [(caravan.id, peddler.id), (peddler.id, peddler.id)] {
        assert!(matches!(
            transfer_item(&pool, id, token, from, to).await,
            Err(AppError::InvalidTransfer)
        ));
    }
    if let Some(rival) = rival {
        assert!(matches!(
            transfer_item(&pool, id, token, peddler.id, rival.id).await,
            Err(AppError::InvalidTransfer)
        ));
    }
    transfer_item(&pool, id, token, peddler.id, caravan.id)
        .await
        .unwrap();
    let item_of = |pawns: &[PawnRecord], id: i64| pawns.iter().find(|p| p.id == id).unwrap().item;
    let after = pawns(&pool, id).await;
    assert_eq!(item_of(&after, peddler.id), None);
    assert_eq!(item_of(&after, caravan.id), Some(MagicItem::Toratsa));
    let game = game_record(&pool, id).await;
    assert_eq!(
        (game.phase, game.active_seat),
        (Phase::Peddlers, Some(HUMAN_SEAT))
    );

    let mut conn = pool.acquire().await.unwrap();
    peddler.item = Some(MagicItem::Xephis);
    peddler.save(&mut conn).await.unwrap();
    drop(conn);
    transfer_item(&pool, id, token, caravan.id, peddler.id)
        .await
        .unwrap();
    let after = pawns(&pool, id).await;
    assert_eq!(item_of(&after, peddler.id), Some(MagicItem::Toratsa));
    assert_eq!(item_of(&after, caravan.id), Some(MagicItem::Xephis));
    let view = GameView::load(&pool, id, token).await.unwrap();
    let messages: Vec<&str> = view.events.iter().map(|e| e.message.as_str()).collect();
    assert!(messages.contains(&"Ana : le colporteur passe son objet à la caravane à Vignelles."));
    sell_items(&pool, id, token, Vec::new()).await.unwrap();
    assert!(
        !GameView::load(&pool, id, token)
            .await
            .unwrap()
            .can_sell_items()
    );
}

#[tokio::test]
async fn rebels_reaching_a_pawn_fight_it() {
    let (pool, created) = new_game(30).await;
    let (id, token) = (created.game_id, &created.human_token);
    let ready = play_until(&pool, &created, |game, _| {
        game.phase == Phase::RebelMovement && game.active_seat == Some(HUMAN_SEAT)
    })
    .await;
    assert!(ready, "the human never had to move rebels");
    let view = GameView::load(&pool, id, token).await.unwrap();
    let choice = &view.board.rebel_choices[0];
    let target = SiteId(choice.destinations[0].site);

    let human = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    let mut conn = pool.acquire().await.unwrap();
    PawnRecord::insert(&mut conn, id, human.id, PawnKind::Caravan)
        .await
        .unwrap();
    let mut caravan = PawnRecord::list_for_game(&mut conn, id)
        .await
        .unwrap()
        .into_iter()
        .last()
        .unwrap();
    caravan.location = Location::Site(target);
    caravan.save(&mut conn).await.unwrap();
    drop(conn);

    let rebels_before = view
        .board
        .sites
        .iter()
        .map(|site| site.rebels)
        .sum::<usize>();
    move_rebels_first_option(&pool, &created).await;
    assert_eq!(game_record(&pool, id).await.phase, Phase::RebelFights);
    assert!(matches!(
        attack_pawn(&pool, id, token, -1).await,
        Err(AppError::InvalidFightTarget)
    ));
    while game_record(&pool, id).await.phase == Phase::RebelFights {
        attack_first_option(&pool, &created).await;
    }
    let view = GameView::load(&pool, id, token).await.unwrap();
    let rebels_after = view
        .board
        .sites
        .iter()
        .map(|site| site.rebels)
        .sum::<usize>();
    assert!(
        rebels_after < rebels_before,
        "the rebels left after fighting"
    );
}

#[tokio::test]
async fn city_sales_need_the_item_in_stock() {
    let (pool, created) = new_game(30).await;
    let (id, token) = (created.game_id, &created.human_token);
    let ready = play_until(&pool, &created, |game, _| {
        game.phase == Phase::CitySales && game.active_seat == Some(HUMAN_SEAT)
    })
    .await;
    assert!(ready, "the city never bought anything");
    let human = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    let missing = MagicItem::ALL
        .into_iter()
        .find(|&item| human.stock.get(item) == 0);
    if let Some(item) = missing {
        assert!(matches!(
            sell_in_city(&pool, id, token, Some(item)).await,
            Err(AppError::NotInStock)
        ));
    }
    let item = MagicItem::ALL
        .into_iter()
        .find(|&item| human.stock.get(item) > 0)
        .unwrap();
    let left = game_record(&pool, id).await.city_sales_left;
    sell_in_city(&pool, id, token, Some(item)).await.unwrap();
    let after = players(&pool, id).await.remove(HUMAN_SEAT as usize);
    assert_eq!(after.stock.total(), human.stock.total() - 1);
    assert_eq!(
        after.money,
        human.money + wizard_war::domain::market::city_price(item)
    );
    let game = game_record(&pool, id).await;
    assert!(game.phase != Phase::CitySales || game.city_sales_left < left);
}

#[tokio::test]
async fn hall_fanatics_stay_hidden_until_phase_iv_then_leave_at_turn_end() {
    let (pool, created) = new_game(8).await;
    let (id, token) = (created.game_id, &created.human_token);
    let view = GameView::load(&pool, id, token).await.unwrap();
    let mine = view.available_fanatics()[0].id;
    announce_fanatic(&pool, id, token, Some(mine))
        .await
        .unwrap();
    let view = GameView::load(&pool, id, token).await.unwrap();
    let sent = view.city.hall.len();
    assert!(sent >= 1, "the human's fanatic is in the hall");
    assert!(
        view.city
            .hall
            .iter()
            .all(|token| token.image == "/img/token-fanatic-hidden.png")
    );

    pay_expense(&pool, id, token, 0).await.unwrap();
    loop {
        match game_record(&pool, id).await.phase {
            Phase::Conversions => convert_first_option(&pool, &created).await,
            Phase::RebelMovement => move_rebels_first_option(&pool, &created).await,
            Phase::RebelFights => attack_first_option(&pool, &created).await,
            Phase::Production => produce_first_items(&pool, &created).await,
            _ => break,
        }
    }
    let view = GameView::load(&pool, id, token).await.unwrap();
    assert_eq!(view.turn, 1);
    assert_eq!(
        view.city.hall.len(),
        sent,
        "revealed pawns stay in the hall"
    );
    assert!(
        view.city
            .hall
            .iter()
            .all(|token| token.image != "/img/token-fanatic-hidden.png")
    );

    while game_record(&pool, id).await.turn == 1 {
        match game_record(&pool, id).await.phase {
            Phase::Peddlers => move_and_sell_first_options(&pool, &created).await,
            Phase::CitySales => sell_first_in_city(&pool, &created).await,
            Phase::Purchases => buy_a_peddler(&pool, &created).await,
            phase => panic!("unexpected phase {phase:?}"),
        }
    }
    let view = GameView::load(&pool, id, token).await.unwrap();
    assert!(view.my_fanatics.iter().all(|record| record.id != mine));
    assert!(
        view.city
            .hall
            .iter()
            .all(|token| token.title == "Fanatiques face cachée")
    );
}
