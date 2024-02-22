# Lyra

![](assets/lyra-256.png)

Lyra is a music bot written in Rust.

## Getting Started

Lyra is an open source, discord music bot written in Rust.

The idea behind this project is to allow a user to self-host one's own instance of the bot.

User no longer has to rely on 3rd parties to provide them an invite link.

The bot can be run even on a desktop or a phone because after compilation, it's just a simple binary.

As of now, the bot supports spotify url track recognition through a separate nodejs script. I plan to write the actual parser inside the bot iteself but as of now I postponed it into future release.

Slash commands are still work in progress! Currently bot is still heavily in development!

## Setting up

To compile the source code on your own, you need `rust` and `cargo`

To run a dev version use
```bash
$ cargo run
```

To build a production version use
```bash
$ cargo build --release
```

If you need an ARM version and just don't want to wait for ages for the program to compile, use
```bash
$ cross build -r --target aarch64-unknown-linux-gnu
```

To run a program, just type
```bash
$ ./lyra
```

if you want to disown it from the shell, I recommend using the script I provided in `scripts` folder

## Commands

As of now, working commands are:

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
  /skip         Skips the currently playing song
  /stop         Stops playback and destroys the queue; aliases: stop, end
  /volume       Changes output volume
  /effect       Plays one of available audio effects
  /stream       Hijacks output and plays audio; search by query or paste an url; aliases: stream, override, hijack

Tools:
  /ai           Asks AI
  /dice         Rolls a dice
  /owoify       Owoifies whatever you want uwu
  /ping         Pings you backs with a response time
  /posix        Prints current time in POSIX format
  /qr           Creates a qr code from text
  /verse        Reference Bible by verse

Help:
  /help         Prints this help message; aliases: help, huh, welp

```
