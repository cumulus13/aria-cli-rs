// File: src/monitor.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Live-refreshing download monitor: a status table plus global and
// per-download throughput charts rendered with `rasciichart`. Mirrors the Python
// tool's `monitor_downloads` (which used `asciichartpy` + `rich.live.Live`).

use crate::display::{self, format_size, get_display_name, status_colored, task_len, task_percent, Table};
use crate::rpc::Aria2Client;
use crossterm::cursor::MoveTo;
use crossterm::execute;
use crossterm::terminal::{Clear, ClearType};
use rasciichart::{plot_with_config, Config as ChartConfig};
use serde_json::Value;
use std::collections::{HashMap, VecDeque};
use std::io::stdout;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;

const MAX_SAMPLES: usize = 60;
const TOP_K_GRAPHS: usize = 6;

struct SpeedScale {
    divisor: f64,
    unit: &'static str,
}

fn choose_scale(max_val: f64) -> SpeedScale {
    const KB: f64 = 1024.0;
    const MB: f64 = KB * 1024.0;
    const GB: f64 = MB * 1024.0;
    if max_val >= GB {
        SpeedScale { divisor: GB, unit: "GB/s" }
    } else if max_val >= MB {
        SpeedScale { divisor: MB, unit: "MB/s" }
    } else if max_val >= KB {
        SpeedScale { divisor: KB, unit: "KB/s" }
    } else {
        SpeedScale { divisor: 1.0, unit: "B/s" }
    }
}

fn clear_screen() {
    // Portable clear + cursor-home; works correctly on Windows consoles too
    // (unlike hand-rolled ANSI escapes, which need VT100 mode explicitly enabled).
    let _ = execute!(stdout(), Clear(ClearType::All), MoveTo(0, 0));
}

pub fn run(client: &Aria2Client, refresh_interval: u64, chart_height: usize) {
    let running = Arc::new(AtomicBool::new(true));
    {
        let running = running.clone();
        let _ = ctrlc::set_handler(move || running.store(false, Ordering::SeqCst));
    }

    let mut global_speeds: VecDeque<f64> = VecDeque::with_capacity(MAX_SAMPLES);
    let mut per_gid_speeds: HashMap<String, VecDeque<f64>> = HashMap::new();

    println!("{}", display::info("\u{1F525} Starting live download monitor (Ctrl+C to exit)..."));
    std::thread::sleep(Duration::from_millis(400));

    while running.load(Ordering::SeqCst) {
        let all_tasks = client.all_downloads();

        let active_gids: std::collections::HashSet<String> = all_tasks
            .iter()
            .filter(|t| {
                let status = t.get("status").and_then(Value::as_str).unwrap_or("");
                status != "complete" && status != "removed"
            })
            .filter_map(|t| t.get("gid").and_then(Value::as_str).map(String::from))
            .collect();

        per_gid_speeds.retain(|gid, _| active_gids.contains(gid));

        let mut total_speed: f64 = 0.0;
        for task in &all_tasks {
            let status = task.get("status").and_then(Value::as_str).unwrap_or("");
            if status == "complete" || status == "removed" {
                continue;
            }
            let speed = task_len(task, "downloadSpeed") as f64;
            total_speed += speed;
            if let Some(gid) = task.get("gid").and_then(Value::as_str) {
                let dq = per_gid_speeds.entry(gid.to_string()).or_insert_with(|| VecDeque::with_capacity(MAX_SAMPLES));
                if dq.len() == MAX_SAMPLES {
                    dq.pop_front();
                }
                dq.push_back(speed);
            }
        }
        if global_speeds.len() == MAX_SAMPLES {
            global_speeds.pop_front();
        }
        global_speeds.push_back(total_speed);

        clear_screen();
        render_table(&all_tasks);
        render_graphs(&global_speeds, &per_gid_speeds, &all_tasks, chart_height);

        for _ in 0..refresh_interval {
            if !running.load(Ordering::SeqCst) {
                break;
            }
            std::thread::sleep(Duration::from_secs(1));
        }
    }

    println!("\n{}", display::success("\u{1F44B} Exiting monitor mode..."));
}

fn render_table(all_tasks: &[Value]) {
    let mut table = Table::new(vec!["#", "GID", "Status", "File/URL", "Progress", "Size", "Speed"])
        .with_title("\u{1F525} Live Download Monitor (All Status)")
        .with_max_width(3, 40);

    for (i, task) in all_tasks.iter().enumerate() {
        let gid = task.get("gid").and_then(Value::as_str).unwrap_or("");
        let status_raw = task.get("status").and_then(Value::as_str).unwrap_or("");
        let name = get_display_name(task);
        let percent = task_percent(task);
        let completed = task_len(task, "completedLength");
        let total = task_len(task, "totalLength");
        let speed = task_len(task, "downloadSpeed");

        table.add_row(vec![
            (i + 1).to_string(),
            display::shorten(gid, 8),
            status_colored(status_raw),
            name,
            format!("{percent:.1}%"),
            format!("{}/{}", format_size(completed), format_size(total)),
            format!("{}/s", format_size(speed)),
        ]);
    }
    table.print();
}

fn render_graphs(
    global_speeds: &VecDeque<f64>,
    per_gid_speeds: &HashMap<String, VecDeque<f64>>,
    all_tasks: &[Value],
    chart_height: usize,
) {
    let max_global = global_speeds.iter().cloned().fold(0.0_f64, f64::max);
    let scale = choose_scale(max_global);

    println!("\n{}", display::hex_bold(&format!("\u{1F4CA} Speed Monitor ({})", scale.unit), display::CYAN));
    println!("{}", display::hex(&format!("Global throughput ({})", scale.unit), display::GREY));

    if global_speeds.len() >= 2 {
        let scaled: Vec<f64> = global_speeds.iter().map(|v| v / scale.divisor).collect();
        let cfg = ChartConfig::new()
            .with_height(chart_height)
            .with_width(60)
            .with_label_format("{:8.1}".to_string());
        match plot_with_config(&scaled, cfg) {
            Ok(chart) => println!("{chart}"),
            Err(_) => println!("(not enough variance to chart yet)"),
        }
    } else {
        println!("(warming up...)");
    }
    println!(
        "max observed: {}",
        display::hex(&format!("{}/s", format_size(max_global as u64)), display::GREEN)
    );

    let mut latest: Vec<(&String, f64)> = per_gid_speeds
        .iter()
        .filter_map(|(gid, dq)| dq.back().map(|v| (gid, *v)))
        .collect();
    latest.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

    if !latest.is_empty() {
        println!("\n{}", display::hex(&format!("Top file speeds ({})", scale.unit), display::GREY));
        for (gid, _) in latest.into_iter().take(TOP_K_GRAPHS) {
            let dq = &per_gid_speeds[gid];
            let series: Vec<f64> = dq.iter().cloned().collect();
            let scaled: Vec<f64> = series.iter().map(|v| v / scale.divisor).collect();
            let latest_val = *series.last().unwrap_or(&0.0);

            let name = all_tasks
                .iter()
                .find(|t| t.get("gid").and_then(Value::as_str) == Some(gid.as_str()))
                .map(get_display_name)
                .unwrap_or_else(|| gid.clone());
            let label = display::shorten(&name, 24);

            print!(
                "{} ({}/s): ",
                display::hex(&label, display::MAGENTA),
                format_size(latest_val as u64)
            );

            if scaled.len() >= 2 {
                let cfg = ChartConfig::new()
                    .with_height(chart_height.saturating_sub(1).max(1))
                    .with_width(40);
                match plot_with_config(&scaled, cfg) {
                    Ok(chart) => println!("\n{chart}"),
                    Err(_) => println!("(warming up)"),
                }
            } else {
                println!("(warming up)");
            }
        }
    }
}
