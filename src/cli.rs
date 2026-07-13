// File: src/cli.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: clap argument definitions for aria-cli, styled via `clap-color-help`
// (rich-argparse-style colored `--help`) with `--version`/`-V` handled by
// `clap-version-flag` (colored `name vX.Y.Z by Author` banner) instead of clap's
// plain-text default.

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
    /// \u{1F4C2} URLs to add (multiple allowed, or "c" to use clipboard contents)
    pub urls: Vec<String>,

    /// \u{1F310} URL(s) to be downloaded. Repeat for many URLs.
    #[arg(short = 'u', long = "url", action = clap::ArgAction::Append)]
    pub url: Vec<String>,

    /// \u{1F4CB} HTTP header, e.g. 'User-Agent: CustomAgent'. Repeatable.
    #[arg(short = 'H', long = "header", action = clap::ArgAction::Append)]
    pub header: Vec<String>,

    /// \u{1F36A} HTTP cookie, e.g. 'sessionid=abc123; logged_in=true'
    #[arg(short = 'C', long = "cookie")]
    pub cookie: Option<String>,

    /// \u{1F4C1} Directory to save files
    #[arg(short = 'd', long = "dir")]
    pub dir: Option<String>,

    /// \u{1F510} RPC token secret for Aria2 (if set)
    #[arg(short = 's', long = "secret")]
    pub secret: Option<String>,

    /// \u{1F517} Aria2 JSON-RPC endpoint URL. Default: http://<host>:<port>/jsonrpc from config
    #[arg(short = 'R', long = "rpc")]
    pub rpc: Option<String>,

    /// \u{1F4CB} List all current downloads in Aria2
    #[arg(short = 'l', long = "list")]
    pub list: bool,

    /// \u{1F4E6} List all downloaded/finished files
    #[arg(short = 'L', long = "list-downloaded")]
    pub list_downloaded: bool,

    /// \u{23F3} List waiting or stopped downloads
    #[arg(short = 'w', long = "list-wait")]
    pub list_wait: bool,

    /// \u{1F39B}\u{FE0F} Enter interactive mode for download management
    #[arg(short = 'i', long = "interactive")]
    pub interactive: bool,

    /// \u{1F4BE} Save table/listing output to a file
    #[arg(short = 'o', long = "save-as")]
    pub save_as: Option<String>,

    /// \u{1F9F9} Delete ALL download lists from Aria2
    #[arg(long = "purge")]
    pub purge: bool,

    /// \u{1F4CA} Monitor downloads with a live table + speed charts
    #[arg(short = 'm', long = "monitor")]
    pub monitor: bool,

    /// \u{1F4CF} Height of the speed charts in monitor mode
    #[arg(long = "height", default_value_t = 4)]
    pub height: usize,

    /// \u{23F1}\u{FE0F} Refresh interval (seconds) for monitor mode
    #[arg(long = "interval", default_value_t = 2)]
    pub interval: u64,

    /// \u{1F504} Retry a failed download by index (from --list) or GID
    #[arg(short = 'r', long = "retry")]
    pub retry: Option<String>,

    /// Debugging process (verbose logging + backtraces)
    #[arg(long = "debug")]
    pub debug: bool,
}
