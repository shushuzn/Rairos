"""Research Loop and Daemon web routes."""

from __future__ import annotations

from fastapi import APIRouter, Request
from fastapi.responses import RedirectResponse

from web.shared import get_db, templates

router = APIRouter()


@router.get("/daemon")
async def daemon_dashboard(request: Request):
    """Unified daemon dashboard — orchestrator + evolution + credibility in one view."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=False)
        status = orch.get_status()
        alerts_raw = orch.get_recent_alerts(limit=20)

        # Gene Pool stats
        try:
            from llm.insight.tracker import EvolutionTracker

            tracker = EvolutionTracker()
            pool_stats = tracker.get_gene_pool_stats()
        except Exception:
            pool_stats = {}

        # Credibility stats
        try:
            from llm.insight.credibility import CredibilityScorer
            from llm.insight.gene import CapsuleGene

            scorer = CredibilityScorer()
            gene_file = Path.home() / ".ai_research_os" / "evolution" / "gene_pool.jsonl"
            capsules = []
            if gene_file.exists():
                import json as _json

                with open(gene_file, encoding="utf-8") as f:
                    for line in f:
                        if line.strip():
                            try:
                                capsules.append(CapsuleGene.from_dict(_json.loads(line)))
                            except Exception:
                                continue
            cred_scores = scorer.compute_novelty_scores(capsules) if capsules else {}
            trendslop_count = sum(1 for s in cred_scores.values() if s.trendslop)
            high_count = sum(1 for s in cred_scores.values() if s.badge == "high")
            cred_stats = {
                "total": len(cred_scores),
                "trendslop": trendslop_count,
                "high": high_count,
            }
        except Exception:
            cred_stats = {}
    except Exception:
        status = {
            "running": False,
            "interval_minutes": 30,
            "last_check": "",
            "alerts_count": 0,
            "error": "Orchestrator unavailable",
        }
        alerts_raw = []
        pool_stats = {}
        cred_stats = {}

    alerts = []
    for a in alerts_raw:
        created = a.created_at if hasattr(a, "created_at") else a.get("created_at", "")
        alerts.append(
            (
                a.alert_id,
                a.session_id,
                a.topic,
                a.triggered_by,
                a.trigger_title,
                a.gaps_found,
                a.top_gap_title,
                a.top_gap_type,
                a.severity,
                a.gene_pool_score,
                a.preference_boost,
                created,
            )
        )

    return templates.TemplateResponse(
        request,
        "daemon.html",
        {
            "page": "daemon",
            "status": status,
            "alerts": alerts[:20],
            "pool_stats": pool_stats,
            "cred_stats": cred_stats,
        },
    )


@router.get("/research-loop")
async def research_loop(request: Request):
    """Research Loop dashboard — status, alerts, subscriptions."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=False)
        status = orch.get_status()
        alerts_raw = orch.get_recent_alerts(limit=20)
    except Exception:
        status = {
            "running": False,
            "interval_minutes": 30,
            "last_check": "",
            "alerts_count": 0,
            "error": "Orchestrator unavailable",
        }
        alerts_raw = []

    alerts = []
    for a in alerts_raw:
        created = a.created_at if hasattr(a, "created_at") else a.get("created_at", "")
        alerts.append(
            (
                a.alert_id,
                a.session_id,
                a.topic,
                a.triggered_by,
                a.trigger_title,
                a.gaps_found,
                a.top_gap_title,
                a.top_gap_type,
                a.severity,
                a.gene_pool_score,
                a.preference_boost,
                created,
            )
        )

    try:
        db = get_db()
        subs_raw = db.list_arxiv_subscriptions()
    except Exception:
        subs_raw = []

    subscriptions = []
    for s in subs_raw:
        subscriptions.append(
            {
                "id": s.get("id", ""),
                "topic": s.get("topic", ""),
            }
        )

    return templates.TemplateResponse(
        request,
        "research_loop.html",
        {
            "page": "research-loop",
            "status": status,
            "alerts": alerts,
            "subscriptions": subscriptions,
        },
    )


@router.post("/research-loop/start")
async def research_loop_start(request: Request):
    """Start the autonomous watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=False)
        orch.start_watch(interval_minutes=30)
    except Exception as e:
        import logging

        logging.getLogger(__name__).warning(f"Could not start orchestrator: {e}")
    return RedirectResponse(url="/research-loop", status_code=303)


@router.post("/research-loop/stop")
async def research_loop_stop(request: Request):
    """Stop the autonomous watch loop."""
    try:
        from research_loop.orchestrator import AutonomousOrchestrator

        orch = AutonomousOrchestrator(webhook_enabled=False)
        orch.stop_watch()
    except Exception:
        pass
    return RedirectResponse(url="/research-loop", status_code=303)


@router.post("/research-loop/run-cycle")
async def research_loop_run_cycle(request: Request):
    """Manually trigger one orchestrator cycle."""
    import threading

    def run():
        try:
            from research_loop.orchestrator import AutonomousOrchestrator

            orch = AutonomousOrchestrator(webhook_enabled=False)
            orch.run_cycle()
        except Exception as e:
            import logging

            logging.getLogger(__name__).error(f"Run cycle failed: {e}")

    threading.Thread(target=run, daemon=True).start()
    return RedirectResponse(url="/research-loop", status_code=303)


@router.get("/research-loop/squad")
async def squad_dashboard(request: Request):
    """Squad dashboard — multi-agent activity stream."""
    try:
        from research_loop.agents.squad import SquadCoordinator

        coord = SquadCoordinator()
        squad_status = coord.get_status()
        activity = coord.get_activity(limit=50)
        alerts = coord.get_alerts(limit=20)
    except Exception as e:
        squad_status = {
            "running": False,
            "agents": {},
            "error": str(e),
            "interval_minutes": 30,
            "last_cycle": "",
        }
        activity = []
        alerts = []

    return templates.TemplateResponse(
        request,
        "squad_dashboard.html",
        {
            "page": "squad-dashboard",
            "squad_status": squad_status,
            "activity": activity,
            "alerts": alerts,
        },
    )


@router.post("/research-loop/squad/start")
async def squad_start(request: Request):
    """Start the multi-agent squad."""
    try:
        from research_loop.agents.squad import SquadCoordinator

        coord = SquadCoordinator()
        coord.start_watch(interval_minutes=30)
    except Exception as e:
        import logging

        logging.getLogger(__name__).warning(f"Could not start squad: {e}")
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@router.post("/research-loop/squad/stop")
async def squad_stop(request: Request):
    """Stop the multi-agent squad."""
    try:
        from research_loop.agents.squad import SquadCoordinator

        coord = SquadCoordinator()
        coord.stop_watch()
    except Exception:
        pass
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@router.post("/research-loop/squad/run-cycle")
async def squad_run_cycle(request: Request):
    """Manually trigger one squad cycle."""
    import threading

    def run():
        try:
            from research_loop.agents.squad import SquadCoordinator

            coord = SquadCoordinator()
            coord.run_cycle()
        except Exception as e:
            import logging

            logging.getLogger(__name__).error(f"Squad cycle failed: {e}")

    threading.Thread(target=run, daemon=True).start()
    return RedirectResponse(url="/research-loop/squad", status_code=303)


@router.get("/research-loop/squad/stream")
async def squad_stream():
    """Server-Sent Events — real-time agent activity stream."""
    from fastapi.responses import StreamingResponse
    import asyncio

    async def event_generator():
        import time as _time

        _last_len = 0
        try:
            from research_loop.agents.squad import SquadCoordinator

            coord = SquadCoordinator()
            while True:
                activity = coord.get_activity(limit=50)
                status = coord.get_status()
                alerts = coord.get_alerts(limit=10)

                # Only emit if something changed
                if len(activity) != _last_len:
                    _last_len = len(activity)
                    payload = {
                        "activity": activity[-20:],
                        "agents": status.get("agents", {}),
                        "alerts": alerts,
                        "running": status.get("running", False),
                    }
                    yield f"data: {__import__('json').dumps(payload)}\n\n"

                await asyncio.sleep(3)
        except Exception as e:
            yield f"data: {__import__('json').dumps({'error': str(e)})}\n\n"

    return StreamingResponse(
        event_generator(),
        media_type="text/event-stream",
        headers={
            "Cache-Control": "no-cache",
            "Connection": "keep-alive",
            "X-Accel-Buffering": "no",
        },
    )


@router.get("/research-loop/squad/activity")
async def squad_activity():
    """JSON endpoint for squad activity stream + gap watch stats."""
    try:
        from research_loop.agents.squad import SquadCoordinator

        coord = SquadCoordinator()
        activity = coord.get_activity(limit=50)

        # Extract arXiv-related events for gap watch stats
        arxiv_events = [e for e in activity if "arxiv" in (e.get("payload") or "").lower()]
        watch_stats = {
            "arxiv_events_today": len(arxiv_events),
            "last_arxiv_event": arxiv_events[0]["ts"] if arxiv_events else None,
            "squad_running": coord.get_status().get("running", False),
        }
        return {
            "activity": activity,
            "status": coord.get_status(),
            "watch_stats": watch_stats,
        }
    except Exception as e:
        return {"activity": [], "error": str(e)}
