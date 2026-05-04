"""
Benchmark visualization — chart generation for cross-paper comparison results.

Generates HTML (interactive D3.js), SVG (pure Python), and chart-compatible JSON
from BenchmarkResult objects.

Usage:
    from llm.benchmark import BenchmarkComparator
    from viz.benchmark_viz import BenchmarkViz

    bc = BenchmarkComparator(db)
    result = bc.compare(["2604.22754", "2302.00763"])
    viz = BenchmarkViz()
    viz.render_html(result, "chart.html")
"""
from __future__ import annotations

import json
from typing import Dict, List

from llm.benchmark import BenchmarkResult, _is_higher_better

# ── Colour palette (12 colours, safe for colour-blindness) ─────────────────

_COLORS = [
    "#4E79A7",  # blue
    "#F28E2B",  # orange
    "#E15759",  # red
    "#76B7B2",  # teal
    "#59A14F",  # green
    "#EDC948",  # yellow
    "#B07AA1",  # purple
    "#FF9DA7",  # pink
    "#9C755F",  # brown
    "#BAB0AC",  # grey
    "#86BCB6",  # mint
    "#D37295",  # rose
]


# ── Chart data conversion ──────────────────────────────────────────────────


def to_chart_json(result: BenchmarkResult) -> Dict:
    """Convert BenchmarkResult to chart-compatible JSON.

    Returns a dict with:
        papers: list of paper_ids in order
        charts: list of {benchmark, metric, direction, data: [{label, paper, value}]}
    """
    papers = result.paper_ids
    charts = []
    for match in result.matches:
        direction = "higher" if _is_higher_better(match.metric_name) else "lower"
        data = [
            {"label": model, "paper": pid, "value": val}
            for pid, val, model in match.entries
        ]
        charts.append({
            "benchmark": match.benchmark_name,
            "metric": match.metric_name,
            "direction": direction,
            "data": data,
        })
    return {"papers": papers, "charts": charts}


# ── HTML / D3.js renderer ──────────────────────────────────────────────────


def render_html(result: BenchmarkResult, output_path: str = "benchmark_chart.html") -> str:
    """Generate a self-contained HTML file with interactive D3.js bar charts.

    Each benchmark/metric pair renders as a horizontal bar chart,
    colour-coded by paper, sorted by score descending.

    Returns the output path.
    """
    chart_data = to_chart_json(result)
    json_data = json.dumps(chart_data, indent=2, ensure_ascii=False)
    colors_json = json.dumps(_COLORS)

    html_template = """<!DOCTYPE html>
<html lang="en">
<head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width, initial-scale=1.0">
<title>Benchmark Comparison</title>
<script src="https://cdn.jsdelivr.net/npm/d3@7/dist/d3.min.js"></script>
<style>
  * { box-sizing: border-box; margin: 0; padding: 0; }
  body { font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
         background: #f8f9fa; color: #333; padding: 24px; }
  h1 { font-size: 22px; margin-bottom: 4px; color: #1a1a2e; }
  .subtitle { color: #666; font-size: 14px; margin-bottom: 24px; }
  .chart-card { background: #fff; border-radius: 8px; box-shadow: 0 1px 4px rgba(0,0,0,0.08);
                 padding: 20px; margin-bottom: 20px; }
  .chart-title { font-size: 16px; font-weight: 600; margin-bottom: 2px; }
  .chart-direction { font-size: 12px; color: #888; margin-bottom: 12px; }
  .bar { transition: opacity 0.15s; cursor: pointer; }
  .bar:hover { opacity: 0.8; }
  .axis-label { font-size: 11px; fill: #666; }
  .tick text { font-size: 11px; fill: #555; }
  .tooltip { position: absolute; background: rgba(0,0,0,0.85); color: #fff;
              padding: 8px 12px; border-radius: 6px; font-size: 13px;
              pointer-events: none; opacity: 0; transition: opacity 0.15s;
              max-width: 360px; line-height: 1.4; }
  .empty { color: #999; font-style: italic; padding: 20px 0; }
</style>
</head>
<body>
  <h1>📊 Benchmark Comparison</h1>
"""

    papers_str = ", ".join(result.paper_ids)
    html_template += f'  <p class="subtitle">Papers: {papers_str} &middot; {len(result.matches)} benchmark(s)</p>\n'
    html_template += """  <div id="charts"></div>
  <div class="tooltip" id="tooltip"></div>

<script>
  const DATA = """ + json_data + """;
  const COLORS = """ + colors_json + """;
  const paperColors = {};
  DATA.papers.forEach((p, i) => { paperColors[p] = COLORS[i % COLORS.length]; });

  const container = document.getElementById('charts');
  const tooltip = document.getElementById('tooltip');

  if (DATA.charts.length === 0) {
    container.innerHTML = '<div class="empty">No matching benchmarks found.</div>';
  }

  DATA.charts.forEach((chart, idx) => {
    const card = document.createElement('div');
    card.className = 'chart-card';
    card.innerHTML = '<div class="chart-title">' + chart.benchmark + ' \\u2014 ' + chart.metric + '</div>'
      + '<div class="chart-direction">' + (chart.direction === 'higher' ? '\\u2191' : '\\u2193') + ' ' + chart.direction + ' is better</div>'
      + '<svg width=\"100%\" height=\"' + Math.max(100, chart.data.length * 36 + 40) + '\"></svg>';
    container.appendChild(card);

    const svg = card.querySelector('svg');
    const width = svg.clientWidth || 800;
    const height = parseFloat(svg.getAttribute('height'));
    const margin = { left: 160, right: 80, top: 8, bottom: 8 };
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

    chart.data.forEach((d, i) => {
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
        .on('mouseover', (ev) => {
          tooltip.style.opacity = 1;
          tooltip.innerHTML = '<strong>' + d.label + '</strong><br>Paper: ' + d.paper + '<br>Score: ' + d.value.toFixed(4);
          tooltip.style.left = (ev.pageX + 12) + 'px';
          tooltip.style.top = (ev.pageY - 10) + 'px';
        })
        .on('mousemove', (ev) => {
          tooltip.style.left = (ev.pageX + 12) + 'px';
          tooltip.style.top = (ev.pageY - 10) + 'px';
        })
        .on('mouseout', () => { tooltip.style.opacity = 0; });

      svg.append('text')
        .attr('x', margin.left - 8)
        .attr('y', yPos + barH / 2)
        .attr('text-anchor', 'end')
        .attr('dominant-baseline', 'middle')
        .attr('class', 'axis-label')
        .text(d.label.length > 24 ? d.label.slice(0, 22) + '\\u2026' : d.label);

      svg.append('text')
        .attr('x', margin.left + barW + 6)
        .attr('y', yPos + barH / 2)
        .attr('dominant-baseline', 'middle')
        .attr('class', 'axis-label')
        .text(d.value.toFixed(4));
    });
  });
</script>
</body>
</html>"""

    with open(output_path, "w", encoding="utf-8") as f:
        f.write(html_template)

    return output_path


# ── SVG renderer (pure Python, no JS) ──────────────────────────────────────


def render_svg(result: BenchmarkResult) -> str:
    """Generate SVG bar chart as a string (pure Python, no JS dependency).

    Suitable for embedding in Markdown or other static documents.
    Returns an SVG string containing one chart-group per benchmark match.
    """
    if not result.matches:
        return "<!-- No benchmark matches to visualize -->"

    lines: List[str] = []
    height = _svg_height(result)
    lines.append('<svg xmlns="http://www.w3.org/2000/svg" '
                 f'width="720" height="{height}" '
                 'style="font-family: -apple-system, sans-serif;">')
    lines.append('  <rect width="100%" height="100%" fill="#f8f9fa"/>')
    lines.append('  <text x="20" y="28" font-size="16" font-weight="bold" fill="#333">'
                 'Benchmark Comparison</text>')
    lines.append(f'  <text x="20" y="46" font-size="12" fill="#666">'
                 f'Papers: {", ".join(result.paper_ids)}</text>')

    y_offset = 64
    bar_height = 20
    row_gap = 4
    label_width = 150
    bar_max_width = 480
    color_idx = 0

    for match in result.matches:
        # Section header
        lines.append(f'  <text x="20" y="{y_offset + 14}" font-size="14" '
                     f'font-weight="600" fill="#333">{match.benchmark_name} — {match.metric_name}</text>')
        direction = "↑ higher is better" if _is_higher_better(match.metric_name) else "↓ lower is better"
        lines.append(f'  <text x="20" y="{y_offset + 30}" font-size="11" fill="#888">{direction}</text>')
        y_offset += 38

        sorted_entries = sorted(match.entries, key=lambda e: e[1], reverse=True)
        max_val = max(abs(v) for _, v, _ in sorted_entries) if sorted_entries else 1.0
        if max_val == 0:
            max_val = 1.0

        for rank, (_pid, val, model) in enumerate(sorted_entries):
            bar_y = y_offset + rank * (bar_height + row_gap)
            bar_w = max((val / max_val) * bar_max_width, 2) if val > 0 else 2

            color = _COLORS[color_idx % len(_COLORS)]
            lines.append(f'  <rect x="{label_width + 10}" y="{bar_y}" '
                         f'width="{bar_w}" height="{bar_height}" '
                         f'fill="{color}" rx="3" opacity="0.85"/>')
            lines.append(f'  <text x="{label_width + 6}" y="{bar_y + bar_height - 4}" '
                         f'text-anchor="end" font-size="10" fill="#555">'
                         f'{model[:20]}</text>')
            lines.append(f'  <text x="{label_width + 14 + bar_w}" y="{bar_y + bar_height - 4}" '
                         f'font-size="10" fill="#666">{val:.4f}</text>')
            color_idx += 1

        y_offset += len(sorted_entries) * (bar_height + row_gap) + 16

    lines.append('</svg>')
    return "\n".join(lines)


def _svg_height(result: BenchmarkResult) -> int:
    """Compute required SVG height."""
    h = 80  # header
    for match in result.matches:
        h += 38  # section header
        h += len(match.entries) * 24  # bars
        h += 16  # spacing
    return max(h, 120)


# ── Convenience wrapper ────────────────────────────────────────────────────


class BenchmarkViz:
    """Visualize BenchmarkResult as charts."""

    @staticmethod
    def to_json(result: BenchmarkResult) -> Dict:
        """Chart-compatible JSON with benchmark data."""
        return to_chart_json(result)

    @staticmethod
    def render_html(result: BenchmarkResult, output_path: str = "benchmark_chart.html") -> str:
        """Generate self-contained D3.js HTML file. Returns output path."""
        return render_html(result, output_path)

    @staticmethod
    def render_svg(result: BenchmarkResult) -> str:
        """Generate SVG string (pure Python, no JS)."""
        return render_svg(result)
