"""Gene Pool web routes — credibility, trust, evolution, heatmap, alerts."""

from __future__ import annotations

import json
import time
from pathlib import Path
from typing import Any, Dict, List

from fastapi import APIRouter, Request
from fastapi.responses import HTMLResponse, RedirectResponse

from web.shared import templates, get_db, get_tracker

router = APIRouter()


# ── Helper: Evolution Log HTML ──────────────────────────────────────────


def _render_evolution_log_html(events: list) -> str:
    ACTION_ICON = {
        "created": "🆕", "merged": "🔀", "evolved": "🧬",
        "archived": "📦", "consumed": "⚡",
    }
    ACTION_COLOR = {
        "created": "#4CAF50", "merged": "#9C27B0", "evolved": "#2196F3",
        "archived": "#757575", "consumed": "#FF9800",
    }

    rows = []
    for ev in events:
        icon = ACTION_ICON.get(ev["action"], "📌")
        color = ACTION_COLOR.get(ev["action"], "#666")
        ts = ev.get("timestamp", "")
        date_str = ts.replace("T", " ").split(".")[0] if ts else "—"
        details = ev.get("details", "")
        gap_title = ev.get("gap_title", "") or "—"
        gap_type = ev.get("gap_type", "") or "—"
        cap_id = ev.get("capsule_id", "") or ""
        details_html = f"<div class='ev-details'>{details}</div>" if details else ""
        rows.append(f"""
        <div class='ev-row'>
            <span class='ev-icon'>{icon}</span>
            <div class='ev-body'>
                <div class='ev-header'>
                    <span class='ev-action' style='color:{color}'>{ev["action"].upper()}</span>
                    <span class='ev-time'>{date_str}</span>
                </div>
                <div class='ev-title'>{gap_title}</div>
                <div class='ev-meta'>
                    <span class='ev-type'>{gap_type}</span>
                    <span class='ev-id'>{cap_id}</span>
                </div>
                {details_html}
            </div>
        </div>""")

    if not rows:
        return """
        <div class="evo-empty">
            <div class="evo-empty-icon">🧬</div>
            <div class="evo-empty-msg">No evolution events yet.</div>
            <div class="evo-empty-sub">Accept suggestions or submit verdicts to grow your Gene Pool.</div>
        </div>"""

    rows_html = "\n".join(rows)
    return f"""
    <style>
    .evo-log {{ font-family: 'Courier New', monospace; }}
    .ev-row {{ display: flex; gap: 14px; padding: 10px 0; border-bottom: 1px solid #eee; align-items: flex-start; }}
    .ev-icon {{ font-size: 18px; flex-shrink: 0; width: 28px; text-align: center; padding-top: 2px; }}
    .ev-body {{ flex: 1; min-width: 0; }}
    .ev-header {{ display: flex; align-items: center; gap: 10px; margin-bottom: 3px; }}
    .ev-action {{ font-size: 11px; font-weight: bold; letter-spacing: 0.05em; }}
    .ev-time {{ font-size: 11px; color: #999; margin-left: auto; }}
    .ev-title {{ font-size: 14px; font-weight: 600; color: #333; }}
    .ev-meta {{ display: flex; gap: 10px; margin-top: 2px; font-size: 11px; color: #888; }}
    .ev-type {{ background: #eee; padding: 1px 6px; border-radius: 3px; }}
    .ev-id {{ font-family: monospace; color: #aaa; }}
    .ev-details {{ font-size: 12px; color: #666; margin-top: 4px; padding: 4px 8px; background: #f9f9f9; border-radius: 4px; }}
    .evo-empty {{ text-align: center; padding: 60px 20px; }}
    .evo-empty-icon {{ font-size: 48px; margin-bottom: 12px; }}
    .evo-empty-msg {{ font-size: 18px; color: #666; margin-bottom: 8px; }}
    .evo-empty-sub {{ font-size: 14px; color: #999; }}
    </style>
    <div class="evo-log">{rows_html}</div>"""


# ── Routes ──────────────────────────────────────────────────────────────


@router.get("/trust-scores")
async def trust_scores(request: Request):
    """Source Trust Scores — per-arXiv-category credibility ratings."""
    from llm.insight.trust_tracker import SourceTrustTracker

    tracker = SourceTrustTracker()
    html = tracker.render_html()
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "trust-scores", "title": "Source Trust Scores", "content": html},
    )


@router.get("/gene-pool/credibility")
async def gene_pool_credibility(request: Request):
    """Gap Credibility — flags trendslop capsules with high keyword overlap."""
    from llm.insight.credibility import CredibilityScorer

    scorer = CredibilityScorer()
    html = scorer.render_html()
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-credibility", "title": "Gap Credibility", "content": html},
    )


@router.get("/gene-pool/graph")
async def gene_pool_graph(request: Request):
    """D3.js force-directed graph of all Gene Pool capsules."""
    from web.renderers import render_gene_pool_graph_html

    return HTMLResponse(content=render_gene_pool_graph_html())


@router.get("/gene-pool/evolution-log")
async def gene_pool_evolution_log(request: Request):
    """Evolution Log — shows what the Gene Pool learned over time."""
    from llm.insight.tracker import EvolutionTracker

    tracker = EvolutionTracker()
    events = tracker.get_evolution_log(limit=100)
    html = _render_evolution_log_html(events)
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-evolution-log", "title": "Evolution Log", "content": html},
    )


@router.get("/heatmap")
async def contradiction_heatmap(request: Request):
    """Contradiction Heatmap — papers colored by contradiction count."""
    from llm.contradiction_heatmap import compute_paper_contradictions, render_heatmap_html

    db = get_db()
    rows, _ = db.list_papers(limit=200, offset=0)
    papers = [
        {"id": r.id, "title": r.title,
         "primary_category": getattr(r, "primary_category", "") or "",
         "published": r.published}
        for r in rows
    ]
    contrad_map = compute_paper_contradictions()
    html = render_heatmap_html(papers, contrad_map)
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "heatmap", "title": "Contradiction Heatmap", "content": html},
    )


@router.get("/game-mode")
async def game_mode(request: Request):
    """Research Game Mode — badges and progression."""
    html = '<p>Game mode unavailable</p>'
    try:
        from llm.game_mode import compute_badges, render_game_mode_html
        badges = compute_badges()
        html = render_game_mode_html(badges)
    except Exception:
        pass
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "game-mode", "title": "Research Game Mode", "content": html},
    )


@router.get("/alerts/paradigm")
async def paradigm_alert(request: Request):
    """Paradigm Concentration Alert."""
    from llm.paradigm_monitor import check_paradigm_concentration, render_html

    result = check_paradigm_concentration("all")
    html = render_html(result)
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "paradigm-alert", "title": "Paradigm Alert", "content": html},
    )


@router.get("/alerts/eval-gap")
async def eval_gap_alert(request: Request):
    """Evaluation Gap Monitor."""
    html = "<p>Evaluation gap monitor unavailable</p>"
    try:
        from llm.eval_gap_monitor import check_eval_gaps, render_eval_gap_html
        data = check_eval_gaps()
        html = render_eval_gap_html(data)
    except Exception:
        pass
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "eval-gap-alert", "title": "Evaluation Gap", "content": html},
    )


@router.get("/gene-pool/bold")
async def gene_pool_bold(request: Request):
    """Bold Hypothesis Vault."""
    from llm.bold_vault import get_bold_capsules, render_html

    capsules = get_bold_capsules()
    html = render_html(capsules)
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-bold", "title": "Bold Hypothesis Vault", "content": html},
    )


@router.get("/gene-pool/backup")
async def gene_pool_backup(request: Request):
    """Gene Pool Backup — create and restore snapshots."""
    from llm.gene_pool_backup import get_backup_info, create_backup
    info = get_backup_info()
    stamps = info.get("stamps", []) if isinstance(info, dict) else []
    if stamps:
        rows = ''.join(f'<tr><td>{s}</td><td><form action="/gene-pool/backup/restore/{s.replace(".tar","")}" method="post" style="display:inline"><button class="btn" style="font-size:12px;padding:2px 10px;">Restore</button></form></td></tr>' for s in stamps)
        html = f'<table class="credibility-table"><thead><tr><th>Backup</th><th></th></tr></thead><tbody>{rows}</tbody></table>'
    else:
        html = "<p>No backups yet.</p>"
    html += '<form action="/gene-pool/backup/create" method="post" style="margin-top:16px"><button class="btn btn-primary">Create Backup</button></form>'
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-backup", "title": "Gene Pool Backup", "content": html},
    )


@router.post("/gene-pool/backup/create")
async def create_backup(request: Request):
    from llm.gene_pool_backup import create_backup as _create

    path = _create()
    return templates.TemplateResponse(
        request, "generic.html",
        {
            "page": "gene-pool-backup",
            "title": "Gene Pool Backup",
            "content": f"<p>Backup created: {path}</p>"
        },
    )


@router.post("/gene-pool/backup/restore/{stamp}")
async def restore_backup(stamp: str, request: Request):
    from llm.gene_pool_backup import restore_backup as _restore

    ok = _restore(stamp)
    msg = f"<p>Restored from {stamp}</p>" if ok else f"<p>Restore failed: {stamp} not found</p>"
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-backup", "title": "Gene Pool Backup", "content": msg},
    )


@router.get("/gene-pool/at-risk")
async def gene_pool_at_risk(request: Request):
    """At-Risk Capsules — low-score capsules nearing archive."""
    from llm.insight.tracker import EvolutionTracker as _Tracker
    from llm.insight.evolution import InsightEvolution

    tracker = _Tracker()
    evolver = InsightEvolution(tracker=tracker)
    capsules = evolver._load_capsules()
    at_risk = [c for c in capsules if c.low_score_streak >= 2 and c.status == "active"]
    html = _render_at_risk_html(at_risk)
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-at-risk", "title": "At-Risk Capsules", "content": html},
    )


@router.post("/gene-pool/at-risk/keep-active")
async def at_risk_keep_active(request: Request):
    from llm.insight.tracker import EvolutionTracker as _Tracker
    from llm.insight.evolution import InsightEvolution

    data = await request.form()
    cid = data.get("capsule_id", "")
    tracker = _Tracker()
    evolver = InsightEvolution(tracker=tracker)
    capsules = evolver._load_capsules()
    for c in capsules:
        if c.capsule_id == cid:
            c.low_score_streak = 0
            break
    evolver._save_capsules(capsules)
    return RedirectResponse(url="/gene-pool/at-risk", status_code=303)


@router.post("/gene-pool/at-risk/pin")
async def at_risk_pin(request: Request):
    from llm.insight.tracker import EvolutionTracker as _Tracker
    from llm.insight.evolution import InsightEvolution

    data = await request.form()
    cid = data.get("capsule_id", "")
    tracker = _Tracker()
    evolver = InsightEvolution(tracker=tracker)
    capsules = evolver._load_capsules()
    for c in capsules:
        if c.capsule_id == cid:
            c.status = "consumed"
            break
    evolver._save_capsules(capsules)
    return RedirectResponse(url="/gene-pool/at-risk", status_code=303)


@router.get("/gene-pool/io")
async def gene_pool_io(request: Request):
    """Gene Pool Import/Export."""
    from llm.gene_pool_io import render_io_html

    html = render_io_html()
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "gene-pool-io", "title": "Gene Pool Import/Export", "content": html},
    )


@router.get("/gene-pool/io/export")
async def export_pool(request: Request):
    from llm.gene_pool_io import export_pool
    from fastapi.responses import JSONResponse

    data = export_pool()
    return JSONResponse(content=data, media_type="application/json",
                        headers={"Content-Disposition": "attachment; filename=gene_pool.json"})


@router.post("/gene-pool/io/import")
async def import_pool(request: Request):
    from llm.gene_pool_io import import_pool

    data = await request.json()
    result = import_pool(data, merge=True)
    return templates.TemplateResponse(
        request, "generic.html",
        {
            "page": "gene-pool-io",
            "title": "Gene Pool Import/Export",
            "content": f"<p>Imported {result.get('imported', 0)} capsules.</p>"
        },
    )


@router.get("/gene-pool/cross-domain")
async def cross_domain_bridge(request: Request):
    """Cross-Domain Gap Bridge — find connections between distant categories."""
    from llm.cross_domain_bridge import get_bridges, render_html
    bridges = get_bridges() if 'get_bridges' in dir() else []
    html = render_html(bridges) if 'render_html' in dir() else '<p>Cross-domain bridge not available</p>'
    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "cross-domain", "title": "Cross-Domain Bridges", "content": html},
    )


# ── HTML Renderers ──────────────────────────────────────────────────────


def _render_backup_html(backups: list) -> str:
    if not backups:
        return "<p>No backups yet.</p>"
    rows = []
    for b in backups:
        stamp = b.get("stamp", "")
        ts = b.get("created_at", "")[:19]
        size = b.get("size_bytes", 0)
        size_str = f"{size / 1024:.1f} KB" if size < 1024 * 1024 else f"{size / 1024 / 1024:.1f} MB"
        rows.append(f"""
        <tr>
            <td>{ts}</td>
            <td>{stamp}</td>
            <td>{size_str}</td>
            <td>
                <form action="/gene-pool/backup/restore/{stamp}" method="post" style="display:inline;">
                    <button class="btn" style="font-size:12px;padding:2px 10px;">Restore</button>
                </form>
            </td>
        </tr>""")
    return f"""
    <table class="credibility-table">
        <thead><tr><th>Created</th><th>Stamp</th><th>Size</th><th></th></tr></thead>
        <tbody>{"".join(rows)}</tbody>
    </table>
    <form action="/gene-pool/backup/create" method="post" style="margin-top:16px;">
        <button class="btn btn-primary">Create Backup</button>
    </form>"""


def _render_at_risk_html(capsules: list) -> str:
    if not capsules:
        return "<p>No at-risk capsules. All capsules are healthy.</p>"
    rows = []
    for c in capsules:
        rows.append(f"""
        <tr>
            <td><code>{c.action_gap_title[:40]}</code></td>
            <td><code>{c.action_gap_type}</code></td>
            <td>{c.outcome_success_score:.2f}</td>
            <td>{c.low_score_streak}/3</td>
            <td>
                <form action="/gene-pool/at-risk/keep-active" method="post" style="display:inline;">
                    <input type="hidden" name="capsule_id" value="{c.capsule_id}">
                    <button class="btn" style="font-size:11px;padding:2px 8px;">Keep Active</button>
                </form>
                <form action="/gene-pool/at-risk/pin" method="post" style="display:inline;">
                    <input type="hidden" name="capsule_id" value="{c.capsule_id}">
                    <button class="btn" style="font-size:11px;padding:2px 8px;">Pin</button>
                </form>
            </td>
        </tr>""")
    return f"""
    <table class="credibility-table">
        <thead><tr><th>Capsule</th><th>Type</th><th>Score</th><th>Streak</th><th>Action</th></tr></thead>
        <tbody>{"".join(rows)}</tbody>
    </table>"""
