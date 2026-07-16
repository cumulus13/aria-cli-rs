// File: src/config.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Config discovery/reading for aria-cli, backed by the `config-get` crate.
//
// Mirrors the Python version's `configset`-based `[main] host / port` lookup, but uses
// config-get's cross-platform auto-discovery (searches ~/aria-cli, ~/.config/aria-cli,
// ~/.config, ~/, then CWD last, on Unix; %APPDATA%\aria-cli, %USERPROFILE%\aria-cli, etc.
// on Windows) instead of hardcoding a path next to the executable or requiring CWD.
//
// IMPORTANT caveat about that auto-discovery: config-get's home/`.config` search roots
// depend on the `dirs` crate resolving a home directory. If `HOME` (Unix) or
// `USERPROFILE`/`APPDATA` (Windows) aren't set -- common in stripped-down containers,
// some service/daemon contexts, or misconfigured shells -- `dirs::home_dir()` returns
// `None`, every home-relative candidate is silently skipped, and the current working
// directory ends up being the *only* place actually searched. That looks identical to
// "the config file only loads from CWD" from the outside, so this module does two
// things to make that failure mode diagnosable and, ideally, moot:
//   1. `--config <PATH>` (or `$ARIA_CLI_CONFIG`) lets a user point at an exact file,
//      bypassing discovery entirely -- the reliable fix regardless of root cause.
//   2. In `--debug` mode, every path config-get would check is logged, in priority
//      order, along with which one (if any) it picked -- turns "why isn't this
//      loading?" into a one-glance answer instead of a guessing game.

use config_get::ConfigGet;
use log::debug;
use std::path::Path;

pub const DEFAULT_HOST: &str = "222.222.222.5";
pub const DEFAULT_PORT: u16 = 6800;

const APP_NAME: &str = "aria-cli";
const CONFIG_SECTION: &str = "main";

#[derive(Debug, Clone)]
pub struct AriaConfig {
    pub host: String,
    pub port: u16,
}

impl Default for AriaConfig {
    fn default() -> Self {
        Self {
            host: DEFAULT_HOST.to_string(),
            port: DEFAULT_PORT,
        }
    }
}

impl AriaConfig {
    pub fn rpc_url(&self) -> String {
        format!("http://{}:{}/jsonrpc", self.host, self.port)
    }

    /// Load `[main] host` / `[main] port`, either from an explicit file (when
    /// `explicit_path` is `Some`, e.g. from `--config`/`$ARIA_CLI_CONFIG`) or by
    /// auto-discovering `aria-cli.ini` / `.env` / `.toml` / etc. via config-get's
    /// standard search order. Falls back to built-in defaults if nothing is found
    /// (explicit path missing/unreadable is also non-fatal: it falls back too, after
    /// logging why, rather than aborting the whole command over an optional file).
    pub fn load(explicit_path: Option<&str>) -> Self {
        log_search_plan(explicit_path);

        let builder = match explicit_path {
            Some(p) => ConfigGet::builder(APP_NAME).path(Path::new(p)),
            None => ConfigGet::builder(APP_NAME).config_dir(APP_NAME),
        };

        match builder.build() {
            Ok(cfg) => {
                if let Some(path) = cfg.loaded_from() {
                    debug!("aria-cli: loaded config from {}", path.display());
                }
                let host = cfg
                    .get_in_or(CONFIG_SECTION, "host", DEFAULT_HOST)
                    .to_string();
                let port = cfg
                    .get_in_or(CONFIG_SECTION, "port", &DEFAULT_PORT.to_string())
                    .parse::<u16>()
                    .unwrap_or(DEFAULT_PORT);
                Self { host, port }
            }
            Err(e) => {
                debug!("aria-cli: no usable config found, using built-in defaults ({e})");
                Self::default()
            }
        }
    }
}

/// Emits (at --debug level) exactly which paths will be tried, in priority order, and
/// whether `dirs::home_dir()` came back empty -- the concrete symptom behind "config
/// only loads from the current directory".
fn log_search_plan(explicit_path: Option<&str>) {
    if let Some(p) = explicit_path {
        debug!("aria-cli: --config given, loading exactly {p} (auto-discovery skipped)");
        return;
    }

    if dirs::home_dir().is_none() {
        debug!(
            "aria-cli: dirs::home_dir() returned None (HOME/USERPROFILE not set?); \
             only the current directory will be searched for {APP_NAME}.ini"
        );
    }

    let candidates = ConfigGet::search_paths(APP_NAME, APP_NAME);
    debug!("aria-cli: config search order ({} candidates):", candidates.len());
    for (i, p) in candidates.iter().enumerate() {
        let marker = if p.is_file() { " <- found" } else { "" };
        debug!("  [{i}] {}{marker}", p.display());
    }
}
