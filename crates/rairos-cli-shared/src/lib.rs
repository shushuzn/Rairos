//! Rairos CLI Shared — ANSI colors, printing utilities, Warp-style blocks
//!
//! Reference: Python cli/_shared.py
//!
//! Provides terminal colors, formatted output helpers, and Warp-style panels.

use serde::{Deserialize, Serialize};
use std::io::{self, IsTerminal};

// ============================================================================
// Colors
// ============================================================================

/// ANSI color codes for terminal output
#[derive(Debug, Clone, Copy)]
pub struct Colors;

impl Colors {
    pub const HEADER: &'static str = "\x1b[95m";
    pub const OKBLUE: &'static str = "\x1b[94m";
    pub const OKGREEN: &'static str = "\x1b[92m";
    pub const WARNING: &'static str = "\x1b[93m";
    pub const FAIL: &'static str = "\x1b[91m";
    pub const ENDC: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const UNDERLINE: &'static str = "\x1b[4m";
    pub const CYAN: &'static str = "\x1b[96m";
    pub const GREEN: &'static str = "\x1b[92m";
    pub const YELLOW: &'static str = "\x1b[93m";
    pub const END: &'static str = "\x1b[0m";
    pub const SUCCESS: &'static str = "\x1b[92m";
}

/// Return text with ANSI color code if stdout is a TTY
pub fn colored(text: &str, color: &str) -> String {
    if !io::stdout().is_terminal() {
        return text.to_string();
    }
    format!("{}{}{}", color, text, Colors::ENDC)
}

/// Print success message in green
pub fn print_success(message: &str) {
    println!("{}", colored(message, Colors::OKGREEN));
}

/// Print error message in red (to stderr)
pub fn print_error(message: &str) {
    eprintln!("{}", colored(message, Colors::FAIL));
}

/// Print warning message in yellow
pub fn print_warning(message: &str) {
    println!("{}", colored(message, Colors::WARNING));
}

/// Print info message in blue
pub fn print_info(message: &str) {
    println!("{}", colored(message, Colors::OKBLUE));
}

/// Print header message in purple
pub fn print_header(message: &str) {
    println!("{}", colored(message, Colors::HEADER));
}

// ============================================================================
// Warp-Style Blocks
// ============================================================================

/// Warp-style panel block
pub fn warp_panel(title: &str, body: &str, width: usize) -> String {
    let horizontal = "─".repeat(width.min(80));
    format!(
        "┌{}┐\n│ {} │\n├{}┤\n│ {} │\n└{}┘",
        horizontal,
        center_text(title, width.min(80)),
        horizontal,
        wrap_text(body, width.min(80)),
        horizontal
    )
}

/// Warp-style code block
pub fn warp_code_block(lang: &str, code: &str, title: Option<&str>, width: usize) -> String {
    let w = width.min(80);
    let header = if let Some(t) = title {
        format!("│ {} ({}): │", t, lang)
    } else {
        format!("│ {} code: │", lang)
    };
    let horizontal = "─".repeat(w);
    format!(
        "┌{}┐\n{}\n├{}┤\n│ {} │\n└{}┘",
        horizontal,
        header,
        horizontal,
        wrap_text(code, w),
        horizontal
    )
}

/// Warp-style section block
pub fn warp_section(title: &str, body_lines: &[&str], width: usize) -> String {
    let w = width.min(80);
    let horizontal = "─".repeat(w);
    let mut result = format!(
        "┌{}┐\n│ {} │\n├{}┤\n",
        horizontal,
        center_text(title, w),
        horizontal
    );
    for line in body_lines {
        result.push_str(&format!("│ {} │\n", pad_right(line, w)));
    }
    result.push_str(&format!("└{}┘", horizontal));
    result
}

/// Warp-style table block
pub fn warp_table(headers: &[&str], rows: &[Vec<&str>], width: usize) -> String {
    let w = width.min(80);
    let col_widths: Vec<usize> = headers
        .iter()
        .enumerate()
        .map(|(i, h)| {
            rows.iter()
                .map(|r| r.get(i).unwrap_or(&"").len())
                .max()
                .unwrap_or(h.len())
                .max(h.len())
        })
        .collect();

    let total_width: usize = col_widths.iter().sum::<usize>() + col_widths.len() * 3 + 1;
    let horizontal = "─".repeat(total_width.min(w));

    let mut result = format!("┌{}┐\n│ ", horizontal);
    for (i, h) in headers.iter().enumerate() {
        result.push_str(&format!("{:<width$} │ ", h, width = col_widths[i]));
    }
    result.push_str(&format!("\n├{}┤\n", horizontal));

    for row in rows {
        result.push_str("│ ");
        for (i, cell) in row.iter().enumerate() {
            result.push_str(&format!("{:<width$} │ ", cell, width = col_widths[i]));
        }
        result.push_str("\n");
    }
    result.push_str(&format!("└{}┘", horizontal));
    result
}

// ============================================================================
// Helpers
// ============================================================================

fn center_text(text: &str, width: usize) -> String {
    if text.len() >= width {
        text[..width.min(text.len())].to_string()
    } else {
        let padding = (width - text.len()) / 2;
        format!("{}{}", " ".repeat(padding), text)
    }
}

fn pad_right(text: &str, width: usize) -> String {
    if text.len() >= width {
        text[..width.min(text.len())].to_string()
    } else {
        format!("{}{}", text, " ".repeat(width - text.len()))
    }
}

fn wrap_text(text: &str, width: usize) -> String {
    let words: Vec<&str> = text.split_whitespace().collect();
    let mut lines: Vec<String> = Vec::new();
    let mut current_line = String::new();

    for word in words {
        if current_line.len() + word.len() + 1 <= width {
            if !current_line.is_empty() {
                current_line.push(' ');
            }
            current_line.push_str(word);
        } else {
            if !current_line.is_empty() {
                lines.push(current_line);
            }
            current_line = word.to_string();
        }
    }
    if !current_line.is_empty() {
        lines.push(current_line);
    }
    lines.join(&format!("\n│ "))
}

// ============================================================================
// Environment / Dotenv
// ============================================================================

/// Load .env from current working directory if present.
/// Only loads variables that are not already set.
pub fn load_dotenv() -> io::Result<()> {
    let env_file = std::path::Path::new(".env");
    if env_file.exists() {
        for line in std::fs::read_to_string(env_file)?.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if let Some((key, _, value)) = line.split_once('=') {
                let key = key.trim();
                let value = value.trim();
                if std::env::var(key).is_err() {
                    std::env::set_var(key, value);
                }
            }
        }
    }
    Ok(())
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_colored_returns_plain_when_not_tty() {
        // When not a TTY, should return plain text
        let result = colored("hello", Colors::OKGREEN);
        assert_eq!(result, "hello");
    }

    #[test]
    fn test_print_success_does_not_panic() {
        print_success("test message");
    }

    #[test]
    fn test_print_error_does_not_panic() {
        print_error("error message");
    }

    #[test]
    fn test_print_warning_does_not_panic() {
        print_warning("warning message");
    }

    #[test]
    fn test_print_info_does_not_panic() {
        print_info("info message");
    }

    #[test]
    fn test_print_header_does_not_panic() {
        print_header("header message");
    }

    #[test]
    fn test_warp_panel_format() {
        let result = warp_panel("Test", "Body", 60);
        assert!(result.contains("┌"));
        assert!(result.contains("┐"));
        assert!(result.contains("Test"));
    }

    #[test]
    fn test_warp_code_block_format() {
        let result = warp_code_block("rust", "fn main() {}", Some("Example"), 60);
        assert!(result.contains("rust"));
        assert!(result.contains("Example"));
    }

    #[test]
    fn test_warp_section_format() {
        let result = warp_section("Title", &["line1", "line2"], 60);
        assert!(result.contains("Title"));
        assert!(result.contains("line1"));
    }

    #[test]
    fn test_warp_table_format() {
        let headers = vec!["Name", "Age"];
        let rows = vec![vec!["Alice", "30"], vec!["Bob", "25"]];
        let result = warp_table(&headers, &rows, 60);
        assert!(result.contains("Name"));
        assert!(result.contains("Age"));
        assert!(result.contains("Alice"));
    }

    #[test]
    fn test_load_dotenv_nonexistent_file() {
        // Should not panic even if file doesn't exist
        let result = load_dotenv();
        assert!(result.is_ok());
    }

    #[test]
    fn test_center_text() {
        assert_eq!(center_text("hi", 5), "  hi");
        assert_eq!(center_text("hello", 5), "hello");
        assert_eq!(center_text("toolong", 3), "too");
    }

    #[test]
    fn test_pad_right() {
        assert_eq!(pad_right("hi", 5), "hi   ");
        assert_eq!(pad_right("hello", 5), "hello");
        assert_eq!(pad_right("toolong", 3), "too");
    }
}
