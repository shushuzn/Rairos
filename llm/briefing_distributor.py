"""Briefing Distributor — audience-specific research digests.

Renders a paper briefing in different audience formats:
  phd_advisor, industry_engineer, policy_maker, researcher
Also generates shareable short links via short_id → briefing mapping.
"""

from __future__ import annotations

import hashlib
import json

from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

BRIEFINGS_DIR = Path.home() / ".ai_research_os" / "briefings"
LINKS_FILE = Path.home() / ".ai_research_os" / "briefing_links.json"
SHORTCODE_CHARS = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"


def _load_links() -> Dict[str, Any]:
    if not LINKS_FILE.exists():
        return {}
    return json.loads(LINKS_FILE.read_text(encoding="utf-8"))  # type: ignore[no-any-return]


def _save_links(links: Dict[str, Any]) -> None:
    LINKS_FILE.parent.mkdir(parents=True, exist_ok=True)
    LINKS_FILE.write_text(json.dumps(links, indent=2, ensure_ascii=False), encoding="utf-8")


def make_short_id(title: str, arxiv_id: str) -> str:
    raw = f"{arxiv_id}:{title[:30]}"
    h = hashlib.sha256(raw.encode()).digest()
    code = "".join(SHORTCODE_CHARS[h[i] % len(SHORTCODE_CHARS)] for i in range(6))
    return code


def create_share_link(arxiv_id: str, title: str, audience: str = "researcher") -> str:
    links = _load_links()
    short_id = make_short_id(title, arxiv_id)
    links[short_id] = {
        "arxiv_id": arxiv_id,
        "title": title,
        "audience": audience,
        "created_at": datetime.now().isoformat(),
        "clicks": 0,
    }
    _save_links(links)
    return short_id


def get_latest_briefing_markdown(arxiv_id: str) -> Optional[str]:
    if not BRIEFINGS_DIR.exists():
        return None
    candidates = list(BRIEFINGS_DIR.glob(f"*{arxiv_id}*briefing*"))
    if not candidates:
        return None
    latest = max(candidates, key=lambda p: p.stat().st_mtime)
    return latest.read_text(encoding="utf-8")


AUDIENCE_PROMPTS = {
    "phd_advisor": "You are reviewing this for a PhD student. Focus on: methodology rigor, open research questions, how this relates to the student's existing work, and what experiments to run next.",
    "industry_engineer": "You are an ML engineer at a tech company. Focus on: how to actually implement or use this, compute/benchmark requirements, code availability, and practical deployment considerations.",
    "policy_maker": "You are a policy advisor. Focus on: societal impact, regulatory implications, timeline for real-world deployment, affected stakeholders, and risk factors.",
    "researcher": "You are a research peer. Provide a concise critical summary: key contribution, limitations, relationship to existing work, and concrete next steps.",
}


def _parse_markdown_sections(md: str) -> Dict[str, str]:
    sections: Dict[str, str] = {}
    current = "header"
    body_lines: List[str] = []

    for line in md.splitlines():
        line = line.strip()
        if line.startswith("## "):
            if body_lines:
                sections[current] = "\n".join(body_lines).strip()
                body_lines = []
            current = line[3:].strip().lower().replace(" ", "_")
        elif line.startswith("# "):
            sections["_title"] = line[2:].strip()
        else:
            body_lines.append(line)

    if body_lines and current != "header":
        sections[current] = "\n".join(body_lines).strip()
    elif current == "header":
        sections["_body"] = "\n".join(body_lines).strip()

    return sections


def _escape_html(text: str) -> str:
    return (
        text.replace("&", "&amp;")
        .replace("<", "&lt;")
        .replace(">", "&gt;")
        .replace('"', "&quot;")
        .replace("'", "&#39;")
    )


def _render_phd_advisor(sections: Dict[str, str], raw: str) -> str:
    def extract(t):
        return sections.get(t, "")[:300] or raw[200:500]

    return (
        _section_html(
            "📚 Paper Summary", sections.get("_body", sections.get("summary", raw[:300]))[:400]
        )
        + _section_html("🔬 Methodology Assessment", extract("methodology"))
        + _section_html("❓ Open Questions for Student", extract("research_gaps"))
    )


def _render_industry_engineer(sections: Dict[str, str], raw: str) -> str:
    def extract(t):
        return sections.get(t, "")[:300] or raw[200:500]

    return (
        _section_html(
            "⚡ What It Does", sections.get("_body", sections.get("summary", raw[:300]))[:200]
        )
        + _section_html("🛠️ Implementation Signals", extract("methodology"))
        + _section_html("📊 Benchmark / Compute", extract("experiments"))
    )


def _render_policy_maker(sections: Dict[str, str], raw: str) -> str:
    def extract(t):
        return sections.get(t, "")[:300] or raw[300:600]

    return (
        _section_html(
            "🏛️ What This Means", sections.get("_body", sections.get("summary", raw[:300]))[:300]
        )
        + _section_html("⚠️ Risks & Concerns", extract("limitations"))
        + _section_html("📅 Deployment Timeline", raw[200:500])
    )


def _render_researcher(sections: Dict[str, str], raw: str) -> str:
    v = sections.get("verdict", "neutral").lower()
    badge_map = {
        "validates": ("✅ Validates", "verdict-validates"),
        "contradicts": ("❌ Contradicts", "verdict-contradicts"),
    }
    badge_text, badge_cls = badge_map.get(v, ("➖ Neutral", "verdict-neutral"))

    lines = [f"<span class='verdict-badge {badge_cls}'>{badge_text}</span>"]
    lines.append(
        f"<p style='margin-top:8px'>{sections.get('_body', sections.get('summary', raw[:400]))[:400]}</p>"
    )

    gaps = sections.get("research_gaps", sections.get("gaps", ""))
    if gaps:
        lines.append(_section_html("🎯 Research Gaps", gaps[:300]))

    return "\n".join(lines)


def _section_html(heading: str, content: str) -> str:
    return f"<div class='digest-section'><h4>{heading}</h4><p>{content[:300] if content else ''}</p></div>"


def render_distributed_briefing(arxiv_id: str, title: str, markdown: str, audience: str) -> str:
    audience_labels = {
        "phd_advisor": "🎓 PhD Advisor Digest",
        "industry_engineer": "⚙️ Industry Engineer Digest",
        "policy_maker": "🏛️ Policy Maker Digest",
        "researcher": "🔬 Researcher Digest",
    }
    label = audience_labels.get(audience, audience)
    short_id = create_share_link(arxiv_id, title, audience)
    sections = _parse_markdown_sections(markdown)

    if audience == "phd_advisor":
        body_content = _render_phd_advisor(sections, markdown)
    elif audience == "industry_engineer":
        body_content = _render_industry_engineer(sections, markdown)
    elif audience == "policy_maker":
        body_content = _render_policy_maker(sections, markdown)
    else:
        body_content = _render_researcher(sections, markdown)

    lines = ['<div class="briefing-dist">']
    lines.append(
        "<div style='display:flex;justify-content:space-between;align-items:center;margin-bottom:16px'>"
    )
    lines.append(f"<h3 style='margin:0'>{label}</h3>")
    lines.append(
        f"<span style='font-size:11px;color:#A89E8C;background:#f5f0e8;padding:3px 10px;border-radius:12px'>"
        f"Share: <code style='font-size:11px'>rairos.app/b/{short_id}</code></span>"
    )
    lines.append("</div>")
    lines.append(f"<div class='digest-body'>{body_content}</div>")
    lines.append("<details style='margin-top:20px'>")
    lines.append(
        "<summary style='cursor:pointer;font-size:12px;color:#A89E8C'>View Raw Briefing</summary>"
    )
    lines.append(
        f"<pre style='font-size:11px;background:#f8f4ef;padding:12px;border-radius:4px;overflow:auto'>"
        f"{_escape_html(markdown[:2000])}</pre>"
    )
    lines.append("</details>")
    lines.append("<style>")
    lines.append(".briefing-dist { font-family: Georgia, serif; max-width: 800px; }")
    lines.append(
        ".digest-section { margin-bottom: 16px; padding-bottom: 16px; border-bottom: 1px solid #e8e4dc; }"
    )
    lines.append(
        ".digest-section h4 { font-size: 13px; font-weight: 700; color: #2a4a6a; margin-bottom: 6px; }"
    )
    lines.append(".digest-section p { font-size: 13px; color: #444; line-height: 1.6; margin: 0; }")
    lines.append(
        ".verdict-badge { display: inline-block; padding: 2px 10px; border-radius: 12px; font-size: 11px; font-weight: 600; }"
    )
    lines.append(".verdict-validates { background: rgba(107,191,138,0.15); color: #4a8a5a; }")
    lines.append(".verdict-contradicts { background: rgba(196,112,106,0.15); color: #C4706A; }")
    lines.append(".verdict-neutral { background: rgba(168,158,140,0.15); color: #7a7570; }")
    lines.append("</style>")
    lines.append("</div>")
    return "\n".join(lines)


def render_distributor_panel(arxiv_id: str, title: str) -> str:
    create_share_link(arxiv_id, title, "researcher")

    lines = ['<div class="dist-panel">']
    lines.append("<h3>📬 Briefing Distributor</h3>")
    lines.append(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:14px'>"
        "Render this briefing for different audiences, or share a public link.</p>"
    )

    audiences = [
        ("researcher", "🔬 Researcher", "Concise technical summary with gap analysis"),
        ("phd_advisor", "🎓 PhD Advisor", "Methodology critique and open questions"),
        ("industry_engineer", "⚙️ Industry Engineer", "Practical applicability and benchmarks"),
        ("policy_maker", "🏛️ Policy Maker", "Societal impact and regulatory implications"),
    ]

    for aud_id, aud_name, aud_desc in audiences:
        s = create_share_link(arxiv_id, title, aud_id)
        lines.append(
            "<div style='margin-bottom:14px;padding:12px;background:#f8f4ef;border-radius:6px'>"
        )
        lines.append(
            f"<div style='font-weight:700;font-size:13px;margin-bottom:2px'>{aud_name}</div>"
        )
        lines.append(
            f"<div style='font-size:12px;color:#A89E8C;margin-bottom:6px'>{aud_desc}</div>"
        )
        lines.append(
            f"<button id='btn-{aud_id}' style='background:#6B8FB5;color:white;border:none;border-radius:4px;"
            f"padding:5px 12px;cursor:pointer;font-size:12px'>Preview</button> "
        )
        lines.append(
            f"<button onclick=\"copyShareLink('{s}')\" "
            f"style='background:transparent;color:#6B8FB5;border:1px solid #6B8FB5;"
            f"border-radius:4px;padding:5px 12px;cursor:pointer;font-size:12px;margin-left:6px'>"
            f"Copy Link</button>"
        )
        lines.append("</div>")

    lines.append("<div id='audience-preview' style='margin-top:16px'></div>")

    lines.append(f"""
<script>
document.querySelectorAll('button[id^="btn-"]').forEach(function(btn) {{
    var aud = btn.id.replace('btn-', '');
    btn.addEventListener('click', function() {{
        var preview = document.getElementById('audience-preview');
        preview.innerText = 'Loading...';
        fetch('/briefing/distribute/{arxiv_id}?audience=' + aud)
          .then(function(r) {{ return r.text(); }})
          .then(function(html) {{
              var tmp = document.createElement('div');
              tmp.innerHTML = html;
              preview.innerHTML = tmp.querySelector('.briefing-dist') ? tmp.querySelector('.briefing-dist').innerHTML : html;
          }});
    }});
}});
function copyShareLink(shortId) {{
    navigator.clipboard.writeText(window.location.origin + '/b/' + shortId)
      .then(function() {{ alert('Link copied!'); }});
}}
</script>""")

    lines.append("<style>.dist-panel { font-family: Georgia, serif; }</style>")
    lines.append("</div>")
    return "\n".join(lines)
