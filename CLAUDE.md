# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this is

A standalone web implementation of *La guerre des magiciens* ("The Wizard War"), an old French game published in *Jeux et Stratégie* (issue 46, 1987). It started as a Board Game Arena Studio prototype in PHP/JS; that code was removed on the `rust_conversion` branch and replaced by this Rust crate (`wizard_war`). `RULES.md` holds the translated rules and the list of unclear points; scans of the original and `.xcf` (GIMP) sources for the images are in `work/`.

## Commands

```sh
cargo run                                   # serve on http://127.0.0.1:3000 (DATABASE_URL, BIND_ADDR, ASSET_ROOT, RUST_LOG)
cargo test                                  # unit tests + tests/ (in-memory SQLite)
cargo test --test game_flow <name>          # one integration test
cargo clippy --all-targets -- -D warnings
cargo fmt
```

## Architecture

Stack: axum (HTTP), askama (server-rendered HTML in `templates/`), sqlx on SQLite. No front-end build: `static/style.css` and `static/poll.js`, images served from `img/`.

Layers, from the inside out:

- **`src/domain/`**: pure rules, no I/O, unit-tested. `board` (map), `Site`/`SiteId`/`SiteKind`/`Zone`, `CultMarker`, `Conversion`, `BoardState`, `rebel_movement`, `ItemCounts` (supply/stock/production of the three items), `MagicItem`, `Cult`, `Fanatic`, `FanaticPool` (24 pawns, dealt 6/5/4 per player), `FanaticVerdict` + `dominant_cult` (phase IV), `TurnOrder` (seat rotation), `Phase`, `rules` constants (`PLAYER_COUNT`, `HUMAN_SEAT`, `AI_NAMES`, `LABORATORY_PRICE`…), `District` (the six colours), `PawnKind` (peddler/caravan: speed, price, limit), `Location` (city or site), `travel` (movement graph through the city gates), `PawnOrder`, `market` (Table 1 prices, sale rules, Table 2 quota), `FightOutcome` (die table), `Purchase`/`Holdings`/`PurchaseError` (phase VII), `ai_strategy` and `ai_trading` (pure decisions of the computer players).
- **`src/db/`**: one record type per table (`GameRecord`, `PlayerRecord`, `FanaticRecord`, `EventRecord`, `CultMarkerRecord`, `RebelRecord`, `PawnRecord`, `SaleRecord`) with async functions taking `&mut SqliteConnection`. Pattern: load a record, change it in Rust, call `save`. `GameRecord::save` increments `games.version`. Queries use runtime `sqlx::query`/`query_as` (no compile-time macros, so no `DATABASE_URL` at build time). `db::connect` runs `migrations/` (embedded with `sqlx::migrate!`). Schema changes go in a new migration file, never by editing an applied one.
- **`src/service/`**: one use case per file. Each player action opens `begin_write` (`BEGIN IMMEDIATE`), loads `PlayerAction` (game + player from the link token, checks the phase), validates, writes, logs a French message to `event_log`, then `PlayerAction::finish` lets the AIs play (`ai_players::play_ai_players`), saves the game (bumping its version) and commits. Each action's rules live in a `record_*` function shared by the human wrapper and the AI driver. `GameView` is the player-visible snapshot used by the page (`BoardView` for the map, `CityView` for the laboratories drawn in each district and the fanatics in the great hall, `TradeView` for pawns, fights, sales and purchases). Sequential phases pick the next player with `seat_cycle::next_eligible_seat`, skipping players with nothing to do.
- **`src/web/`**: `router.rs` lists the routes; handlers parse forms, call a service, then redirect (303) to the player page, or re-render it with the error (400). `AppError` (`src/error.rs`) has French `Display` texts and becomes an HTML error page; internal errors are logged and shown as a generic message.

Every game has 6 players: the human at seat 0 and 5 computer-controlled alchemists (`players.is_ai`). AIs act synchronously, inside the transaction of the human's action (and at game creation), until the human has to act again, so the human never waits. The AI is deliberately basic: it backs the cult its fanatics are strongest in, sends its strongest pawn of that cult one turn in two, makes that cult's items only when it sent a pawn (Toratsa talismans otherwise), pays a random 0–1,000 écus expense, spreads its cult on the map (replace hidden rival markers, then place its own, castles and monasteries first, then flip) and moves rebels at random. Rebels attack a rival's pawn when possible. Its pawns load the stock item that sells best and head for the best buyer, sell as soon as they can, go home when empty, and never stop on rebels. It sells in the city whenever asked, and buys one pawn per laboratory (caravans first), then laboratories, keeping 1,000 écus. It only reads its own hand, pawns and money, the supply and the map.

Game flow (`games.phase`): `announcements` (simultaneous, each player may send one fanatic to the hall) → `expenses` (simultaneous, secret, multiples of 100 écus) → `conversions` (sequential from `first_seat` via `active_seat`, each player places, flips or replaces one cult marker) → `rebel_movement` (only the first player, skipped when no rebel can move) → `rebel_fights` (only when rebels reached peddlers or caravans: the first player picks each target, `rebels.must_fight` marks pending fights) → `production` (sequential, exactly `min(laboratories, supply)` items) → after the last producer, `resolve_fanatics` (phase IV: the hall's fanatics get `fanatics.revealed` and stay face up on the map until `end_turn` discards them) → `peddlers` (phase V, sequential, players with a movable pawn only: moves, then sales, tracked by `players.moves_done`; sites that bought this turn are in `sales`) → `city_sales` (phase VI, round after round, `games.city_sales_left` and `city_passes`) → `purchases` (phase VII, sequential) → `end_turn` (first player rotates left, or `finished` after `max_turns`).

The map (`src/domain/board.rs`) is a static table transcribed from `img/board.jpg`: 32 sites with invented French names, zone, pixel position, whether a path reaches the board's edge, and the paths between sites. The city has one gate in front of each outskirts village (`GATE_VILLAGES`, 27 to 32). Per-game state is in `cult_markers` (one per site, cult + face up/down), `rebels` and `pawns` (peddlers and caravans: owner, kind, site or `NULL` for the city, carried item, `stopped_until_turn`); `BoardState` holds the phase II rules (restrictions a/b/c, 21 markers per cult, 7 rebels).

`img/board.jpg` (2780×2289) is the map cut out of the board photo (`work/6a013486df16fc970c0133f3bcf0b2970b.jpg`) at the photo's own resolution, inside its black frame, with the perspective and the poster's curvature straightened (scripts in `work/extraction/`, see its README). Pawn images are `img/token-*.png`, cut out at the same resolution from the counter sheet on the left of that photo (about 55 px for small pawns, 72 px for magic items, 122×26 for fanatic strips, turned to lie flat). The printed pawns have no back face, so a face-down cult marker has none in the original game either: `img/token-cult-hidden.png` (the Xéphis sign emptied, with a brown "?") is invented for this implementation, as is `img/token-fanatic-hidden.png` (a strip with a "?") for fanatics lying face down in the great hall. The district and hall positions on the map are in `District::board_position` and `board::HALL_POSITION`. The game page draws the map and its pawns at the pictures' own size (never scaled), the map in a scrolling viewport, and puts the log in a full-height column beside it, or below it on windows narrower than 1480 px.

Players have no accounts: creating a game redirects the human to their private URL (`/games/{id}/players/{token}`); AIs have tokens too but nobody uses them. Open pages poll `/games/{id}/version` every 2 s and reload when it changes. Only information a player may see goes into `GameView` (own fanatics, stock, production and pawn loads; others' stock is shown as a count, their money as 1 to 4 coins (`Wealth`: under 5,000, under 10,000, under 20,000, more) and their pawns without their items). The human chooses their district (pawn colour) when creating the game; the AIs get the others.

## Development guidelines

### Imports

- Prefer `crate::`-rooted paths for imports across modules; treat this as the Rust equivalent of "absolute imports."
- `super::` is acceptable within tightly nested module hierarchies (e.g. a tests submodule referencing its parent, or a submodule referencing its direct parent's types).
- Avoid deep `super::super::` chains — restructure the module tree instead.
- Avoid glob imports (`use foo::*;`) except for prelude modules explicitly designed for it, or in test modules (`use super::*;`).

### Type Annotations

- Annotate all `pub` function signatures explicitly (parameters and return type), even where inference would work.
- Let type inference handle local variable bindings inside function bodies.
- Prefer concrete types over generics unless genericity provides real, used value (called with more than one concrete type, or part of a public API meant to be generic).
- Avoid `Box<dyn Any>` and other type-erasure patterns unless strictly necessary; justify with a short comment.
- Avoid `Box<dyn Error>` for library code — prefer a concrete, well-typed error enum. `Box<dyn Error>` is acceptable at binary/application boundaries (e.g. `main`).

### Docstrings

- All `pub` items (functions, methods, structs, enums, traits) require a rustdoc comment (`///`), following standard rustdoc sections (`# Arguments`, `# Returns`, `# Errors`, `# Panics` as applicable).
- Private (non-`pub`) functions get a docstring only when the logic isn't obvious from the signature and name.
- Keep docstrings proportionate: short for simple functions, more detailed only when it adds real clarity.
- Do not restate information already conveyed by the signature (parameter types, obvious return values).
- Enforce at the crate level with `#![warn(missing_docs)]` (or `#![deny(missing_docs)]` for libraries).

### Comments

- Do not add comments to explain straightforward or self-evident code.
- Add comments only when the logic is genuinely complex, non-obvious, or could be misread.
- Keep comments short and synthetic — a brief note on intent or reasoning, not a narration of what the code does.
- Avoid verbose or redundant comments; never caption the obvious.

### Error Handling

- Use `Result<T, E>` for recoverable errors and `Option<T>` for absence. Do not use panics as control flow.
- Propagate errors with `?` rather than manual `match`/`if let` chains, unless the error needs to be transformed or contextualized at that point.
- Do not use `unwrap()` or `expect()` in library or production code paths. Reserve for tests, examples, and prototypes.
- If used outside those contexts because a failure is provably impossible, justify with a short comment (e.g. `// safe: regex is a compile-time constant`).
- Define proper error types (plain enums, or via `thiserror`) rather than stringly-typed errors (`Err("something failed".to_string())`) or blanket `Box<dyn Error>` in library code.
- Do not silently discard errors with `.ok()`, `let _ =`, or similar — handle, log, or propagate explicitly.
- `panic!` (and `unreachable!`, `todo!`, `unimplemented!` left in shipped code) is reserved for genuine invariant violations (bugs), not expected failure modes.

### Function Size & Decomposition

- Soft limit of 50 lines per function.
- If a function exceeds this limit, break it into smaller, well-named sub-functions, each with a single clear responsibility and its own concise docstring.
- Prefer composition of small functions over deeply nested logic or long procedural blocks.
- Prefer early returns / `?`-based flattening over deeply nested `match`/`if let` pyramids.

### Types & Modules

- One primary public type (struct, enum, or trait) per file, with its associated `impl` block(s).
- File name (snake_case) matches the type name (e.g. `struct UserAccount` → `user_account.rs`).
- Exceptions: small, tightly-coupled types may share the file with the primary type — e.g. an error enum returned by its methods, a config/builder struct feeding it, or a result type it produces.
- A closely-related family of small types with no independent identity (e.g. AST node variants, a state-machine's states) may live together in one file organized around the concept they jointly express, rather than one type per file. Split it out once it grows large enough to be its own topic.
- Traits implemented by multiple unrelated types get their own file; a trait implemented by only one type may be co-located with that type.

### File & Module Organization

- Avoid monolithic files. Split code by topic/responsibility into modules (`mod foo;`).
- Group related modules into a directory-based module tree (subpackages) when a topic grows large enough.
- Keep `mod.rs` / inline module-declaration files thin — `pub use` re-exports only, no implementation logic.
- Keep `lib.rs` / `main.rs` as a thin entry point: module declarations, top-level wiring, and re-exports — no business logic.

### General Principles

- Favor readability and small, testable units over cleverness.
- Prefer explicit over implicit in naming, typing, and structure.
- Split files as soon as they start doing "too much" rather than waiting for a refactor pass.
- Prefer composition over inheritance-like patterns; use traits for shared behavior, not deep hierarchies.
- Align with idiomatic Rust and clippy/rustfmt defaults rather than fighting them; deviate only when this style guide explicitly says so.

### Language

- Code is English, whatever language the prompt is in. Identifiers, function names, comments, docstrings, commit messages and log lines are written in English even when the request or the discussion is in French.
- User-facing text is French, and stays French. Error messages surfaced to users, CLI output, labels, templates — anything a reader sees is French, UTF-8, accents kept. Do not translate it, and write new user-facing text in French.
