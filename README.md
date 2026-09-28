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

> **Status: early development.** píxiū serves the music you upload, hunts
> music on YouTube Music, keeps up with the playlists and artists you watch,
> tags it all with MusicBrainz and lyrics, and streams it in the format each
> client wants.

## What works

- **Hunting.** Search YouTube Music in the WebUI and grab songs or whole
  albums. Downloads run in the background (the Jobs page shows their progress
  live): the Opus audio stream is remuxed losslessly into `.opus`, tagged, and
  filed into the treasure with its cover. When YouTube refuses a direct
  download, píxiū falls back to `yt-dlp`.
- **Watches.** Paste a YouTube Music playlist or artist link, or watch your
  liked music. Watched playlists are mirrored as (read-only) Subsonic
  playlists and their songs are downloaded as they appear; artists bring in
  their new releases (optionally singles and EPs, or their whole
  discography). Syncing is one-way and runs on a schedule, or on demand.
  Liked music needs a login; while the session is expired its syncs wait,
  and resume by themselves once you log in again.
- **Tagging.** Each album píxiū takes in is looked up on
  [MusicBrainz](https://musicbrainz.org/). A certain match retags and
  refiles it (titles, track numbers, MusicBrainz ids), takes a larger cover
  from the Cover Art Archive when there is one, and brings the artist's
  biography and picture from Wikipedia. When several releases might be it,
  the album waits on the Library page for you to pick one (or paste a
  MusicBrainz release link); albums MusicBrainz doesn't know keep their tags.
  You can also edit an album's tags by hand.
- **Lyrics** come from the file itself, [LRCLIB](https://lrclib.net) (often
  time-synced) or YouTube Music. Instrumentals are recognized as such; songs
  with no lyrics anywhere are looked up again a month later, or whenever you
  look their album up.
- **Orphans.** Every track records why it is kept (an offering, a grab, a
  watch, a playlist). When a song leaves a watched playlist, or you stop
  watching something, its files stay; tracks nothing keeps any more are
  listed as orphans, for you to delete (one by one or all at once) or keep.
- **YouTube Music login.** Log in through a real browser that runs on the
  server and appears in the WebUI, so two-factor prompts work as usual. In
  browsers with the experimental
  [HTML-in-Canvas](https://github.com/WICG/html-in-canvas) API (Chrome, with
  `chrome://flags/#canvas-draw-element`), its sign-in fields are mirrored into
  the page as real inputs, so your password manager can fill them; add
  píxiū's address to your Google login for it to offer them. A
  session warden checks the login every half hour and refreshes its cookies
  twice a day. If Google ends the session (a password change, "sign out
  everywhere"), every page says so until you log in again.
- **Offerings.** Upload audio files or zip archives in the WebUI. píxiū reads
  their tags, you review, and accepted files are filed into the treasure,
  with the cover saved next to them. Nothing is ever deleted behind your
  back.
- **Your file layout.** Tracks are filed as
  `Album Artist/Year - Album/Disc-Track Title.ext` unless you set a template
  of your own in Settings, e.g. `{genre}/{album_artist}/[{year} - ]{album}/{track:02} {title}`
  (a part in brackets is left out when a value in it is missing). Existing
  files move to a new layout when you ask.
- **Streaming.** Files are served as they are, with seeking (HTTP ranges),
  unless a client asks for another format (MP3, Opus or AAC), a lower
  bitrate, or a later start: then píxiū transcodes on the fly with FFmpeg's
  libraries, and clients seek with `timeOffset` (OpenSubsonic's
  `transcodeOffset`).
- **OpenSubsonic API** at `/rest`: browsing (artists, albums, songs, folders,
  genres), album lists, random songs, search (`search2`/`search3`, including
  empty queries for clients that sync the whole library), downloads, resized
  cover art, play counts (`scrobble`), what is playing now, stars and
  ratings, play queues that follow you across devices, playlists (your own,
  plus the mirrors of watched playlists), lyrics (time-synced through
  `getLyricsBySongId`), artist biographies and pictures, and top and
  similar songs worked out from your own library. Starring a song or an
  album also keeps it from ever becoming an orphan. Tested with Feishin and
  Airsonic Refix.
- **Authentication**: your píxiū password (plain or token authentication), or
  OpenSubsonic API keys created in Settings. Browser-based clients may call the
  API from other origins (CORS).

## Running

### Docker

```sh
mkdir -p data treasure   # must be writable by uid 1000
docker compose up -d --build
```

The image is built on Alpine Linux (musl) and includes everything hunting
needs: Chromium for the login browser, the FFmpeg libraries, and `yt-dlp`
with Deno. The compose file runs it
hardened: a read-only root filesystem (only the volumes and two scratch
`tmpfs` mounts are writable), no Linux capabilities, and no way to gain
privileges.

Open <http://localhost:4533> and claim the hoard: the first visitor creates the
admin account. Then point your Subsonic client at `http://<host>:4533` with the
same username and password.

### From source

Requirements:

- [rustup](https://rustup.rs/); the toolchain is pinned in `rust-toolchain.toml`.
- FFmpeg's development libraries (any version from 3.0 to 9.0), `clang` and
  `pkg-config`. On Debian: `apt install libavcodec-dev libavformat-dev
  libavutil-dev libswresample-dev libclang-dev pkg-config`.
- For hunting: Chromium (to log in to YouTube Music), and `yt-dlp` with
  [Deno](https://deno.com/) for the download fallback.
- The Topcoat CLI, which bundles the WebUI's assets:

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
| `browser.executable` | from `PATH` | Chromium, for the login browser. |
| `browser.no_sandbox` | `false` | Needed in most containers; the Docker image sets it. |
| `stream.format` | `mp3` | What transcodes use when a client asks for a lower bitrate but names no format, and the file's own format cannot be made (FLAC, say): `mp3`, `opus` or `aac`. |
| `stream.max_transcodes` | `4` | Transcodes running at once; more wait their turn. |
| `enrich.contact` | unset | An email address or URL of yours, which MusicBrainz and Wikimedia ask for. It goes in the `User-Agent` of píxiū's lookups (MusicBrainz, the Cover Art Archive, LRCLIB and Wikipedia) and nowhere else. |
| `hunt.botguard` | unset | [`rustypipe-botguard`](https://codeberg.org/ThetaDev/rustypipe-botguard), which answers YouTube's proof-of-origin challenges so more YouTube clients can be used for direct downloads. Not in the Docker image, since it is only built for glibc; downloads go through `yt-dlp` meanwhile. |

## Development

```sh
cargo test --workspace                 # unit + end-to-end tests
cargo test --workspace -- --ignored    # also drive Chromium, YouTube Music, MusicBrainz and LRCLIB
cargo clippy --workspace --all-targets
```

| Crate | |
|---|---|
| `pixiu` | The binary: configuration, wiring, serving. |
| `pixiu-browser` | The login browser: Chromium over CDP, its screen streamed as JPEG frames. |
| `pixiu-core` | Configuration, secrets at rest and password hashing. |
| `pixiu-db` | [Toasty](https://github.com/tokio-rs/toasty) models and migrations (SQLite). |
| `pixiu-enrich` | MusicBrainz lookups and matching, the Cover Art Archive, LRCLIB and Wikipedia. |
| `pixiu-hunt` | YouTube Music ([rustypipe](https://codeberg.org/ThetaDev/rustypipe)), downloads and tagging. |
| `pixiu-jobs` | The job queue and the session warden. |
| `pixiu-media` | Remuxing and transcoding with FFmpeg's libraries. |
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

## License

Not chosen yet. Note that píxiū depends on rustypipe, which is licensed under
the GPL-3.0; that constrains how builds of píxiū can be distributed.
