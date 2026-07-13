// File: src/display.rs
// Author: Hadi Cahyadi <cumulus13@gmail.com>
// Description: Terminal color helpers, status icons, size formatting, and a lightweight
// box-drawing table renderer. Colors are produced with the `make_colors` crate (hex/RGB
// true-color, matching the palette used by the original Python `rich`-based CLI).

use make_colors::{make_colors, make_colors_hex, make_colors_with_attrs};
use serde_json::Value;

// Roughly the same accent palette the Python tool used via `rich` markup.
pub const YELLOW: &str = "#FFFF00";
pub const CYAN: &str = "#00FFFF";
pub const RED: &str = "#FF5555";
pub const GREEN: &str = "#55FF55";
pub const MAGENTA: &str = "#FF55FF";
pub const GREY: &str = "#AAAAAA";

pub fn hex(text: &str, fg: &str) -> String {
    make_colors_hex(text, fg, None).unwrap_or_else(|_| text.to_string())
}

pub fn hex_bold(text: &str, fg: &str) -> String {
    make_colors::ColorBuilder::new(text)
        .fg_hex(fg)
        .unwrap_or_else(|_| make_colors::ColorBuilder::new(text))
        .bold()
        .build()
}

pub fn success(text: &str) -> String {
    make_colors_with_attrs(text, "green", None, &["bold"])
}

pub fn error(text: &str) -> String {
    make_colors_with_attrs(text, "red", None, &["bold"])
}

pub fn warn(text: &str) -> String {
    make_colors(text, "yellow", None)
}

pub fn info(text: &str) -> String {
    make_colors(text, "cyan", None)
}

/// Human-readable byte size, e.g. `12.3 MB`.
pub fn format_size(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    for unit in UNITS {
        if size < 1024.0 {
            return format!("{size:.1} {unit}");
        }
        size /= 1024.0;
    }
    format!("{:.1} PB", size)
}

pub fn get_icon_status(status: &str) -> &'static str {
    match status {
        "active" => "\u{1F504}",   // 🔄
        "waiting" => "\u{23F3}",   // ⏳
        "paused" => "\u{23F8}",    // ⏸
        "error" => "\u{274C}",     // ❌
        "complete" => "\u{2705}",  // ✅
        "removed" => "\u{1F5D1}",  // 🗑️
        _ => "\u{1F480}",          // 💀
    }
}

pub fn status_colored(status: &str) -> String {
    let (label, color) = match status {
        "active" => (format!("{} Active", get_icon_status(status)), GREEN),
        "waiting" => (format!("{} Waiting", get_icon_status(status)), YELLOW),
        "paused" => (format!("{} Paused", get_icon_status(status)), CYAN),
        "error" => (format!("{} Error", get_icon_status(status)), RED),
        "complete" => (format!("{} Complete", get_icon_status(status)), GREEN),
        "removed" => (format!("{} Removed", get_icon_status(status)), MAGENTA),
        other => (format!("\u{2753} {}", capitalize(other)), GREY),
    };
    hex(&label, color)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        None => String::new(),
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
    }
}

/// Best display name for a task: filename from `files[].path`, else basename
/// extracted from the first URI, else a shortened URL, else "N/A".
pub fn get_display_name(task: &Value) -> String {
    if let Some(files) = task.get("files").and_then(Value::as_array) {
        for f in files {
            if let Some(path) = f.get("path").and_then(Value::as_str) {
                if !path.is_empty() {
                    if let Some(name) = path.rsplit(['/', '\\']).next() {
                        if !name.is_empty() {
                            return name.to_string();
                        }
                    }
                }
            }
        }
        // Fallback to the first URI.
        for f in files {
            if let Some(uris) = f.get("uris").and_then(Value::as_array) {
                if let Some(uri) = uris.first().and_then(|u| u.get("uri")).and_then(Value::as_str) {
                    if !uri.is_empty() {
                        if let Ok(parsed) = url::Url::parse(uri) {
                            if let Some(segments) = parsed.path_segments() {
                                if let Some(last) = segments.last() {
                                    if !last.is_empty() && !last.starts_with('?') {
                                        return urlencoding_decode(last);
                                    }
                                }
                            }
                        }
                        return shorten(uri, 50);
                    }
                }
            }
        }
    }
    "N/A".to_string()
}

fn urlencoding_decode(s: &str) -> String {
    percent_decode(s)
}

/// Minimal percent-decoding (no external dependency needed for filenames).
fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(hex_str) = std::str::from_utf8(&bytes[i + 1..i + 3]) {
                if let Ok(byte) = u8::from_str_radix(hex_str, 16) {
                    out.push(byte);
                    i += 3;
                    continue;
                }
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).to_string()
}

pub fn shorten(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let truncated: String = s.chars().take(max.saturating_sub(3)).collect();
        format!("{truncated}...")
    } else {
        s.to_string()
    }
}

pub fn get_first_uri(task: &Value) -> Option<String> {
    task.get("files")
        .and_then(Value::as_array)
        .and_then(|files| {
            files.iter().find_map(|f| {
                f.get("uris")
                    .and_then(Value::as_array)
                    .and_then(|uris| uris.first())
                    .and_then(|u| u.get("uri"))
                    .and_then(Value::as_str)
                    .map(|s| s.to_string())
            })
        })
}

pub fn task_percent(task: &Value) -> f64 {
    let completed = task
        .get("completedLength")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    let total = task
        .get("totalLength")
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);
    if total > 0 {
        completed as f64 / total as f64 * 100.0
    } else {
        0.0
    }
}

pub fn task_len(task: &Value, key: &str) -> u64 {
    task.get(key)
        .and_then(Value::as_str)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

/// A minimal, dependency-free box-drawing table renderer, colored via `make_colors`.
pub struct Table {
    pub title: Option<String>,
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    /// Visible-width cap per column (0 = unlimited); content is shortened with `...`.
    pub max_widths: Vec<usize>,
}

impl Table {
    pub fn new(headers: Vec<&str>) -> Self {
        let n = headers.len();
        Self {
            title: None,
            headers: headers.into_iter().map(String::from).collect(),
            rows: Vec::new(),
            max_widths: vec![0; n],
        }
    }

    pub fn with_title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn with_max_width(mut self, col: usize, width: usize) -> Self {
        if col < self.max_widths.len() {
            self.max_widths[col] = width;
        }
        self
    }

    pub fn add_row(&mut self, row: Vec<String>) {
        self.rows.push(row);
    }

    /// Strip ANSI escape codes to measure true visible width.
    fn visible_len(s: &str) -> usize {
        let mut len = 0;
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
            len += 1;
        }
        len
    }

    pub fn print(&self) {
        let cols = self.headers.len();
        let mut widths: Vec<usize> = self
            .headers
            .iter()
            .map(|h| Self::visible_len(h))
            .collect();

        let processed_rows: Vec<Vec<String>> = self
            .rows
            .iter()
            .map(|row| {
                row.iter()
                    .enumerate()
                    .map(|(i, cell)| {
                        let cap = self.max_widths.get(i).copied().unwrap_or(0);
                        if cap > 0 && Self::visible_len(cell) > cap {
                            shorten(cell, cap)
                        } else {
                            cell.clone()
                        }
                    })
                    .collect()
            })
            .collect();

        for row in &processed_rows {
            for (i, cell) in row.iter().enumerate() {
                if i < cols {
                    widths[i] = widths[i].max(Self::visible_len(cell));
                }
            }
        }

        let total_width: usize = widths.iter().sum::<usize>() + cols * 3 + 1;

        if let Some(title) = &self.title {
            let pad = total_width.saturating_sub(Self::visible_len(title)) / 2;
            println!("{}{}", " ".repeat(pad), hex_bold(title, CYAN));
        }

        let draw_sep = |left: &str, mid: &str, right: &str, fill: char| {
            let mut line = String::from(left);
            for (i, w) in widths.iter().enumerate() {
                line.push_str(&fill.to_string().repeat(w + 2));
                line.push_str(if i + 1 < widths.len() { mid } else { right });
            }
            println!("{}", hex(&line, GREY));
        };

        draw_sep("\u{256d}", "\u{252c}", "\u{256e}", '\u{2500}');

        let mut header_line = String::from("\u{2502}");
        for (i, h) in self.headers.iter().enumerate() {
            let w = widths[i];
            let pad = w.saturating_sub(Self::visible_len(h));
            header_line.push(' ');
            header_line.push_str(&hex_bold(h, MAGENTA));
            header_line.push_str(&" ".repeat(pad + 1));
            header_line.push('\u{2502}');
        }
        println!("{header_line}");

        draw_sep("\u{251c}", "\u{253c}", "\u{2524}", '\u{2500}');

        for row in &processed_rows {
            let mut line = String::from("\u{2502}");
            for (i, _) in self.headers.iter().enumerate() {
                let cell = row.get(i).map(String::as_str).unwrap_or("");
                let w = widths[i];
                let pad = w.saturating_sub(Self::visible_len(cell));
                line.push(' ');
                line.push_str(cell);
                line.push_str(&" ".repeat(pad + 1));
                line.push('\u{2502}');
            }
            println!("{line}");
        }

        draw_sep("\u{2570}", "\u{2534}", "\u{256f}", '\u{2500}');
    }
}
