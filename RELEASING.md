# Releasing Lyra

Releases are cut by hand and built by [cargo-dist](https://opensource.axo.dev/cargo-dist/).
`release-plz` is **not** used (removed in 0.11.2).

1. On `senpai`, make sure CI is green (`cargo fmt --check`,
   `cargo clippy --locked --all-targets -- -D warnings`, `cargo test --locked`).
2. Bump `version` in `Cargo.toml`, run `cargo build` so `Cargo.lock` follows,
   and add a `## [X.Y.Z] - YYYY-MM-DD` section to `CHANGELOG.md`
   (cargo-dist uses it for the GitHub Release notes).
3. Commit, then tag and push:

   ```bash
   git tag -a vX.Y.Z -m "vX.Y.Z"
   git push origin senpai vX.Y.Z
   ```

4. The `Release` workflow (`.github/workflows/release.yml`) builds
   `x86_64-unknown-linux-gnu`, the shell installer and checksums, and creates
   the GitHub Release.
5. The same tag triggers `Docker image` (`.github/workflows/docker.yml`), which
   builds `amd64` and `arm64` natively and pushes the multi-arch
   `ghcr.io/ergo35/lyra:X.Y.Z`, `:X.Y` and `:latest`. The first time,
   make the package public under the repository's *Packages* settings so
   `docker compose pull` works without logging in.

   To refresh the pinned `yt-dlp`/`deno` in the image, bump their version and
   SHA-256 `ARG`s in `Dockerfile` (checksums from each release's
   `SHA2-256SUMS` / `.sha256sum` files). `yt-dlp` also updates itself on start.

## Changing release config

`release.yml` is generated. Edit `[workspace.metadata.dist]` in `Cargo.toml`
and run `dist generate`; do not hand-edit the workflow. To check the plan
locally: `dist plan`.

If the `Release` run sits in "queued" for a long time, a runner label in
`release.yml` is probably retired (this is what stalled v0.11.0 and v0.11.1
on `ubuntu-20.04`); regenerate with a current `cargo-dist-version`.

## Note on crates.io

`cargo publish` ignores `[patch.crates-io]`, so a build of Lyra from crates.io
uses upstream `serenity` 0.12.5 (with `tokio-tungstenite` 0.21), not the fork;
only GitHub/cargo-dist builds from this repository get the patched dependency.
