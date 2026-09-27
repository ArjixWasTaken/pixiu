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

> **Status: early development.** píxiū is a working OpenSubsonic server for
> music you upload. The YouTube Music hunter is next.

## What works

- **Offerings.** Upload audio files or zip archives in the WebUI. píxiū reads
  their tags, you review, and accepted files are filed into the treasure as
  `Album Artist/Year - Album/Disc-Track Title.ext`, with the cover saved next to
  them. Nothing is ever deleted behind your back.
- **OpenSubsonic API** at `/rest`: browsing (artists, albums, songs, folders,
  genres), album lists, random songs, search (`search2`/`search3`, including
  empty queries for clients that sync the whole library), streaming and
  downloads with seeking (HTTP ranges), resized cover art, play counts
  (`scrobble`), and play queues that follow you across devices. Tested with
  Feishin and Airsonic Refix.
- **Authentication**: your píxiū password (plain or token authentication), or
  OpenSubsonic API keys created in Settings. Browser-based clients may call the
  API from other origins (CORS).

## Running

### Docker

```sh
mkdir -p data treasure   # must be writable by uid 1000
docker compose up -d --build
```

Open <http://localhost:4533> and claim the hoard: the first visitor creates the
admin account. Then point your Subsonic client at `http://<host>:4533` with the
same username and password.

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
| `paths.data_dir` | `data` | Database, the secret key, caches and staged uploads. |
| `paths.treasure_dir` | `treasure` | The music library, owned by píxiū. |

## Development

```sh
cargo test --workspace                 # unit + end-to-end tests
cargo clippy --workspace --all-targets
```

| Crate | |
|---|---|
| `pixiu` | The binary: configuration, wiring, serving. |
| `pixiu-core` | Configuration, secrets at rest and password hashing. |
| `pixiu-db` | [Toasty](https://github.com/tokio-rs/toasty) models and migrations (SQLite). |
| `pixiu-subsonic` | The OpenSubsonic REST API (axum, mounted at `/rest`). |
| `pixiu-treasury` | The library on disk: tags, layout, ingest, covers and offerings. |
| `pixiu-web` | The WebUI, built with [Topcoat](https://github.com/tokio-rs/topcoat). |

### Changing the database schema

Edit the models in `crates/pixiu-db/src/models.rs`, then generate a migration
and commit the files it writes under `crates/pixiu-db/toasty/`:

```sh
cargo run -p pixiu-db --features cli -- migration generate --name describe_change
```

The server applies pending migrations on startup. A test fails if the models
drift from the latest migration.
