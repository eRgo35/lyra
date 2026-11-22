#!/usr/bin/env bash
# Installs Lyra as a systemd service.
#
#   sudo deploy/systemd/install.sh              # binary from the latest GitHub release
#   sudo deploy/systemd/install.sh ./lyra       # or a binary you built
#
# Creates the `lyra` system user, /etc/lyra/lyra.env (from .env.example, only if it does
# not exist yet) and the lyra.service unit. Re-run it to update the binary.
set -euo pipefail

repo=eRgo35/lyra
target=x86_64-unknown-linux-gnu
here=$(cd "$(dirname "$0")" && pwd)
root=$(cd "$here/../.." && pwd)

if [[ $EUID -ne 0 ]]; then
    echo "Run as root, e.g. with sudo." >&2
    exit 1
fi

if [[ $# -ge 1 ]]; then
    install -m 0755 "$1" /usr/local/bin/lyra
else
    tmp=$(mktemp -d)
    trap 'rm -rf "$tmp"' EXIT
    archive="lyra-$target.tar.xz"
    echo "Downloading $archive from the latest release of $repo..."
    curl -fsSL -o "$tmp/$archive" "https://github.com/$repo/releases/latest/download/$archive"
    curl -fsSL -o "$tmp/$archive.sha256" "https://github.com/$repo/releases/latest/download/$archive.sha256"
    (cd "$tmp" && sha256sum -c "$archive.sha256")
    tar -xJf "$tmp/$archive" -C "$tmp"
    install -m 0755 "$tmp/lyra-$target/lyra" /usr/local/bin/lyra
fi

if ! id lyra &>/dev/null; then
    useradd --system --home-dir /var/lib/lyra --no-create-home --shell /usr/sbin/nologin lyra
fi

install -d -m 0755 /etc/lyra
if [[ ! -f /etc/lyra/lyra.env ]]; then
    install -m 0600 "$root/.env.example" /etc/lyra/lyra.env
    created_env=1
fi

install -m 0644 "$here/lyra.service" /etc/systemd/system/lyra.service
systemctl daemon-reload

if ! command -v yt-dlp &>/dev/null; then
    echo "warning: yt-dlp is not on PATH; music commands need it." >&2
    echo "  Install the standalone binary (it can update itself with 'yt-dlp -U'):" >&2
    echo "  sudo curl -fsSLo /usr/local/bin/yt-dlp https://github.com/yt-dlp/yt-dlp/releases/latest/download/yt-dlp_linux && sudo chmod +x /usr/local/bin/yt-dlp" >&2
fi

if [[ -n "${created_env:-}" ]]; then
    echo "Created /etc/lyra/lyra.env. Fill in DISCORD_TOKEN and PREFIX, then run:"
    echo "  sudo systemctl enable --now lyra"
else
    systemctl enable lyra
    systemctl restart lyra
    echo "Lyra (re)started. Logs: journalctl -u lyra -f"
fi
