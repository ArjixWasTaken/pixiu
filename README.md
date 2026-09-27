<div align="center">
  <img src="crates/pixiu-web/assets/emblem-512.webp" alt="píxiū emblem" width="200">
  <h1>píxiū</h1>
  <p><em>Gathers music from afar and never lets it go.</em></p>
</div>

The píxiū (貔貅) is a mythical beast, part dragon and part lion with feathered
wings, that devours treasure and never gives any back. This píxiū does the same
with music. It is an [OpenSubsonic](https://opensubsonic.netlify.app/) server
that also hunts: it downloads music from streaming platforms (YouTube Music
first) into a library it owns, and serves that hoard to any Subsonic client.

> **Status: early development.** The skeleton is in place: WebUI with first-run
> setup and login, database migrations, and the Subsonic API envelope (`ping`,
> `getLicense`, `getOpenSubsonicExtensions`). The library, the Subsonic browsing
> and streaming endpoints, and the YouTube Music hunter are next.

## Running

### Docker

```sh
mkdir -p data treasure   # must be writable by uid 1000
docker compose up -d --build
```

Open <http://localhost:4533> and claim the hoard: the first visitor creates the
admin account.

### From source

Requirements: [rustup](https://rustup.rs/) (the toolchain is pinned in
`rust-toolchain.toml`) and the Topcoat CLI, which bundles the WebUI's assets:

```sh
cargo install topcoat-cli@0.9.0 --locked

topcoat asset bundle --package pixiu   # builds the binary and bundles assets next to it
./target/debug/pixiu
```

`topcoat dev --package pixiu` rebuilds and reloads on every change.

## Configuration

Settings come from built-in defaults, then `pixiu.toml` (or the file named by
`PIXIU_CONFIG`), then `PIXIU_`-prefixed environment variables, with `__` for
nesting (`PIXIU_SERVER__PORT=4533`). See [`pixiu.example.toml`](pixiu.example.toml).

| Setting | Default | |
|---|---|---|
| `server.host` | `127.0.0.1` | The Docker image uses `0.0.0.0`. |
| `server.port` | `4533` | |
| `server.cookie_security` | `auto` | Mark the session cookie `Secure` over HTTPS. |
| `paths.data_dir` | `data` | Database and internal state. |
| `paths.treasure_dir` | `treasure` | The music library, owned by píxiū. |

## Development

```sh
cargo test --workspace                 # unit + end-to-end tests
cargo clippy --workspace --all-targets
```

| Crate | |
|---|---|
| `pixiu` | The binary: configuration, wiring, serving. |
| `pixiu-core` | Configuration and shared types. |
| `pixiu-db` | [Toasty](https://github.com/tokio-rs/toasty) models and migrations (SQLite). |
| `pixiu-subsonic` | The OpenSubsonic REST API (axum, mounted at `/rest`). |
| `pixiu-web` | The WebUI, built with [Topcoat](https://github.com/tokio-rs/topcoat). |

### Changing the database schema

Edit the models in `crates/pixiu-db/src/models.rs`, then generate a migration
and commit the files it writes under `crates/pixiu-db/toasty/`:

```sh
cargo run -p pixiu-db --features cli -- migration generate --name describe_change
```

The server applies pending migrations on startup. A test fails if the models
drift from the latest migration.
