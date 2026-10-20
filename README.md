<div align="center">

<picture>
  <source srcset="assets/lyra.svg" type="image/svg+xml" />
  <img src="assets/lyra-512.png" alt="Lyra logo" width="160" height="160" />
</picture>

# Lyra

**A self-hosted Discord music bot written in Rust.**

[![CI](https://github.com/eRgo35/lyra/actions/workflows/ci.yml/badge.svg?branch=senpai)](https://github.com/eRgo35/lyra/actions/workflows/ci.yml)
[![Latest release](https://img.shields.io/github/v/release/eRgo35/lyra)](https://github.com/eRgo35/lyra/releases/latest)
[![License: MIT](https://img.shields.io/github/license/eRgo35/lyra)](LICENSE.md)
[![Rust edition 2021](https://img.shields.io/badge/rust-edition%202021-orange?logo=rust)](Cargo.toml)
[![Homepage](https://img.shields.io/badge/homepage-lyra.c2yz.com-ff3a61)](https://lyra.c2yz.com)

[Quick start](#quick-start) · [Configuration](#configuration) · [Commands](#commands) · [Building](#building-from-source) · [Contributing](CONTRIBUTING.md) · [Changelog](CHANGELOG.md)

</div>

Lyra is a Discord bot you run yourself: one binary, one `.env` file, no database and no
third-party bot service in the middle. Point it at a Discord application token and it plays
music in your voice channels. It runs on a desktop, a VPS or anything else that can run a Linux binary.

> **Status:** Lyra is still under heavy development. Some tool commands are placeholders
> (see [Commands](#commands)), and the project is built on `serenity` 0.12 / `poise` 0.7.
> Known, unresolved dependency advisories are listed in the [changelog](CHANGELOG.md).

## Features

- **Music playback** in voice channels: queue, pause/resume, skip, stop, seek, volume, repeat, mute/deafen.
- **Search or paste a link.** Tracks are resolved with [`yt-dlp`](https://github.com/yt-dlp/yt-dlp), so plain
  search terms (searched on YouTube) and any URL `yt-dlp` understands work. YouTube playlist URLs and Spotify
  links are expanded into individual tracks.
- **`effect` and `stream`** commands that play a track right away, on top of the queue.
- **Prefix _and_ slash commands.** Every command works both ways; slash commands are registered automatically on startup.
- **Edit tracking:** edit your prefix command message (within an hour) and the bot updates its reply.
- A few **utility commands** (dice, QR codes, dictionary, Bible verses, uptime, ...).
- **Self-hosted and key-free:** the only secret is your Discord bot token.

## Requirements

| What | Needed for | Notes |
| --- | --- | --- |
| Discord application + bot token | running | Create one in the [Developer Portal](https://discord.com/developers/applications). |
| **Message Content Intent** | running | Privileged intent; must be enabled on the bot page of the Developer Portal. |
| [`yt-dlp`](https://github.com/yt-dlp/yt-dlp) on `PATH` | music commands | Invoked by name (`yt-dlp`) for every track lookup. |
| Rust (recent stable), C compiler, `cmake`, `pkg-config` | building from source | `cmake` builds the bundled libopus when no system libopus is found. No MSRV is declared. |

`ffmpeg` is **not** called by Lyra's code; audio is decoded in-process with Symphonia. `yt-dlp`
itself may want `ffmpeg` for some sites; that is outside what has been verified here.
Playlist expansion calls `yt-dlp --flat-playlist` with `--username oauth2 --password ''`, so
YouTube playlist behaviour depends on your `yt-dlp` version and setup.

## Quick start

1. **Create the bot.** In the [Developer Portal](https://discord.com/developers/applications) create an
   application, add a bot, copy its token, and enable **Message Content Intent** under *Bot → Privileged Gateway Intents*.
2. **Invite it.** Replace `<APPLICATION_ID>` with your application's ID:

   ```text
   https://discord.com/oauth2/authorize?client_id=<APPLICATION_ID>&scope=bot%20applications.commands&permissions=3230720
   ```

   Scopes: `bot`, `applications.commands`. Permissions (`3230720`): View Channels, Send Messages,
   Embed Links, Read Message History, Connect, Speak.
3. **Install `yt-dlp`** with your package manager or from its [releases](https://github.com/yt-dlp/yt-dlp/releases).
4. **Get Lyra.** Either use the installer from GitHub Releases (installs to `~/.cargo/bin`;
   prebuilt for `x86_64-unknown-linux-gnu` only):

   ```bash
   curl --proto '=https' --tlsv1.2 -LsSf https://github.com/eRgo35/lyra/releases/latest/download/lyra-installer.sh | sh
   ```

   This works once the release for the version tag has been published; if there is no
   installer for the version you want yet, [build from source](#building-from-source).
5. **Configure.** In the directory you will run Lyra from, create a `.env`
   (see [`.env.example`](.env.example)):

   ```bash
   DISCORD_TOKEN=your-bot-token
   PREFIX=!
   ```

6. **Run it** from that directory: `lyra` (or `./target/release/lyra` after a source build).
   Join a voice channel and try `!play never gonna give you up` or `/play`.

## Configuration

Lyra reads its settings from the environment. A `.env` file in the working directory
(or a parent) is loaded if it exists; real environment variables work too.

| Variable | Required | Description |
| --- | --- | --- |
| `DISCORD_TOKEN` | yes | Bot token. Startup fails if it is missing or invalid. |
| `PREFIX` | yes | Prefix for text commands, e.g. `!`. Also shown in the bot's "Listening to `<prefix>help`" status. |
| `GUILD_ID` | no | Register slash commands in this one guild (updates are instant). If unset they are registered globally, which Discord may take up to an hour to propagate. A value that is not a number is a startup error. |
| `RUST_LOG` | no | Log filter for `tracing` (e.g. `info`, `lyra=debug`). When unset, only errors are logged, so set `RUST_LOG=info` to see connection and command logs. |

No other API keys are used. The `dictionary`, `qr` and `verse` commands call free public
web APIs (dictionaryapi.dev, api.qrserver.com, bible-api.com) without credentials.

## Commands

Every command below works as a prefix command (`<PREFIX>name`) and a slash command (`/name`).
Music commands only work inside a server. Aliases apply to prefix commands.

### Music

| Command | Aliases | Description |
| --- | --- | --- |
| `play <query or url>` | `p`, `enqueue` | Queue a track, playlist or search result; joins your voice channel if the bot is not already in one. |
| `join` | | Join your voice channel. |
| `leave` | `qa!` | Leave the voice channel. |
| `pause` / `resume` | | Pause or resume the current track. |
| `skip` | `skipper:` | Skip the current track. |
| `stop` | `end` | Stop playback and clear the queue (the bot stays in the channel). |
| `queue` | `q` | Show the next tracks in the queue. |
| `repeat <times>` | `loop`, `while`, `for` | Loop the current track. |
| `seek <seconds>` | | Seek the current track by the given number of seconds. |
| `volume <percent>` | | Set the volume of the current track. |
| `mute` | `shhh`, `unmute` | Toggle the bot's self-mute. |
| `deafen` | `shuush`, `undeafen` | Toggle the bot's self-deafen. |
| `effect [pause] <query or url>` | | Play an audio effect (a search or URL) immediately, outside the queue. Add the word `pause` to pause the queue first. |
| `stream [pause] <query or url>` | `override`, `hijack` | Hijack the output: play a track immediately, outside the queue (same `pause` option). |

### Tools

| Command | Aliases | Description |
| --- | --- | --- |
| `help [command]` | `huh`, `welp` | List commands, or show details for one. |
| `ping` | | Reply with the response time. |
| `uptime` | | How long the bot has been running. |
| `dice` | | Roll a six-sided die. |
| `posix` | | Current time as milliseconds since the Unix epoch. |
| `qr <text>` | | Generate a QR code image for the text. |
| `dictionary <word>` | `dict` | English dictionary lookup. |
| `verse [latin] <book chapter:verse>` | | Look up a Bible verse (WEB translation, or Clementine Vulgate with `latin`). |
| `owoify <text>` | | Owoify text. |
| `ai <prompt>` | | A joke: replies with a canned "as an AI language model" apology. No model is involved. |
| `ip`, `metar`, `taf`, `weather` | | Placeholders that currently reply "Work in progress". |

A hidden, owner-only prefix command `register` re-registers slash commands with buttons; you do not
need it for normal setup because slash commands are registered at startup.

## Building from source

```bash
git clone https://github.com/eRgo35/lyra.git
cd lyra
git checkout senpai            # development branch
cargo build --release          # binary: target/release/lyra
```

You need a Rust toolchain, a C compiler, `cmake` and `pkg-config`
(for example `build-essential cmake pkg-config` on Debian/Ubuntu).
`cargo run` starts a debug build. For an `aarch64-unknown-linux-gnu` binary,
[`scripts/cross-build.sh`](scripts/cross-build.sh) wraps `cross build -r --target aarch64-unknown-linux-gnu`
(requires [`cross`](https://github.com/cross-rs/cross) and a container engine).

With Nix, `nix develop` opens a shell with the Rust toolchain, OpenSSL, `pkg-config`, `cmake`, opus and
`cross`. The flake provides a dev shell only, not an installable package.

## Self-hosting tips

[`scripts/launch.sh`](scripts/launch.sh) starts `./lyra` detached with `nohup`, writing `lyra.log` and
`lyra.err` in the current directory.

A minimal systemd unit, assuming the binary and `.env` live in `/opt/lyra`
(`yt-dlp` must be on the service's `PATH`):

```ini
[Unit]
Description=Lyra Discord bot
After=network-online.target
Wants=network-online.target

[Service]
WorkingDirectory=/opt/lyra
ExecStart=/opt/lyra/lyra
Environment=RUST_LOG=info
Restart=on-failure
User=lyra

[Install]
WantedBy=multi-user.target
```

Keep `.env` out of version control (it is already in `.gitignore`) and readable only by the service user.

## Contributing, security, license

- Contributions are welcome: see [CONTRIBUTING.md](CONTRIBUTING.md).
- Report vulnerabilities privately: see [SECURITY.md](SECURITY.md).
- Release process: [RELEASING.md](RELEASING.md). History: [CHANGELOG.md](CHANGELOG.md).
- Licensed under the MIT License, see [LICENSE.md](LICENSE.md).

Built with [serenity](https://github.com/serenity-rs/serenity), [poise](https://github.com/serenity-rs/poise)
and [songbird](https://github.com/serenity-rs/songbird).
