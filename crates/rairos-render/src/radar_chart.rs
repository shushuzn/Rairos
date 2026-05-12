//! Radar chart SVG renderer for paper rubric scores.
//!
//! Renders a 6-axis radar chart (Novelty, Leverage, Evidence, Cost, Moat, Adoption)
//! Each score is 1-5; larger is better on all axes. Cost is inverted so higher = cheaper.
//!
//! # Axes
//! - Novelty (创新性)
//! - Leverage (杠杆效应)
//! - Evidence (实验证据)
//! - Cost (成本)
//! - Moat (护城河)
//! - Adoption (采纳信号)

use std::collections::HashMap;

/// Radar chart axes definition.
const AXES: &[(&str, &str)] = &[
    ("Novelty", "创新性"),
    ("Leverage", "杠杆效应"),
    ("Evidence", "实验证据"),
    ("Cost", "成本"),
    ("Moat", "护城河"),
    ("Adoption", "采纳信号"),
];

/// Colour palette (professional, accessible).
const FILL_COLOUR: &str = "#3b82f6";
const STROKE_COLOUR: &str = "#1d4ed8";
const GRID_COLOUR: &str = "#94a3b8";
const LABEL_COLOUR: &str = "#334155";
const BG_COLOUR: &str = "#f8fafc";

/// Render a 6-axis radar chart SVG from rubric scores.
///
/// Each score should be 1-5; larger is better on all axes.
/// Cost is treated as "inverted" so higher score = lower cost = better.
///
/// # Arguments
/// * `scores` - HashMap with keys: novelty, leverage, evidence, cost, moat, adoption (1-5)
/// * `size` - SVG viewBox width/height (default 280)
///
/// # Returns
/// SVG markup string, or empty string if insufficient valid data.
pub fn render_radar_chart(scores: HashMap<String, i32>, size: usize) -> String {
    let size = size.max(100);
    let rings = 5; // 1-5 scale

    // Filter to only valid axes with scores in 1-5 range
    let valid_axes: Vec<(&&str, &&str, i32)> = AXES
        .iter()
        .filter_map(|(en, zh)| {
            let key = en.to_lowercase();
            let score = scores.get(&key).copied().unwrap_or(0);
            if (1..=5).contains(&score) {
                Some((en, zh, score))
            } else {
                None
            }
        })
        .collect();

    if valid_axes.len() < 3 {
        return String::new();
    }

    let n = valid_axes.len();
    let angle_step = 2.0 * std::f64::consts::PI / n as f64;

    let cx = size as f64 / 2.0;
    let cy = size as f64 / 2.0;
    let max_radius = size as f64 / 2.0 - 42.0; // leave room for labels

    let mut parts = Vec::new();

    parts.push(format!(
        r#"<svg viewBox="0 0 {} {}" xmlns="http://www.w3.org/2000/svg" role="img" aria-label="论文评分雷达图">"#,
        size, size
    ));
    parts.push("  <title>论文评分雷达图</title>".to_string());
    parts.push(format!(
        r#"  <rect width="{}" height="{}" fill="{}" rx="8"/>"#,
        size, size, BG_COLOUR
    ));

    // Grid rings
    for ring in 1..=rings {
        let r = max_radius * ring as f64 / rings as f64;
        let ring_pts: Vec<String> = (0..n)
            .map(|i| {
                let angle = i as f64 * angle_step - std::f64::consts::FRAC_PI_2;
                format!("{:.1},{:.1}", cx + r * angle.sin(), cy + r * angle.cos())
            })
            .collect();
        parts.push(format!(
            r#"  <polygon points="{}" fill="none" stroke="{}" stroke-width="0.6" stroke-dasharray="2,2"/>"#,
            ring_pts.join(" "),
            GRID_COLOUR
        ));
        if ring == rings {
            parts.push(format!(
                r#"  <text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">5</text>"#,
                cx,
                cy - r - 3.0,
                GRID_COLOUR
            ));
            parts.push(format!(
                r#"  <text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">{}</text>"#,
                cx,
                cy - r / 2.0 - 3.0,
                GRID_COLOUR,
                rings / 2
            ));
            parts.push(format!(
                r#"  <text x="{}" y="{}" text-anchor="middle" font-size="9" fill="{}">1</text>"#,
                cx,
                cy - 3.0,
                GRID_COLOUR
            ));
        }
    }

    // Axes (spokes)
    for i in 0..n {
        let angle = i as f64 * angle_step - std::f64::consts::FRAC_PI_2;
        let x2 = cx + max_radius * angle.sin();
        let y2 = cy + max_radius * angle.cos();
        parts.push(format!(
            r#"  <line x1="{:.1}" y1="{:.1}" x2="{:.1}" y2="{:.1}" stroke="{}" stroke-width="0.8"/>"#,
            cx, cy, x2, y2, GRID_COLOUR
        ));
    }

    // Data polygon
    let data_pts: Vec<String> = (0..n)
        .map(|i| {
            let angle = i as f64 * angle_step - std::f64::consts::FRAC_PI_2;
            let s = valid_axes[i].2 as f64;
            format!(
                "{:.1},{:.1}",
                cx + max_radius * (s / rings as f64) * angle.sin(),
                cy + max_radius * (s / rings as f64) * angle.cos()
            )
        })
        .collect();
    parts.push(format!(
        r#"  <polygon points="{}" fill="{}" fill-opacity="0.35" stroke="{}" stroke-width="1.5" stroke-linejoin="round"/>"#,
        data_pts.join(" "),
        FILL_COLOUR,
        STROKE_COLOUR
    ));

    // Data point dots
    for i in 0..n {
        let angle = i as f64 * angle_step - std::f64::consts::FRAC_PI_2;
        let s = valid_axes[i].2 as f64;
        let x = cx + max_radius * (s / rings as f64) * angle.sin();
        let y = cy + max_radius * (s / rings as f64) * angle.cos();
        parts.push(format!(
            r#"  <circle cx="{:.1}" cy="{:.1}" r="3" fill="{}" stroke="{}" stroke-width="1"/>"#,
            x, y, STROKE_COLOUR, BG_COLOUR
        ));
    }

    // Axis labels
    for i in 0..n {
        let angle = i as f64 * angle_step - std::f64::consts::FRAC_PI_2;
        let label_r = max_radius + 20.0;
        let lx = cx + label_r * angle.sin();
        let ly = cy + label_r * angle.cos();
        let s = valid_axes[i].2;
        let (en, zh, _) = valid_axes[i];

        let anchor = if lx > cx + 10.0 {
            "start"
        } else if lx < cx - 10.0 {
            "end"
        } else {
            "middle"
        };

        parts.push(format!(
            r#"  <text x="{:.1}" y="{:.1}" text-anchor="{}" dominant-baseline="middle" font-size="10.5" font-weight="600" fill="{}">{}</text>"#,
            lx, ly, anchor, LABEL_COLOUR, zh
        ));
        parts.push(format!(
            r#"  <text x="{:.1}" y="{:.1}" text-anchor="{}" dominant-baseline="middle" font-size="9" fill="{}">{}={}</text>"#,
            lx, ly + 13.0, anchor, GRID_COLOUR, en, s
        ));
    }

    // Summary badge
    let total: i32 = valid_axes.iter().map(|(_, _, s)| s).sum();
    let avg = total as f64 / n as f64;
    let badge_r = 18.0;
    let bx = cx + max_radius * 0.6;
    let by = cy - max_radius * 0.55;
    parts.push(format!(
        r#"  <circle cx="{:.1}" cy="{:.1}" r="{}" fill="{}" opacity="0.9"/>"#,
        bx, by, badge_r, STROKE_COLOUR
    ));
    parts.push(format!(
        r#"  <text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-size="11" font-weight="bold" fill="white">{:.1}</text>"#,
        bx, by - 3.0, avg
    ));
    parts.push(format!(
        r#"  <text x="{:.1}" y="{:.1}" text-anchor="middle" dominant-baseline="middle" font-size="8" fill="white">avg</text>"#,
        bx, by + 9.0
    ));

    parts.push("</svg>".to_string());
    parts.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::iter::FromIterator;

    #[test]
    fn test_render_radar_chart_valid() {
        let scores: HashMap<String, i32> = HashMap::from_iter([
            ("novelty".into(), 4),
            ("leverage".into(), 5),
            ("evidence".into(), 3),
            ("cost".into(), 4),
            ("moat".into(), 3),
            ("adoption".into(), 2),
        ]);
        let svg = render_radar_chart(scores, 280);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("</svg>"));
        assert!(svg.contains("polygon"));
    }

    #[test]
    fn test_render_radar_chart_empty() {
        let scores = HashMap::new();
        let svg = render_radar_chart(scores, 280);
        assert_eq!(svg, "");
    }

    #[test]
    fn test_render_radar_chart_partial() {
        let scores: HashMap<String, i32> =
            HashMap::from_iter([("novelty".into(), 4), ("leverage".into(), 5)]);
        let svg = render_radar_chart(scores, 280);
        // Only 2 valid axes - should return empty
        assert_eq!(svg, "");
    }
}
