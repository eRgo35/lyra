# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

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
