"""Insights, impact, and experiment tracking routes."""
from __future__ import annotations

from fastapi import APIRouter, Request
import json
from pathlib import Path
from web.shared import templates, get_db

router = APIRouter()


@router.get("/insights")
async def insights(request: Request):
    """Research Insights board."""
    suggestions = []
    prefetched_ids = set()
    capsules = []
    stats = {}
    archetype = {}
    gap_prefs = {}
    topic_freq = {}
    events_display = []
    exp_stats = {}
    try:
        from pathlib import Path
        import json
        from llm.insight.tracker import EvolutionTracker

        tracker = EvolutionTracker()
        caps = tracker._load_capsules()
        for c in caps[-20:]:
            capsules.append((c.capsule_id[:12], c.trigger_topic[:60], c.action_gap_type,
                           c.action_gap_title[:80], c.outcome_success_score,
                           c.created_at[:10], c.trigger_keywords[:5], c.status))
        stats = tracker.get_gene_pool_stats()
        profile = tracker.get_profile()
        gap_prefs = dict(sorted((profile.gap_type_preferences or {}).items(), key=lambda x: x[1], reverse=True))
        topic_freq = dict(sorted((profile.topic_frequency or {}).items(), key=lambda x: x[1], reverse=True)[:8])
        exp_stats = tracker.get_exploration_stats()
    except Exception:
        pass
    return templates.TemplateResponse(
        request, "insights.html",
        {"page": "insights", "capsules": capsules, "gene_pool_stats": stats,
         "archetype": archetype, "gap_prefs": gap_prefs, "topic_freq": topic_freq,
         "events": events_display, "exp_stats": exp_stats,
         "suggestions": suggestions, "prefetched_ids": prefetched_ids},
    )



@router.post("/insights/accept-suggestion")
async def accept_suggestion(request: Request):
    """Accept an actionable suggestion — record it as a gap acceptance event.

    闭环: marks the source capsule as 'consumed' so it won't repeat.
    """
    body = await request.json()
    topic = body.get("topic", "")
    gap_type = body.get("gap_type", "")
    gap_title = body.get("title", "")
    description = body.get("body", "")
    s_type = body.get("type", "")
    source_cap_id = body.get("source_cap_id") or None

    try:
        from llm.insight.tracker import EvolutionTracker

        tracker = EvolutionTracker()
        tracker.record_gap_accept(
            topic=topic or "insights",
            gap_type=gap_type,
            gap_title=gap_title[:200],
            gap_description=description,
        )
        # Also encode as a capsule
        tracker.encode_capsule(
            topic=topic or "insights",
            gap_type=gap_type,
            gap_title=gap_title[:200],
            gap_description=description,
            success_score=0.7,
        )
        _mark_suggestion_consumed(gap_type, topic, gap_title, s_type)

        # Mark source capsule as 'consumed' — prevents duplicate suggestions
        if source_cap_id:
            mark_capsule_consumed(source_cap_id, tracker)

        # Trigger evolution闭环 — this is what actually IMPROVES the Gene Pool
        try:
            from llm.insight.evolution import InsightEvolution

            evo = InsightEvolution(tracker=tracker)
            evo_result = evo.evolve(topic=topic or "insights")
            improved = evo_result.get("result", {}).get("added", 0)
            return {"success": True, "evolved": improved}
        except Exception as evo_err:
            import logging

            logging.getLogger(__name__).warning(f"Evolution trigger failed: {evo_err}")
            return {"success": True, "evolved": 0}
    except Exception as e:
        import logging

        logging.getLogger(__name__).warning(f"accept_suggestion failed: {e}")
        return {"success": False, "error": str(e)}




@router.get("/impact")
async def impact(request: Request):
    """Impact Ranking — leaderboard."""
    db = get_db()
    rows, _ = db.list_papers(
        limit=100
    )  # no citation_count column — sort in Python after fetching real counts

    papers = []
    for r in rows:
        pid = getattr(r, "paper_id", "") or getattr(r, "id", "")
        if not pid:
            continue
        citation_data = db.get_citation_count(pid)
        year_raw = getattr(r, "published", "") or ""
        try:
            year = int(str(year_raw)[:4]) if year_raw else 2020
        except (ValueError, TypeError):
            year = 2020
        papers.append(
            {
                "paper_id": pid,
                "title": r.title,
                "year": year,
                "citation_count": citation_data.get("forward", 0) or 0,
            }
        )

    # Sort by citation_count desc, then score in Python
    papers.sort(key=lambda p: p["citation_count"], reverse=True)
    papers = papers[:50]

    try:
        from llm.impact_scorer import ImpactScorer

        scorer = ImpactScorer(db=db)
        ranking = scorer.rank_papers(papers, top_k=min(20, len(papers)))
    except Exception:
        ranking = []

    return templates.TemplateResponse(
        request,
        "impact.html",
        {
            "page": "impact",
            "ranking": ranking,
        },
    )


@router.get("/insights/experiments")
async def insights_experiments(request: Request):
    """List queued experiment proposals."""
    queue = get_experiment_queue()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "experiments",
            "title": "🔬 Experiment Proposals",
            "content": render_experiments_html(queue),
        },
    )




@router.post("/insights/generate-experiment")
async def generate_experiment(request: Request):
    """Generate a concrete experiment proposal from a gap/suggestion.

    If source_cap_id is provided, resolves it to a paper_id so the
    experiment can run Paper2Code when executed.
    """
    body = await request.json()
    gap_type = body.get("gap_type", "")
    topic = body.get("topic", "")
    gap_title = body.get("title", "")
    description = body.get("body", "")
    keywords = body.get("keywords", [])
    source_cap_id = body.get("source_cap_id", "")

    # Resolve source_cap_id → paper_id
    paper_id = ""
    if source_cap_id:
        try:
            from llm.gene_pool_io import load_capsules
            capsules = load_capsules()
            for c in capsules:
                if c.get("capsule_id", "") == source_cap_id:
                    paper_id = c.get("archetype", {}).get("source_paper_id", "") or ""
                    break
        except Exception:
            pass

    try:
        from llm.paper_gap_extractor import gaps_to_research_questions

        frontier_gaps = [
            {
                "gap_title": gap_title or topic or "Research gap",
                "gap_type": gap_type,
                "keywords": keywords,
                "summary": description,
            }
        ]
        questions_result = gaps_to_research_questions(frontier_gaps)
        questions = questions_result.get("questions", [])
        if questions:
            q = questions[0]  # take top question as experiment
            exp_title = q.get("question", gap_title or topic)[:100]
        else:
            q = {}
            exp_title = gap_title or topic or "Experiment"
        exp_id = f"exp_{gap_type[:10]}_{topic[:15].replace(' ', '_')}"
        exp = {
            "id": exp_id,
            "title": exp_title,
            "gap_type": gap_type,
            "topic": topic,
            "paper_id": paper_id,
            "description": q.get("question", description),
            "hypothesis": q.get("hypothesis", ""),
            "difficulty": q.get("difficulty", "medium"),
            "method": q.get("question", ""),
            "keywords": keywords,
            "status": "pending",
            "created_at": datetime.now().isoformat(),
        }
        save_experiment(exp)
        return {"success": True, "experiment": exp}
    except Exception as e:
        import logging

        logging.getLogger(__name__).warning(f"Experiment generation failed: {e}")
        return {"success": False, "error": str(e)}



@router.post("/insights/run-experiment")
async def run_experiment(request: Request):
    """Trigger paper2code pipeline for an experiment proposal (background)."""
    body = await request.json()
    exp_id = body.get("id", "")
    queue = get_experiment_queue()
    exp = next((e for e in queue if e.get("id") == exp_id), None)
    if not exp:
        return {"success": False, "error": "Experiment not found"}

    # Update status
    exp["status"] = "running"
    exp["started_at"] = datetime.now().isoformat()
    _save_experiment(exp)

    # Run in background thread
    import threading

    def _run():
        try:
            paper_id = exp.get("paper_id", "")
            if paper_id:
                from research_loop.paper2code_integration import PaperPipeline

                pipeline = PaperPipeline()
                result = pipeline.run(paper_id)
                exp["status"] = "done"
                exp["result"] = result
            else:
                # No specific paper — just update Gene Pool with verdict-based score
                tracker = _get_tracker()
                tracker.record_gap_accept(
                    topic=exp.get("topic", "experiment"),
                    gap_type=exp.get("gap_type", ""),
                    gap_title=exp.get("title", "")[:200],
                    gap_description=exp.get("description", ""),
                )
                exp["status"] = "done"
                exp["result"] = {"verdict_encoded": True}
        except Exception as e:
            exp["status"] = "failed"
            exp["error"] = str(e)
        finally:
            save_experiment(exp)

    t = threading.Thread(target=_run, daemon=True)
    t.start()
    return {"success": True, "message": f"Experiment '{exp_id}' started in background"}


