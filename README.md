# aria-cli

> A Rust CLI for sending URLs to [aria2](https://aria2.github.io/)
> (with header/cookie support), listing, live-monitoring, and interactively managing downloads over
> its JSON-RPC interface.

[![CI](https://github.com/cumulus13/aria-cli-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/cumulus13/aria-cli-rs/actions)
[![crates.io](https://img.shields.io/crates/v/aria-cli.svg)](https://crates.io/crates/aria-cli)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

| Concern                     | Crate                                                                  |
|------------------------------|------------------------------------------------------------------------|
| Argument parsing             | [`clap`](https://crates.io/crates/clap) (derive)                       |
| Colored `--help`             | [`clap-color-help`](https://crates.io/crates/clap-color-help)          |
| Colored `--version`          | [`clap-version-flag`](https://crates.io/crates/clap-version-flag)      |
| Hex/RGB terminal colors      | [`make_colors`](https://crates.io/crates/make_colors)                  |
| Config-file discovery        | [`config-get`](https://crates.io/crates/config-get)                    |
| Live ASCII speed charts      | [`rasciichart`](https://crates.io/crates/rasciichart)                  |
| JSON-RPC transport           | `reqwest` (blocking) + `serde_json`                                    |

## Features

- 🎯 Send one or many URLs to aria2, with custom `--header` / `--cookie` / `--dir`, and a `c` shortcut
  that pulls the URL from the clipboard
- 📋 `--list` / `--list-downloaded` / `--list-wait` — colorized tables of active, finished, or
  waiting/stopped downloads
- 🎛️ `--interactive` — a menu-driven manager: delete / pause / resume / retry / purge / refresh
- 📊 `--monitor` — a live-refreshing dashboard: status table plus global and per-file throughput
  graphs, auto-scaled between B/s, KB/s, MB/s and GB/s
- 🔄 `--retry <index|gid>` — retries a failed download by re-adding its original URI(s)
- 🧹 `--purge` — clears every active/waiting/stopped download from aria2
- 🔐 Full `--secret` (RPC token) and `--rpc` (custom JSON-RPC endpoint) support
- ⚙️ Cross-platform config-file auto-discovery for the default aria2 host/port (`[main] host` /
  `[main] port` in `aria-cli.ini`, or `.env`/`.toml`/`.json`/`.yml` — see `aria-cli.ini.example`)
- 🌈 Full 24-bit truecolor output end-to-end, including in `--help`

## Installation

```bash
cargo install aria-cli
```

Or build from source:

```bash
git clone https://github.com/cumulus13/aria-cli-rs
cd aria-cli-rs
cargo build --release
# binary at target/release/aria-cli (aria-cli.exe on Windows)
```

**MSRV:** Rust 1.86+ (the binding floor is `idna`/`url`'s `icu_*` backend, not `clap-version-flag`'s
2024-edition dependencies, which Cargo resolves to older, edition-2021-compatible versions on request).

## Quick start

```bash
# Add a single URL to a locally-running aria2c --enable-rpc
aria-cli https://example.com/file.zip

# Multiple URLs, custom header and cookie, custom save directory
aria-cli -H "User-Agent: Mozilla/5.0" -C "session=abc123" -d ~/Downloads \
  https://example.com/a.zip https://example.com/b.zip

# Paste a URL from the clipboard
aria-cli c

# Talk to a remote aria2 with an RPC secret token
aria-cli --rpc http://192.168.1.10:6800/jsonrpc --secret mytoken https://example.com/file.iso

# List / manage
aria-cli --list
aria-cli --list-downloaded
aria-cli --list-wait
aria-cli --interactive
aria-cli --retry 2
aria-cli --purge

# Live dashboard with taller charts, refreshed every second
aria-cli --monitor --height 6 --interval 1
```

Run `aria-cli --help` for the full, colorized option reference, or `aria-cli --version` /
`aria-cli -V` for a colored version banner (via `clap-version-flag`).

## Configuration

By default aria-cli talks to `http://222.222.222.5:6800/jsonrpc` (matching the original tool's
placeholder default) unless overridden by:

1. `--rpc <URL>` on the command line (highest priority)
2. `--config <PATH>` / `$ARIA_CLI_CONFIG` — load an exact file, skipping auto-discovery entirely
3. `[main] host` / `[main] port` in an auto-discovered `aria-cli.ini` (see
   [`aria-cli.ini.example`](aria-cli.ini.example) for search paths and format; `.env`, `.toml`,
   `.json`, and `.yml` are also supported via `config-get`)
4. Built-in defaults

For a typical local aria2c setup, drop this at `~/.config/aria-cli/aria-cli.ini`:

```ini
[main]
host = 127.0.0.1
port = 6800
```

**If the config file only seems to load when it's in the current directory:** that almost
always means `dirs::home_dir()` can't resolve a home directory in your environment (unset
`HOME` on Unix, or `USERPROFILE`/`APPDATA` on Windows — common in containers, services, and
some minimal shells), which silently drops every home/`.config`-relative search path and
leaves only the current directory. Two ways to confirm and work around it:

- Run with `--debug`: aria-cli logs every path it checked, in order, and flags explicitly
  whether `dirs::home_dir()` came back empty.
- Use `--config /exact/path/to/aria-cli.ini` (or `$ARIA_CLI_CONFIG`) to bypass discovery
  entirely — this always works regardless of the environment.

## Development

`Cargo.lock` is committed (this is a binary, not a library) so CI's MSRV job checks against
pinned dependency versions rather than silently re-resolving to "latest compatible" on every
run — that re-resolution is exactly what broke the MSRV job originally, when a transitive
dependency (`icu_*`, pulled in by `idna`/`url`) bumped its own minimum Rust version. Run
`cargo update` deliberately and re-verify `cargo check --locked` against the MSRV toolchain
before committing an updated lockfile.

```bash
cargo build
cargo test
cargo clippy --all-targets -- -D warnings
cargo fmt --check
```

## Releasing

Tagging `vX.Y.Z` and pushing triggers `.github/workflows/release.yml`, which cross-builds
release binaries for Linux (x86_64/aarch64, gnu + musl), macOS (x86_64/aarch64) and Windows
(x86_64), and attaches them to a GitHub Release. Publishing to crates.io is a separate,
manually-triggered workflow (`.github/workflows/publish.yml`) so releases and crates.io
publishes stay decoupled.

## License

MIT — see [LICENSE](LICENSE).

## 👤 Author
        
[Hadi Cahyadi](mailto:cumulus13@gmail.com)
    

[![Buy Me a Coffee](https://www.buymeacoffee.com/assets/img/custom_images/orange_img.png)](https://www.buymeacoffee.com/cumulus13)

[![Donate via Ko-fi](https://ko-fi.com/img/githubbutton_sm.svg)](https://ko-fi.com/cumulus13)
 
[Support me on Patreon](https://www.patreon.com/cumulus13)
