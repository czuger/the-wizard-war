# The wizard war (la guerre des magiciens)

A web version of *La guerre des magiciens*, an old French board game published in *Jeux et Stratégie* no. 46 (1987). See `RULES.md` for the translated rules.

## Run

```sh
cargo run
```

Then open <http://127.0.0.1:3000>, enter your name and play against five alchemists controlled by the computer. Bookmark the page you land on: its URL is your private link to the game.

Settings (environment variables):

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | `sqlite://data/wizard_war.db` | SQLite database, created with its folder if missing |
| `BIND_ADDR` | `127.0.0.1:3000` | Address the server listens on |
| `ASSET_ROOT` | `.` | Folder holding `img/` and `static/` |
| `RUST_LOG` | `info` | Log filter |
