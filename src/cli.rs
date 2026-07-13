// File: src/cli.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: clap argument definitions for aria-cli, styled via `clap-color-help`
// (rich-argparse-style colored `--help`) with `--version`/`-V` handled by
// `clap-version-flag` (colored `name vX.Y.Z by Author` banner) instead of clap's
// plain-text default.
//
// NOTE: emoji in the `///` doc comments below are literal UTF-8 characters, not
// `\u{...}` escapes -- escape sequences are only expanded inside string literals
// (e.g. the `about = "..."` attribute below), never inside doc comments, which
// clap turns verbatim into `--help` text.

use clap::Parser;
use clap_color_help::default_styles;

#[derive(Parser, Debug)]
#[command(
    name = "aria-cli",
    about = "\u{1F3AF} Send URLs to Aria2 with header/cookie support, monitor and manage downloads",
    long_about = None,
    styles = default_styles(),
    disable_version_flag = true,
    disable_help_subcommand = true
)]
pub struct Cli {
    /// 📂 URLs to add (multiple allowed, or "c" to use clipboard contents)
    pub urls: Vec<String>,

    /// 🌐 URL(s) to be downloaded. Repeat for many URLs.
    #[arg(short = 'u', long = "url", action = clap::ArgAction::Append)]
    pub url: Vec<String>,

    /// 📋 HTTP header, e.g. 'User-Agent: CustomAgent'. Repeatable.
    #[arg(short = 'H', long = "header", action = clap::ArgAction::Append)]
    pub header: Vec<String>,

    /// 🍪 HTTP cookie, e.g. 'sessionid=abc123; logged_in=true'
    #[arg(short = 'C', long = "cookie")]
    pub cookie: Option<String>,

    /// 📁 Directory to save files
    #[arg(short = 'd', long = "dir")]
    pub dir: Option<String>,

    /// 🔐 RPC token secret for Aria2 (if set)
    #[arg(short = 's', long = "secret")]
    pub secret: Option<String>,

    /// 🔗 Aria2 JSON-RPC endpoint URL. Default: http://<host>:<port>/jsonrpc from config
    #[arg(short = 'R', long = "rpc")]
    pub rpc: Option<String>,

    /// 📋 List all current downloads in Aria2
    #[arg(short = 'l', long = "list")]
    pub list: bool,

    /// 📦 List all downloaded/finished files
    #[arg(short = 'L', long = "list-downloaded")]
    pub list_downloaded: bool,

    /// ⏳ List waiting or stopped downloads
    #[arg(short = 'w', long = "list-wait")]
    pub list_wait: bool,

    /// 🎛️ Enter interactive mode for download management
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// 💾 Save table/listing output to a file
    #[arg(short = 'o', long = "save-as")]
    pub save_as: Option<String>,

    /// 🧹 Delete ALL download lists from Aria2
    #[arg(long = "purge")]
    pub purge: bool,

    /// 📊 Monitor downloads with a live table + speed charts
    #[arg(short = 'm', long = "monitor")]
    pub monitor: bool,

    /// 📏 Height of the speed charts in monitor mode
    #[arg(long = "height", default_value_t = 4)]
    pub height: usize,

    /// ⏱️ Refresh interval (seconds) for monitor mode
    #[arg(long = "interval", default_value_t = 2)]
    pub interval: u64,

    /// 🔄 Retry a failed download by index (from --list) or GID
    #[arg(short = 'r', long = "retry")]
    pub retry: Option<String>,

    /// Debugging process (verbose logging + backtraces)
    #[arg(long = "debug")]
    pub debug: bool,
}
