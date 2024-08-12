<h2 align="center">
  <a href="https://lyra.c2yz.com" target="blank_">
    <img height="100" alt="Lyra" src="assets/lyra-256.png" />
  </a>
  <br />
  Lyra: a discord music bot written in Rust :crab:
</h2>

Lyra is an open source, discord music bot written in Rust.
The idea behind this project is to allow a user to self-host one's own instance of the bot.
User no longer has to rely on 3rd-parties to provide them a service.
The bot can be run on a desktop, a vps or a phone as it's just a simple binary.

Currently bot is still heavily in development!

## Getting started

Before you start, you need to create a discord bot and get a token.
You can do it [here](https://discord.com/developers/applications).

After you create a bot, you need to invite it to your server.

Then, head to download section and download the latest release (or compile it yourself).

After you download the binary, you need to create a `.env` file in the same directory as the binary.
Example can be found in `.env.example` file.

```
DISCORD_TOKEN=<YOUR_DISCORD_TOKEN>
PREFIX=<YOUR_PREFIX>
```

DISCORD_TOKEN is the token you got from discord developers page and PREFIX is the prefix you want to use for your bot.

Bot by default reacts only to the prefix. To enable slash commands, while the bot is running type `:register` in the chat (where `:` is your bot prefix).

## Features

- Music playback
- Audio effects (soon)
- Some multipurpose commands
- Slash commands
- Self-hosted

## Compilation

To compile the source code on your own, you need `rust` and `cargo`

To run a dev version, `cd` into the project directory and type

```bash
$ cargo run
```

To build a production version use

```bash
$ cargo build --release
```

If you need a version for a different system or architecture, you can use `cross` crate

```bash
$ cross build -r --target aarch64-unknown-linux-gnu
```

To run a program, just type

```bash
$ ./lyra
```

Remember to provide a `.env` file in the same directory as the binary.

If you want to disown the bot from the shell, I recommend using the script I provided in `scripts` folder

## Commands

As of now, the commands are:

```
Music:
  /deafen       Deafens itself while in a voice channel; aliases: deafen, undeaden, shuush
  /join         Joins your voice channel
  /leave        Leaves the voice channel; aliases: leave, qa!
  /mute         Mutes itself while in a voice channel; aliases: mute, unmute, shhh
  /pause        Pauses the currently playing song
  /play         Plays a song; you can search by query or paste an url; aliases: play, p, enqueue
  /queue        Shows next tracks in queue; aliases: queue, q
  /repeat       Loops currently playing song provided amount of times; aliases: repeat, loop, while, for
  /resume       Resumes currently paused song
  /seek         Seeks a track by provided seconds
  /skip         Skips the currently playing song; aliases: skip, :skipper:
  /stop         Stops playback and destroys the queue; aliases: stop, end
  /volume       Changes output volume
  /effect       Plays one of available audio effects
  /stream       Hijacks output and plays audio; search by query or paste an url; aliases: stream, override, hijack

Tools:
  /ai           Asks AI
  /dice         Rolls a dice
  /dictionary   Explains provided query
  /ip           Shows IP information
  /metar        Prints metar for provided airport
  /owoify       Owoifies whatever you want uwu
  /ping         Pings you backs with a response time
  /posix        Prints current time in POSIX format
  /qr           Creates a qr code from text
  /taf          Returns taf for provided airport
  /uptime       Checks how long the bot has been running
  /verse        Reference Bible by verse
  /weather      Shows weather for provided location

Help:
  /help         Prints this help message; aliases: help, huh, welp

Use /help command for more info on a command.
You can edit you message to the bot and the bot will edit its response.

```
