# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Fixed

- `/play` and `/forceplay` failing with "Unknown interaction" when yt-dlp takes
  more than 3 seconds to resolve the track.

## [0.17.1] - 2026-11-22

### Added

- The Docker image is now also published for `arm64`.

### Security

- Updated `url` to 2.5.9 (RUSTSEC-2026-0592).

## [0.17.0] - 2026-11-22

### Added

- `remind [dm] <when> <text>` (`/remind set`), `remind list` and `remind
  cancel <id>`. Reminders are kept in `data/reminders.json`, so they survive
  restarts; ones that came due while the bot was offline are delivered late
  with a note. They only ever ping their owner, and a channel reminder that
  can't be posted is sent as a DM.
- `urban <term>` (Urban Dictionary, shown behind spoilers), `userinfo [user]`
  and `serverinfo`.
- Minecraft: `mcstatus <server>` (online, MOTD, players, version via
  mcsrvstat.us) and `mcplayer <name>` (UUID, head and skin render).

## [0.16.0] - 2026-11-22

### Added

- Soundboard (#7): `soundboard add <name> <url or search>`, `remove`, `play`,
  and `show`, which posts the server's sounds as buttons that play on top of
  the queue. Sounds are stored per server in `data/soundboard/<guild>.json`
  under the working directory: a `lyra-data` volume in `compose.yaml`, or
  `/var/lib/lyra/data` under systemd.
- Music: `nowplaying` (`np`) with a progress bar, `lyrics [song]` (lrclib.net;
  the current track by default), `shuffle`, `remove <n>` (`rm`), `move <from>
  <to>` (`mv`) and `clear`. Tracks removed with `remove` or `clear` stay
  removed with `repeat all` on.

### Fixed

- `seek` shows times past an hour as `h:mm:ss` instead of `62:03`.

## [0.15.0] - 2026-11-22

### Added

- Fun commands: `8ball`, `coinflip`, `choose a | b | c`, `rps` and `mock`.
  `dice` (now also `roll`) takes dice notation like `2d20+3`. These are listed
  under a new "Fun" category in `help` (#12).
- Working `weather`, `metar` and `taf` commands (they used to reply "Work in
  progress"), plus new `whois <domain>` and `ip <address or hostname>`. All use
  free services that need no API key: Open-Meteo, aviationweather.gov and RDAP
  via rdap.org. `ip` refuses private and reserved addresses, so it can't reveal
  the bot host's network (#12).

## [0.14.0] - 2026-11-22

### Added

- Docker image (`ghcr.io/ergo35/lyra`, published on every release) with
  `yt-dlp` (self-updating on start), `deno` and `ffmpeg`, plus `compose.yaml`:
  `cp .env.example .env && docker compose up -d`. Restarts on crashes and
  reboots.
- systemd service (`deploy/systemd/lyra.service`, sandboxed, restarts on
  failure) and `deploy/systemd/install.sh`, which installs the release binary,
  a `lyra` user and `/etc/lyra/lyra.env`.

### Fixed

- `uptime` uses singular units ("1 minute") and says "0 seconds" right
  after start.

### Changed

- The bot is split into a library and a thin binary so `tests/` can use it,
  with unit, integration (wiremock) and opt-in end-to-end tests. No
  behaviour change.

## [0.13.0] - 2026-11-22

### Added

- `repeat` modes: `repeat one` loops the current track, `repeat all` loops
  the whole queue (finished and skipped tracks are enqueued again at the
  end), `repeat <n>` loops the current track n times and `repeat off` turns
  both off. The modes are exclusive; `stop` and `leave` turn `all` off, and
  `queue` shows the active mode (#11).
- `jump <position>` (`j`) skips to a queue position as numbered by `queue`,
  dropping the tracks before it (#10).
- `forceplay` (`fp`, `playnow`) plays a track immediately; the interrupted
  track is queued right after it and restarts from the beginning. Only the
  first track of a playlist is used (#5).

### Fixed

- Enqueueing playlists is much faster. YouTube playlist tracks reuse the
  title, duration, channel and thumbnail from the single `--flat-playlist`
  call instead of one `yt-dlp` run per track (a 200-track playlist went from
  over 5 minutes to about 2 seconds); Spotify and search entries are looked
  up 4 at a time, still enqueued in order (#6).
- The track error logger was registered again on every `play` and soundboard
  command; it is now registered once when the bot joins.

## [0.12.0] - 2026-11-22

### Added

- CI runs `cargo deny check advisories`. Accepted advisories are listed in
  `deny.toml` with their reasons; any new RustSec advisory fails the build.

### Fixed

- `seek` now does what it says: `90` or `1:30` jumps to that position and
  `+30` / `-15` moves relative to the current one (clamped at the start).
  It reports the new position, or an error when the target is past the end
  of the track or the track cannot be seeked. Previously it always jumped to
  the absolute second while claiming to seek "by" it, and ignored failures.
  The slash option is now text; it is re-registered on startup (globally
  this can take up to an hour to show up) (#8).
- `dictionary` shows the word, its phonetic and the first definition per part
  of speech instead of an empty embed. `dictionary` and `verse` reply with an
  error for unknown words/verses, API errors and network failures instead of
  staying silent or panicking, and reuse the shared HTTP client (#28).
- `play` no longer blocks a runtime worker while `yt-dlp` expands a
  playlist, and replies that `yt-dlp` was not found instead of panicking
  when it is missing. `ai` no longer blocks the runtime for a second (#27).
- YouTube and YouTube Music playlist links work again. The removed yt-dlp
  OAuth login is no longer passed, every entry is read from yt-dlp's JSON
  (video IDs with `-`/`_` and `music.youtube.com` links were previously
  dropped), and `play` replies with an error when no track resolves.
  Unavailable playlist entries are skipped instead of panicking (#25, #18).
- `seek`, `volume` and `repeat` reply "Nothing is playing!" instead of
  panicking when the queue is empty (#26).
- `queue`, `skip`, `play` and the soundboard commands no longer panic on
  tracks without a title, artist or source URL; they show placeholder text
  instead. `skip` on an empty queue no longer underflows (#26).

### Changed

- `dotenv` 0.15 (unmaintained, RUSTSEC-2021-0141) replaced by its maintained
  drop-in fork `dotenvy`.

### Security

- `play` text containing `?list=` could be read by `yt-dlp` as a command-line
  option; it is now always passed after `--`. Playlist entries are only
  enqueued if they are `http(s)` URLs, since songbird passes them to
  `yt-dlp` without a separator.
- `serenity` 0.12.5 is now taken from a one-line fork
  ([`eRgo35/serenity`](https://github.com/eRgo35/serenity), branch
  `lyra/tokio-tungstenite-0.23`, rev `c897f7d`) through `[patch.crates-io]`.
  The fork bumps only `tokio-tungstenite` 0.21 → 0.23.1 (same change as
  serenity-rs/serenity#3599, see #3557; upstream will not merge it into its
  frozen `current` branch). This removes `rustls` 0.22.4, `rustls-webpki`
  0.102.8 and `tokio-rustls` 0.25 from the dependency tree; `tungstenite`
  0.23 uses `rustls` 0.23 / `rustls-webpki` 0.103. Closes Dependabot alerts
  #31, #21, #24 and #23 and RUSTSEC-2026-0382 / -0346 / -0349 on the gateway
  path, i.e. those three items under "Security (known, unresolved)" in 0.11.2.
  No Lyra source change; a real `wss://gateway.discord.gg` handshake was
  verified. Remove the patch when serenity 0.13 is released.
- RUSTSEC-2026-0381 (`webpki-roots`) is resolved: 0.26.11 is a shim over
  1.0.10, which has the fix, and the advisory now lists it as unaffected.
- Still unresolved: the lock-only `libcrux` advisories, `ringbuf` 0.4.8
  (RUSTSEC-2026-0293), `crypto-bigint` 0.5.5 (RUSTSEC-2026-0387) and the
  unmaintained-crate warnings; all are accepted in `deny.toml` with reasons.

## [0.11.2] - 2026-10-20

### Added

- Rewritten `README.md`: requirements (`yt-dlp`, Message Content intent),
  quick start with invite URL, configuration table and a command reference
  verified against the source.
- SVG logo (`assets/lyra.svg`), a 512px icon (`assets/lyra-512.png`) and a
  GitHub social preview image (`assets/social-preview.png`).
- `SECURITY.md`, `CONTRIBUTING.md`, GitHub issue templates (bug report,
  feature request), a pull request template and a Dependabot config
  (cargo and github-actions, weekly, targeting `senpai`).

### Fixed

- `Release` workflow no longer hangs: regenerated with cargo-dist 0.33
  (was 0.20.0), which targets `ubuntu-22.04` instead of the retired
  `ubuntu-20.04` runner image. The v0.11.0 and v0.11.1 release runs
  were cancelled after 24h waiting for a runner, so no GitHub Release
  was published for them.
- Startup no longer panics when no `.env` file exists (variables from the
  real environment are used); an unparseable `GUILD_ID` is now a hard
  startup error instead of being silently ignored.
- Music and soundboard commands are now `guild_only`: using them in a DM
  gets poise's "guild only" rejection instead of panicking on
  `ctx.guild_id().unwrap()`.
- The process now exits with status 1 when the Discord client fails to
  start (e.g. invalid token) instead of staying alive until Ctrl-C.

### Changed

- Removed the `release-plz` workflow: its `RELEASE_PLZ_TOKEN` was
  rejected (HTTP 401) on every push, and releases are cut by hand-made
  tags built by cargo-dist. See `RELEASING.md`.
- CI now runs `cargo fmt --check` and `cargo clippy -D warnings`, uses
  `--locked`, and caches the cargo build.
- Added unit tests for the inline `owoify` implementation.
- Crate metadata: new `description`, updated `keywords` and added
  `categories` in `Cargo.toml`.
- `spotify-parser` 1.0.1 → 1.0.2 (reqwest default features disabled upstream);
  `native-tls`, `hyper-tls` and OpenSSL are no longer in the dependency tree or
  linked into the binary.

### Security

- `cargo update` (`Cargo.lock` only, no `Cargo.toml` change) bumps
  `rustls 0.23.45 → 0.23.46` (RUSTSEC-2026-0346, -0347, -0348, -0349),
  `rustls-webpki 0.103.15 → 0.103.16` (RUSTSEC-2026-0382, distrusted
  emSign intermediates) and `webpki-roots 1.0.9 → 1.0.10`
  (RUSTSEC-2026-0381). Other lock updates include `songbird 0.6.0 → 0.6.1`,
  `davey 0.1.4 → 0.1.5`, `poise 0.7.0 → 0.7.1`, `tokio 1.53.2 → 1.54.0` and
  `hyper 1.11.1 → 1.12.0` (31 packages in total).

### Security (known, unresolved)

- Dependabot #31, #21, #24, #23 (`rustls-webpki 0.102.8`, RUSTSEC-2026-0104,
  -0049, -0099, -0098) are compiled via `serenity 0.12.5 → tokio-tungstenite
  0.21 → tokio-rustls 0.25 → rustls 0.22.4`. #31 and #21 are CRL-only paths,
  and this bot never configures CRLs. #24 and #23 need a misissuing,
  name-constrained CA; the discord.com chain (GTS Root R4 → WE1) has no name
  constraints. Fix: serenity PR #3599 (tokio-tungstenite 0.23.1) via a
  `[patch.crates-io]` fork, or serenity `next` once released (breaking,
  together with poise).
- RUSTSEC-2026-0382 (distrusted emSign intermediates) also hits this
  `rustls-webpki 0.102.8` on the gateway path. RUSTSEC-2026-0381 hits
  `webpki-roots 0.26.11` (`tokio-tungstenite 0.21` in serenity, `0.26` in
  songbird). 0382 is the one with a real, if narrow, attack. No 0.26.x fix
  exists; 0381 needs `webpki-roots 1.x`.
- RUSTSEC-2026-0346 and -0349 on `rustls 0.22.4` (same serenity path): 0349
  affects only ML-KEM key exchange, which 0.22.4 lacks; 0346 needs a custom
  session store that logs values, and none exists. The same serenity fix
  removes this path.
- Dependabot #36 (`libcrux-chacha20poly1305 0.0.7`, RUSTSEC-2026-0124, patched
  in 0.0.8) is a lock-only entry and is never compiled (`cargo tree -i
  libcrux-chacha20poly1305 --target all` is empty). It enters only through
  `hpke-rs`'s weak, inactive `libcrux` feature edge (`songbird 0.6.1 → davey
  0.1.5 → openmls_rust_crypto 0.5.1 → hpke-rs 0.6.1`). It cannot be bumped
  until davey moves to openmls ≥ 0.9.
- RUSTSEC-2026-0209, -0211 (`libcrux-aesgcm 0.0.7`) and -0330, -0331
  (`libcrux-kem 0.0.7`) are on the same lock-only path and are never compiled.
- RUSTSEC-2026-0207, -0208 (`libcrux-sha3 0.0.8`, via `hpke-rs 0.6.1`) and
  -0212 (`libcrux-secrets 0.0.5`, aarch64 only) are compiled. Source review
  finds the affected functions are not called by the MLS/DAVE ciphersuites.
  Upstream only.
- RUSTSEC-2026-0293 (`ringbuf 0.4.8`, via songbird) is compiled. The
  `Consumer::skip` path is used on `HeapCons<u8>`, whose `Drop` cannot panic.
  Needs songbird on `ringbuf >= 0.5.2`.
- Unmaintained warnings: `derivative 2.2.0` and `instant 0.1.13` (songbird
  → davey → openmls, upstream), `proc-macro-error2 2.0.1` (build time, via
  libcrux), `fxhash 0.2.1` (spotify-parser → scraper 0.19.1) and `dotenv
  0.15.0` (lyra direct; replaceable with `dotenvy`).
- Details, evidence and commands: `docs/research/2026-10-20-dependency-blockers.md`.

## [0.11.1] - 2026-10-05

### Security

- Replaced the `owoify 0.1.5` direct dependency with an inline
  implementation in `src/commands/tools/owoify.rs`. `owoify 0.1.5`
  hard-pinned `rand 0.7.0` → `0.7.3`, which is flagged in Dependabot
  alert #25 (low) for an unsoundness when combined with a custom logger
  that uses `rand::rng()`. The inline implementation uses `rand 0.10`
  (already in the tree) and preserves the trait surface
  (`OwOifiable for String { fn owoify(&self) -> Self }`).
- Bumped `songbird` from 0.4 to 0.6 and `reqwest` from 0.11 to 0.12.
  songbird 0.6 requires `reqwest ^0.12.2`, which upgrades the reqwest /
  hyper-rustls side of the TLS chain to rustls 0.23 / rustls-webpki
  0.103. **Correction (see 0.11.2):** this did *not* close Dependabot
  alerts #31, #21, #24 and #23; `rustls-webpki 0.102.8` is still pulled
  in by `serenity 0.12.5`'s websocket stack (`tokio-tungstenite 0.21`).
- Dropped the `owoify` and `rustls-webpki 0.101.7` / `rustls 0.21.x`
  transitive paths from `Cargo.lock`. `ring 0.16.20` (Dependabot alert
  #10) also leaves the lockfile automatically.

### Changed

- Migrated music commands to the songbird 0.6 API surface:
  `enqueue_input(...).await.typemap().write().await.insert::<Metadata>(...)`
  is replaced by `enqueue(Track::new_with_data(input, Arc::new(meta))).await`
  + retrieval via `TrackHandle::data::<AuxMetadata>()`. The
  `src/commands/music/metadata.rs` marker type is no longer needed and
  was removed.
- `TrackHandle::typemap()` (songbird 0.4) no longer exists in songbird
  0.6; per-track user data is now attached at `Track::new_with_data`
  time and read via `TrackHandle::data::<T>()`.
- `TrackQueue::loop_for` now takes `nonmax::NonMaxU32` instead of
  `usize`; the `repeat` command wraps its argument accordingly.
- `YoutubeDl` lifetime annotations now require explicit `<'_>` in
  helper signatures in `play.rs`, `soundboard/effect.rs` and
  `soundboard/stream.rs`.

## [0.11.0] - 2026-10-05

### Changed

- Replaced `songbird` git fork (`https://github.com/eRgo35/songbird`,
  no longer reachable) with the crates.io release.
- Removed unused dependencies: `json`, `tracing-futures`.
- Bumped `poise` to 0.7 and `serenity` to the latest 0.12.x release.
- Bumped `rand` to 0.10, `tokio` to 1.x latest, `tracing` and
  `tracing-subscriber` to 0.3.x latest. `reqwest` stays at 0.11.x:
  `songbird` 0.4.x hard-pins `reqwest = "0.11"` and `songbird` 0.6
  requires `reqwest ^0.12.2` (not 0.13), so reaching reqwest 0.13
  requires either forking songbird or replacing it. Out of scope for
  this release.
- Switched `reqwest` to `rustls-tls` only (`default-features = false`,
  features = `["json", "rustls-tls"]`); the vendored `openssl`
  dependency is no longer required.
- Slash-command registration now respects `GUILD_ID` from the environment.
  When set, commands register against that guild (instant). When unset,
  commands register globally.

### Removed

- `[patch.crates-io.serenity-voice-model]` table (no longer required).
- Hard-coded user-block for ID `123456789` in `command_check`.
- Hard-coded guild ID `512680330495524873` in `register_in_guild`.

## [0.10.8](https://github.com/eRgo35/lyra/compare/v0.10.7...v0.10.8) - 2024-12-15

### Fixed

- nix minor overlay changes

## [0.10.7](https://github.com/eRgo35/lyra/compare/v0.10.6...v0.10.7) - 2024-08-16

### Other
- update Cargo.lock dependencies

## [0.10.4](https://github.com/eRgo35/lyra/compare/v0.10.3...v0.10.4) - 2024-08-13

### Fixed
- added wip info for some commands

### Other
- cargo-dist action setup
- repo added to Cargo.toml
- using cargo-dist for package building

## [0.10.3](https://github.com/eRgo35/lyra/compare/v0.10.2...v0.10.3) - 2024-08-13

### Other
- release-plz

[0.11.0]: https://github.com/eRgo35/lyra/compare/v0.10.8...v0.11.0
[0.11.1]: https://github.com/eRgo35/lyra/compare/v0.11.0...v0.11.1
[0.12.0]: https://github.com/eRgo35/lyra/compare/v0.11.2...v0.12.0
[0.13.0]: https://github.com/eRgo35/lyra/compare/v0.12.0...v0.13.0
[0.14.0]: https://github.com/eRgo35/lyra/compare/v0.13.0...v0.14.0
[0.15.0]: https://github.com/eRgo35/lyra/compare/v0.14.0...v0.15.0
[0.16.0]: https://github.com/eRgo35/lyra/compare/v0.15.0...v0.16.0
[0.17.1]: https://github.com/eRgo35/lyra/compare/v0.17.0...v0.17.1
[0.17.0]: https://github.com/eRgo35/lyra/compare/v0.16.0...v0.17.0
