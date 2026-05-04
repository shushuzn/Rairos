"""Web routes: news reader — browse and read Jin10 articles."""

from __future__ import annotations

from fastapi import APIRouter, Request
from web.shared import templates

router = APIRouter()


@router.get("/news")
async def news_list(request: Request, keyword: str = ""):
    """Latest financial/political news from Jin10."""
    try:
        from llm.mcp_jin10 import Jin10Client
        client = Jin10Client()
        client.ensure_init()

        if keyword:
            raw = client.search_news(keyword)
        else:
            raw = client.list_flash()

        inner = raw.get("data", raw) if isinstance(raw, dict) else {}
        items = inner.get("items", []) if isinstance(inner, dict) else inner
        if not isinstance(items, list):
            items = []

        next_cursor = inner.get("next_cursor", "") if isinstance(inner, dict) else ""
        has_more = inner.get("has_more", False) if isinstance(inner, dict) else False

        cards = []
        for item in items[:30]:
            if not isinstance(item, dict):
                continue
            content = str(item.get("content", item.get("title", "")))
            ts = str(item.get("time", ""))[:19]
            item_id = str(item.get("url", item.get("id", "")))
            cards.append(
                f'<div style="border:1px solid var(--border);border-radius:8px;padding:14px;margin-bottom:10px;">'
                f'<div style="font-size:11px;color:var(--ink-faint);margin-bottom:6px;">{ts}</div>'
                f'<div style="font-size:14px;line-height:1.6;color:var(--ink);">{content}</div>'
                f'</div>'
            )
    except Exception as e:
        cards = [f"<p>News unavailable: {e}</p>"]
        next_cursor = ""
        has_more = False

    html = f"""
    <style>
    .news-container {{ max-width: 800px; margin: 0 auto; }}
    .news-header {{ font-size: 18px; font-weight: 700; margin-bottom: 16px; }}
    .news-bar {{ margin-bottom: 16px; }}
    .news-bar form {{ display: flex; gap: 8px; }}
    .news-bar input {{ flex: 1; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; font-size: 13px; }}
    </style>
    <div class="news-container">
        <div class="news-header">Financial & Political News</div>
        <div class="news-bar">
            <form method="get" action="/news">
                <input type="text" name="keyword" placeholder="Search news..." value="{keyword}">
                <button class="btn btn-primary" type="submit">Search</button>
            </form>
        </div>
        {''.join(cards)}
    </div>"""

    return templates.TemplateResponse(
        request, "generic.html",
        {"page": "news", "title": "News", "content": html},
    )
