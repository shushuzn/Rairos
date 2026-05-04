"""Web routes: intelligence dashboard + daemon controls."""
from __future__ import annotations
import re
from fastapi import APIRouter, Request
from fastapi.responses import RedirectResponse
from web.shared import templates
from llm.intelligence import intelligence, render_report

router = APIRouter()

@router.get("/report")
async def live_report(request: Request):
    """Live situation report."""
    import re
    from llm.report import generate
    try:
        raw = generate()
        clean = re.sub(r'\x1b\[[0-9;]*m', '', raw)
        html = "<pre style='font-family:monospace;font-size:13px;line-height:1.5;color:#333;white-space:pre-wrap'>" + clean + "</pre>"
    except Exception as e:
        html = f"<p>{e}</p>"
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "report", "title": "Report", "content": html},
    )

@router.get("/brief")
async def daily_brief(request: Request):
    """Daily Brief with real analysis."""
    from llm.daily_brief import generate
    try:
        raw = generate()
        html = "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>" + raw + "</pre>"
    except Exception as e:
        html = f"<p>{e}</p>"
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "brief", "title": "Daily Brief", "content": html},
    )

@router.get("/intel")
async def intel_dashboard(request: Request):
    """Unified intelligence dashboard."""
    try:
        report = intelligence()
        raw = render_report(report)
        clean = re.sub(r'\x1b\[[0-9;]*m', '', raw)
        html = "<pre style='font-family:monospace;font-size:13px;line-height:1.5;color:#333;white-space:pre-wrap'>" + clean + "</pre>"
    except Exception as e:
        html = f"<p>Error: {e}</p>"
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "intel", "title": "Intelligence", "content": html},
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
    status_html = "RUNNING" if running else "STOPPED"
    btn = "stop" if running else "start"
    btn_label = "Stop" if running else "Start"

    gp_lines = []
    for c in sorted(geo, key=lambda x: x.outcome_success_score, reverse=True)[:5]:
        gp_lines.append(f"{c.credibility_badge.upper()} score={c.outcome_success_score:.2f} {c.action_gap_title[:60]}")

    html = f"""
    <h3>Daemon Status: {status_html}</h3>
    <p>Interval: {status.get('interval', 300)}s | Last: {str(status.get('last_check', ''))[:19] or 'never'}</p>
    <p>Events: {status.get('total_events', 0)} | Gene Pool: {status.get('gene_pool_size', 0)}</p>
    <form action="/daemon/{btn}" method="post" style="display:inline;">
        <button class="btn btn-primary">{btn_label} Daemon</button>
    </form>
    <form action="/daemon/cycle" method="post" style="display:inline;">
        <button class="btn">Run Cycle</button>
    </form>
    <h4>Top Geopolitical Capsules</h4>
    <pre>{chr(10).join(gp_lines)}</pre>
    <h4>Links</h4>
    <p><a href="/intel">Intelligence Report</a> | <a href="/gene-pool/credibility">Credibility</a></p>
    """
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "daemon", "title": "Daemon", "content": html},
    )

@router.post("/daemon/start")
async def daemon_start(request: Request):
    from llm.watch import WatchDaemon
    WatchDaemon(interval=300).start()
    return RedirectResponse(url="/daemon", status_code=303)

@router.post("/daemon/stop")
async def daemon_stop(request: Request):
    from llm.watch import WatchDaemon
    WatchDaemon().stop()
    return RedirectResponse(url="/daemon", status_code=303)

@router.post("/daemon/cycle")
async def daemon_cycle(request: Request):
    from llm.report import save
    from llm.discover import discover
    from llm.watch import WatchDaemon
    save()
    discover()
    WatchDaemon(interval=300).start()
    return RedirectResponse(url="/daemon", status_code=303)
