//! rairos-user-helpers — User-friendly error messages and CLI helpers.
//!
//! Ported from `core/user_helpers.py`.

use std::fmt;

/// Base user-friendly error with optional suggestion.
#[derive(Debug)]
pub struct UserError {
    pub message: String,
    pub suggestion: Option<String>,
}

impl UserError {
    pub fn new(message: &str, suggestion: Option<&str>) -> Self {
        Self {
            message: message.to_string(),
            suggestion: suggestion.map(|s| s.to_string()),
        }
    }

    pub fn helpful_message(&self) -> String {
        let mut msg = format!("❌ {}", self.message);
        if let Some(ref s) = self.suggestion {
            msg.push_str(&format!("\n💡 建议: {}", s));
        }
        msg
    }
}

impl fmt::Display for UserError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for UserError {}

/// Database-related error.
#[derive(Debug)]
pub struct DatabaseError {
    inner: UserError,
}

impl DatabaseError {
    pub fn not_found(item: &str, item_id: &str) -> Self {
        Self {
            inner: UserError::new(
                &format!("未找到 {}: {}", item, item_id),
                Some(&format!(
                    "请检查 {} 是否正确，或使用 'search' 命令搜索相关 {}",
                    item_id, item
                )),
            ),
        }
    }

    pub fn connection_failed() -> Self {
        Self {
            inner: UserError::new(
                "数据库连接失败",
                Some("请确保数据库文件存在，或运行 'python cli.py init' 初始化数据库"),
            ),
        }
    }

    pub fn helpful_message(&self) -> String {
        self.inner.helpful_message()
    }
}

impl fmt::Display for DatabaseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.message)
    }
}

impl std::error::Error for DatabaseError {}

/// API-related error.
#[derive(Debug)]
pub struct APIError {
    inner: UserError,
}

impl APIError {
    pub fn rate_limit(endpoint: &str, wait_seconds: u32) -> Self {
        Self {
            inner: UserError::new(
                &format!("API请求过于频繁 ({})", endpoint),
                Some(&format!(
                    "请等待 {} 秒后重试，或使用 'rate-limit' 命令查看API使用统计",
                    wait_seconds
                )),
            ),
        }
    }

    pub fn network_failed() -> Self {
        Self {
            inner: UserError::new("网络连接失败", Some("请检查网络连接，或使用代理设置")),
        }
    }

    pub fn auth_failed() -> Self {
        Self {
            inner: UserError::new(
                "API认证失败",
                Some("请检查API密钥是否正确，或使用 'export OPENAI_API_KEY=your-key' 设置"),
            ),
        }
    }

    pub fn helpful_message(&self) -> String {
        self.inner.helpful_message()
    }
}

impl fmt::Display for APIError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.message)
    }
}

impl std::error::Error for APIError {}

/// Paper parsing error.
#[derive(Debug)]
pub struct ParseError {
    inner: UserError,
}

impl ParseError {
    pub fn pdf_failed(paper_id: &str) -> Self {
        Self {
            inner: UserError::new(
                &format!("解析论文失败: {}", paper_id),
                Some("请检查PDF文件是否可访问，或使用 '--no-pdf' 跳过PDF下载"),
            ),
        }
    }

    pub fn helpful_message(&self) -> String {
        self.inner.helpful_message()
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.inner.message)
    }
}

impl std::error::Error for ParseError {}

/// Format an error for user display.
pub fn format_error(error: &dyn std::error::Error) -> String {
    // Check concrete error types by matching on the error's Display
    let msg = error.to_string();
    if msg.starts_with("未找到") {
        format!(
            "❌ {}\n💡 建议: 请检查是否正确，或使用 'search' 命令搜索相关",
            msg
        )
    } else if msg.contains("API") && msg.contains("频繁") {
        format!("❌ {}\n💡 建议: 请等待后重试", msg)
    } else if msg.starts_with("网络连接失败") {
        format!("❌ {}\n💡 建议: 请检查网络连接，或使用代理设置", msg)
    } else if msg.starts_with("API认证失败") {
        format!("❌ {}\n💡 建议: 请检查API密钥是否正确", msg)
    } else if msg.starts_with("解析论文失败") {
        format!("❌ {}\n💡 建议: 请检查PDF文件是否可访问", msg)
    } else {
        format!("❌ 发生错误: {}", msg)
    }
}

/// Simple progress indicator for long operations.
pub struct ProgressIndicator {
    total: usize,
    current: usize,
    description: String,
}

impl ProgressIndicator {
    pub fn new(total: usize, description: &str) -> Self {
        Self {
            total,
            current: 0,
            description: description.to_string(),
        }
    }

    pub fn update(&mut self, increment: usize) {
        self.current += increment;
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };
        print!(
            "\r{}: {}/{} ({}%)",
            self.description, self.current, self.total, percentage
        );
        std::io::Write::flush(&mut std::io::stdout()).ok();
    }

    pub fn finish(self) {
        let percentage = if self.total > 0 {
            (self.current as f64 / self.total as f64 * 100.0) as usize
        } else {
            0
        };
        println!(
            "\r{}: 完成！{}/{} ({}%)",
            self.description, self.current, self.total, percentage
        );
    }
}

impl Default for ProgressIndicator {
    fn default() -> Self {
        Self::new(0, "处理中")
    }
}

/// Print data as a formatted ASCII table.
pub fn print_table(headers: &[&str], rows: &[Vec<&str>], max_width: usize) {
    if headers.is_empty() {
        return;
    }

    let col_count = headers.len();
    let mut col_widths: Vec<usize> = headers.iter().map(|h| h.len()).collect();

    for row in rows {
        for (i, cell) in row.iter().take(col_count).enumerate() {
            col_widths[i] = col_widths[i].max(cell.chars().count().min(max_width));
        }
    }

    // Print headers
    let header_line: String = headers
        .iter()
        .enumerate()
        .map(|(i, h)| h.chars().take(col_widths[i]).collect::<String>())
        .collect::<Vec<_>>()
        .join(" | ");
    println!("{}", header_line);
    println!("{}", "-".repeat(header_line.chars().count()));

    // Print rows
    for row in rows {
        let row_line: String = row
            .iter()
            .take(col_count)
            .enumerate()
            .map(|(i, cell)| cell.chars().take(col_widths[i]).collect::<String>())
            .collect::<Vec<_>>()
            .join(" | ");
        println!("{}", row_line);
    }
}

/// Print a banner.
pub fn print_banner(text: &str, width: usize) {
    let padding = (width.saturating_sub(text.len())) / 2;
    println!("{}", "=".repeat(width));
    println!("{}", " ".repeat(padding) + text);
    println!("{}", "=".repeat(width));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_user_error() {
        let e = UserError::new("something went wrong", Some("try again"));
        assert_eq!(e.message, "something went wrong");
        assert!(e.helpful_message().contains("💡 建议: try again"));
    }

    #[test]
    fn test_database_error_not_found() {
        let e = DatabaseError::not_found("paper", "abc123");
        assert!(e.helpful_message().contains("未找到 paper: abc123"));
    }

    #[test]
    fn test_api_error_rate_limit() {
        let e = APIError::rate_limit("/search", 30);
        assert!(e.helpful_message().contains("API请求过于频繁"));
    }

    #[test]
    fn test_parse_error() {
        let e = ParseError::pdf_failed("2401.12345");
        assert!(e.helpful_message().contains("解析论文失败"));
    }

    #[test]
    fn test_progress_indicator() {
        let mut p = ProgressIndicator::new(100, "测试");
        p.update(50);
        p.finish();
    }

    #[test]
    fn test_print_banner() {
        print_banner("Hello", 40);
    }
}
