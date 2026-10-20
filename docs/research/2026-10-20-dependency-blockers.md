# Dependency blockers for the 0.11.x security alerts

Research date: 2026-10-20 (RustSec advisory-db HEAD `e6f94f3`, crates.io queried live).
Nothing in the lyra repo was changed; every experiment ran in `/tmp` scratch copies
(`cp -r` of the repo, `CARGO_TARGET_DIR` outside the repo), now deleted.
Tags: **[run]** = observed by running a command, **[src]** = read from source/advisory text,
**[inf]** = inference, not directly observed.

## Summary

| # | Item | Reachable at runtime? | Fix path | Effort | Recommendation |
|---|------|----------------------|----------|--------|----------------|
| 0 | **Not in the changelog:** plain `cargo update` (lock only) | – | clears rustls 0.23.45 (4 advisories), rustls-webpki 0.103.15 (RUSTSEC-2026-0382), webpki-roots 1.0.9 (RUSTSEC-2026-0381) | minutes; 33 lock entries; `cargo check` + 5 tests pass **[run]** | **Do first.** |
| 1 | Dependabot #31, #21, #24, #23 (`rustls-webpki 0.102.8`) | #31 and #21: **no** (CRL only, bot never configures CRLs). #24, #23: **no** in practice (needs a misissuing name-constrained CA). But the same crate now carries RUSTSEC-2026-0382 (distrusted emSign intermediates), which **is** on the gateway TLS path. | `[patch.crates-io] serenity = <fork with one-line commit of serenity PR #3599>`: tokio-tungstenite 0.21 → 0.23.1, removes rustls 0.22 / webpki 0.102 / tokio-rustls 0.25. Zero source changes; builds; tests pass; real gateway handshake OK **[run]**. Full migration to serenity `next` also works but touches 18 files / 34 hunks **[run]**. | patch: ~30 min. `next`: 0.5–1 day + runtime testing; depends on unreleased branches. | Patch via our own fork now. Do not dismiss. Move to serenity 0.13 / poise 0.8 when released. |
| 2 | Dependabot #36 (`libcrux-chacha20poly1305 0.0.7`) | **No.** Lock-only entry, never compiled **[run]** | Blocked: `hpke-rs-libcrux 0.6.1` pins `libcrux-aead =0.0.7`. Needs davey → openmls ≥ 0.9 → hpke-rs ≥ 0.7. davey 0.1.5 (2026-10-12) still uses openmls 0.8. | – (upstream) | Dismiss, reason "Vulnerable code is not actually used" (wording below). Expect more lock-only libcrux alerts. |
| 3 | `native-tls` / `hyper-tls` / `openssl-sys` compiled | Yes, compiled in; used by `spotify-parser`'s default-TLS reqwest build (its code calls `use_rustls_tls()`, so the OpenSSL code is dead weight, but it is linked) | spotify-parser 1.0.2 with `default-features = false` on reqwest. One line. Removes native-tls, hyper-tls, openssl, openssl-sys, openssl-macros, tokio-native-tls, system-configuration(-sys), vcpkg, windows-registry from the lock **[run]**. Nothing else in the tree pulls them. | 15 min + publish | Do it (maintainer owns the crate). |
| 4 | rand 0.8.8 (lock) | n/a | none needed: RUSTSEC-2026-0097 patched in `>= 0.8.6`, so 0.8.8 is not affected **[src]**. No open alert for it. | 0 | Ignore. |
| 5 | Other advisories not in the 0.11.2 changelog | see E | see E | | add to changelog |

---

## A. serenity / poise `next`

### State of the branches (all **[run]** via `gh api` / crates.io API)

* crates.io `serenity`: latest **0.12.5** (2025-12-20). No 0.13 or pre-release exists (0.12.0-rc/rc2 are the only pre-releases ever).
* GitHub `serenity-rs/serenity`: default branch `current`, plus `next`.
  * `current` @ `db6c7e18` (2026-05-13): `tokio-tungstenite = "0.21.0"`, frozen. Maintainer on PR #3532: "Except for bug fixes, the `current` branch is frozen and will not be seeing another release. tokio-tungstenite has already been bumped in the `next` branch."
  * `next` @ `d51ce700` (2026-10-20 18:30, i.e. active today; 7 commits in the last month, 16 open PRs): `tokio-tungstenite = "0.28.0"`, `reqwest = ">=0.13"`, feature `rustls_backend = ["reqwest/rustls", "tokio-tungstenite/rustls-tls-native-roots"]`. Cargo.toml still says `version = "0.12.5"`. No release milestone/date.
* poise: crates.io 0.7.1 (2026-10-12) still depends on serenity 0.12.5. Branch **`serenity-next`** (`a1d0e3ee`, 2026-09-29, `version = "0.7.0"`, `rust-version = "1.95.0"`) depends on `serenity = { git, branch = "next" }`. poise's `next`/`current` branches are still on serenity 0.12.
* songbird: crates.io 0.6.1 (2026-10-10) depends on `serenity ^0.12`. Branch **`serenity-next`** (`3fe7289`, 2026-04-05) depends on serenity `next`. It removes the `SerenityInit` / `songbird::get` helpers; you now create `Songbird::serenity()` yourself and call `ClientBuilder::voice_manager`.
* Upstream discussion: issue #3557 "Multiple vulnerabilities with the current version of tokio-tungstenite" (open). PR **#3599** "Update tokio-tungstenite to 0.23.1" (open, base `current`, from `slowchop/serenity` branch `codex/rustls-0.23`, head `72d99a7a`, one-line change in `Cargo.toml`: `tokio-tungstenite = { version = "0.23.1", features = ["url"] }`). A maintainer replied "`tungstenite` errors are exposed in serenity's public API so this cannot be done without a breaking change release. `next` already uses newer versions." So #3599 will probably never be merged.
* No serenity feature flag selects another tokio-tungstenite; the version is hard-coded in Cargo.toml. **[src]**

### Why this is the only blocker

`cargo tree -i rustls-webpki@0.102.8 --target all` **[run]**:

```
rustls-webpki v0.102.8
└── rustls v0.22.4
    ├── tokio-rustls v0.25.0
    │   └── tokio-tungstenite v0.21.0
    │       └── serenity v0.12.5 (← lyra, poise, songbird)
```

songbird already uses tokio-tungstenite 0.26 and reqwest uses rustls 0.23 / webpki 0.103.x; serenity's gateway is the single user of rustls 0.22.

### Experiment 1 (recommended): serenity PR #3599 as `[patch.crates-io]`

Scratch copy of lyra, original `Cargo.lock`, appended:

```toml
[patch.crates-io]
serenity = { git = "https://github.com/slowchop/serenity", rev = "72d99a7a477ac17618585dd306e0687b64d0070b" }
```

Results **[run]**:

* `cargo update -p serenity`: removes `rustls 0.22.4`, `rustls-webpki 0.102.8`, `tokio-rustls 0.25.0`; `tokio-tungstenite 0.21.0 → 0.23.1`, `tungstenite 0.21.0 → 0.23.0`.
* `cargo tree -i rustls-webpki` then shows **only 0.103.x**.
* `cargo build`, `cargo test` (5 passed), `cargo fmt --check`, `cargo clippy -- -D warnings`: all clean, **no lyra source change**. (poise 0.7.0 and songbird 0.6.0 are fine because the patched crate keeps version 0.12.5.)
* rustls crypto provider: only `ring` is enabled in the tree (no aws-lc-rs), so rustls 0.23 auto-selects it, same as today.
* Smoke test with a bogus token: HTTPS REST to `discord.com` reaches the API and gets `401` as expected.
* Real gateway TLS: a throwaway example using `tokio-tungstenite 0.23.1` with `rustls-tls-webpki-roots` (the same feature serenity enables) connected to `wss://gateway.discord.gg/?v=10&encoding=json`: `HTTP status 101 Switching Protocols`, first frame `{"op":10,"d":{"heartbeat_interval":41250,...}}`. So rustls 0.23.46 + webpki 0.103.16 + webpki-roots 0.26.11 verify Discord's chain (leaf `discord.gg` ← GTS `WE1` ← GTS Root R4 ← GlobalSign Root CA, from `openssl s_client`).
* The pinned commit is `current` (11 commits after v0.12.5, by title: Discord docs links #3486/#3491, a reqwest-0.13 feature rename #3492 and its revert, clippy #3504/#3549, doc fixes #3520/#3521/#3524/#3527, CONTRIBUTING #3543) + the one `Cargo.toml` line (`gh api .../compare/current...slowchop:72d99a7a` → ahead 1, files `["Cargo.toml"]`) **[run]**. Content of those 11 commits is documentation/lint by their titles **[inf]**.

Caveats: a git dependency on a personal fork of a third party is a supply-chain decision. Create the equivalent commit on a fork we control (`eRgo35/serenity`, branch from tag `v0.12.5` or `current`, change that one line, `features = ["url"]` is required because tungstenite 0.23 made `url` optional) and pin by `rev`. `serenity::Error::Tungstenite` changes type (only matters if lyra matched on it; it does not **[run]** compiled unchanged). Not verified: the cargo-dist release build in CI with a git dependency [inf: works, CI has network].
After the patch, Dependabot reads the new `Cargo.lock` and closes #31, #21, #24, #23 itself.

### Experiment 2: full migration to serenity `next` + poise `serenity-next` + songbird `serenity-next`

Scratch `Cargo.toml`: `poise`, `songbird`, `serenity` as git deps on those three branches (and `cargo update`). Resolution works; `cargo tree -i rustls-webpki` shows only `0.103.16` **[run]**. Then I ported the code until `cargo check` was clean (`cargo build` OK; also a bogus-token smoke run reaches `discord.com` over TLS and gets 401 **[run]**; the gateway websocket and all commands are untested, there is no real token). Diff vs original `src/`: **18 files, 34 hunks, +93/−156 lines** **[run]**. Breaking changes hit, in the order the compiler reported them:

1. `Cargo.toml`: serenity feature `standard_framework` no longer exists (dropped).
2. `songbird::SerenityInit` / `.register_songbird()` / `songbird::get(ctx)` are gone: **15 call sites** (every music/soundboard command) now use an `Arc<Songbird>` stored in `Data` + `ClientBuilder::voice_manager`.
3. serenity `TypeMap` (`TypeMapKey`, `.type_map_insert`, `ctx.data.read()`) is gone: `src/http.rs` deleted, 2 call sites, `Data` carries the reqwest client.
4. poise: `Framework::builder().setup(...)` and `FrameworkError::Setup` removed; user data is passed via `ClientBuilder::data(Arc<Data>)`; command registration and `set_activity` had to move into a custom `serenity::EventHandler` (`FullEvent::Ready`); `FrameworkOptions::event_handler` removed. `main.rs` is substantially rewritten.
5. `ClientBuilder::new` takes a `Token` (parse the string).
6. Builder lifetimes: `CreateEmbed` is now `CreateEmbed<'a>`; helper fns returning `CreateEmbed` needed `'static` + owned strings (5 functions, ~10 lines), `skip.rs` needed clones.
7. `CreateEmbed::thumbnail` / `image` take a second arg (`Option<alt text>`): 5 call sites.
8. poise: `usize` is no longer a valid command parameter type and `#[rest]` is only valid for string-like types: `repeat` changed to `u32` without `#[rest]` (behaviour change for prefix command).

Side effects of the `next` stack: serenity then uses reqwest 0.13 (second reqwest version in the tree, songbird still on 0.12), pulls `aws-lc-rs`/`aws-lc-sys` (C build; the flake devshell already lists cmake/openssl) next to `ring`, and switches serenity to the OS root store (`rustls-tls-native-roots`, `rustls-platform-verifier`) so deployment images need `ca-certificates`. `webpki-roots 0.26.11` still remains via songbird's tokio-tungstenite 0.26 (see E). Rust ≥ 1.95 is required by the poise branch (toolchain here is 1.98).
Recommendation: do **not** adopt branches that have no release; do it when serenity 0.13 / poise 0.8 / songbird ≥ 0.7 ship (the migration diff above is the shopping list).

---

## B. Is the webpki 0.102.8 path reachable?

Path: serenity gateway → `tokio-tungstenite 0.21.0` `src/tls.rs`: `ClientConfig::builder().with_root_certificates(root_store).with_no_client_auth()` with `webpki_roots::TLS_SERVER_ROOTS` **[src]**. In rustls 0.22.4 `with_root_certificates` builds `WebPkiServerVerifier::new_without_revocation(root_store, …)` (`src/client/builder.rs:41-42`) **[src]**. A grep for `crl`/`Revocation` in tungstenite 0.21, tokio-tungstenite 0.21 and serenity 0.12.5 sources returns nothing **[run]**. So CRLs are never loaded and `RevocationOptions` is never passed. Discord's chain has no name constraints visible in the `openssl s_client` output (GTS Root R4 → WE1 → leaf) **[run, inf for constraints]**. REST calls (`discord.com`) go through reqwest → rustls 0.23.45 / webpki 0.103.15, not the vulnerable crate.

| Advisory | Alert | Vulnerable code | Triggered by a client connecting to discord.com with default config? |
|---|---|---|---|
| GHSA-82j2-j2ch-gfr8 / RUSTSEC-2026-0104 (patched ≥ 0.103.13) | #31 high | `BorrowedCertRevocationList::from_der` panics on malformed CRL BIT STRING | **No.** Advisory: "CRL checking is opt-in … The default rustls configuration (no `RevocationOptions`) is not affected." No CRL is ever parsed. |
| GHSA-pwjx-qhcg-rvj4 / RUSTSEC-2026-0049 (patched ≥ 0.103.10; 0.102.x listed as vulnerable) | #21 medium | CRL `distributionPoint` matching | **No.** Only affects revocation checking; none configured. |
| GHSA-xgp8-3hg3-c2mh / RUSTSEC-2026-0099 (≥ 0.103.12) | #24 low | DNS name constraints accepted for wildcard names | **Not realistically.** Advisory: "reachable only after signature verification and requires misissuance to exploit"; needs a trusted, name-constrained CA to issue a certificate it should not, plus an on-path attacker. Code runs, but no exploit precondition holds for GTS roots. |
| GHSA-965h-392x-2mh5 / RUSTSEC-2026-0098 (≥ 0.103.12) | #23 low | URI name constraints ignored | **Not realistically**, same reasoning. |
| RUSTSEC-2026-0382 (new, 2026-10-20; patched ≥ 0.103.16) | no alert yet | accepts paths with distrusted emSign intermediate keys; their private keys are "now well known" | **Yes in principle**: webpki-roots 0.26.11 (what the gateway uses) contains the affected emSign root (RUSTSEC-2026-0381, patched only in webpki-roots 1.0.10, no 0.26.x patch). Requires an on-path attacker (bot → gateway) holding a certificate from those intermediates. This is the one advisory on this path with a real, if narrow, attack. |
| rustls 0.22.4 RUSTSEC-2026-0349 / 0346 (patched 0.23.46) | no alert yet | key-share downgrade; `Debug` of session values leaks secrets | 0349 harms only post-quantum key-exchange (`X25519MLKEM768`); rustls 0.22.4 has no ML-KEM at all (`grep -rli mlkem rustls-0.22.4/src` empty **[run]**). 0346 needs a custom `ClientSessionStore` that logs values; none exists. **No.** |

### Recommendation

Preferred: **fix, do not dismiss** (Experiment 1 above): it is one patched line, it closes all four alerts for real, and it also removes the one advisory (0382) that is genuinely reachable. If the patch is not adopted, dismiss in Dependabot with these exact reasons:

* #31 and #21: reason **"Vulnerable code is not actually used"**, comment:
  `Reachable only through CRL parsing/revocation checking (RevocationOptions / WebPkiServerVerifier::with_crls). Lyra's only user of this crate version is serenity 0.12.5 -> tokio-tungstenite 0.21 -> rustls 0.22.4, which builds ClientConfig::with_root_certificates() = WebPkiServerVerifier::new_without_revocation; no CRL is ever loaded. Fix requires serenity's unreleased 'next' branch (current is frozen; upstream PR #3599 unmerged).`
* #24 and #23: reason **"Risk is tolerable to this project"** (the code is used; exploitation needs a name-constrained trusted CA to misissue), comment:
  `Name-constraint handling is only exercised when a trusted CA in the chain carries nameConstraints and misissues; Lyra only connects to discord.com (GTS Root R4 -> WE1) so the precondition cannot occur. Blocked on serenity 0.12 -> tokio-tungstenite 0.21 -> rustls 0.22.`
  Dismissals would not cover RUSTSEC-2026-0382, which is the reason to prefer the patch.

---

## C. libcrux

* Chain **[run]** (`cargo metadata`, `cargo tree`): `songbird 0.6.0 → davey 0.1.4 → openmls_rust_crypto 0.5.1 → hpke-rs 0.6.1`. openmls_rust_crypto depends on `hpke-rs` with features `hazmat, serialization` only; it depends on **`hpke-rs-rust-crypto`** (RustCrypto backend), not on `hpke-rs-libcrux` **[src]**.
* Why the crate is in `Cargo.lock`: `hpke-rs 0.6.1` declares `libcrux = ["dep:hpke-rs-libcrux"]` (optional, off) but also lists weak feature edges (`std = [..., "hpke-rs-libcrux?/std"]`, `hpke-test-prng = ["hpke-rs-libcrux?/deterministic-prng"]`). Cargo records such optional deps in the lock/metadata resolve graph even though they are never activated: `cargo metadata` shows `hpke-rs` features `[hazmat, serde, serialization, std, tls_codec]` (no `libcrux`), yet `hpke_rs_libcrux`, `libcrux-aead`, `libcrux-aesgcm`, `libcrux-chacha20poly1305`, `libcrux-poly1305` appear in the resolve. **[run]** The same mechanism gives `openssl-probe` (via reqwest's weak `rustls-native-certs`) a lock entry **[inf]**.
* Never compiled **[run]**: `cargo tree -i libcrux-chacha20poly1305 --target all` → "nothing to print", same for `hpke-rs-libcrux`, `libcrux-aead`, `libcrux-aesgcm`, `libcrux-kem`, `libcrux-ecdh`, `libcrux-hkdf`, `libcrux-poly1305`. `cargo build -v --locked` log (511 rustc invocations): zero `libcrux_chacha20poly1305`, `libcrux_aead`, `libcrux_aesgcm`, `libcrux_poly1305`, `hpke_rs_libcrux` crates. Compiled libcrux crates are only `libcrux_intrinsics, libcrux_platform, libcrux_secrets, libcrux_sha3, libcrux_traits`. Dependabot cannot see this because it parses `Cargo.lock`; `cargo deny` does not report the lock-only libcrux crates (its graph excludes them), `cargo audit` does.
* Fix availability **[run]** (crates.io API): `hpke-rs-libcrux` 0.6.1 requires `libcrux-aead = "0.0.7"` (for 0.0.x that is exactly 0.0.7). No hpke-rs 0.6.2. Newer: hpke-rs / hpke-rs-libcrux 0.7.0 (2026-07-15, aead 0.0.9) and 0.8.0 (2026-10-07, aead 0.0.10); openmls_rust_crypto 0.6.0 (2026-08-25) uses `hpke-rs ^0.7`; openmls 0.9.1 uses `openmls_rust_crypto ^0.6`. But **davey 0.1.5 (2026-10-12) and davey `main` still use `openmls 0.8.1` + `openmls_rust_crypto 0.5.1`**, and songbird 0.6.1 / `serenity-next` still take `davey ^0.1.2`. `cargo update -p songbird -p davey -p openmls` does not change this (done as part of a full `cargo update` **[run]**: songbird 0.6.1, davey 0.1.5, openmls 0.8.2, hpke-rs stays 0.6.1).
* Hack tested **[run]**: `[patch.crates-io] hpke-rs-libcrux = { path = <copy with libcrux-aead = "0.0.8"> }` does resolve (libcrux-aead/chacha20poly1305 → 0.0.8) and clears #36, but `cargo audit` then still flags `libcrux-aesgcm 0.0.8` (RUSTSEC-2026-0209/0210/0211), `libcrux-kem 0.0.7` (0330, 0331), `libcrux-secrets`, `libcrux-sha3`. Whack-a-mole on unused crates, plus a vendored crate in the repo; not worth it.
* Compiled libcrux crates that do have advisories **[run]**: `libcrux-sha3 0.0.8` (RUSTSEC-2026-0207, -0208; patched 0.0.10) and `libcrux-secrets 0.0.5` (-0212, aarch64 only; patched 0.0.6). `hpke-rs 0.6.1` pins `libcrux-sha3 =0.0.8`, and libcrux-traits 0.0.6 pins secrets 0.0.5 (`cargo update -p libcrux-secrets` is a no-op). Reachability **[src]**: 0207's affected functions are `portable::incremental::Shake{128,256}Xof::squeeze`, 0208's is `avx2::x4::shake256`; hpke-rs 0.6.1 only calls the one-shot `libcrux_sha3::shake256` and only in `derive_key_pair` for X-Wing / ML-KEM, never for the P-256/Curve25519 ciphersuites MLS/DAVE uses [inf]. 0212 is aarch64-only; the dist target is `x86_64-unknown-linux-gnu`.

### Recommendation

Dismiss #36 with reason **"Vulnerable code is not actually used"**, comment:
`libcrux-chacha20poly1305 0.0.7 is a Cargo.lock-only entry. It is reachable only through hpke-rs's optional 'libcrux' feature (hpke-rs-libcrux), which openmls_rust_crypto 0.5.1 (davey 0.1.4/0.1.5 <- songbird) does not enable; it uses hpke-rs-rust-crypto. 'cargo tree -i libcrux-chacha20poly1305 --target all' is empty and a verbose build compiles no libcrux-aead/chacha20poly1305/hpke-rs-libcrux crate. The advisory also needs an attacker-controlled ciphertext buffer length in the encrypt API, which no code here calls. Cannot be bumped: hpke-rs-libcrux 0.6.1 pins libcrux-aead =0.0.7; needs davey on openmls >= 0.9.`
Ask upstream (Snazzah/davey) to move to openmls 0.9 / openmls_rust_crypto 0.6; that is the real fix for the whole libcrux family. Do not file it from here.

---

## D. spotify-parser

* Registry copy and `github.com/eRgo35/spotify-parser` HEAD (`senpai`, `dc1dfbf` "chore: bump", 2024-08-12) have the same `Cargo.toml`: `reqwest = { version = "0.12.5", features = ["blocking", "rustls-tls"] }` — default features on, so `default-tls` (native-tls) is enabled too. Only version on crates.io: 1.0.1; owner `eRgo35`. Tags are `1.0.0`, `1.0.1` (no `v` prefix). No CI/workflows, no `tests/` or `examples/`; 4 unit tests in `src/lib.rs`, all hitting live Spotify. Code uses `.use_rustls_tls()` on both the blocking and async clients (`src/retriever.rs`), so rustls was always the intended backend; lyra calls `retrieve_async_url` only.
* Minimal change:

  ```diff
  -reqwest = { version = "0.12.5", features = ["blocking", "rustls-tls"] }
  +reqwest = { version = "0.12.5", default-features = false, features = ["blocking", "rustls-tls"] }
  ```

  Dropped defaults are `default-tls`, `charset`, `http2`, `macos-system-configuration`. `charset` only matters for non-UTF-8 `response.text()` (Spotify embed pages are UTF-8) and `http2` only for negotiation (HTTP/1.1 still works); if you want to keep them add `"charset", "http2"` to the feature list, still no native-tls.
* Verified in `/tmp/spotify-parser-research` **[run]**: `cargo build` OK; `cargo test`: **identical before and after the edit: 3 passed, 1 failed (`check_playlist`)**. The failure is pre-existing live-data drift (playlist contents changed; also the scraped artist list contains `&nbsp;`, e.g. `"New Kids On The Block,&nbsp;Salt-N-Pepa,&nbsp;Rick Astley,&nbsp;En Vogue - Bring Back The Time"`, i.e. HTML entities are not decoded; this is a lyra-visible query-quality bug on playlist imports, independent of TLS). `cargo tree -i native-tls` in the crate: "did not match any packages". `cargo publish --dry-run --allow-dirty`: packaging + verify build OK (for 1.0.1; a version bump is needed to publish).
* 1.0.2 release steps (maintainer):
  1. Apply the diff, bump `version = "1.0.2"`, `cargo build` (updates `Cargo.lock`), optionally fix/relax the live-data playlist test and decode HTML entities.
  2. `git commit -am "fix: disable reqwest default features (drop native-tls/openssl)"`, `git tag 1.0.2`, `git push origin senpai 1.0.2`.
  3. `cargo publish` (token for `eRgo35`).
  4. In lyra: `spotify-parser = "1.0.2"`, `cargo update -p spotify-parser`, update CHANGELOG.
* Proof in lyra (scratch, original lock, `[patch.crates-io] spotify-parser = { path = "/tmp/spotify-parser-research" }`) **[run]**:
  * Before: `cargo tree -i native-tls --target all` → `native-tls 0.2.18 ← hyper-tls 0.6.0 ← reqwest 0.12.28 ← {lyra, serenity, songbird, spotify-parser, stream_lib}` and `← tokio-native-tls`; `cargo tree -i openssl-sys --target all` → `openssl-sys ← native-tls ← …` (same set).
  * After `cargo update -p spotify-parser`: lock removes `hyper-tls, native-tls, openssl, openssl-macros, openssl-sys, tokio-native-tls, system-configuration, system-configuration-sys, vcpkg, windows-registry`; `cargo tree -i native-tls|openssl|openssl-sys|hyper-tls --target all` → "did not match any packages". So **native-tls disappears entirely**; serenity, songbird, stream_lib were only pulling it via reqwest's default features unified with spotify-parser's. (Combined with the serenity patch: `cargo build`, 5 tests pass, clippy/fmt clean.)

---

## E. cargo audit / cargo deny against the current `Cargo.lock`

`cargo audit --db /tmp/advisory-db --no-fetch` (cargo-audit 0.22.2, 1348 advisories): **23 vulnerabilities + 6 warnings**. `cargo deny check advisories` (0.20.2, no config) reports the same set except the lock-only libcrux crates (aesgcm 0209/0210/0211/0210-warning, chacha20poly1305 0124, kem 0330/0331), which it omits as unused. All **[run]**.

"Changelog" = whether the 0.11.2 Security section mentions it. `P` = patched by plain `cargo update`.

| Advisory | Crate | Path | Compiled? | Changelog | Fix |
|---|---|---|---|---|---|
| RUSTSEC-2026-0104, -0099, -0098, -0049 (+ **-0382 new**) | rustls-webpki 0.102.8 | serenity → tokio-tungstenite 0.21 → tokio-rustls 0.25 → rustls 0.22.4 | yes | 4 of 5 (#31/#21/#24/#23); **0382 not** | serenity patch (A) |
| RUSTSEC-2026-0346, -0349 | rustls 0.22.4 | same | yes | **no** | serenity patch (A) |
| RUSTSEC-2026-0346, -0347, -0348, -0349 | rustls 0.23.45 | reqwest 0.12.28 / hyper-rustls, tokio-tungstenite 0.26 (songbird) | yes | **no** | **P** (→ 0.23.46) |
| RUSTSEC-2026-0382 | rustls-webpki 0.103.15 | same as above | yes | **no** | **P** (→ 0.103.16) |
| RUSTSEC-2026-0381 | webpki-roots 1.0.9 | reqwest → hyper-rustls | yes | **no** | **P** (→ 1.0.10) |
| RUSTSEC-2026-0381 | webpki-roots 0.26.11 | tokio-tungstenite 0.21 (serenity), 0.26 (songbird) | yes | **no** | no 0.26.x patch exists. Serenity patch moves it to tungstenite 0.23.1, still 0.26.11, but webpki ≥ 0.103.16 rejects the distrusted intermediates, which mitigates it **[src]**; the audit line stays until songbird's tokio-tungstenite moves to ≥ 0.27 (webpki-roots 1.x). |
| RUSTSEC-2026-0124 (#36), -0209, -0210 (unmaintained, renamed `libcrux-aes`), -0211 | libcrux-chacha20poly1305 / libcrux-aesgcm 0.0.7 | songbird → davey → openmls_rust_crypto → hpke-rs → (hpke-rs-libcrux, weak) | **no** | only #36 | dismiss (C) |
| RUSTSEC-2026-0330, -0331 | libcrux-kem 0.0.7 | same | **no** | **no** | dismiss (C) |
| RUSTSEC-2026-0207, -0208 | libcrux-sha3 0.0.8 | hpke-rs 0.6.1 → libcrux-sha3 | yes, but affected fns not called [src] | **no** | needs hpke-rs ≥ 0.8 (davey) |
| RUSTSEC-2026-0212 | libcrux-secrets 0.0.5 | libcrux-traits 0.0.6 → libcrux-sha3 | yes; aarch64-only code | **no** | upstream |
| RUSTSEC-2026-0293 | ringbuf 0.4.8 | songbird → ringbuf | yes; `Consumer::skip` is used (`async_adapter.rs`) but on `HeapCons<u8>`, element `Drop` cannot panic [src] | **no** | needs songbird on ringbuf ≥ 0.5.2 |
| RUSTSEC-2024-0388 unmaintained | derivative 2.2.0 | songbird | yes | no | upstream |
| RUSTSEC-2021-0141 unmaintained | dotenv 0.15.0 | lyra direct | yes | no | swap to `dotenvy` (drop-in `dotenvy::dotenv()`), our own code |
| RUSTSEC-2025-0057 unmaintained | fxhash 0.2.1 | spotify-parser → scraper 0.19.1 → selectors | yes | no | spotify-parser: bump `scraper` |
| RUSTSEC-2024-0384 unmaintained | instant 0.1.13 | songbird → davey → openmls → fluvio-wasm-timer → parking_lot 0.11 | yes | no | upstream |
| RUSTSEC-2026-0173 unmaintained | proc-macro-error2 2.0.1 | hax-lib-macros → … → libcrux-intrinsics → libcrux-sha3 | yes (proc-macro, build time) | no | upstream |

Effect of the steps on `cargo audit` **[run]**, each in an isolated scratch copy:

* baseline: 29 distinct crate/version/ID lines (23 vulnerabilities + 6 warnings). Plain `cargo update`: 22 remain (all rustls 0.23.x, webpki 0.103.x, webpki-roots 1.x gone; webpki 0.102.8 ×5, rustls 0.22.4 ×2, webpki-roots 0.26.11, libcrux ×9, ringbuf, 5 unmaintained remain).
* `cargo update`-equivalent bumps + serenity patch + spotify-parser patch: 16 lines; only webpki-roots 0.26.11 (0381), the libcrux set (lock-only ones plus compiled sha3/secrets), ringbuf, derivative, dotenv, fxhash, instant, proc-macro-error2 remain. No rustls or rustls-webpki advisory is left. `cargo deny` shows 10 advisories (0293, 0207, 0208, 0212, 0381 and the 5 unmaintained).

## Priority list

1. `cargo update` (lock only), changelog entry. 2. Release spotify-parser 1.0.2, bump in lyra. 3. Serenity one-line patch on a fork we control (A, Exp. 1). 4. Dismiss #36 as not used (C); optionally `dotenv` → `dotenvy`. 5. Track serenity 0.13 / poise 0.8 / songbird 0.7 and davey ≥ 0.2 (openmls 0.9) for the rest.
