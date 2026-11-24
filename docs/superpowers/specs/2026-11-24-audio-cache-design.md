# Faster track start and a local audio cache

## Why

`play` takes 3–5 s to reply and up to 10–15 s more before audio starts. Each
track ran yt-dlp three times in a row (embed metadata, songbird's preload-time
lookup on enqueue, stream URL on start), ~3 s each, more through `MEDIA_PROXY`.
The enqueue lookup is already removed (`Ytdl::with_metadata`, uncommitted). This
removes the remaining duplicate, makes the `available_at` wait optional, and adds
a disk cache so replayed and looped tracks start immediately and cost no YouTube
traffic.

## Goals

- One yt-dlp run per new track at most; zero for a cached track whose video ID is
  known (repeat-all requeues, tracks queued by URL, forceplay's requeued copy).
- Long tracks start as fast as they do now: playback begins on the first bytes,
  never after a full download.
- `WAIT_FOR_STREAM` env var, **off by default**: when on, wait for yt-dlp's
  `available_at` (+1 s margin, capped at 30 s) as today; when off, fetch at once.
- `AUDIO_CACHE_SIZE` env var, default `512M`, `0` disables the cache.
- The soundboard (`board`, `effect`, `stream`) uses the same path: same cache
  and budget, same `WAIT_FOR_STREAM`, and one yt-dlp run (`effect`/`stream`
  currently run it twice: once on a clone for the embed, once to play).

### Refused URLs (applies everywhere, whatever `WAIT_FOR_STREAM` is)

A stream fetch refused with 403 is retried once: wait until that URL's
`available_at` (+1 s margin, capped at 30 s; no wait if absent or past), then fetch
again. For a carried URL (see below) the retry re-runs yt-dlp first and uses the
fresh URL. A second refusal fails the track as today. The first response is
checked in `create_async` before it returns, so a refusal never reaches the
reader as a mid-stream error.

## My calls (check these)

- Carried URLs are reused for any track, not just ones that start immediately,
  as long as the URL's `expire` is more than 60 s away.
- Cache is YouTube-only, keyed by video ID; HLS streams aren't cached.
- `AUDIO_CACHE_SIZE=0` falls back to today's streaming exactly.
- Eviction is least-recently-played, checked after each completed download.

## Non-goals

- Caching non-YouTube sources (they stream as today).
- Cache persistence of metadata: metadata still comes from the embed lookup or
  the flat-playlist output, as today.
- Changing how the call lock is held during lookups.

## Anti-goals

- A track waiting on a full download before it plays.
- Downloading a track twice on first play.
- New dependencies.

## Approaches

1. **Tee the playback stream** into a file; cache only if read start-to-end.
   Smallest, but any seek (including the demuxer's own) breaks the copy, so
   seeked/skipped tracks never cache.
2. **Download in the background, play from the growing file** — chosen. One
   download; reads block until bytes arrive; backward seeks are instant; a skipped
   track keeps downloading and is cached for next time. Forward seeks past the
   downloaded part wait for the download.
3. **Stream as today plus a separate cache download.** Dropped: downloads every
   new track twice.

## Design

### Reusing the embed lookup (one yt-dlp run)

- The embed lookup for a track with no known metadata runs the full `yt-dlp -j`
  (same args as `Ytdl::resolve`) and yields both the `AuxMetadata` and the
  resolved stream info (`url`, `http_headers`, `filesize`, `protocol`, `ext`,
  `available_at`). The track's `Ytdl` carries the resolved info.
- On start, the carried URL is used if its `expire` query parameter is more than
  60 s away; otherwise yt-dlp runs again. Refusals: see "Refused URLs".
- Builder's call: the songbird `YoutubeDl` inside `Ytdl` can be dropped if
  metadata is built from the same JSON.

### Cache

- Directory `data/audio-cache` (inside the existing `lyra-data` volume).
- Key: YouTube video ID, from the track's `source_url` (or the query when it is a
  YouTube URL): `watch?v=`, `youtu.be/`, `music.youtube.com`, `shorts/`. No ID →
  no caching, stream as today.
- Files: `<id>.<ext>` complete, `<id>.<ext>.part` in progress. Startup deletes
  leftover `.part` files and indexes complete files with their size and mtime.
- On track start (`Ytdl::create_async`):
  1. complete file for the ID → play from it, touch its mtime;
  2. download for the ID in progress (same track queued twice / loop during first
     play) → play from that growing file;
  3. otherwise resolve (above), optionally wait for `available_at`, start the
     download task, and play from the growing file.
- HLS (`m3u8_native`) is not cached; it streams as today.
- Growing-file reader: a sync `Read + Seek` `MediaSource` over the `.part` file
  that blocks until the downloader has written enough bytes, reports EOF only
  when the download finished, and errors if the download failed. `byte_len` is
  the `filesize`/`Content-Length` when known. When the download completes the
  file is renamed; open readers keep working.
- A failed download deletes its `.part`; the playing track errors as streams do
  today (`TrackErrorNotifier`).
- Eviction: after each completed download, if the total size of complete files
  exceeds `AUDIO_CACHE_SIZE`, delete least-recently-played files (oldest mtime)
  until it fits. A single file larger than the limit is not kept. Files currently
  being read may be deleted (open handles stay valid on Linux).
- `AUDIO_CACHE_SIZE` accepts bytes or a `K`/`M`/`G` suffix (binary units);
  invalid values log a warning and use the default.
- `AUDIO_CACHE_SIZE=0`: no downloads to disk; stream through songbird's
  `HttpRequest` as today.

### Configuration and docs

- `WAIT_FOR_STREAM` (`true`/`1` to enable) and `AUDIO_CACHE_SIZE` documented in
  `.env.example` and the README; CHANGELOG entry.

## Left to the builder

- Module layout (a new `src/audio_cache.rs` is expected), download chunking, the
  synchronisation used by the growing-file reader, test structure (`wiremock` is
  available for the downloader).
