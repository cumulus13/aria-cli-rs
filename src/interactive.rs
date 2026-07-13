// File: src/interactive.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Interactive terminal menu for managing aria2 downloads (delete/pause/
// resume/retry/purge/refresh), mirroring the Python tool's `interactive_menu`.

use crate::display::{self, get_display_name, status_colored, task_percent, Table};
use crate::rpc::Aria2Client;
use serde_json::Value;
use std::io::{self, Write};

fn prompt(msg: &str) -> String {
    print!("{msg}");
    let _ = io::stdout().flush();
    let mut line = String::new();
    if io::stdin().read_line(&mut line).is_err() {
        return String::new();
    }
    line.trim().to_string()
}

fn confirm(msg: &str) -> bool {
    let ans = prompt(&format!("{msg} [y/N]: ")).to_lowercase();
    matches!(ans.as_str(), "y" | "yes")
}

fn render_downloads(downloads: &[Value]) {
    let mut table = Table::new(vec![
        "#", "GID", "Status", "File/URL", "Progress", "Size", "Speed",
    ])
    .with_title("\u{1F4CA} Current Downloads")
    .with_max_width(3, 45);

    for (i, task) in downloads.iter().enumerate() {
        let gid = task.get("gid").and_then(Value::as_str).unwrap_or("");
        let gid_display = display::shorten(gid, 8);
        let status_raw = task.get("status").and_then(Value::as_str).unwrap_or("");
        let name = get_display_name(task);
        let percent = task_percent(task);
        let completed = display::task_len(task, "completedLength");
        let total = display::task_len(task, "totalLength");
        let speed = display::task_len(task, "downloadSpeed");

        table.add_row(vec![
            (i + 1).to_string(),
            gid_display,
            status_colored(status_raw),
            name,
            format!("{percent:.1}%"),
            format!(
                "{}/{}",
                display::format_size(completed),
                display::format_size(total)
            ),
            format!("{}/s", display::format_size(speed)),
        ]);
    }
    table.print();
}

pub fn run(client: &Aria2Client, mut downloads: Vec<Value>) {
    if downloads.is_empty() {
        println!("{}", display::warn("\u{1F4ED} No downloads to manage."));
        return;
    }

    println!("\n{}", "=".repeat(60));
    println!("{}", display::hex_bold("\u{1F39B}\u{FE0F}  Interactive Download Manager", display::CYAN));
    println!("{}", "=".repeat(60));
    println!("\n{}", display::warn("\u{1F4CB} Available Actions:"));
    println!("  d<N> or <N>d      : Delete download");
    println!("  r<N> or <N>r      : Resume / retry download");
    println!("  p<N> or <N>p      : Pause download");
    println!("  retry<N>          : Retry a failed download");
    println!("  purge             : Delete ALL downloads");
    println!("  refresh           : Refresh download list");
    println!("  q                 : Quit interactive mode");

    loop {
        render_downloads(&downloads);
        let action = prompt("\n\u{1F3AF} Enter action [q]: ").to_lowercase();
        let action = if action.is_empty() { "q".to_string() } else { action };

        match action.as_str() {
            "q" | "quit" => {
                println!("{}", display::success("\u{1F44B} Exiting interactive mode..."));
                break;
            }
            "refresh" => {
                downloads = client.all_downloads();
                println!("{}", display::success("\u{1F504} Download list refreshed!"));
            }
            "purge" => {
                if confirm("\u{1F5D1}\u{FE0F} Are you sure you want to delete ALL downloads?") {
                    let count = client.purge_all();
                    println!("{}", display::success(&format!("\u{1F9F9} Purged {count} downloads.")));
                    downloads = client.all_downloads();
                }
            }
            other => handle_indexed_action(client, other, &mut downloads),
        }
    }
}

fn parse_indexed(action: &str, prefix_first: char) -> Option<usize> {
    if let Some(rest) = action.strip_prefix(prefix_first) {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.parse::<usize>().ok();
        }
    }
    if let Some(rest) = action.strip_suffix(prefix_first) {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            return rest.parse::<usize>().ok();
        }
    }
    None
}

fn handle_indexed_action(client: &Aria2Client, action: &str, downloads: &mut Vec<Value>) {
    if let Some(n) = parse_indexed(action, 'd') {
        let idx = n.wrapping_sub(1);
        if let Some(task) = downloads.get(idx) {
            let gid = task.get("gid").and_then(Value::as_str).unwrap_or("").to_string();
            let name = get_display_name(task);
            if confirm(&format!("\u{1F5D1}\u{FE0F} Delete #{n} ('{name}')?")) {
                match client.remove_download(&gid) {
                    Ok(true) => {
                        println!("{}", display::success("\u{2705} Download deleted successfully!"));
                        *downloads = client.all_downloads();
                    }
                    _ => println!("{}", display::error("\u{274C} Failed to delete download!")),
                }
            }
        } else {
            println!("{}", display::error("\u{274C} Invalid download number!"));
        }
        return;
    }

    if let Some(n) = parse_indexed(action, 'r') {
        let idx = n.wrapping_sub(1);
        if let Some(task) = downloads.get(idx) {
            let gid = task.get("gid").and_then(Value::as_str).unwrap_or("").to_string();
            let status = task.get("status").and_then(Value::as_str).unwrap_or("");
            let name = get_display_name(task);
            if status == "error" {
                if confirm(&format!("\u{1F504} #{n} ('{name}') is error. Retry?")) {
                    report(client.retry_download(&gid), "retried");
                    *downloads = client.all_downloads();
                }
            } else if confirm(&format!("\u{23EF}\u{FE0F} Resume/retry #{n} ('{name}')?")) {
                match client.resume_download(&gid) {
                    Ok(true) => {
                        println!("{}", display::success("\u{2705} Download resumed successfully!"));
                    }
                    _ => {
                        println!("{}", display::warn("\u{26A0}\u{FE0F} Resume failed, trying retry..."));
                        report(client.retry_download(&gid), "retried");
                    }
                }
                *downloads = client.all_downloads();
            }
        } else {
            println!("{}", display::error("\u{274C} Invalid download number!"));
        }
        return;
    }

    if let Some(n) = parse_indexed(action, 'p') {
        let idx = n.wrapping_sub(1);
        if let Some(task) = downloads.get(idx) {
            let gid = task.get("gid").and_then(Value::as_str).unwrap_or("").to_string();
            match client.pause_download(&gid) {
                Ok(true) => {
                    println!("{}", display::success("\u{2705} Download paused successfully!"));
                    *downloads = client.all_downloads();
                }
                _ => println!("{}", display::error("\u{274C} Failed to pause download!")),
            }
        } else {
            println!("{}", display::error("\u{274C} Invalid download number!"));
        }
        return;
    }

    if let Some(rest) = action.strip_prefix("retry") {
        if !rest.is_empty() && rest.chars().all(|c| c.is_ascii_digit()) {
            if let Ok(n) = rest.parse::<usize>() {
                let idx = n.wrapping_sub(1);
                if let Some(task) = downloads.get(idx) {
                    let gid = task.get("gid").and_then(Value::as_str).unwrap_or("").to_string();
                    let name = get_display_name(task);
                    if confirm(&format!("\u{1F504} Retry #{n} ('{name}')?")) {
                        report(client.retry_download(&gid), "retried");
                        *downloads = client.all_downloads();
                    }
                } else {
                    println!("{}", display::error("\u{274C} Invalid download number!"));
                }
                return;
            }
        }
    }

    println!("{}", display::error("\u{274C} Invalid action! Please try again."));
}

fn report(result: crate::error::Result<bool>, past_tense: &str) {
    match result {
        Ok(true) => println!(
            "{}",
            display::success(&format!("\u{2705} Download {past_tense} successfully!"))
        ),
        _ => println!(
            "{}",
            display::error("\u{274C} Failed to retry download!")
        ),
    }
}
