"""Embodied planning web routes."""

from __future__ import annotations

from typing import Dict
from fastapi import APIRouter, Request
from web.shared import get_db, templates
from web.app import _notification_store


# ── Global representation type counts ──────────────────────────────────────
def _get_global_rep_type_counts() -> Dict[str, int]:
    """Return aggregate representation type counts from the Gene Pool."""
    from llm.gene_pool_io import load_capsules
    capsules = load_capsules(gap_type="embodied_planning")
    counts: Dict[str, int] = {"discrete": 0, "continuous": 0, "hybrid": 0, "unknown": 0}
    for c in capsules:
        rt = c.get("representation_type", c.get("action_gap_type", "unknown"))
        counts[rt] = counts.get(rt, 0) + 1
    return counts

router = APIRouter()


@router.get("/embodied-planning/batch")
async def embodied_planning_batch(request: Request, ids: str = ""):
    """Batch analyze multiple papers for embodied planning representation types.

    Query param: ids=pid1,pid2,pid3
    Returns comparative report: discrete vs continuous vs hybrid grouping,
    contradiction pairs, and summary statistics.
    """
    if not ids:
        return {"error": "Provide paper IDs via ?ids=pid1,pid2,pid3"}
    paper_ids = [p.strip() for p in ids.split(",") if p.strip()]
    if not paper_ids:
        return {"error": "No valid paper IDs provided"}

    from llm.paper_gap_extractor import batch_analyze_embodied_planning

    result = batch_analyze_embodied_planning(paper_ids=paper_ids)
    return result


@router.get("/embodied-planning/dashboard")
async def embodied_planning_dashboard(request: Request):
    """Render the embodied planning domain-wide dashboard.

    Shows all analyzed papers grouped by representation type (discrete/
    continuous/hybrid), with confidence scores and contradiction pairs.
    """
    from llm.paper_gap_extractor import render_embodied_planning_dashboard

    html = render_embodied_planning_dashboard()
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-planning-dashboard",
            "title": "🦾 Embodied Planning — Representation Atlas",
            "content": html,
        },
    )


@router.get("/embodied-planning/evolution")
async def embodied_evolution_timeline(request: Request):
    """Render Mermaid Gantt chart showing belief evolution over time."""
    from llm.paper_gap_extractor import render_evolution_timeline

    graph = render_evolution_timeline()
    if not graph:
        graph = "<div style='text-align:center;padding:40px;color:#888;'>No timeline data yet — analyze some papers first.</div>"
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-evolution",
            "title": "🦾 Belief Evolution Timeline",
            "content": f"<div style='overflow-x:auto;'>{graph}</div>",
        },
    )


@router.get("/embodied-planning/compare")
async def embodied_planning_compare(request: Request, ids: str = ""):
    """Compare representation types across 2 papers side-by-side."""
    from llm.paper_gap_extractor import render_compare_view

    paper_ids = [p.strip() for p in ids.split(",") if p.strip()][:2]
    html = render_compare_view(paper_ids)
    return templates.TemplateResponse(
        request,
        "generic.html",
        {
            "page": "embodied-compare",
            "title": "🦾 Embodied Planning — Compare",
            "content": html,
        },
    )


@router.get("/embodied-planning/semantic-search")
async def semantic_search(request: Request, q: str = "", top_k: int = 5):
    """Semantic search across analyzed papers."""
    from fastapi.responses import JSONResponse
    from llm.paper_gap_extractor import semantic_search_papers

    results = semantic_search_papers(q, top_k=top_k)
    return JSONResponse({"query": q, "results": results})
    return JSONResponse({"query": q, "results": results})


@router.post("/embodied-planning/auto-scan")
async def embodied_planning_auto_scan(request: Request):
    """Auto-scan new VLA/robotics papers from subscriptions for embodied planning analysis.

    Runs subscription check, then for each new paper auto-analyzes
    embodied planning representation type and saves to Gene Pool.
    """
    from fastapi.responses import JSONResponse

    try:
        db = get_db()
        from llm.subscription_monitor import SubscriptionMonitor
        from llm.paper_gap_extractor import run_embodied_analysis

        monitor = SubscriptionMonitor(db)
        results = monitor.check_all()  # {topic: [paper_ids]}
        all_new_ids = []
        for papers in results.values():
            all_new_ids.extend(papers)

        # Filter to VLA/robotics papers only, then run shared analysis pipeline
        vla_ids = [
            pid
            for pid in all_new_ids
            if any(
                c in (db.get_paper(pid).categories or "").lower()
                for c in ["cs.ro", "cs.cv", "cs.ai", "cs.lg"]
            )
        ]
        result = run_embodied_analysis(vla_ids, db=db, save_to_pool=True)
        analyzed = result["analyzed"]
        contradictions = result["contradictions"]
        type_counts = result["type_counts"]
        total_analyzed = result["total_analyzed"]
        trend = result["trend"]
        trend_pct = result["trend_pct"]

        # Recommend next paper: under-represented type
        all_counts = _get_global_rep_type_counts()
        for rt in ["discrete", "continuous", "hybrid"]:
            all_counts[rt] = all_counts.get(rt, 0) + type_counts.get(rt, 0)
        underrep = min(all_counts, key=lambda k: all_counts[k]) if all_counts else "hybrid"
        recommend_msg = ""
        if total_analyzed > 0:
            recommend_msg = (
                f"Only {type_counts.get(underrep, 0)}/{total_analyzed} papers "
                f"in this batch used {underrep} — consider searching for more."
            )

        notification = None
        if contradictions:
            notification = {
                "type": "contradiction",
                "uid": f"contra_{all_new_ids[0] if all_new_ids else 'none'}",
                "message": f"⚠️ {len(contradictions)} contradiction(s) detected — papers disagree on representation type",
                "details": contradictions[:3],
            }
            _notification_store.append(notification)

            # ── Hypothesis generation from contradictions ──────────────────────
            try:
                from llm.paper_gap_extractor import (
                    generate_hypothesis_from_contradiction,
                    append_hypothesis_to_roadmap,
                )

                for contra in contradictions:
                    pid_a = contra.get("paper_id", "")
                    pid_b = contra.get("contradiction_with", "")
                    contra_pair = {
                        "paper_a_id": pid_a,
                        "paper_b_id": pid_b,
                        "paper_a_title": contra.get("title", ""),
                        "paper_b_title": "",
                        "representation_a": contra.get("representation_type", "unknown"),
                        "representation_b": contra.get("contradiction_type", "unknown"),
                        "effectiveness_a": "effective",
                        "effectiveness_b": "ineffective",
                    }
                    hyp = generate_hypothesis_from_contradiction(contra_pair)
                    append_hypothesis_to_roadmap(hyp, pid_a, pid_b)
            except Exception:
                pass  # Non-critical: don't fail scan if hypothesis generation fails
        elif total_analyzed > 2 and trend_pct > 0.7:
            notification = {
                "type": "trend",
                "uid": f"trend_{all_new_ids[0] if all_new_ids else 'none'}",
                "message": f"📊 Strong trend: {trend} representation dominates ({int(trend_pct * 100)}% of this batch)",
            }
            _notification_store.append(notification)

        # ── Task 2: Append recommended papers to ROADMAP.md ──────────────────
        if total_analyzed > 0 and analyzed:
            try:
                recommended_type = underrep
                # Pick the top paper from the batch that matches the under-represented type
                rec_paper = next(
                    (r for r in analyzed if r.get("representation_type") == recommended_type),
                    analyzed[0] if analyzed else None,
                )
                if rec_paper:
                    from pathlib import Path as _Path

                    _rm_path = _Path("D:/OpenClaw/workspace/80-PROJECTS/ai_research_os/ROADMAP.md")
                    if _rm_path.exists():
                        existing_content = _rm_path.read_text(encoding="utf-8")
                        # Only write if not already listed (avoid duplicates)
                        rec_id = rec_paper.get("paper_id", "unknown")
                        rec_title = rec_paper.get("title", "Unknown Title")
                        marker = f"[ ] {recommended_type} paper: {rec_title} ({rec_id})"
                        if marker not in existing_content:
                            # Append under v2.2 section
                            with open(_rm_path, "a", encoding="utf-8") as _f:
                                _f.write(f"\n### Pending Readings\n- {marker}\n")
            except Exception as _e:
                pass  # Non-critical: don't fail the scan if roadmap write fails

        # ── Fallback: Gene Pool keyword-driven scan if no subscriptions configured ──
        if not all_new_ids:
            try:
                from llm.gene_pool_io import load_capsules, get_capsule_by_paper
                from llm.paper_gap_extractor import analyze_gap
                from llm.subscription_monitor import search_arxiv

                GAP = "embodied_planning"
                capsules = load_capsules(status="active", gap_type=GAP)
                kw_set: set = set()
                for cap in capsules:
                    for kw in cap.get("trigger_keywords", []):
                        if len(kw) > 3:
                            kw_set.add(kw.lower())
                kw_list = list(kw_set)[:8]
                query = " AND ".join(f'"{k}"' for k in kw_list)
                papers = search_arxiv(query, max_results=6)

                db = get_db()
                new_analyzed = []
                for p in papers:
                    arxiv_id = p.get("arxiv_id", "")
                    if get_capsule_by_paper(arxiv_id, gap_type=GAP):
                        continue  # already in Gene Pool
                    r = analyze_gap(
                        paper_id=arxiv_id,
                        title=p.get("title", ""),
                        abstract=p.get("abstract", ""),
                        gap_type=GAP,
                    )
                    capsule_id = r.get("saved_to_pool")
                    new_analyzed.append(
                        {
                            "paper_id": arxiv_id,
                            "title": p.get("title", "")[:80],
                            "representation_type": r.get("representation_type", "unknown"),
                            "confidence": r.get("confidence", 0),
                            "capsule_id": capsule_id,
                        }
                    )
                if new_analyzed:
                    all_new_ids = [p["paper_id"] for p in new_analyzed]
                    analyzed = new_analyzed
                    notification = None
                    recommend_msg = (
                        f"Scanned {len(new_analyzed)} new papers via Gene Pool keywords "
                        f"(no subscriptions configured — using keyword fallback)."
                    )
            except Exception:
                pass  # Non-critical fallback

        return JSONResponse(
            {
                "success": True,
                "total_new": len(all_new_ids),
                "analyzed": len(analyzed),
                "results": analyzed,
                "contradictions": contradictions,
                "trend": {"dominant": trend, "pct": int(trend_pct * 100), "counts": type_counts},
                "recommended_next_type": underrep,
                "recommend_msg": recommend_msg,
                "notification": notification,
            }
        )
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)


@router.post("/embodied-planning/search")
async def embodied_planning_search(request: Request):
    """主动搜索arXiv论文并分析embodied planning representation type.

    Query: "latent reasoning" OR "physical reasoning" OR "embodied planning" site:arxiv.org
    """
    from fastapi.responses import JSONResponse
    from pathlib import Path as _Path

    try:
        body = await request.json()
        query = body.get("query", "latent reasoning OR physical reasoning OR embodied planning")
        max_results = body.get("max_results", 10)

        # Use SubscriptionMonitor.search_arxiv — unified, no duplicate XML parsing
        from llm.subscription_monitor import search_arxiv

        papers = search_arxiv(query, max_results)

        if not papers:
            return JSONResponse({"success": True, "query": query, "results": [], "analyzed": []})

        # Analyze each paper with analyze_embodied_planning
        from llm.paper_gap_extractor import analyze_embodied_planning

        analyzed = []
        for p in papers:
            result = analyze_embodied_planning(
                paper_id=p["arxiv_id"],
                title=p["title"],
                abstract=p["abstract"],
            )
            result["arxiv_id"] = p["arxiv_id"]
            result["published"] = p["published"]
            analyzed.append(result)

        return JSONResponse(
            {
                "success": True,
                "query": query,
                "total": len(papers),
                "analyzed": len(analyzed),
                "results": analyzed,
            }
        )
    except Exception as e:
        return JSONResponse({"success": False, "error": str(e)}, status_code=500)
