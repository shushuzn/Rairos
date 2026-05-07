"""Web routes: theme-based research reports."""

from __future__ import annotations

from fastapi import APIRouter, Request
from web.shared import templates

router = APIRouter()


@router.get("/report")
async def report_index(request: Request):
    """Reports index — list all themes."""
    return templates.TemplateResponse(
        request,
        "reports.html",
        {"page": "reports", "title": "Reports"},
    )


@router.get("/report/vla")
async def report_vla(request: Request):
    from llm.reports import report_vla as _fn

    content = _fn()
    html = (
        "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>"
        + content
        + "</pre>"
    )
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "reports", "title": "VLA / Robotics", "content": html},
    )


@router.get("/report/geopolitics")
async def report_geopolitics(request: Request):
    from llm.reports import report_geopolitics as _fn

    content = _fn()
    html = (
        "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>"
        + content
        + "</pre>"
    )
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "reports", "title": "Geopolitics / Energy", "content": html},
    )


@router.get("/report/economy")
async def report_economy(request: Request):
    from llm.reports import report_economy as _fn

    content = _fn()
    html = (
        "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>"
        + content
        + "</pre>"
    )
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "reports", "title": "Economy / Markets", "content": html},
    )


@router.get("/report/theory")
async def report_theory(request: Request):
    from llm.reports import report_theory as _fn

    content = _fn()
    html = (
        "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>"
        + content
        + "</pre>"
    )
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "reports", "title": "Theory / Representation", "content": html},
    )


@router.get("/report/safety")
async def report_safety(request: Request):
    from llm.reports import report_safety as _fn

    content = _fn()
    html = (
        "<pre style='font-family:serif;font-size:14px;line-height:1.8;color:#333;white-space:pre-wrap;max-width:800px;margin:0 auto;'>"
        + content
        + "</pre>"
    )
    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "reports", "title": "Safety / Incidents", "content": html},
    )
