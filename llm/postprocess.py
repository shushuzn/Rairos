"""
Research Deep Dive Pipeline — 6-stage automated post-processing.

Triggered automatically after paper ingestion or manually via CLI.
Each stage is independently try/except wrapped so partial success is preserved.

Stages:
  1. PAPER_ANALYSIS — deep section filling + rubric scoring
  2. BENCHMARK — benchmark table detection + cross-paper comparison
  3. CROSS_REFERENCE — contradiction/alignment/extension detection
  4. INSIGHT — insight card extraction + research question generation
  5. KG_SYNC — knowledge graph integration
  6. PNODE_UPDATE — re-render P-note with analysis content
"""
from __future__ import annotations

import json
import logging
import os
from dataclasses import dataclass, field, asdict
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional, Tuple, TYPE_CHECKING

if TYPE_CHECKING:
    from pdf.extract import StructuredPdfContent

from core import Paper

logger = logging.getLogger(__name__)


class PostStage(Enum):
    """Pipeline execution stages."""
    PAPER_ANALYSIS = "paper_analysis"
    BENCHMARK = "benchmark"
    CROSS_REFERENCE = "cross_reference"
    INSIGHT = "insight"
    KG_SYNC = "kg_sync"
    PNODE_UPDATE = "pnote_update"


@dataclass
class StageResult:
    """Result of a single pipeline stage."""
    stage: str
    success: bool = False
    error: str = ""
    data: Any = None


@dataclass
class PostProcessingResult:
    """Complete result of the post-processing pipeline."""
    paper_id: str
    stages_completed: List[str] = field(default_factory=list)
    stages_failed: List[str] = field(default_factory=list)
    stage_results: Dict[str, StageResult] = field(default_factory=dict)
    pnote_updated: bool = False
    start_time: str = ""
    end_time: str = ""

    @property
    def all_succeeded(self) -> bool:
        return len(self.stages_failed) == 0

    @property
    def summary(self) -> str:
        total = len(self.stage_results)
        ok = len(self.stages_completed)
        fail = len(self.stages_failed)
        return f"[{ok}/{total}] stages OK, {fail} failed"


# ── Pipeline ─────────────────────────────────────────────────────────────────


class ResearchDeepDivePipeline:
    """Orchestrate 6-stage post-processing after paper ingestion.

    Usage:
        pipeline = ResearchDeepDivePipeline(db=db, data_dir=Path("AI-Research"))
        result = pipeline.run(paper_id="2604.22755", ...)
    """

    def __init__(
        self,
        db=None,
        data_dir: Optional[Path] = None,
        analysis_dir: Optional[Path] = None,
    ):
        self.db = db
        self.data_dir = data_dir or Path.cwd() / "AI-Research"
        # Analysis results stored in data_root/.analysis/{paper_id}/
        self.analysis_dir = analysis_dir or self.data_dir / ".analysis"

    def run(
        self,
        paper_id: str,
        extracted_text: str,
        paper: Optional[Paper] = None,
        tags: Optional[List[str]] = None,
        pnote_path: Optional[Path] = None,
        stages: Optional[List[PostStage]] = None,
        llm_config: Optional[Dict[str, Any]] = None,
        structured_content: Optional["StructuredPdfContent"] = None,
    ) -> PostProcessingResult:
        """Run the post-processing pipeline.

        Args:
            paper_id: Unique paper identifier.
            extracted_text: Full extracted PDF text.
            paper: Optional Paper dataclass (from _main_legacy). If None,
                   reconstructed from DB.
            tags: Paper tags.
            pnote_path: Path to the P-note file for updates.
            stages: Subset of stages to run (default: all).
            llm_config: LLM configuration dict with keys:
                api_key, base_url, model, timeout.
            structured_content: Optional structured PDF content for citation-grounded analysis.

        Returns:
            PostProcessingResult with per-stage outcomes.
        """
        # Normalize extracted_text: if it's a StructuredPdfContent, extract plain text
        if hasattr(extracted_text, 'text_blocks'):
            structured_content = extracted_text
            extracted_text = "\n".join(b.text for b in extracted_text.text_blocks)
        use_llm = bool(llm_config and llm_config.get("api_key"))
        if stages is None:
            stages = list(PostStage)

        tags = tags or []
        result = PostProcessingResult(
            paper_id=paper_id,
            start_time=datetime.now().isoformat(),
        )

        # Ensure analysis output directory exists
        paper_out = self.analysis_dir / paper_id
        paper_out.mkdir(parents=True, exist_ok=True)

        # ── Stage 1: Paper Analysis ─────────────────────────────────────
        sections_dict: Dict[str, str] = {}
        rubric_dict: Dict[str, Any] = {}

        if PostStage.PAPER_ANALYSIS in stages:
            sr = await_stage(PostStage.PAPER_ANALYSIS.value)
            try:
                from llm.paper_analyzer import PaperAnalyzer

                title, abstract, authors = self._get_paper_meta(paper, paper_id)
                analyzer = PaperAnalyzer(llm_config=llm_config)
                analysis = analyzer.analyze(
                    paper_id=paper_id,
                    title=title,
                    abstract=abstract,
                    body_text=extracted_text,
                    tags=tags,
                    authors=authors,
                    use_llm=use_llm,
                    structured_content=structured_content,
                )

                # Verify citation claims if structured content available
                if structured_content and analysis.llm_used:
                    analysis = analyzer.verify_claims(analysis, structured_content)
                sections_dict = analysis.sections
                rubric_dict = analysis.rubric
                sr.success = True
                sr.data = asdict(analysis)
                result.stages_completed.append(PostStage.PAPER_ANALYSIS.value)

                # Persist
                _write_json(paper_out / "paper_analysis.json", asdict(analysis))
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.PAPER_ANALYSIS.value)
                logger.warning("PAPER_ANALYSIS failed: %s", e)
            result.stage_results[PostStage.PAPER_ANALYSIS.value] = sr

        # ── Stage 2: Benchmark ──────────────────────────────────────────
        if PostStage.BENCHMARK in stages:
            sr = await_stage(PostStage.BENCHMARK.value)
            try:
                if self.db:
                    from llm.benchmark import BenchmarkComparator

                    comparator = BenchmarkComparator(db=self.db)
                    tables = comparator.detect_tables(paper_id)

                    benchmark_summary = {
                        "paper_id": paper_id,
                        "tables_found": len(tables),
                        "benchmarks": [
                            {"name": t.benchmark_name, "metrics": t.metrics}
                            for t in tables
                        ],
                    }
                    sr.success = True
                    sr.data = benchmark_summary
                    result.stages_completed.append(PostStage.BENCHMARK.value)
                    _write_json(paper_out / "benchmark.json", benchmark_summary)
                else:
                    sr.data = {"skipped": "no database"}
                    result.stages_completed.append(PostStage.BENCHMARK.value)
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.BENCHMARK.value)
                logger.warning("BENCHMARK failed: %s", e)
            result.stage_results[PostStage.BENCHMARK.value] = sr

        # ── Stage 3: Cross Reference ────────────────────────────────────
        if PostStage.CROSS_REFERENCE in stages:
            sr = await_stage(PostStage.CROSS_REFERENCE.value)
            try:
                from llm.cross_referencer import CrossReferencer

                title, abstract, _ = self._get_paper_meta(paper, paper_id)
                xref = CrossReferencer(db=self.db, llm_config=llm_config)
                xr_result = xref.analyze(
                    paper_id=paper_id,
                    title=title,
                    abstract=abstract,
                    body_text=extracted_text,
                    tags=tags,
                    use_llm=use_llm,
                )
                sr.success = True
                sr.data = asdict(xr_result)
                result.stages_completed.append(PostStage.CROSS_REFERENCE.value)
                _write_json(paper_out / "cross_reference.json", asdict(xr_result))
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.CROSS_REFERENCE.value)
                logger.warning("CROSS_REFERENCE failed: %s", e)
            result.stage_results[PostStage.CROSS_REFERENCE.value] = sr

        # ── Stage 4: Insight Cards ──────────────────────────────────────
        if PostStage.INSIGHT in stages:
            sr = await_stage(PostStage.INSIGHT.value)
            try:
                from llm.insight_cards import InsightManager
                from llm.question_tracker import QuestionTracker

                title, abstract, _ = self._get_paper_meta(paper, paper_id)
                insight_mgr = InsightManager()
                question_tracker = QuestionTracker()

                # Extract heuristic insights from text
                cards = insight_mgr.extract_from_text(paper_id, title, extracted_text)
                insight_ids = [c.card_id for c in cards]

                # Add a research question if LLM is available
                if use_llm:
                    question_tracker.add(
                        question=f"如何评估和验证 {title} 提出的方法在实际场景中的表现?",
                        source="literature_review",
                        topic=tags[0] if tags else "",
                        priority=6,
                    )

                insight_data = {
                    "paper_id": paper_id,
                    "cards_created": len(cards),
                    "card_ids": insight_ids,
                }
                sr.success = True
                sr.data = insight_data
                result.stages_completed.append(PostStage.INSIGHT.value)
                _write_json(paper_out / "insight_cards.json", insight_data)
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.INSIGHT.value)
                logger.warning("INSIGHT failed: %s", e)
            result.stage_results[PostStage.INSIGHT.value] = sr

        # ── Stage 5: KG Sync ────────────────────────────────────────────
        if PostStage.KG_SYNC in stages:
            sr = await_stage(PostStage.KG_SYNC.value)
            try:
                from kg.integration import KGIntegration

                # Try to get a KGManager instance
                kg = self._get_kg()
                if kg:
                    integration = KGIntegration(kg)
                    integration.on_paper_processed(
                        paper_uid=paper_id,
                        pnote_path=pnote_path,
                        paper_title=title if paper else None,
                        paper_tags=tags,
                    )
                    sr.success = True
                    sr.data = {"synced": True}
                    result.stages_completed.append(PostStage.KG_SYNC.value)
                    _write_json(paper_out / "kg_sync.json", {"synced": True})
                else:
                    sr.data = {"skipped": "no KG available"}
                    result.stages_completed.append(PostStage.KG_SYNC.value)
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.KG_SYNC.value)
                logger.warning("KG_SYNC failed: %s", e)
            result.stage_results[PostStage.KG_SYNC.value] = sr

        # ── Stage 6: P-Note Update ──────────────────────────────────────
        if PostStage.PNODE_UPDATE in stages and pnote_path:
            sr = await_stage(PostStage.PNODE_UPDATE.value)
            try:
                if sections_dict or rubric_dict:
                    self._update_pnote(
                        pnote_path=pnote_path,
                        paper=paper,
                        paper_id=paper_id,
                        tags=tags,
                        extracted_text=extracted_text,
                        sections_dict=sections_dict,
                        rubric_dict=rubric_dict,
                    )
                    result.pnote_updated = True

                sr.success = True
                result.stages_completed.append(PostStage.PNODE_UPDATE.value)
            except Exception as e:
                sr.error = str(e)
                result.stages_failed.append(PostStage.PNODE_UPDATE.value)
                logger.warning("PNODE_UPDATE failed: %s", e)
            result.stage_results[PostStage.PNODE_UPDATE.value] = sr

        result.end_time = datetime.now().isoformat()

        # Write pipeline summary
        _write_json(paper_out / "pipeline_result.json", _result_summary(result))
        return result

    # ── Helpers ──────────────────────────────────────────────────────────

    def _get_paper_meta(
        self, paper: Optional[Paper], paper_id: str,
    ) -> Tuple[str, str, Optional[List[str]]]:
        """Get paper metadata from Paper object or database."""
        if paper is not None:
            return (
                paper.title,
                paper.abstract or "",
                paper.authors,
            )
        if self.db:
            try:
                rec = self.db.get_paper(paper_id)
                if rec:
                    return (
                        getattr(rec, "title", paper_id),
                        getattr(rec, "abstract", ""),
                        getattr(rec, "authors", None),
                    )
            except Exception:
                pass
        return (paper_id, "", None)

    def _get_kg(self):
        """Get KGManager instance if available."""
        try:
            from kg.manager import KGManager
            return KGManager(data_dir=str(self.data_dir))
        except Exception:
            try:
                from kg.manager import GraphManager
                return GraphManager()
            except Exception:
                return None

    def _update_pnote(
        self,
        pnote_path: Path,
        paper: Optional[Paper],
        paper_id: str,
        tags: List[str],
        extracted_text: str,
        sections_dict: Dict[str, str],
        rubric_dict: Dict[str, Any],
    ) -> None:
        """Re-render P-note with analysis content injected."""
        from renderers.pnote import render_pnote

        if paper is not None:
            p = paper
        elif self.db:
            rec = self.db.get_paper(paper_id)
            if rec:
                p = Paper(
                    source=getattr(rec, "source", "arxiv"),
                    uid=getattr(rec, "id", paper_id),
                    title=getattr(rec, "title", paper_id),
                    authors=getattr(rec, "authors", []),
                    abstract=getattr(rec, "abstract", "") or "",
                    published=getattr(rec, "published", "") or "",
                    updated=getattr(rec, "updated", "") or "",
                    abs_url=getattr(rec, "abs_url", f"https://arxiv.org/abs/{paper_id}"),
                    pdf_url=getattr(rec, "pdf_url", ""),
                    primary_category=getattr(rec, "primary_category", ""),
                )
            else:
                logger.warning("Paper %s not found in DB, using stub", paper_id)
                p = Paper(
                    source="arxiv", uid=paper_id, title=paper_id,
                    authors=[], abstract="", published="", updated="",
                    abs_url=f"https://arxiv.org/abs/{paper_id}", pdf_url="",
                )
        else:
            logger.warning("No paper data available, using stub for %s", paper_id)
            p = Paper(
                source="arxiv", uid=paper_id, title=paper_id,
                authors=[], abstract="", published="", updated="",
                abs_url=f"https://arxiv.org/abs/{paper_id}", pdf_url="",
            )

        # Add raw_llm_output as __raw__ for display
        sections_with_raw = dict(sections_dict)
        if "__raw__" not in sections_with_raw:
            sections_with_raw["__raw__"] = rubric_dict.get("_raw_llm_output", "")

        rendered = render_pnote(
            p=p,
            tags=tags,
            extracted_sections_md=extracted_text,
            parsed_ai=(sections_with_raw, rubric_dict),
        )
        pnote_path.write_text(rendered, encoding="utf-8")
        logger.info("P-note updated: %s", pnote_path)


# ── Helpers ──────────────────────────────────────────────────────────────────


def await_stage(name: str) -> StageResult:
    """Create a new StageResult for the given stage name."""
    return StageResult(stage=name)


def _result_summary(result: PostProcessingResult) -> dict:
    """Serialize PostProcessingResult to a JSON-safe dict."""
    return {
        "paper_id": result.paper_id,
        "stages_completed": result.stages_completed,
        "stages_failed": result.stages_failed,
        "pnote_updated": result.pnote_updated,
        "start_time": result.start_time,
        "end_time": result.end_time,
        "summary": result.summary,
    }


def _write_json(path: Path, data: Any) -> None:
    """Write JSON data to a file (safe for dataclass dicts)."""
    path.write_text(
        json.dumps(data, ensure_ascii=False, indent=2, default=str),
        encoding="utf-8",
    )


# ── LLM config factory ───────────────────────────────────────────────────────


def make_llm_config(
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> Optional[Dict[str, Any]]:
    """Build an LLM config dict from environment variables or explicit args.

    Returns None if no API key is available (graceful degradation).
    """
    api_key = api_key or os.environ.get("OPENAI_API_KEY", "")
    if not api_key:
        return None

    return {
        "api_key": api_key,
        "base_url": base_url or os.environ.get(
            "OPENAI_BASE_URL",
            os.environ.get(
                "AIROS_DEFAULT_OPENAI_BASE_URL",
                "https://api.openai.com/v1",
            ),
        ),
        "model": model or os.environ.get(
            "AIROS_DEFAULT_MODEL_CLI",
            "gpt-4o-mini",
        ),
        "timeout": int(os.environ.get("AIROS_LLM_TIMEOUT", "300")),
    }
