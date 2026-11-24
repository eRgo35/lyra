# Faster track start and audio cache Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** One yt-dlp run per new track (zero for cached ones), optional `available_at` wait, and a 512M disk cache that plays while it downloads.

**Architecture:** `http::Ytdl` resolves once (`yt-dlp -j`) into metadata plus stream info and carries both. On start it plays from `audio_cache` if the video is cached or downloading, otherwise downloads into the cache while a growing-file reader feeds songbird. With the cache off, or no video ID, or HLS, it streams through songbird's `HttpRequest` as today.

**Tech Stack:** Rust, songbird 0.6.1, symphonia `MediaSource`, reqwest 0.12, tokio, wiremock (dev).

**Spec:** `docs/superpowers/specs/2026-11-24-audio-cache-design.md`

## Global Constraints

- No new dependencies.
- `WAIT_FOR_STREAM`: off by default; `true`/`1` enables the `available_at` wait (+1 s margin, max 30 s).
- `AUDIO_CACHE_SIZE`: default `512M`; bytes or `K`/`M`/`G` (binary); `0` disables; invalid → warn, default.
- Cache dir `data/audio-cache`; files `<id>.<ext>`, in-progress `<id>.<ext>.part`.
- A 403 (or, on the `HttpRequest` path, any first-fetch error) is retried once after `wait_for(available_at)`; a carried URL is re-resolved before the retry.
- Carried URL reused only if its `expire` query param is > 60 s away.
- CI gates: `cargo fmt --check`, `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`.

## Review Focus

- Track skipped mid-download: the download keeps going and completes into the cache (Task 2 test `download_completes_after_reader_dropped`).
- Same video started twice while downloading (loop, forceplay requeue): second reader attaches to the same download, no second GET (Task 3 test `second_start_attaches_to_running_download`).
- Download fails midway: reader errors, `.part` removed, nothing indexed (Task 2 test `failed_download_errors_reader_and_removes_part`).
- Restart with leftover `.part` files: deleted on open, not served (Task 2 test `open_deletes_part_files_and_indexes_complete`).
- Track bigger than the whole limit: plays, then not kept (Task 2 test `oversized_file_is_not_kept`).

---

### Task 1: Resolve once, carry metadata and stream info; `WAIT_FOR_STREAM`

**Files:**
- Modify: `src/http.rs`

**Interfaces:**
- Produces: `pub async fn http::lookup(client: HttpClient, query: impl Into<String>) -> Result<Ytdl, AudioStreamError>` (one yt-dlp run; result has known metadata and carried stream info); `Ytdl::metadata(&self) -> Option<&AuxMetadata>`; `Ytdl::with_metadata` (exists); `fn url_fresh(url: &str, now: SystemTime) -> bool`; `fn wait_enabled(v: Option<&str>) -> bool`; `Resolved` gains `ext: Option<String>`, `Clone`.
- `Ytdl` drops the songbird `YoutubeDl` field; `aux_metadata()` returns known metadata or runs `resolve()` and stores both.

- [ ] **Step 1: Tests** in `http.rs` tests:
  - `resolve_json_yields_metadata_and_stream`: parse a fixture line `{"url":"https://a/b?expire=2000","webpage_url":"https://www.youtube.com/watch?v=abc","title":"T","uploader":"U","duration":61.0,"ext":"webm","protocol":"https","filesize":10}` with `fn parse_line(&str) -> Result<(AuxMetadata, Resolved), AudioStreamError>`; assert title `T`, artist `U`, duration 61 s, source_url, `ext == Some("webm")`.
  - `url_fresh_uses_expire`: now = 1000 s; `expire=1061` → true, `expire=1060` → false, no `expire` → false.
  - `wait_flag_off_by_default`: `wait_enabled(None)`, `Some("")`, `Some("false")` → false; `Some("true")`, `Some("1")` → true.
- [ ] **Step 2:** `cargo test --lib http::` → fails (missing items).
- [ ] **Step 3:** Implement. `parse_line` deserializes the line into both `songbird::input::YoutubeDlOutput` (`.as_aux_metadata()`) and `Resolved`. `static WAIT_FOR_STREAM: LazyLock<bool>`. Keep the existing `wait_for` and its test.
- [ ] **Step 4:** `cargo test --lib http::` → pass.
- [ ] **Step 5:** Commit `feat(music): resolve tracks once and make the availability wait optional`.

### Task 2: `audio_cache` module

**Files:**
- Create: `src/audio_cache.rs`; Modify: `src/lib.rs` (`pub mod audio_cache;`, `pub const AUDIO_CACHE_DIR: &str = "data/audio-cache";`)

**Interfaces:**
- Produces:
  - `pub fn parse_size(v: Option<&str>) -> u64` (default 512 MiB, `0` → 0)
  - `pub fn video_id(url: &str) -> Option<String>` (`watch?v=`, `youtu.be/`, `/shorts/`, any `*.youtube.com`; ID `[A-Za-z0-9_-]{11}`)
  - `pub struct AudioCache`; `AudioCache::open(dir: impl Into<PathBuf>, limit: u64) -> std::io::Result<Arc<Self>>`
  - `AudioCache::get(&self, id: &str) -> Option<Box<dyn MediaSource>>`: complete file (touch mtime) or a reader on a running download
  - `AudioCache::download(self: &Arc<Self>, id: &str, ext: &str, resp: reqwest::Response) -> std::io::Result<Box<dyn MediaSource>>`: spawns the writer task, returns a reader; on success renames, indexes, evicts; on failure removes `.part`
  - `pub fn global() -> Option<Arc<AudioCache>>` (LazyLock from `AUDIO_CACHE_SIZE` + `AUDIO_CACHE_DIR`; `None` when 0 or `open` fails, with a warning)
- Growing reader: sync `Read + Seek`, `MediaSource` with `is_seekable() = true`, `byte_len()` = Content-Length if known. Blocks (Mutex + Condvar on `{written, done: Option<Result<(), String>>}`) until bytes past `pos` exist or the download ends; EOF only when done OK; `SeekFrom::End` waits for the length if unknown.
- Eviction: complete files only, oldest mtime first, until total ≤ limit; a file > limit is removed right after completing.

- [ ] **Step 1: Tests** (`#[cfg(test)]`, temp dirs under `std::env::temp_dir()` with a unique name, wiremock for responses):
  - `parse_size_units`: `None`→536870912, `"0"`→0, `"1024"`→1024, `"2K"`→2048, `"512M"`→536870912, `"1G"`→1073741824, `"junk"`→536870912.
  - `video_id_forms`: `https://www.youtube.com/watch?v=dQw4w9WgXcQ&t=1`, `https://youtu.be/dQw4w9WgXcQ`, `https://music.youtube.com/watch?v=dQw4w9WgXcQ`, `https://www.youtube.com/shorts/dQw4w9WgXcQ` → `dQw4w9WgXcQ`; `https://example.com/x.mp3`, `ytsearch:foo` → None.
  - `download_streams_then_serves_from_disk`: serve 64 KiB; reader returns all bytes equal to body; afterwards `<id>.webm` exists and `get(id)` returns a reader with the same bytes.
  - `download_completes_after_reader_dropped`: drop reader after 1 byte; poll until file exists (≤5 s).
  - `failed_download_errors_reader_and_removes_part`: response with Content-Length larger than body (wiremock `set_body_bytes` + header) or a server that closes early → reader `read` returns `Err`; no `.part`, `get` → None.
  - `open_deletes_part_files_and_indexes_complete`: pre-create `a.webm.part`, `dQw4w9WgXcQ.webm`; after `open`, `.part` gone, `get("dQw4w9WgXcQ")` is Some.
  - `evicts_least_recently_played`: limit 300 bytes, three 100-byte cached files with increasing mtimes, download a fourth of 100 → oldest removed, others kept.
  - `oversized_file_is_not_kept`: limit 10, download 100 bytes → reader gets all 100 bytes, file then absent.
- [ ] **Step 2:** `cargo test --lib audio_cache::` → fails.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo test --lib audio_cache::` → pass.
- [ ] **Step 5:** Commit `feat(music): disk audio cache that plays while it downloads`.

### Task 3: Start tracks through the cache, with the 403 retry

**Files:**
- Modify: `src/http.rs` (`Compose::create_async` for `Ytdl`)

**Interfaces:**
- Consumes: Task 1 `Ytdl` (carried `Resolved`, `url_fresh`, `wait_for`, `WAIT_FOR_STREAM`), Task 2 `AudioCache::{get, download}`, `audio_cache::{global, video_id}`.
- Produces: `async fn start(&mut self, cache: Option<Arc<AudioCache>>) -> Result<AudioStream<Box<dyn MediaSource>>, AudioStreamError>` (`create_async` calls it with `global()`; tests pass their own cache).
- Order: ID from known metadata `source_url`, else the query → `cache.get` hit returns at once (no yt-dlp, no HTTP). Else stream info = carried if `url_fresh`, else `resolve()`. If `WAIT_FOR_STREAM`, sleep `wait_for`. HLS → `HlsRequest` as today. Cache + ID → GET with headers; on 403 retry per Global Constraints; non-success → `Fail`; then `cache.download(id, ext or "bin", resp)`. Otherwise `HttpRequest::create_async`, retried once on error the same way.

- [ ] **Step 1: Tests** (wiremock as the stream host; a `Ytdl` built via a `#[cfg(test)]` constructor with known metadata `source_url = https://www.youtube.com/watch?v=dQw4w9WgXcQ` and a carried `Resolved` pointing at the mock, `expire` far ahead):
  - `cached_track_starts_without_network`: pre-populate cache file; mock expects 0 requests; `start` OK and reads the file bytes.
  - `refused_url_is_retried_once`: test `async fn get_with_retry(client: &HttpClient, first: Resolved, refresh: impl Future<Output = Result<Resolved, AudioStreamError>>) -> Result<(Resolved, reqwest::Response), AudioStreamError>` directly (`refresh` = `resolve()` for a carried URL, `Ok(first.clone())` otherwise). Mock returns 403 once then 200; `refresh` yields the same URL; OK with 2 requests. Second case: 403 twice → `Err`, 2 requests.
  - `second_start_attaches_to_running_download`: slow body (`set_delay`), two `start`s on clones → 1 request.
- [ ] **Step 2:** `cargo test --lib http::` → fails.
- [ ] **Step 3:** Implement.
- [ ] **Step 4:** `cargo test --lib` → pass.
- [ ] **Step 5:** Commit `feat(music): play from the audio cache and retry refused streams`.

### Task 4: Callers use one lookup

**Files:**
- Modify: `src/commands/music/play.rs`, `src/commands/music/repeat_all.rs`, `src/commands/music/soundboard/{effect,stream,board}.rs`

**Interfaces:**
- Consumes: `http::lookup`, `Ytdl::metadata`, `Ytdl::with_metadata`.
- Produces: `Enqueuer::enqueue_source(&self, call: &mut Call, query: String, src: Ytdl, metadata: AuxMetadata) -> TrackHandle`; `Enqueuer::enqueue` keeps its signature and calls it with `ytdl(query).with_metadata(..)`.
- `play.rs`: `lookup_metadata` → returns the `Ytdl` from `http::lookup`; `handle_play` enqueues that source for the first track; playlist lookups likewise. `effect`/`stream`: `http::lookup` once, embed from `src.metadata()`, play `src` (error embed on lookup failure instead of `unwrap`). `board.rs` add: `http::lookup(..)` then `.metadata()`; play path unchanged (URL → cache hit when cached).

- [ ] **Step 1:** Implement; `cargo build` clean.
- [ ] **Step 2:** `cargo fmt --check && cargo clippy --locked --all-targets -- -D warnings && cargo test --locked` → all pass.
- [ ] **Step 3:** Commit `feat(music): use a single yt-dlp lookup for play and the soundboard`.

### Task 5: Docs, release, deploy

**Files:**
- Modify: `.env.example`, `README.md`, `CHANGELOG.md`, `Cargo.toml`, `Cargo.lock`

- [ ] **Step 1:** Document `WAIT_FOR_STREAM` and `AUDIO_CACHE_SIZE` in `.env.example` and README; CHANGELOG `## [0.20.0] - 2026-11-24` (Added: audio cache, `WAIT_FOR_STREAM`; Changed: one yt-dlp run per track, availability wait off by default with 403 retry). Bump version to 0.20.0, `cargo build`.
- [ ] **Step 2:** Branch, commit, PR, merge after CI green (same flow as #66), tag `v0.20.0`, push tag; wait for `Release` and `Docker image` workflows.
- [ ] **Step 3:** On `apex.c2yz.com`: `podman pull ghcr.io/ergo35/lyra:latest && systemctl --user restart lyra`; check `podman ps` shows the new image and `journalctl --user -u lyra` has no startup errors.
