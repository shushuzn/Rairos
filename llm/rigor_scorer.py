"""Research Rigor Scorer.

Rates papers by methodology transparency, reproducibility signals, and
dataset/code sharing indicators. Returns a RigorScore badge (A/B/C/D).
"""

from __future__ import annotations

import json
import re
from dataclasses import dataclass
from typing import Any, Dict, List, Optional

from llm.client import get_client


@dataclass
class RigorScore:
    paper_id: str
    overall: float  # 0.0–1.0
    has_code: bool
    has_dataset: bool
    methodology_clarity: str  # "high" / "medium" / "low"
    reproducibility_signals: List[str]
    badge: str  # "A" / "B" / "C" / "D"

    def to_dict(self) -> Dict[str, Any]:
        return {
            "paper_id": self.paper_id,
            "overall": self.overall,
            "has_code": self.has_code,
            "has_dataset": self.has_dataset,
            "methodology_clarity": self.methodology_clarity,
            "reproducibility_signals": self.reproducibility_signals,
            "badge": self.badge,
        }

    @staticmethod
    def from_dict(d: Dict[str, Any]) -> "RigorScore":
        return RigorScore(
            paper_id=d["paper_id"],
            overall=d["overall"],
            has_code=d["has_code"],
            has_dataset=d["has_dataset"],
            methodology_clarity=d["methodology_clarity"],
            reproducibility_signals=d.get("reproducibility_signals", []),
            badge=d["badge"],
        )


# Pre-screening patterns — fast regex pass before LLM
_CODE_SIGNALS = [
    r"github\.com/[a-zA-Z0-9_\-]+/[a-zA-Z0-9_\-]+",
    r"https?://github\.com",
    r"code\s+(?:available|at|on)\s+github",
    r"implementation\s+(?:available|at|on|on\s+github)",
    r"open\s+source",
    r"repository\s+(?:available|on)",
    r"supplementary\s+code",
    r"bit\.ly/\w+",  # shortened GitHub links
]

_DATASET_SIGNALS = [
    r"dataset\s+(?:available|at|from|from\s+the|from\s+authors?)",
    r"data\s+(?:available|at|from|upon\s+request)",
    r"benchmark\s+(?:dataset|data)",
    r"download\s+(?:dataset|data)",
    r"http[s]?://[^\s]*(?:dataset|data\.csv|data\.json|data\.zip)",
    r"zenodo",
    r"figshare",
    r"dryad",
    r"osf\.io",
    r"kaggle\.com",
    r"huggingface\.co/(?:datasets|spaces)",
]


def _fast_scan(text: str) -> tuple[bool, bool, List[str]]:
    """Quick regex scan of abstract/title for code/dataset signals."""
    has_code = any(re.search(p, text, re.I) for p in _CODE_SIGNALS)
    has_dataset = any(re.search(p, text, re.I) for p in _DATASET_SIGNALS)

    signals: List[str] = []
    if has_code:
        signals.append("Code/GitHub mentioned")
    if has_dataset:
        signals.append("Dataset mentioned")

    return has_code, has_dataset, signals


def _llm_refine(text: str, fast_has_code: bool, fast_has_dataset: bool) -> Dict[str, Any]:
    """Use LLM to assess methodology clarity and refine signals."""
    prompt = f"""You are a research methodology reviewer. Given this paper abstract, rate its reproducibility and methodology transparency.

Rate each dimension:
1. code_availability: Does the paper share code, implementation, or software? (yes/partial/no)
2. dataset_availability: Does the paper share or reference a dataset, benchmark, or data? (yes/partial/no)
3. methodology_clarity: How clearly is the method described? (high/medium/low)
4. reproducibility_notes: Any other reproducibility concerns or strengths? (1-2 sentences)

Return ONLY valid JSON with keys: code_availability, dataset_availability, methodology_clarity, reproducibility_notes

PAPER ABSTRACT:
{text[:2000]}

JSON:"""

    try:
        llm = get_client()
        response = llm.generate(prompt)
        # Try to extract JSON
        match = re.search(r"\{[\s\S]*\}", response.strip())
        if match:
            return json.loads(match.group())
    except Exception:
        pass

    # Fallback: use fast scan results
    return {
        "code_availability": "yes" if fast_has_code else "no",
        "dataset_availability": "yes" if fast_has_dataset else "no",
        "methodology_clarity": "medium",
        "reproducibility_notes": "Could not analyze further.",
    }


def _compute_badge(has_code: bool, has_dataset: bool, clarity: str) -> str:
    """Map signals to A/B/C/D badge."""
    score = 0
    if has_code:
        score += 1
    if has_dataset:
        score += 1
    if clarity == "high":
        score += 1
    elif clarity == "low":
        score -= 1

    if score >= 3:
        return "A"
    elif score == 2:
        return "B"
    elif score == 1:
        return "C"
    else:
        return "D"


class RigorScorer:
    """Score a paper's research rigor via fast scan + LLM refinement."""

    def __init__(self):
        self._cache: Dict[str, RigorScore] = {}

    def score_paper(self, paper_id: str, abstract: str = "", title: str = "") -> RigorScore:
        """Score a paper by ID (fetches abstract from DB if not provided)."""
        if paper_id in self._cache:
            return self._cache[paper_id]

        if not abstract:
            abstract = self._fetch_abstract(paper_id)
        if not title:
            title = self._fetch_title(paper_id)

        text = f"{title}\n\n{abstract}"

        # Fast regex pre-scan
        has_code, has_dataset, signals = _fast_scan(text)

        # LLM refinement
        llm_result = _llm_refine(text, has_code, has_dataset)

        has_code = llm_result.get("code_availability", "no") in ("yes", "partial")
        has_dataset = llm_result.get("dataset_availability", "no") in ("yes", "partial")
        clarity = llm_result.get("methodology_clarity", "medium")
        extra_notes = llm_result.get("reproducibility_notes", "")
        if extra_notes and extra_notes not in signals:
            signals.append(extra_notes[:100])

        overall = 0.0
        if has_code:
            overall += 0.35
        if has_dataset:
            overall += 0.35
        if clarity == "high":
            overall += 0.30
        elif clarity == "medium":
            overall += 0.15

        badge = _compute_badge(has_code, has_dataset, clarity)

        result = RigorScore(
            paper_id=paper_id,
            overall=round(overall, 2),
            has_code=has_code,
            has_dataset=has_dataset,
            methodology_clarity=clarity,
            reproducibility_signals=signals,
            badge=badge,
        )
        self._cache[paper_id] = result
        return result

    def _fetch_abstract(self, paper_id: str) -> str:
        try:
            from db.database import Database

            db = Database()
            db.init()
            paper = db.get_paper(paper_id)
            if paper:
                return getattr(paper, "abstract", "") or ""
        except Exception:
            pass
        return ""

    def _fetch_title(self, paper_id: str) -> str:
        try:
            from db.database import Database

            db = Database()
            db.init()
            paper = db.get_paper(paper_id)
            if paper:
                return getattr(paper, "title", "") or ""
        except Exception:
            pass
        return ""

    @staticmethod
    def render_badge_html(score: RigorScore) -> str:
        colors = {"A": "#7A9E7A", "B": "#6B8FB5", "C": "#D4A84B", "D": "#C4706A"}
        color = colors.get(score.badge, "#888")
        clarity_labels = {"high": "High", "medium": "Medium", "low": "Low"}
        signals_html = (
            "<br>".join(f"• {s}" for s in score.reproducibility_signals) or "No signals detected"
        )

        return f"""
<span class="rigor-badge" style="
    display: inline-block;
    background: {color};
    color: white;
    font-family: 'Caveat', cursive;
    font-size: 1.1em;
    font-weight: 700;
    width: 2em;
    height: 2em;
    line-height: 2em;
    text-align: center;
    border-radius: 6px;
    cursor: help;
    title='code: {score.has_code}, dataset: {score.has_dataset}, clarity: {clarity_labels.get(score.methodology_clarity, "?")}'
" title="code: {score.has_code} | dataset: {score.has_dataset} | clarity: {clarity_labels.get(score.methodology_clarity, "?")}">
    {score.badge}
</span>
<div class="rigor-tooltip" style="display:none; position: absolute; background:#2a2a2a; color:#e8e4de; padding:10px; border-radius:6px; font-size:13px; max-width:260px; z-index:100; font-family:Lora,serif;">
    <strong style="color:{color}">Rigor: {score.badge}</strong>
    <hr style="border-color:#555; margin:6px 0">
    <div>Overall: {score.overall:.0%}</div>
    <div>Code shared: {"✓" if score.has_code else "✗"}</div>
    <div>Dataset shared: {"✓" if score.has_dataset else "✗"}</div>
    <div>Methodology: {clarity_labels.get(score.methodology_clarity, "?")}</div>
    <hr style="border-color:#555; margin:6px 0">
    <div style="color:#aaa">Signals:</div>
    <div>{signals_html}</div>
</div>
"""
