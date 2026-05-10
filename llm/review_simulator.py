"""LLM-Powered Auto-Reviewer Simulator.

Simulates adversarial peer reviewers stress-testing a paper or proposal.
Plays hostile reviewer personas to surface weaknesses before submission.
"""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


class ReviewDimension(Enum):
    METHODOLOGY = "methodology"
    NOVELTY_CONTRIBUTION = "novelty_contribution"
    CLARITY_PRESENTATION = "clarity_presentation"
    BASELINES_COMPARISON = "baselines_comparison"
    REPRODUCIBILITY = "reproducibility"
    OVERCLAIMING = "overclaiming"
    RELATED_WORK = "related_work"


class Severity(Enum):
    CRITICAL = "critical"  # must fix before submission
    MAJOR = "major"  # significant weakness
    MINOR = "minor"  # optional improvement
    PRAISE = "praise"  # genuinely good


@dataclass
class ReviewAnnotation:
    """A single annotated comment on the paper."""

    annotation_id: str
    dimension: ReviewDimension
    severity: Severity
    location: str  # "abstract", "introduction", "section 3", "table 2", etc.
    headline: str  # one-line summary of the issue
    comment: str  # detailed reviewer comment
    suggestion: str = ""  # concrete fix suggestion
    page_line: str = ""  # optional specific location
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "annotation_id": self.annotation_id,
            "dimension": self.dimension.value,
            "severity": self.severity.value,
            "location": self.location,
            "headline": self.headline,
            "comment": self.comment,
            "suggestion": self.suggestion,
            "page_line": self.page_line,
            "created_at": datetime.fromtimestamp(self.created_at).isoformat(),
        }


@dataclass
class ReviewPersona:
    """A simulated reviewer persona with a specific lens."""

    name: str  # e.g. "Methodology Reviewer"
    focus: List[ReviewDimension]
    tone: str  # "hostile", "constructive", "technical"
    priority_instructions: str


@dataclass
class SimulatedReview:
    """Complete simulated review from one persona."""

    review_id: str
    persona: str
    overall_score: float  # 1–10
    summary: str  # 2–3 sentence overall assessment
    strengths: List[str]
    weaknesses: List[str]
    annotations: List[ReviewAnnotation]
    recommendation: str  # accept / borderline / reject / strong reject
    created_at: float = field(default_factory=time.time)

    def to_dict(self) -> Dict[str, Any]:
        return {
            "review_id": self.review_id,
            "persona": self.persona,
            "overall_score": self.overall_score,
            "summary": self.summary,
            "strengths": self.strengths,
            "weaknesses": self.weaknesses,
            "annotations": [a.to_dict() for a in self.annotations],
            "recommendation": self.recommendation,
            "created_at": datetime.fromtimestamp(self.created_at).isoformat(),
        }


# ─── Review Personas ────────────────────────────────────────────────


_REVIEW_PERSONAS = [
    ReviewPersona(
        name="Methodology Reviewer",
        focus=[
            ReviewDimension.METHODOLOGY,
            ReviewDimension.BASELINES_COMPARISON,
            ReviewDimension.REPRODUCIBILITY,
        ],
        tone="hostile",
        priority_instructions="Focus on whether the methodology is sound, whether experiments are properly controlled, whether baselines are fair and complete, and whether the approach can be reproduced from the description.",
    ),
    ReviewPersona(
        name="Contributions Reviewer",
        focus=[ReviewDimension.NOVELTY_CONTRIBUTION, ReviewDimension.OVERCLAIMING],
        tone="hostile",
        priority_instructions="Focus on whether the claimed contributions are genuinely novel, whether the paper overstated its significance, whether the novelty compared to prior work is real or incremental, and whether the 'novel' aspects are clearly articulated.",
    ),
    ReviewPersona(
        name="Clarity Reviewer",
        focus=[ReviewDimension.CLARITY_PRESENTATION, ReviewDimension.RELATED_WORK],
        tone="constructive",
        priority_instructions="Focus on whether the paper is clearly written, whether the problem motivation is understandable, whether related work adequately situates the contribution, whether figures and tables are self-contained, and whether the writing obscures weaknesses.",
    ),
    ReviewPersona(
        name="Ethics & Scope Reviewer",
        focus=[ReviewDimension.OVERCLAIMING, ReviewDimension.REPRODUCIBILITY],
        tone="critical",
        priority_instructions="Focus on whether the paper makes claims beyond what the experiments support, whether limitations are honestly discussed, whether potential misuse cases are noted, and whether the scope of claimed applicability matches the evidence.",
    ),
]


# ─── Core Simulator ────────────────────────────────────────────────


class ReviewSimulator:
    """Simulate adversarial peer reviewers for a paper or proposal."""

    def __init__(self, personas: Optional[List[ReviewPersona]] = None):
        self.personas = personas or _REVIEW_PERSONAS

    def review(
        self,
        paper_text: str,
        title: str = "",
        persona: Optional[ReviewPersona] = None,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
    ) -> SimulatedReview:
        """Run a simulated review on the paper text.

        Args:
            paper_text: Full text of the paper (or abstract + intro sections)
            title: Paper title
            persona: Specific persona to use (runs all if None)
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name

        Returns:
            SimulatedReview with annotations and overall assessment
        """

        if persona is None:
            # Run all personas
            reviews = []
            for p in self.personas:
                r = self._review_with_persona(p, paper_text, title, api_key, base_url, model)
                reviews.append(r)
            # Return the most critical review (highest severity annotations)
            return self._merge_reviews(reviews)

        return self._review_with_persona(persona, paper_text, title, api_key, base_url, model)

    def _review_with_persona(
        self,
        persona: ReviewPersona,
        paper_text: str,
        title: str,
        api_key: Optional[str],
        base_url: Optional[str],
        model: Optional[str],
    ) -> SimulatedReview:
        """Run review with a specific persona."""
        import os

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        focus_dims = ", ".join(d.value for d in persona.focus)
        prompt = f"""You are simulating a {persona.tone} peer reviewer for an academic paper.
Your persona: **{persona.name}** — you specialize in: {focus_dims}

{persona.priority_instructions}

Be adversarial. Find real weaknesses. Do not be polite.

PAPER TITLE: {title or "Unknown"}
PAPER TEXT (or key sections):
---
{paper_text[:8000]}
---

TASK: Produce a structured adversarial review. Respond ONLY with valid JSON (no markdown, no explanation):

{{
  "overall_score": <1-10, where 1=reject, 10=accept>,
  "summary": "<2-3 sentence overall assessment>",
  "strengths": ["<strength 1>", "<strength 2>"],
  "weaknesses": ["<weakness 1>", "<weakness 2>", "<weakness 3>"],
  "recommendation": "accept | borderline | reject | strong reject",
  "annotations": [
    {{
      "dimension": "{ReviewDimension.METHODOLOGY.value}" | "{ReviewDimension.NOVELTY_CONTRIBUTION.value}" | "{ReviewDimension.CLARITY_PRESENTATION.value}" | "{ReviewDimension.BASELINES_COMPARISON.value}" | "{ReviewDimension.REPRODUCIBILITY.value}" | "{ReviewDimension.OVERCLAIMING.value}" | "{ReviewDimension.RELATED_WORK.value}",
      "severity": "critical | major | minor | praise",
      "location": "abstract | introduction | related work | methodology | experiments | results | discussion | table N | figure N",
      "headline": "<one-line summary of the issue>",
      "comment": "<detailed reviewer comment, 2-3 sentences>",
      "suggestion": "<concrete fix suggestion, 1-2 sentences>"
    }}
  ]
}}

Only include annotations for genuine issues. Maximum 6 annotations per review. If something is genuinely good, use severity "praise"."""

        try:
            response = call_llm_chat_completions(
                base_url=base_url or os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
                api_key=api_key or os.getenv("OPENAI_API_KEY", ""),
                model=model or os.getenv("LLM_MODEL", "") or LLM_MODEL,
                system_prompt="You are an adversarial peer reviewer. Be critical but constructive. Respond with valid JSON only.",
                user_prompt=prompt,
            )

            parsed = json.loads(response.strip())

            annotations = []
            for a in parsed.get("annotations", []):
                try:
                    dim = ReviewDimension(a.get("dimension", "methodology"))
                    sev = Severity(a.get("severity", "major"))
                    annotations.append(
                        ReviewAnnotation(
                            annotation_id=str(uuid.uuid4())[:8],
                            dimension=dim,
                            severity=sev,
                            location=a.get("location", ""),
                            headline=a.get("headline", ""),
                            comment=a.get("comment", ""),
                            suggestion=a.get("suggestion", ""),
                        )
                    )
                except Exception:
                    pass

            return SimulatedReview(
                review_id=str(uuid.uuid4())[:8],
                persona=persona.name,
                overall_score=parsed.get("overall_score", 5.0),
                summary=parsed.get("summary", ""),
                strengths=parsed.get("strengths", []),
                weaknesses=parsed.get("weaknesses", []),
                annotations=annotations,
                recommendation=parsed.get("recommendation", "borderline"),
            )

        except Exception as e:
            # Return empty review on error
            return SimulatedReview(
                review_id=str(uuid.uuid4())[:8],
                persona=persona.name,
                overall_score=5.0,
                summary=f"Review generation failed: {str(e)}",
                strengths=[],
                weaknesses=[],
                annotations=[],
                recommendation="borderline",
            )

    def _merge_reviews(self, reviews: List[SimulatedReview]) -> SimulatedReview:
        """Merge multiple reviews into a consensus report."""
        # Count severity across all reviews
        severity_scores = {"critical": 4, "major": 3, "minor": 1, "praise": 0}
        all_annotations: List[ReviewAnnotation] = []
        total_score = 0.0

        for r in reviews:
            total_score += r.overall_score
            all_annotations.extend(r.annotations)

        avg_score = total_score / len(reviews) if reviews else 5.0

        # Deduplicate annotations by headline similarity
        seen_headlines = set()
        unique_annotations = []
        for a in sorted(
            all_annotations, key=lambda x: severity_scores.get(x.severity.value, 0), reverse=True
        ):
            h = a.headline[:40].lower()
            if h not in seen_headlines:
                seen_headlines.add(h)
                unique_annotations.append(a)

        recommendations = [r.recommendation for r in reviews]
        final_rec = (
            max(set(recommendations), key=recommendations.count)
            if recommendations
            else "borderline"
        )

        return SimulatedReview(
            review_id=str(uuid.uuid4())[:8],
            persona="Consensus Panel (4 reviewers)",
            overall_score=round(avg_score, 1),
            summary=f"Consensus review from {len(reviews)} adversarial reviewers.",
            strengths=list({s for r in reviews for s in r.strengths})[:5],
            weaknesses=list({w for r in reviews for w in r.weaknesses})[:6],
            annotations=unique_annotations[:8],
            recommendation=final_rec,
        )

    def review_by_dimension(
        self,
        paper_text: str,
        dimension: ReviewDimension,
        api_key: Optional[str] = None,
    ) -> List[ReviewAnnotation]:
        """Focus review on a single dimension only."""
        persona = ReviewPersona(
            name=f"Focused {dimension.value} Reviewer",
            focus=[dimension],
            tone="hostile",
            priority_instructions=f"You specialize exclusively in {dimension.value}. Be extremely thorough in your area of expertise.",
        )
        review = self._review_with_persona(persona, paper_text, "", api_key, None, None)
        return review.annotations


# ─── Storage ────────────────────────────────────────────────────────


def _get_review_path() -> Path:
    path = Path.home() / ".ai_research_os" / "review_simulator"
    path.mkdir(parents=True, exist_ok=True)
    return path


def save_review(review: SimulatedReview) -> Path:
    """Save a review to disk."""
    path = _get_review_path()
    filepath = path / f"review_{review.review_id}.json"
    filepath.write_text(
        json.dumps(review.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8"
    )
    return filepath


def load_review(review_id: str) -> Optional[SimulatedReview]:
    """Load a saved review."""
    path = _get_review_path() / f"review_{review_id}.json"
    if not path.exists():
        return None
    try:
        d = json.loads(path.read_text(encoding="utf-8"))
        d.pop("review_id", None)
        annotations = []
        for a in d.pop("annotations", []):
            dim = ReviewDimension(a.get("dimension", "methodology"))
            sev = Severity(a.get("severity", "major"))
            annotations.append(
                ReviewAnnotation(
                    annotation_id=a.get("annotation_id", str(uuid.uuid4())[:8]),
                    dimension=dim,
                    severity=sev,
                    location=a.get("location", ""),
                    headline=a.get("headline", ""),
                    comment=a.get("comment", ""),
                    suggestion=a.get("suggestion", ""),
                    page_line=a.get("page_line", ""),
                    created_at=a.get("created_at", time.time()),
                )
            )
        return SimulatedReview(annotations=annotations, **d)
    except Exception:
        return None


def list_reviews(limit: int = 20) -> List[Dict[str, Any]]:
    """List saved reviews."""
    path = _get_review_path()
    reviews = []
    for f in sorted(path.glob("review_*.json"), reverse=True)[:limit]:
        try:
            d = json.loads(f.read_text(encoding="utf-8"))
            reviews.append(
                {
                    "review_id": d.get("review_id", f.stem),
                    "persona": d.get("persona", ""),
                    "overall_score": d.get("overall_score", 0),
                    "recommendation": d.get("recommendation", ""),
                    "created_at": d.get("created_at", ""),
                    "annotation_count": len(d.get("annotations", [])),
                }
            )
        except Exception:
            pass
    return reviews
