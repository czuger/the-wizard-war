use askama::Template;
use sqlx::SqlitePool;
use wizard_war::db;
use wizard_war::domain::{AiLevel, Conversion, District};
use wizard_war::service::{
    CreatedGame, GameView, announce_fanatic, convert_site, create_game, pay_expense,
};
use wizard_war::web::{GamePage, Screen};

async fn new_game() -> (SqlitePool, CreatedGame) {
    let pool = db::connect("sqlite::memory:").await.unwrap();
    let game = create_game(&pool, "Ana", 8, District::Green, AiLevel::Beginner)
        .await
        .unwrap();
    (pool, game)
}

async fn page(pool: &SqlitePool, created: &CreatedGame) -> (Screen, String) {
    let view = GameView::load(pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let page = GamePage::new(view, None);
    (page.screen, page.render().unwrap())
}

#[tokio::test]
async fn phases_without_the_map_hide_it() {
    let (pool, created) = new_game().await;
    let (screen, html) = page(&pool, &created).await;
    assert_eq!(screen, Screen::Announcement);
    assert!(html.contains(r#"name="fanatic""#));
    assert!(!html.contains("board.jpg"));
    assert!(html.contains("Journal"));

    announce_fanatic(&pool, created.game_id, &created.human_token, None)
        .await
        .unwrap();
    let (screen, html) = page(&pool, &created).await;
    assert_eq!(screen, Screen::Expense);
    assert!(html.contains(r#"name="amount""#));
    assert!(!html.contains("board.jpg"));
}

#[tokio::test]
async fn conversions_are_offered_on_the_map_sites() {
    let (pool, created) = new_game().await;
    let (id, token) = (created.game_id, &created.human_token);
    announce_fanatic(&pool, id, token, None).await.unwrap();
    pay_expense(&pool, id, token, 0).await.unwrap();
    let (screen, html) = page(&pool, &created).await;
    assert_eq!(screen, Screen::Conversion);
    assert!(html.contains("board.jpg"));
    assert!(html.contains("/static/board.js"));
    assert!(html.contains(r#"<details class="site site-menu""#));
    assert!(html.contains(r#"form="phase-form" name="conversion" value=""#));

    let view = GameView::load(&pool, id, token).await.unwrap();
    let conversion = view
        .board
        .conversions
        .first()
        .and_then(|option| Conversion::from_form_value(&option.value));
    convert_site(&pool, id, token, conversion).await.unwrap();
    let (screen, html) = page(&pool, &created).await;
    if screen == Screen::RebelMovement {
        assert!(html.contains(r#"data-mover="rebel_"#));
        assert!(html.contains("data-require-all"));
    }
}

#[tokio::test]
async fn banner_lists_players_in_seat_order_from_the_first_player() {
    let (pool, created) = new_game().await;
    let view = GameView::load(&pool, created.game_id, &created.human_token)
        .await
        .unwrap();
    let seat_names: Vec<&str> = std::iter::once("Ana")
        .chain(wizard_war::domain::rules::AI_NAMES)
        .collect();
    let first = view.players.iter().position(|p| p.is_first).unwrap();
    assert_eq!(first, 0);
    let first_seat = seat_names
        .iter()
        .position(|name| *name == view.players[0].name)
        .unwrap();
    let shown: Vec<&str> = view.players.iter().map(|p| p.name.as_str()).collect();
    let expected: Vec<&str> = (0..seat_names.len())
        .map(|step| seat_names[(first_seat + step) % seat_names.len()])
        .collect();
    assert_eq!(shown, expected);
}
