#!/bin/sh
set -eu

# YouTube changes often and an outdated yt-dlp is the usual reason playback breaks,
# so update it on every start. Failure (e.g. no network yet) is not fatal.
if ! yt-dlp -U >/dev/null 2>&1; then
    echo "warning: yt-dlp self-update failed, using $(yt-dlp --version)" >&2
fi

exec lyra "$@"
