# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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

### Security (known, unresolved)

- Dependabot #31, #21, #24, #23 (`rustls-webpki 0.102.8`): pulled in via
  `serenity 0.12.5 → tokio-tungstenite 0.21 → rustls 0.22.4`. 0.12.5 is
  the last serenity 0.12 release; the fix needs serenity's `next` branch
  (breaking, together with poise).
- Dependabot #36 (`libcrux-chacha20poly1305 0.0.7`, patched in 0.0.8):
  hard-pinned by `hpke-rs-libcrux 0.6.1 ← openmls_rust_crypto 0.5.1 ←
  davey 0.1.4 ← songbird 0.6.0`. The crate is in `Cargo.lock` but is not
  compiled for any target (`cargo tree -i` is empty).
- `native-tls` / `hyper-tls` are still compiled in: `spotify-parser
  1.0.1` depends on `reqwest` with default features. Needs a
  `spotify-parser` release with `default-features = false`.

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
