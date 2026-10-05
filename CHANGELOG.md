# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
  songbird 0.6 requires `reqwest ^0.12.2`, which transitively upgrades
  the rustls / rustls-webpki chain to the patched 0.23 / 0.103 lines.
  Closes Dependabot alerts #31 (high), #21 (medium), #24 (low) and
  #23 (low) on the `rustls-webpki` side.
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
