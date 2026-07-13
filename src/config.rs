// File: src/config.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Config discovery/reading for aria-cli, backed by the `config-get` crate.
//
// Mirrors the Python version's `configset`-based `[main] host / port` lookup, but uses
// config-get's cross-platform auto-discovery (searches ~/.aria-cli, ~/.config/aria-cli,
// ~/.config, ~/, and CWD on Unix; %APPDATA%\aria-cli, %USERPROFILE%\aria-cli, etc. on
// Windows) instead of hardcoding a path next to the executable.

use config_get::ConfigGet;
use log::debug;

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

    /// Auto-discover `aria-cli.ini` / `.env` / `.toml` / etc. in the standard
    /// config search paths and read `[main] host` / `[main] port`. Falls back
    /// to built-in defaults if no config file is found or a key is missing.
    pub fn load() -> Self {
        match ConfigGet::builder(APP_NAME).config_dir(APP_NAME).build() {
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
                debug!("aria-cli: no config file found, using defaults ({e})");
                Self::default()
            }
        }
    }
}
