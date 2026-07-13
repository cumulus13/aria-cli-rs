// File: src/main.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Date: 2026-07-13
// Description: aria-cli — send URLs to Aria2 (with header/cookie support), list,
// monitor (live speed charts), and interactively manage downloads over JSON-RPC.
// Source: https://github.com/cumulus13/aria-cli-rs
// License: MIT
//
// Rust port of the original Python `aria-cli`, rebuilt with:
//   - clap (derive) for argument parsing
//   - clap-color-help for a richly colored --help screen
//   - clap-version-flag for a colored --version banner
//   - make_colors for hex/RGB true-color terminal output
//   - config-get for cross-platform config-file discovery ([main] host/port)
//   - rasciichart for live ASCII speed graphs in --monitor mode
//   - reqwest (blocking) + serde_json for the aria2 JSON-RPC transport

mod cli;
mod config;
mod display;
mod error;
mod interactive;
mod monitor;
mod rpc;

use clap::CommandFactory;
use clap_color_help::default_styles;
use clap_version_flag::{colorful_version, parse_with_version, ColorfulVersionExt};
use cli::Cli;
use config::AriaConfig;
use rpc::Aria2Client;
use serde_json::Value;
use std::process::ExitCode;

fn init_logging(debug: bool) {
    let level = if debug { "debug" } else { "off" };
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or(level))
        .format_timestamp_millis()
        .try_init();
}

fn build_command() -> clap::Command {
    // `default_styles()` from clap-color-help is already applied via the
    // `#[command(styles = ...)]` attribute on `Cli`; re-asserting it here is a
    // harmless no-op guard in case the derive attribute is ever removed.
    Cli::command().styles(default_styles())
}

fn main() -> ExitCode {
    // Match the Python tool's "no args -> print help, exit 0" behavior (clap's
    // `arg_required_else_help` would exit 2 instead, per clap-color-help's docs).
    if std::env::args().len() <= 1 {
        let mut cmd = build_command().with_colorful_version(&colorful_version!());
        let _ = cmd.print_help();
        println!();
        return ExitCode::SUCCESS;
    }

    let version = colorful_version!();
    let cmd = build_command();
    let cli: Cli = match parse_with_version(cmd, &version) {
        Ok(c) => c,
        // `parse_with_version` already handles `-V`/`--version` internally (prints the
        // colored banner and exits 0); this branch only fires on genuine parse errors.
        Err(e) => e.exit(),
    };

    init_logging(cli.debug);

    match run(cli) {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{}", display::error(&format!("\u{274C} Error: {e}")));
            ExitCode::FAILURE
        }
    }
}

fn run(cli: Cli) -> error::Result<()> {
    let file_cfg = AriaConfig::load();
    let rpc_url = cli
        .rpc
        .clone()
        .unwrap_or_else(|| file_cfg.rpc_url());
    let client = Aria2Client::new(rpc_url, cli.secret.clone());

    if cli.monitor {
        monitor::run(&client, cli.interval.max(1), cli.height.max(1));
        return Ok(());
    }

    if cli.list {
        return cmd_list(&client, cli.interactive, cli.save_as.as_deref());
    }

    if cli.list_downloaded {
        return cmd_list_downloaded(&client, cli.interactive, cli.save_as.as_deref());
    }

    if cli.list_wait {
        return cmd_list_wait(&client, cli.interactive, cli.save_as.as_deref());
    }

    if cli.interactive {
        let downloads = client.all_downloads();
        interactive::run(&client, downloads);
        return Ok(());
    }

    if let Some(retry_arg) = &cli.retry {
        return cmd_retry(&client, retry_arg);
    }

    if cli.purge {
        let count = client.purge_all();
        println!(
            "{}",
            display::success(&format!("\u{1F9F9} Purged {count} downloads from Aria2."))
        );
        return Ok(());
    }

    cmd_send(&client, &cli)
}

fn resolve_urls(cli: &Cli) -> Vec<String> {
    let mut urls: Vec<String> = cli.urls.clone();
    urls.extend(cli.url.iter().cloned());

    let mut seen = std::collections::HashSet::new();
    let mut deduped: Vec<String> = Vec::new();
    for u in urls {
        let trimmed = u.trim().to_string();
        if trimmed.is_empty() {
            continue;
        }
        if seen.insert(trimmed.clone()) {
            deduped.push(trimmed);
        }
    }

    // "c" is a shortcut for "paste from clipboard", matching the Python CLI.
    deduped
        .into_iter()
        .map(|u| {
            if u == "c" {
                match arboard::Clipboard::new().and_then(|mut cb| cb.get_text()) {
                    Ok(text) => text.trim().to_string(),
                    Err(_) => {
                        eprintln!("{}", display::warn("\u{26A0}\u{FE0F} Could not read clipboard, skipping 'c'"));
                        String::new()
                    }
                }
            } else {
                u
            }
        })
        .filter(|u| !u.is_empty())
        .collect()
}

fn is_valid_url(u: &str) -> bool {
    match url::Url::parse(u) {
        Ok(parsed) => {
            matches!(parsed.scheme(), "http" | "https" | "ftp") && parsed.host().is_some()
        }
        Err(_) => false,
    }
}

fn cmd_send(client: &Aria2Client, cli: &Cli) -> error::Result<()> {
    let candidate_urls = resolve_urls(cli);

    let mut valid_urls = Vec::new();
    for u in &candidate_urls {
        if is_valid_url(u) {
            valid_urls.push(u.clone());
        } else {
            println!("{}", display::error(&format!("\u{274C} Invalid URL: {u}")));
        }
    }

    if valid_urls.is_empty() {
        println!("{}", display::error("\u{1F6AB} No valid URLs to add."));
        return Err(error::AriaError::NoValidUrls);
    }

    for u in &valid_urls {
        println!(
            "\u{1F3AF} {} {}",
            display::warn("Adding URL:"),
            display::hex(u, display::CYAN)
        );
        let headers: Option<&[String]> = if cli.header.is_empty() {
            None
        } else {
            Some(cli.header.as_slice())
        };
        match client.add_uri(&[u.clone()], headers, cli.cookie.as_deref(), cli.dir.as_deref()) {
            Ok(gid) => println!(
                "{} \u{1F194} {}",
                display::success("\u{2705} Successfully added to queue"),
                gid
            ),
            Err(e) => {
                eprintln!("{}", display::error(&format!("\u{274C} Error: {e}")));
            }
        }
    }

    Ok(())
}

fn print_and_maybe_save(table: &display::Table, save_as: Option<&str>) -> error::Result<()> {
    table.print();
    if let Some(path) = save_as {
        // Plain-text (non-ANSI) copy, so the saved file is readable outside a terminal.
        use std::fmt::Write as _;
        let mut buf = String::new();
        if let Some(title) = &table.title {
            let _ = writeln!(buf, "{title}");
        }
        let _ = writeln!(buf, "{}", table.headers.join(" | "));
        for row in &table.rows {
            let stripped: Vec<String> = row.iter().map(|c| strip_ansi(c)).collect();
            let _ = writeln!(buf, "{}", stripped.join(" | "));
        }
        std::fs::write(path, buf)?;
        println!("{}", display::info(&format!("\u{1F4BE} Saved to {path}")));
    }
    Ok(())
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\u{1b}' {
            in_escape = true;
            continue;
        }
        if in_escape {
            if c == 'm' {
                in_escape = false;
            }
            continue;
        }
        out.push(c);
    }
    out
}

fn build_listing_table(title: &str, tasks: &[Value]) -> display::Table {
    let mut table = display::Table::new(vec!["#", "GID", "Status", "URL", "Progress", "Size"])
        .with_title(title)
        .with_max_width(3, 50);

    for (i, task) in tasks.iter().enumerate() {
        let gid = task.get("gid").and_then(Value::as_str).unwrap_or("");
        let status = task.get("status").and_then(Value::as_str).unwrap_or("");
        let url = display::get_first_uri(task).unwrap_or_else(|| "N/A".to_string());
        let percent = display::task_percent(task);
        let completed = display::task_len(task, "completedLength");
        let total = display::task_len(task, "totalLength");

        table.add_row(vec![
            (i + 1).to_string(),
            display::shorten(gid, 8),
            display::status_colored(status),
            display::shorten(&url, 50),
            format!("{percent:.1}%"),
            format!("{}/{}", display::format_size(completed), display::format_size(total)),
        ]);
    }
    table
}

fn cmd_list(client: &Aria2Client, interactive: bool, save_as: Option<&str>) -> error::Result<()> {
    if interactive {
        let downloads = client.all_downloads();
        interactive::run(client, downloads);
        return Ok(());
    }
    let tasks = client.tell_active()?;
    if tasks.is_empty() {
        println!("{}", display::warn("\u{1F4ED} No active downloads."));
        return Ok(());
    }
    let table = build_listing_table("\u{1F504} Active Downloads", &tasks);
    print_and_maybe_save(&table, save_as)
}

fn cmd_list_downloaded(client: &Aria2Client, interactive: bool, save_as: Option<&str>) -> error::Result<()> {
    if interactive {
        let downloads = client.all_downloads();
        interactive::run(client, downloads);
        return Ok(());
    }
    let tasks = client.tell_stopped(0, 1000)?;
    if tasks.is_empty() {
        println!("{}", display::warn("\u{1F4ED} No stopped/completed downloads."));
        return Ok(());
    }
    let table = build_listing_table("\u{1F4E6} Downloaded/Stopped Files", &tasks);
    print_and_maybe_save(&table, save_as)
}

fn cmd_list_wait(client: &Aria2Client, interactive: bool, save_as: Option<&str>) -> error::Result<()> {
    if interactive {
        let downloads = client.all_downloads();
        interactive::run(client, downloads);
        return Ok(());
    }
    let waiting = client.tell_waiting(0, 1000)?;
    let stopped = client.tell_stopped(0, 1000)?;

    if waiting.is_empty() && stopped.is_empty() {
        println!("{}", display::warn("\u{1F4ED} No waiting or stopped downloads."));
        return Ok(());
    }

    if !waiting.is_empty() {
        let table = build_listing_table("\u{23F3} Waiting Downloads", &waiting);
        print_and_maybe_save(&table, save_as)?;
    }
    if !stopped.is_empty() {
        let table = build_listing_table("\u{1F6D1} Stopped Downloads", &stopped);
        print_and_maybe_save(&table, save_as)?;
    }
    Ok(())
}

fn cmd_retry(client: &Aria2Client, retry_arg: &str) -> error::Result<()> {
    let all_downloads = client.all_downloads();

    let gid = if retry_arg.chars().all(|c| c.is_ascii_digit()) {
        let idx: usize = retry_arg.parse().unwrap_or(0);
        let idx0 = idx.wrapping_sub(1);
        match all_downloads.get(idx0) {
            Some(task) => task
                .get("gid")
                .and_then(Value::as_str)
                .map(String::from)
                .ok_or_else(|| error::AriaError::Other("No GID found for that entry".into()))?,
            None => {
                println!("{}", display::error("\u{274C} Invalid index for retry"));
                return Ok(());
            }
        }
    } else {
        retry_arg.to_string()
    };

    match client.retry_download(&gid) {
        Ok(true) => println!("{}", display::success(&format!("\u{2705} Retrying {gid}: succeeded"))),
        Ok(false) => println!("{}", display::error(&format!("\u{274C} Retrying {gid}: failed"))),
        Err(e) => println!("{}", display::error(&format!("\u{274C} Retrying {gid}: {e}"))),
    }
    Ok(())
}
