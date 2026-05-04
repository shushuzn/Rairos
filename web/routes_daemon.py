"""Web routes: intelligence dashboard + signal analysis + watch status."""

from __future__ import annotations

from fastapi import APIRouter, Request

from web.shared import templates

router = APIRouter()


@router.get("/intel")
async def intel_dashboard(request: Request):
    """Unified intelligence dashboard."""
    from llm.intelligence import intelligence, render_report

    try:
        report = intelligence()
        html = render_report(report)
    except Exception as e:
        html = f"<p>Error: {e}</p>"

    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "intel", "title": "Intelligence", "content": f"<pre>{html}</pre>"},
    )


@router.get("/daemon")
async def daemon_dashboard(request: Request):
    """Daemon status + watch control."""
    from llm.watch import WatchDaemon
    from llm.insight.tracker import EvolutionTracker

    watch = WatchDaemon()
    status = watch.get_status()
    tracker = EvolutionTracker()
    caps = tracker._load_capsules()
    geo = [c for c in caps if getattr(c, "source_arxiv_category", "") == "cs.GL"]

    running = status.get("running", False)
    status_html = "🟢 RUNNING" if running else "🔴 STOPPED"
    btn = "stop" if running else "start"
    btn_label = "Stop" if running else "Start"

    gp_html = "<br>".join(
        f"[{c.credibility_badge.upper()}] score={c.outcome_success_score:.2f} {c.action_gap_title[:60]}"
        for c in sorted(geo, key=lambda x: x.outcome_success_score, reverse=True)[:5]
    )

    html = f"""
    <h3>Daemon Status: {status_html}</h3>
    <p>Interval: {status.get('interval', 300)}s | Last: {str(status.get('last_check', ''))[:19] or 'never'}</p>
    <p>Events monitored: {status.get('total_events', 0)} | Gene Pool: {status.get('gene_pool_size', 0)}</p>
    <h4>Controls</h4>
    <form action="/daemon/{btn}" method="post" style="display:inline;">
        <button class="btn btn-primary">{btn_label} Daemon</button>
    </form>
    <form action="/daemon/cycle" method="post" style="display:inline;">
        <button class="btn">Run Cycle</button>
    </form>
    <h4>Top Geopolitical Capsules</h4>
    <pre>{gp_html}</pre>
    <h4>Recent Alert Links</h4>
    <p><a href="/intel">Intelligence Report</a> | <a href="/gene-pool/credibility">Credibility</a></p>
    """

    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "daemon", "title": "Daemon", "content": html},
    )


@router.post("/daemon/start")
async def daemon_start(request: Request):
    from llm.watch import WatchDaemon
    from fastapi.responses import RedirectResponse

    WatchDaemon(interval=300).start()
    return RedirectResponse(url="/daemon", status_code=303)


@router.post("/daemon/stop")
async def daemon_stop(request: Request):
    from llm.watch import WatchDaemon
    from fastapi.responses import RedirectResponse

    WatchDaemon().stop()
    return RedirectResponse(url="/daemon", status_code=303)


@router.post("/daemon/cycle")
async def daemon_cycle(request: Request):
    """Run one intelligence cycle."""
    from llm.report import save
    from llm.discover import discover
    from fastapi.responses import RedirectResponse

    save()
    discover()
    return RedirectResponse(url="/daemon", status_code=303)
