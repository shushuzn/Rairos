//! rairos-updaters — Radar heat tracking and Timeline management
//!
//! Port of `updaters/radar.py` and `updaters/timeline.py` to Rust.

use rairos_core::constants::{RADAR_FILE, TIMELINE_FILE};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

use thiserror::Error;

// ============================================================================
// Constants
// ============================================================================

const RADAR_DIR: &str = "00-Radar";

// ============================================================================
// Errors
// ============================================================================

#[derive(Error, Debug)]
pub enum UpdatersError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("parse error: {0}")]
    Parse(String),
}

pub type Result<T> = std::result::Result<T, UpdatersError>;

// ============================================================================
// Radar Types
// ============================================================================

/// A single row in the radar table.
#[derive(Debug, Clone, Default)]
pub struct RadarRow {
    pub topic: String,
    pub heat: u32,
    pub evidence_quality: String,
    pub cost_change: String,
    pub confidence: String,
    pub last_updated: String,
}

/// Radar state: header text + table rows.
#[derive(Debug, Clone, Default)]
pub struct RadarState {
    pub header: String,
    pub rows: Vec<RadarRow>,
}

// ============================================================================
// Timeline Types
// ============================================================================

/// A single entry in a timeline year section.
#[derive(Debug, Clone)]
pub struct TimelineEntry {
    pub pnote_path: String,
    pub title: String,
}

/// Timeline state: header + per-year entries.
#[derive(Debug, Clone, Default)]
pub struct TimelineState {
    pub header: String,
    pub years: HashMap<String, Vec<TimelineEntry>>,
}

// ============================================================================
// Radar Implementation
// ============================================================================

fn radar_path(root: &Path) -> PathBuf {
    root.join(RADAR_DIR).join(RADAR_FILE)
}

fn default_radar_header() -> String {
    "# Radar（长期跟踪页）\n\n".to_string()
}

fn default_radar_table_header() -> &'static str {
    "| 主题 | 热度 | 证据质量 | 成本变化 | 我的信心 | 最近更新 |\n| -- | -- | ---- | ---- | ---- | ---- |\n"
}

fn parse_radar_table(contents: &str) -> (String, Vec<RadarRow>) {
    let lines: Vec<&str> = contents.lines().collect();
    let header_start = lines
        .iter()
        .position(|l| l.trim().starts_with("| 主题 |"))
        .unwrap_or(lines.len());

    let header = if header_start > 0 {
        lines[..header_start].join("\n") + "\n"
    } else {
        String::new()
    };

    let mut rows = Vec::new();
    for ln in lines.iter().skip(header_start + 2) {
        if !ln.trim().starts_with('|') {
            continue;
        }
        let cols: Vec<&str> = (*ln)
            .trim()
            .trim_matches('|')
            .split('|')
            .map(|c| c.trim())
            .collect();
        if cols.len() < 6 {
            continue;
        }
        let heat: u32 = cols[1].parse().unwrap_or(0);
        rows.push(RadarRow {
            topic: cols[0].to_string(),
            heat,
            evidence_quality: cols[2].to_string(),
            cost_change: cols[3].to_string(),
            confidence: cols[4].to_string(),
            last_updated: cols[5].to_string(),
        });
    }
    (header, rows)
}

fn render_radar(header: &str, rows: &[RadarRow]) -> String {
    let mut out = String::new();
    out.push_str(header.trim_end());
    out.push_str("\n\n");
    out.push_str(default_radar_table_header());
    for r in rows {
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            r.topic, r.heat, r.evidence_quality, r.cost_change, r.confidence, r.last_updated
        ));
    }
    out.trim_end().to_string() + "\n"
}

/// Ensure the radar file exists with default content.
pub fn ensure_radar(root: &Path) -> Result<PathBuf> {
    let p = radar_path(root);
    if p.exists() {
        return Ok(p);
    }
    let content = format!(
        "{}\n{}",
        default_radar_header().trim_end(),
        default_radar_table_header().trim_end()
    );
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&p, content)?;
    Ok(p)
}

/// Read the current radar state from disk.
pub fn read_radar(root: &Path) -> Result<RadarState> {
    let p = radar_path(root);
    let contents = if p.exists() {
        fs::read_to_string(&p)?
    } else {
        String::new()
    };
    let (header, rows) = parse_radar_table(&contents);
    Ok(RadarState { header, rows })
}

/// Update radar with incremented heat for the given topics.
pub fn update_radar(root: &Path, tags: &[String], note_date: &str) -> Result<PathBuf> {
    let p = ensure_radar(root)?;
    let contents = fs::read_to_string(&p)?;
    let (header, mut rows) = parse_radar_table(&contents);
    let mut row_map: HashMap<String, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.topic.clone(), i))
        .collect();

    for tag in tags {
        if let Some(idx) = row_map.get(tag) {
            rows[*idx].heat += 1;
            rows[*idx].last_updated = note_date.to_string();
        } else {
            rows.push(RadarRow {
                topic: tag.clone(),
                heat: 1,
                evidence_quality: String::new(),
                cost_change: String::new(),
                confidence: String::new(),
                last_updated: note_date.to_string(),
            });
            row_map.insert(tag.clone(), rows.len() - 1);
        }
    }

    rows.sort_by(|a, b| {
        b.heat
            .cmp(&a.heat)
            .then_with(|| a.topic.to_lowercase().cmp(&b.topic.to_lowercase()))
    });

    let new_contents = render_radar(&header, &rows);
    fs::write(&p, new_contents)?;
    Ok(p)
}

/// Write the given radar state to disk.
pub fn write_radar(root: &Path, state: &RadarState) -> Result<PathBuf> {
    let p = ensure_radar(root)?;
    let contents = render_radar(&state.header, &state.rows);
    fs::write(&p, contents)?;
    Ok(p)
}

// ============================================================================
// Timeline Implementation
// ============================================================================

fn timeline_path(root: &Path) -> PathBuf {
    root.join(RADAR_DIR).join(TIMELINE_FILE)
}

fn default_timeline_header() -> String {
    "# Timeline（技术演进）\n\n按年份记录关键论文与技术拐点。\n\n".to_string()
}

fn is_year_section(line: &str) -> Option<String> {
    let line = line.trim();
    if line.len() >= 6 && line.starts_with("## ") {
        let after_hash = &line[3..];
        let year_chars: String = after_hash
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if year_chars.len() == 4 {
            return Some(year_chars);
        }
    }
    None
}

/// Parse a timeline file into header + per-year entries.
pub fn parse_timeline(contents: &str) -> TimelineState {
    let mut years: HashMap<String, Vec<TimelineEntry>> = HashMap::new();
    let mut header = String::new();

    let mut current_year: Option<String> = None;
    let mut current_entries: Vec<TimelineEntry> = Vec::new();

    for line in contents.lines() {
        if let Some(year) = is_year_section(line) {
            // Save previous year section
            if let Some(y) = current_year.take() {
                if !current_entries.is_empty() {
                    years.insert(y, std::mem::take(&mut current_entries));
                }
            }
            header.push_str(line);
            header.push('\n');
            current_year = Some(year);
        } else if current_year.is_some() {
            let line = line.trim();
            if let Some(body) = line.strip_prefix("- ") {
                let entry = if let Some(at) = body.find(" — ") {
                    // " — " separator: space(1) + em-dash(3) + space(1) = 5 bytes
                    let em_dash_bytes = 3;
                    let sep_len = 1 + em_dash_bytes + 1; // space + em-dash + space
                    let title_part = &body[..at];
                    let rest = &body[at + sep_len..];
                    let (pnote_path, title) = if title_part.starts_with('[') {
                        if let Some(close_paren) = title_part.find("](") {
                            let path = title_part[1..close_paren].to_string();
                            (path, String::new())
                        } else if let Some(close) = title_part.find(']') {
                            let path = title_part[1..close].to_string();
                            let title = title_part[close + 1..].trim().to_string();
                            (path, title)
                        } else {
                            (title_part.to_string(), String::new())
                        }
                    } else {
                        (String::new(), title_part.to_string())
                    };
                    TimelineEntry {
                        pnote_path,
                        title: if title.is_empty() {
                            rest.to_string()
                        } else {
                            format!("{} — {}", title, rest)
                        },
                    }
                } else {
                    TimelineEntry {
                        pnote_path: String::new(),
                        title: body.to_string(),
                    }
                };
                current_entries.push(entry);
            } else if !line.is_empty() {
                header.push_str(line);
                header.push('\n');
            }
        } else {
            header.push_str(line);
            header.push('\n');
        }
    }

    if let Some(y) = current_year {
        if !current_entries.is_empty() {
            years.insert(y, current_entries);
        }
    }

    TimelineState { header, years }
}

/// Serialize a timeline state back to markdown.
pub fn render_timeline(state: &TimelineState) -> String {
    let mut out = state.header.trim_end().to_string();
    out.push_str("\n\n");

    let mut years: Vec<(&String, &Vec<TimelineEntry>)> = state.years.iter().collect();
    years.sort_by_key(|(y, _)| *y);

    for (year, entries) in years {
        out.push_str("## ");
        out.push_str(year);
        out.push('\n');
        for e in entries {
            out.push_str("- ");
            if !e.pnote_path.is_empty() {
                out.push('[');
                out.push_str(&e.pnote_path);
                out.push_str("](");
                out.push_str(&e.pnote_path);
                out.push_str(") — ");
            }
            out.push_str(&e.title);
            out.push('\n');
        }
        out.push('\n');
    }

    out.trim_end().to_string() + "\n"
}

/// Ensure the timeline file exists with default content.
pub fn ensure_timeline(root: &Path) -> Result<PathBuf> {
    let p = timeline_path(root);
    if p.exists() {
        return Ok(p);
    }
    if let Some(parent) = p.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(&p, default_timeline_header())?;
    Ok(p)
}

/// Read the current timeline state from disk.
pub fn read_timeline(root: &Path) -> Result<TimelineState> {
    let p = timeline_path(root);
    let contents = if p.exists() {
        fs::read_to_string(&p)?
    } else {
        String::new()
    };
    Ok(parse_timeline(&contents))
}

/// Add a paper milestone to the timeline under the given year.
pub fn update_timeline(root: &Path, year: &str, pnote_path: &str, title: &str) -> Result<PathBuf> {
    let p = ensure_timeline(root)?;
    let contents = fs::read_to_string(&p)?;
    let mut state = parse_timeline(&contents);

    let year_entries = state.years.entry(year.to_string()).or_default();

    // Check for duplicate
    let new_entry = TimelineEntry {
        pnote_path: pnote_path.to_string(),
        title: title.to_string(),
    };
    for e in year_entries.iter() {
        if e.pnote_path == pnote_path && e.title == title {
            return Ok(p);
        }
    }

    year_entries.push(new_entry);
    let new_contents = render_timeline(&state);
    fs::write(&p, new_contents)?;
    Ok(p)
}

/// Write the given timeline state to disk.
pub fn write_timeline(root: &Path, state: &TimelineState) -> Result<PathBuf> {
    let p = ensure_timeline(root)?;
    let contents = render_timeline(state);
    fs::write(&p, contents)?;
    Ok(p)
}

use std::path::PathBuf;

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_ensure_radar_creates_file() {
        let tmp = TempDir::new().unwrap();
        let p = ensure_radar(tmp.path()).unwrap();
        assert!(p.exists());
        let contents = fs::read_to_string(&p).unwrap();
        assert!(contents.contains("Radar"));
        assert!(contents.contains("| 主题 |"));
    }

    #[test]
    fn test_update_radar_increments_heat() {
        let tmp = TempDir::new().unwrap();
        ensure_radar(tmp.path()).unwrap();

        update_radar(tmp.path(), &["LLM".into(), "LLM".into()], "2025-01-01").unwrap();

        let state = read_radar(tmp.path()).unwrap();
        assert_eq!(state.rows.len(), 1);
        assert_eq!(state.rows[0].heat, 2);
        assert_eq!(state.rows[0].topic, "LLM");
    }

    #[test]
    fn test_update_radar_adds_new_topic() {
        let tmp = TempDir::new().unwrap();
        ensure_radar(tmp.path()).unwrap();

        update_radar(tmp.path(), &["RAG".into()], "2025-01-01").unwrap();
        update_radar(tmp.path(), &["Agents".into()], "2025-01-02").unwrap();

        let state = read_radar(tmp.path()).unwrap();
        assert_eq!(state.rows.len(), 2);
    }

    #[test]
    fn test_parse_timeline_basic() {
        // Uses em-dash " — " format (5 bytes per separator) matching render_timeline output
        let contents = "# Timeline\n\n## 2024\n\n- [p1](p1.md) — Paper One\n- Title Only\n\n## 2023\n\n- Another entry\n";
        let state = parse_timeline(contents);
        assert!(state.header.contains("Timeline"));
        assert_eq!(state.years.len(), 2);
        assert_eq!(state.years.get("2024").unwrap().len(), 2);
        assert_eq!(state.years.get("2023").unwrap().len(), 1);
    }

    #[test]
    fn test_update_timeline_no_duplicate() {
        let tmp = TempDir::new().unwrap();
        ensure_timeline(tmp.path()).unwrap();

        // Uses em-dash " — " matching render_timeline output
        update_timeline(tmp.path(), "2024", "p1.md", "Paper One").unwrap();
        update_timeline(tmp.path(), "2024", "p1.md", "Paper One").unwrap();

        let state = read_timeline(tmp.path()).unwrap();
        // Same entry added twice → duplicate detection should keep only 1
        assert_eq!(state.years.get("2024").unwrap().len(), 1);
    }
}
