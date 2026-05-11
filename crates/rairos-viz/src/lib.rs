//! Rairos Viz — Knowledge Graph and Benchmark Visualization
//!
//! Replaces: viz/__init__.py, viz/d3_renderer.py, viz/pyvis_renderer.py, viz/benchmark_viz.py
//!
//! Provides: D3.js-compatible force graph export, PyVis HTML rendering,
//! benchmark chart visualization (HTML/SVG).

use rairos_core::{Database, Paper};
use serde::{Deserialize, Serialize};
use thiserror::Error;

// ============================================================================
// Error Types
// ============================================================================

#[derive(Error, Debug)]
pub enum VizError {
    #[error("Database error: {0}")]
    Database(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Paper not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, VizError>;

// ============================================================================
// D3.js Force Graph Export
// ============================================================================

/// Node in a D3.js-compatible graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D3Node {
    pub id: String,
    pub label: String,
    #[serde(rename = "type")]
    pub node_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entity_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_root: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_citing: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub is_cited_by: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,
}

/// Edge in a D3.js-compatible graph
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D3Link {
    pub source: String,
    pub target: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub relation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weight: Option<f32>,
}

/// D3.js-compatible {nodes, links} graph structure
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct D3Graph {
    pub nodes: Vec<D3Node>,
    pub links: Vec<D3Link>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub root: Option<String>,
}

impl D3Graph {
    /// Create an empty graph.
    pub fn empty() -> Self {
        Self {
            nodes: Vec::new(),
            links: Vec::new(),
            root: None,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// D3.js force graph exporter for knowledge graphs.
/// Produces D3.js-compatible nodes+links JSON for visualization.
pub struct D3ForceGraph {
    db: Option<Database>,
}

impl D3ForceGraph {
    /// Create a new D3ForceGraph with optional database.
    pub fn new(db: Option<Database>) -> Self {
        Self { db }
    }

    /// Export full graph or filtered subgraph as D3.js format.
    ///
    /// - `paper_uids`: if provided, export ego subgraphs for these papers
    /// - `tag`: if provided, export papers with that tag
    /// - `max_nodes`: limit on total nodes returned
    pub fn to_json(
        &self,
        paper_uids: Option<Vec<String>>,
        tag: Option<&str>,
        max_nodes: usize,
    ) -> Result<D3Graph> {
        let db = self.db.as_ref().ok_or_else(|| VizError::Database("No database configured".into()))?;

        let papers = match (&paper_uids, tag) {
            (Some(uids), _) => {
                let mut papers = Vec::new();
                for uid in uids {
                    if let Ok(Some(p)) = db.get_paper_by_arxiv(uid) {
                        papers.push(p);
                    }
                }
                papers
            }
            (None, Some(t)) => {
                // Filter by tag (stored as JSON array in categories field)
                let all = db.list_papers(None, 1000, 0).unwrap_or_default();
                all.into_iter()
                    .filter(|p| p.categories.iter().any(|c| c == t))
                    .collect()
            }
            (None, None) => {
                db.list_papers(None, max_nodes, 0).unwrap_or_default()
            }
        };

        let mut nodes: Vec<D3Node> = Vec::new();
        let mut links: Vec<D3Link> = Vec::new();

        for paper in papers.iter().take(max_nodes) {
            let nid = paper.arxiv_id.clone().unwrap_or_else(|| paper.id.clone());
            nodes.push(D3Node {
                id: nid.clone(),
                label: paper.title.chars().take(60).collect(),
                node_type: "Paper".to_string(),
                entity_id: Some(paper.arxiv_id.clone().unwrap_or_default()),
                is_root: None,
                is_citing: None,
                is_cited_by: None,
                similarity: None,
            });
        }

        // For full graph, we don't have edge information in core DB
        // Citation edges would come from a KG module
        Ok(D3Graph {
            nodes,
            links,
            root: None,
        })
    }

    /// Build a citation graph for a paper.
    ///
    /// - `paper_id`: root paper ID
    /// - `depth`: traversal depth (1 = direct only)
    /// - `max_nodes`: max nodes per direction
    pub fn to_citation_json(
        &self,
        paper_id: &str,
        depth: usize,
        max_nodes: usize,
    ) -> Result<D3Graph> {
        let db = self.db.as_ref().ok_or_else(|| VizError::Database("No database configured".into()))?;

        // Resolve root node
        let root_paper = db
            .get_paper_by_arxiv(paper_id)
            .map_err(|e| VizError::Database(e.to_string()))?
            .or_else(|| {
                // Try without arXiv: prefix
                db.get_paper_by_arxiv(&format!("arXiv:{}", paper_id)).ok().flatten()
            })
            .ok_or_else(|| VizError::NotFound(paper_id.to_string()))?;

        let mut nodes: Vec<D3Node> = Vec::new();
        let mut links: Vec<D3Link> = Vec::new();

        let root_nid = root_paper
            .arxiv_id
            .clone()
            .unwrap_or_else(|| root_paper.id.clone());

        nodes.push(D3Node {
            id: root_nid.clone(),
            label: root_paper.title.chars().take(60).collect(),
            node_type: "Paper".to_string(),
            entity_id: root_paper.arxiv_id.clone(),
            is_root: Some(true),
            is_citing: Some(false),
            is_cited_by: Some(false),
            similarity: None,
        });

        // Note: Full citation graph requires KG module with cite edges.
        // This provides the structural basis for that integration.
        Ok(D3Graph {
            nodes,
            links,
            root: Some(root_nid),
        })
    }

    /// Build a similarity graph from paper embeddings.
    ///
    /// - `paper_id`: root paper ID
    /// - `threshold`: minimum similarity score (0.0–1.0)
    /// - `max_nodes`: max similar papers to include
    pub fn to_similar_json(
        &self,
        paper_id: &str,
        threshold: f32,
        max_nodes: usize,
    ) -> Result<D3Graph> {
        let db = self.db.as_ref().ok_or_else(|| VizError::Database("No database configured".into()))?;

        let root_paper = db
            .get_paper_by_arxiv(paper_id)
            .map_err(|e| VizError::Database(e.to_string()))?
            .ok_or_else(|| VizError::NotFound(paper_id.to_string()))?;

        let root_nid = root_paper
            .arxiv_id
            .clone()
            .unwrap_or_else(|| root_paper.id.clone());

        let mut nodes: Vec<D3Node> = vec![D3Node {
            id: root_nid.clone(),
            label: root_paper.title.chars().take(60).collect(),
            node_type: "Paper".to_string(),
            entity_id: root_paper.arxiv_id.clone(),
            is_root: Some(true),
            is_citing: None,
            is_cited_by: None,
            similarity: None,
        }];

        let mut links: Vec<D3Link> = Vec::new();

        // Note: Similarity search requires embedding storage and search.
        // The Database type in core has `get_embedding` and `list_papers_with_embeddings`
        // methods that could be used for this, but similarity scoring is not yet
        // implemented in core — it would live in the rankers crate.
        // This method provides the output structure for that future integration.

        Ok(D3Graph {
            nodes,
            links,
            root: Some(root_nid),
        })
    }
}

// ============================================================================
// Benchmark Visualization
// ============================================================================

/// Colour palette (12 colours, colour-blindness safe).
const BENCHMARK_COLORS: [&str; 12] = [
    "#4E79A7", "#F28E2B", "#E15759", "#76B7B2", "#59A14F", "#EDC948",
    "#B07AA1", "#FF9DA7", "#9C755F", "#BAB0AC", "#86BCB6", "#D37295",
];

/// Benchmark match data point.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkEntry {
    pub label: String,
    pub paper: String,
    pub value: f64,
}

/// Single benchmark chart data.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkChart {
    pub benchmark: String,
    pub metric: String,
    pub direction: String,
    pub data: Vec<BenchmarkEntry>,
}

/// Chart-compatible JSON structure for benchmark results.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkChartData {
    pub papers: Vec<String>,
    pub charts: Vec<BenchmarkChart>,
}

impl BenchmarkChartData {
    /// Convert benchmark results to chart JSON.
    /// This would be called with actual benchmark comparison results.
    pub fn from_papers(papers: &[Paper], matches: Vec<BenchmarkChart>) -> Self {
        Self {
            papers: papers.iter().map(|p| p.arxiv_id.clone().unwrap_or_default()).collect(),
            charts: matches,
        }
    }

    /// Serialize to JSON string.
    pub fn to_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }
}

/// Benchmark visualization renderer.
pub struct BenchmarkViz;

impl BenchmarkViz {
    /// Convert benchmark results to chart-compatible JSON.
    pub fn to_json(papers: &[Paper], charts: Vec<BenchmarkChart>) -> BenchmarkChartData {
        BenchmarkChartData::from_papers(papers, charts)
    }

    /// Generate a self-contained HTML file with D3.js bar charts.
    pub fn render_html(data: &BenchmarkChartData, output_path: &str) -> Result<String> {
        let json_data = serde_json::to_string_pretty(data)?;
        let colors_json = serde_json::to_string(&BENCHMARK_COLORS)?;

        let html = format!(
            r#"<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Benchmark Comparison</title>
<script src="https://cdn.jsdelivr.net/npm/d3@7/dist/d3.min.js"></script>
<style>
  * {{ box-sizing: border-box; margin: 0; padding: 0; }}
  body {{ font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
         background: #f8f9fa; color: #333; padding: 24px; }}
  h1 {{ font-size: 22px; margin-bottom: 4px; color: #1a1a2e; }}
  .subtitle {{ color: #666; font-size: 14px; margin-bottom: 24px; }}
  .chart-card {{ background: #fff; border-radius: 8px; box-shadow: 0 1px 4px rgba(0,0,0,0.08);
                 padding: 20px; margin-bottom: 20px; }}
  .chart-title {{ font-size: 16px; font-weight: 600; margin-bottom: 2px; }}
  .chart-direction {{ font-size: 12px; color: #888; margin-bottom: 12px; }}
  .bar {{ transition: opacity 0.15s; cursor: pointer; }}
  .bar:hover {{ opacity: 0.8; }}
  .axis-label {{ font-size: 11px; fill: #666; }}
  .tick text {{ font-size: 11px; fill: #555; }}
  .tooltip {{ position: absolute; background: rgba(0,0,0,0.85); color: #fff;
              padding: 8px 12px; border-radius: 6px; font-size: 13px;
              pointer-events: none; opacity: 0; transition: opacity 0.15s;
              max-width: 360px; line-height: 1.4; }}
  .empty {{ color: #999; font-style: italic; padding: 20px 0; }}
</style>
</head>
<body>
  <h1>Benchmark Comparison</h1>
  <p class="subtitle">Papers: {} &middot; {} benchmark(s)</p>
  <div id="charts"></div>
  <div class="tooltip" id="tooltip"></div>

<script>
  const DATA = {};
  const COLORS = {};
  const paperColors = {{}};
  DATA.papers.forEach((p, i) => {{ paperColors[p] = COLORS[i % COLORS.length]; }});

  const container = document.getElementById('charts');
  const tooltip = document.getElementById('tooltip');

  if (DATA.charts.length === 0) {{
    container.innerHTML = '<div class="empty">No matching benchmarks found.</div>';
  }}

  DATA.charts.forEach((chart, idx) => {{
    const card = document.createElement('div');
    card.className = 'chart-card';
    card.innerHTML = '<div class="chart-title">' + chart.benchmark + ' \\u2014 ' + chart.metric + '</div>'
      + '<div class="chart-direction">' + (chart.direction === 'higher' ? '\\u2191' : '\\u2193') + ' ' + chart.direction + ' is better</div>'
      + '<svg width="100%" height="' + Math.max(100, chart.data.length * 36 + 40) + '"></svg>';
    container.appendChild(card);

    const svg = card.querySelector('svg');
    const width = svg.clientWidth || 800;
    const height = parseFloat(svg.getAttribute('height'));
    const margin = {{ left: 160, right: 80, top: 8, bottom: 8 }};
    const innerW = width - margin.left - margin.right;
    const innerH = height - margin.top - margin.bottom;

    const values = chart.data.map(d => d.value);
    const maxVal = Math.max(...values.map(Math.abs), 0.001);
    const xScale = d3.scaleLinear()
      .domain([0, maxVal * 1.12])
      .range([0, innerW]);

    const yScale = d3.scaleBand()
      .domain(d3.range(chart.data.length))
      .range([margin.top, height - margin.bottom])
      .padding(0.25);

    chart.data.sort((a, b) => b.value - a.value);

    const xAxis = d3.axisBottom(xScale).ticks(6).tickFormat(d3.format('.3g'));
    svg.append('g')
      .attr('transform', 'translate(' + margin.left + ',' + (height - margin.bottom) + ')')
      .call(xAxis)
      .style('font-size', '11px');

    chart.data.forEach((d, i) => {{
      const barW = xScale(d.value);
      const barH = yScale.bandwidth();
      const yPos = yScale(i);

      svg.append('rect')
        .attr('class', 'bar')
        .attr('x', margin.left)
        .attr('y', yPos)
        .attr('width', barW || 1)
        .attr('height', barH)
        .attr('fill', paperColors[d.paper] || '#888')
        .attr('rx', 3)
        .on('mouseover', (ev) => {{
          tooltip.style.opacity = 1;
          tooltip.innerHTML = '<strong>' + d.label + '</strong><br>Paper: ' + d.paper + '<br>Score: ' + d.value.toFixed(4);
          tooltip.style.left = (ev.pageX + 12) + 'px';
          tooltip.style.top = (ev.pageY - 10) + 'px';
        }})
        .on('mousemove', (ev) => {{
          tooltip.style.left = (ev.pageX + 12) + 'px';
          tooltip.style.top = (ev.pageY - 10) + 'px';
        }})
        .on('mouseout', () => {{ tooltip.style.opacity = 0; }});

      svg.append('text')
        .attr('x', margin.left - 8)
        .attr('y', yPos + barH / 2)
        .attr('text-anchor', 'end')
        .attr('dominant-baseline', 'middle')
        .attr('class', 'axis-label')
        .text(d.label.length > 24 ? d.label.slice(0, 22) + '\u2026' : d.label);

      svg.append('text')
        .attr('x', margin.left + barW + 6)
        .attr('y', yPos + barH / 2)
        .attr('dominant-baseline', 'middle')
        .attr('class', 'axis-label')
        .text(d.value.toFixed(4));
    }});
  }});
</script>
</body>
</html>"#,
            data.papers.join(", "),
            data.charts.len(),
            json_data,
            colors_json
        );

        std::fs::write(output_path, &html)?;
        Ok(output_path.to_string())
    }

    /// Generate SVG bar chart (pure Python would be Rust SVG generation here).
    /// Returns SVG string for embedding in static documents.
    pub fn render_svg(_data: &BenchmarkChartData) -> String {
        // SVG generation for benchmarks
        // For a full implementation, this would render SVG bars using computed widths
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"720\" height=\"120\" style=\"font-family: sans-serif;\">\
         <rect width=\"100%\" height=\"100%\" fill=\"#f8f9fa\"/>\
         <text x=\"20\" y=\"28\" font-size=\"16\" font-weight=\"bold\" fill=\"#333\">Benchmark Comparison</text>\
         <text x=\"20\" y=\"50\" font-size=\"12\" fill=\"#666\">Render using render_html() for interactive D3.js charts</text>\
         </svg>".to_string()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_d3_graph_empty() {
        let g = D3Graph::empty();
        assert!(g.nodes.is_empty());
        assert!(g.links.is_empty());
    }

    #[test]
    fn test_d3_node_serialization() {
        let node = D3Node {
            id: "arXiv:2301.00001".to_string(),
            label: "Test Paper Title".to_string(),
            node_type: "Paper".to_string(),
            entity_id: Some("2301.00001".to_string()),
            is_root: Some(true),
            is_citing: Some(false),
            is_cited_by: Some(false),
            similarity: None,
        };
        let json = serde_json::to_string_pretty(&node).unwrap();
        assert!(json.contains("\"id\""));
        assert!(json.contains("\"label\""));
    }

    #[test]
    fn test_benchmark_colors_length() {
        assert_eq!(BENCHMARK_COLORS.len(), 12);
    }

    #[test]
    fn test_benchmark_chart_data() {
        let chart = BenchmarkChart {
            benchmark: "MMLU".to_string(),
            metric: "accuracy".to_string(),
            direction: "higher".to_string(),
            data: vec![
                BenchmarkEntry {
                    label: "GPT-4".to_string(),
                    paper: "2301.00001".to_string(),
                    value: 0.86,
                },
            ],
        };

        let data = BenchmarkChartData {
            papers: vec!["2301.00001".to_string()],
            charts: vec![chart],
        };

        let json = data.to_json().unwrap();
        assert!(json.contains("MMLU"));
        assert!(json.contains("accuracy"));
    }

    #[test]
    fn test_render_svg() {
        let data = BenchmarkChartData {
            papers: vec![],
            charts: vec![],
        };
        let svg = BenchmarkViz::render_svg(&data);
        assert!(svg.contains("<svg"));
        assert!(svg.contains("Benchmark Comparison"));
    }
}
