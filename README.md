<div align="center">
  <img src="web/public/img/logo.png" alt="píxiū" width="320">
  <p><em>Gathers music from afar and never lets it go.</em></p>
</div>

> [!WARNING]
> **Don't use this.** píxiū is vibe-coded: most of it was written by an AI,
> with little human review, and it is not in a state its author would call
> good. Expect bugs, security holes, lost or mangled music, and breaking
> changes between commits. It is public so the code can be read, not because
> it is ready to be run, and no support is offered.

An [OpenSubsonic](https://opensubsonic.netlify.app/) server with a web player
that downloads music from YouTube Music into a library it owns.

## Features

| | |
|---|---|
| **Web player** | Forked from [koel](https://koel.dev): library, search, queue synced across devices, likes, ratings, synced lyrics, equalizer, playlists, smart playlists, folders. |
| **Discover** | Search YouTube Music; download songs or albums. Opus remuxed losslessly, tagged, filed with its cover; `yt-dlp` as fallback. |
| **Watches** | Playlists (mirrored read-only), liked music, and artists' new releases, synced one-way on a schedule. |
| **Jobs** | Downloads, syncs and lookups, live. |
| **Uploads** | Audio files or zip archives, reviewed before they join the library. |
| **Orphans** | Songs nothing keeps any more (left a playlist, excluded, unwatched); kept or deleted by hand, never automatically. |
| **Tagging** | [MusicBrainz](https://musicbrainz.org/) lookups, Cover Art Archive covers, Wikipedia bios; pick a release when unsure; edit albums by hand. |
| **Lyrics** | From the file, [LRCLIB](https://lrclib.net) or YouTube Music; instrumentals recognized. |
| **YouTube Music login** | A real browser on the server, shown in the player (two-factor works; password managers via [HTML-in-Canvas](https://github.com/WICG/html-in-canvas) where available). Cookies checked and refreshed automatically. |
| **File layout** | Template-based, e.g. `{album_artist}/[{year} - ]{album}/{track:02} {title}`; files move on request. |
| **Streaming** | Original files with seeking, or transcoded (MP3, Opus, AAC) with FFmpeg's libraries. |
| **Subsonic API** | At `/rest`: browsing, lists, search, stars, ratings, scrobbles, play queues, playlists, lyrics, covers, similar songs. Password, token or API key auth; CORS. Tested with Feishin and Airsonic Refix. |

## Running

**Docker** (Alpine; includes the player, Chromium, FFmpeg, `yt-dlp` and Deno; runs read-only, without capabilities):

```sh
mkdir -p data treasure   # writable by uid 1000
docker compose up -d --build
```

Open <http://localhost:4533>, create the admin account, and use the same credentials in Subsonic apps.

**From source** needs:

- [rustup](https://rustup.rs/) (toolchain pinned in `rust-toolchain.toml`)
- FFmpeg 3.0–9.0 dev libraries, `clang`, `pkg-config` (Debian: `libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libclang-dev pkg-config`)
- Node.js ≥ 20.19 and [pnpm](https://pnpm.io/) 11
- Optional: Chromium (login), `yt-dlp` with [Deno](https://deno.com/) (download fallback)

```sh
(cd web && pnpm install && pnpm build)
cargo run --release
```

## Configuration

Defaults, then `pixiu.toml` (or `PIXIU_CONFIG`), then `PIXIU_` environment variables (`PIXIU_SERVER__PORT=4533`). See [`pixiu.example.toml`](pixiu.example.toml).

| Setting | Default | |
|---|---|---|
| `server.host` | `127.0.0.1` | `0.0.0.0` in Docker |
| `server.port` | `4533` | |
| `paths.data_dir` | `data` | Database, secret key, caches, staged uploads |
| `paths.treasure_dir` | `treasure` | The music library |
| `paths.web_dir` | `web/` beside the binary, else `web/dist` | The web player |
| `browser.executable` | from `PATH` | Chromium |
| `browser.no_sandbox` | `false` | Set in Docker |
| `stream.format` | `mp3` | Fallback transcode format: `mp3`, `opus`, `aac` |
| `stream.max_transcodes` | `4` | Concurrent transcodes |
| `enrich.contact` | unset | Your email or URL, sent only in the `User-Agent` of lookups |
| `hunt.botguard` | unset | [`rustypipe-botguard`](https://codeberg.org/ThetaDev/rustypipe-botguard) path (glibc only, not in Docker) |

## Development

| Command | |
|---|---|
| `cargo test --workspace` | Unit and end-to-end tests |
| `cargo test --workspace -- --ignored` | Also Chromium, YouTube Music, MusicBrainz, LRCLIB |
| `cargo clippy --workspace --all-targets` | Lints |
| `cd web && pnpm dev` | Player with live reload; proxies `/api` and `/rest` to `PIXIU_URL` (`http://127.0.0.1:4600`) |
| `pnpm typecheck` / `pnpm check` / `pnpm test` | Player types, lint and format, unit tests |
| `cargo run -p pixiu-db --features cli -- migration generate --name <change>` | New migration after editing `crates/pixiu-db/src/models.rs` (commit `crates/pixiu-db/toasty/`) |

| Crate | |
|---|---|
| `pixiu` | Binary: config, wiring, serving |
| `pixiu-api` | Web player API (`/api`) |
| `pixiu-browser` | Login browser (Chromium over CDP) |
| `pixiu-core` | Config, secrets, passwords |
| `pixiu-db` | [Toasty](https://github.com/tokio-rs/toasty) models and migrations (SQLite) |
| `pixiu-enrich` | MusicBrainz, Cover Art Archive, LRCLIB, Wikipedia |
| `pixiu-hunt` | YouTube Music ([rustypipe](https://codeberg.org/ThetaDev/rustypipe)), downloads |
| `pixiu-jobs` | Job queue, session warden |
| `pixiu-media` | Remuxing and transcoding (FFmpeg) |
| `pixiu-subsonic` | OpenSubsonic API (`/rest`) |
| `pixiu-treasury` | Library on disk: tags, layout, ingest, uploads |
| `web/` | Web player (Vue, TypeScript, Vite+) |

## Credits

The web player is a fork of [koel](https://github.com/koel/koel)'s frontend, used under the MIT License.

<details>
<summary>koel's license</summary>

> Copyright (c) 2015 Phan An
>
> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

</details>
