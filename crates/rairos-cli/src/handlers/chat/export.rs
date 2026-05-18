#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

pub fn export_chat_history(history: &[(String, String)], path: &str, fmt: Option<&str>) {
    let ext = std::path::Path::new(path)
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");
    let format = fmt.unwrap_or(match ext {
        "html" | "htm" => "html",
        _ => "markdown",
    });
    let content = match format {
        "html" => export_chat_to_html(history),
        _ => export_chat_to_markdown(history),
    };
    let _ = std::fs::write(path, content);
}

pub fn export_chat_to_markdown(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut md = format!("# AI Research OS — Chat Export\n\n**Exported**: {now}\n\n---\n\n", now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        md.push_str(&format!("## Q{i}: {q}\n\n**A**: {a}\n\n---\n\n", i = i + 1, q = q, a = a));
    }
    md
}

pub fn export_chat_to_html(history: &[(String, String)]) -> String {
    use chrono::Local;
    let now = Local::now().format("%Y-%m-%d %H:%M:%S");
    let mut html = format!(
        r#"<!DOCTYPE html>
<html lang='zh-CN'>
<head>
<meta charset='UTF-8'>
<title>AI Research OS — Chat Export</title>
<style>
body {{ font-family: 'Segoe UI', Arial, sans-serif; max-width: 800px; margin: 0 auto; padding: 20px; }}
h1 {{ color: #1a1a2e; border-bottom: 2px solid #4a4a8a; padding-bottom: 10px; }}
.qa-block {{ background: #f8f9fa; border-radius: 8px; padding: 15px; margin: 15px 0; }}
.question {{ color: #2a5a2a; font-weight: bold; }}
.answer {{ color: #333; margin-top: 10px; line-height: 1.6; }}
.meta {{ color: #666; font-size: 0.85em; }}
</style>
</head>
<body>
<h1>AI Research OS — Chat Export</h1>
<p class='meta'>Exported: {now}</p>
"#, now = now);
    for (i, (q, a)) in history.iter().enumerate() {
        html.push_str(&format!(
            r#"<div class='qa-block'>
<div class='question'>Q{i}: {q}</div>
<div class='answer'>{a}</div>
</div>
"#, i = i + 1, q = q, a = a));
    }
    html.push_str("</body>\n</html>");
    html
}