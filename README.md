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
that downloads music from YouTube Music and Deezer into a library it owns.

## Features

| | |
|---|---|
| **Web player** | Material 3, colored from the cover playing, light and dark; compact with a mouse, touch-sized on phones: library, search, queue synced across devices, likes, ratings, synced lyrics, equalizer, playlists, smart playlists, folders. |
| **Discover** | Search YouTube Music or Deezer; download songs or albums, tagged and filed with their cover. YouTube Music's Opus is remuxed losslessly, with `yt-dlp` as fallback; Deezer's songs come from [Monochrome](https://github.com/monochrome-music/monochrome), found by ISRC, and are stored as Opus. |
| **Watches** | Playlists (mirrored read-only), liked music on YouTube Music, and artists' new releases, synced one-way on a schedule; Deezer share links work too. |
| **Jobs** | Downloads, syncs and lookups, live. |
| **Uploads** | Audio files or zip archives, reviewed before they join the library. |
| **Orphans** | Songs nothing keeps any more (left a playlist, excluded, unwatched); kept or deleted by hand, never automatically. |
| **Tagging** | [MusicBrainz](https://musicbrainz.org/) lookups, Cover Art Archive covers, Wikipedia bios; pick a release when unsure; edit albums by hand. |
| **Lyrics** | From the file, [LRCLIB](https://lrclib.net) or YouTube Music; instrumentals recognized. |
| **YouTube Music login** | A real browser on the server, shown in the player (two-factor works; password managers via [HTML-in-Canvas](https://github.com/WICG/html-in-canvas) where available). Cookies checked and refreshed automatically. |
| **Accounts** | A library, YouTube Music login and settings per account; admins make accounts, turn them off, reset passwords, promote admins, and see what each library takes up on disk. |
| **Registration** | Optional: anyone may ask for an account; an admin approves (the applicant confirms their email) or denies. |
| **Email** | Over SMTP: password resets, email confirmations, and alerts (YouTube Music signed out, a watch failing), each switchable. |
| **Single sign-on** | Any OpenID Connect provider (e.g. [Authelia](https://www.authelia.com/)); people link their account there, then sign in with it. |
| **Storage** | One copy of each file in `treasure/.store`, shared by every library holding it; edits change the library, never the files. |
| **Streaming** | Original files with seeking, or transcoded (MP3, Opus, AAC) with FFmpeg's libraries. |
| **Subsonic API** | At `/rest`: browsing, lists, search, stars, ratings, scrobbles, play queues, playlists, lyrics, covers, similar songs. Password, token or API key auth; CORS. Tested with Feishin and Airsonic Refix. |

## Running

**Docker** (Alpine; includes the player, Chromium, FFmpeg, `yt-dlp` and Deno; runs read-only, without capabilities):

```sh
mkdir -p data treasure   # writable by uid 1000
docker compose up -d --build
```

Open <http://localhost:4533>, create the admin account, and use the same credentials in Subsonic apps. Then, under **Settings**:

| Tab | |
|---|---|
| **Email** | The mail server, with a test email |
| **Sign-in** | The public address (links in emails start with it), registration, single sign-on (with the redirect URI to give the provider) |
| **Users** | Accounts, and requests for one |

**From source** needs:

- [rustup](https://rustup.rs/) (toolchain pinned in `rust-toolchain.toml`)
- FFmpeg 3.0–9.0 dev libraries, `clang`, `pkg-config` (Debian: `libavcodec-dev libavformat-dev libavutil-dev libswresample-dev libclang-dev pkg-config`)
- Node.js ≥ 22.12 and [nub](https://nubjs.com/) 0.9 (it uses the `pnpm-lock.yaml`)
- Optional: Chromium (login), `yt-dlp` with [Deno](https://deno.com/) (download fallback)

```sh
(cd web && nub install && nub run build)
cargo run --release
```

## Configuration

Defaults, then `pixiu.toml` (or `PIXIU_CONFIG`), then `PIXIU_` environment variables (`PIXIU_SERVER__PORT=4533`). See [`pixiu.example.toml`](pixiu.example.toml).

| Setting | Default | |
|---|---|---|
| `server.host` | `127.0.0.1` | `0.0.0.0` in Docker |
| `server.port` | `4533` | |
| `server.trust_proxy_headers` | `false` | Read the client address from `X-Forwarded-For`; only behind a reverse proxy |
| `paths.data_dir` | `data` | Database, secret key, caches, staged uploads |
| `paths.treasure_dir` | `treasure` | The music library |
| `paths.web_dir` | `web/` beside the binary, else `web/dist` | The web player |
| `browser.executable` | from `PATH` | Chromium |
| `browser.no_sandbox` | `false` | Set in Docker |
| `browser.max_open` | `2` | Browsers open at once, for everyone's YouTube Music logins |
| `stream.format` | `mp3` | Fallback transcode format: `mp3`, `opus`, `aac` |
| `stream.max_transcodes` | `4` | Concurrent transcodes |
| `enrich.contact` | unset | Your email or URL, sent only in the `User-Agent` of lookups |
| `hunt.botguard` | unset | [`rustypipe-botguard`](https://codeberg.org/ThetaDev/rustypipe-botguard) path (glibc only, not in Docker) |
| `hunt.monochrome` | `https://tracks.monochrome.st` | [Monochrome](https://github.com/monochrome-music/monochrome)'s API, which serves Deezer's songs |

## Files

| Path | |
|---|---|
| `treasure/.store/audio/<aa>/<sha256>.<ext>` | Audio, named by content |
| `treasure/.store/images/<aa>/<sha256>.<ext>` | Covers and artist pictures |
| `data/pixiu.db` | Libraries, accounts, settings (SQLite) |
| `data/secret.key` | Seals stored passwords and secrets; keep it with the database |
| `data/users/<id>/` | Each account's login browser and YouTube Music session |

## Upgrading to multiple accounts

- Back up `data` and `treasure` first.
- The first start moves every file into `treasure/.store` (hard links, then the old paths go; resumable); files it does not know stay where they are.
- The existing account becomes the admin and keeps the library, its login browser and its YouTube Music session.
- The file layout template and "Move files" are gone; Subsonic apps see a path made up from the tags.

## Development

| Command | |
|---|---|
| `cargo test --workspace` | Unit and end-to-end tests |
| `cargo test --workspace -- --ignored` | Also Chromium, YouTube Music, Deezer, Monochrome, MusicBrainz, LRCLIB |
| `cargo clippy --workspace --all-targets` | Lints |
| `cd web && nub run dev` | Player with live reload; proxies `/api` and `/rest` to `PIXIU_URL` (`http://127.0.0.1:4600`) |
| `nub run typecheck` / `nub run check` / `nub run test` | Player types, lint and format, unit tests |
| `cargo run -p pixiu-db --features cli -- migration generate --name <change>` | New migration after editing `crates/pixiu-db/src/models.rs` (commit `crates/pixiu-db/toasty/`) |

| Crate | |
|---|---|
| `pixiu` | Binary: config, wiring, serving |
| `pixiu-accounts` | Accounts, server settings, email, alerts, registration, single sign-on |
| `pixiu-api` | Web player API (`/api`) |
| `pixiu-browser` | Login browser (Chromium over CDP) |
| `pixiu-core` | Config, secrets, passwords, alerts |
| `pixiu-db` | [Toasty](https://github.com/tokio-rs/toasty) models and migrations (SQLite) |
| `pixiu-enrich` | MusicBrainz, Cover Art Archive, LRCLIB, Wikipedia |
| `pixiu-hunt` | Downloads, from a `Source` per platform; YouTube Music's through [rustypipe](https://codeberg.org/ThetaDev/rustypipe), Deezer's through its public API and Monochrome |
| `pixiu-jobs` | Job queue, session wardens |
| `pixiu-media` | Remuxing and transcoding (FFmpeg) |
| `pixiu-subsonic` | OpenSubsonic API (`/rest`) |
| `pixiu-treasury` | The shared store, ingest, edits, uploads |
| `web/` | Web player (Vue, TypeScript, Vite+, Material 3); vue-router, Pinia, TanStack Query and Virtual, Reka UI, Workbox |

## Credits

Parts of the web player's code come from [koel](https://github.com/koel/koel), used under the MIT License.

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
