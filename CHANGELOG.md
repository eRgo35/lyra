# Changelog
All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- Replaced `songbird` git fork (`https://github.com/eRgo35/songbird`,
  no longer reachable) with the crates.io release.
- Removed unused dependencies: `json`, `tracing-futures`.
- Bumped `poise` to 0.7 and `serenity` to the latest 0.12.x release.
- Bumped `rand` to 0.10 (`thread_rng()` → `rng()`; `gen_range` →
  `random_range`; `rand::Rng` → `rand::RngExt`).
- Bumped `tokio`, `tracing`, and `tracing-subscriber` to their latest
  releases.
- Switched `reqwest` to `rustls-tls` only (dropped `default-features`).
  The `reqwest` major version stays at 0.11 because `songbird` 0.4 pins
  `reqwest` 0.11; bumping `reqwest` further requires a `songbird`
  upgrade, which is out of scope for this phase.

### Removed

- `[patch.crates-io.serenity-voice-model]` table (no longer required).

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
# Changelog
