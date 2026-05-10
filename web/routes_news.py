"""Web routes: news reader — browse and read Jin10 articles."""

from __future__ import annotations
from fastapi import APIRouter, Request
from web.shared import templates
from typing import Any, Dict, List

router = APIRouter()


@router.get("/news")
async def news_list(request: Request, keyword: str = ""):
    """Latest financial/political news from Jin10."""
    try:
        from llm.mcp_jin10 import Jin10Client

        client = Jin10Client()
        client.ensure_init()

        if keyword:
            raw = client.search_flash(keyword)
        else:
            raw = client.list_flash()

        inner: Dict[str, Any] = raw.get("data", raw) if isinstance(raw, dict) else {}
        raw_items: list[Any] = inner.get("items", []) if isinstance(inner, dict) else []
        if not isinstance(raw_items, list):
            raw_items = []
        items = raw_items

        cards = []
        for item in items[:30]:
            if not isinstance(item, dict):
                continue
            content = str(item.get("content", item.get("title", "")))
            ts = str(item.get("time", ""))[:19]
            cards.append(
                '<div style="border:1px solid var(--border);border-radius:8px;padding:14px;margin-bottom:10px;">'
                + '<div style="font-size:11px;color:var(--ink-faint);margin-bottom:6px;">'
                + ts
                + "</div>"
                + '<div style="font-size:14px;line-height:1.6;color:var(--ink);">'
                + content
                + "</div>"
                + "</div>"
            )
    except Exception as e:
        cards = ["<p>News unavailable: " + str(e) + "</p>"]

    cards_joined = "".join(cards)

    html = (
        "<style>"
        ".news-container { max-width: 800px; margin: 0 auto; }"
        ".news-header { font-size: 18px; font-weight: 700; margin-bottom: 16px; }"
        ".news-bar { display: flex; gap: 8px; margin-bottom: 16px; align-items: center; }"
        ".news-bar form { flex: 1; display: flex; gap: 8px; }"
        ".news-bar input { flex: 1; padding: 8px 12px; border: 1px solid var(--border); border-radius: 4px; font-size: 13px; }"
        "</style>"
        '<div class="news-container">'
        '<div class="news-header">Live News</div>'
        '<div class="news-bar">'
        '<form method="get" action="/news">'
        '<input type="text" name="keyword" value="' + keyword + '" placeholder="Search...">'
        '<button class="btn btn-primary" type="submit">Search</button>'
        "</form>"
        '<button class="btn" onclick="refreshNews()">Refresh</button>'
        "</div>"
        '<div id="news-items">' + cards_joined + "</div>"
        '<div style="font-size:11px;color:var(--ink-faint);text-align:center;padding:8px;">Auto-refreshes every 60s</div>'
        "</div>"
        "<script>"
        "function refreshNews() {"
        "var kw = document.querySelector('input[name=keyword]').value;"
        "var url = '/news' + (kw ? '?keyword=' + encodeURIComponent(kw) : '');"
        "fetch(url).then(function(r){return r.text()}).then(function(html){"
        "var d = document.createElement('div');"
        "d.innerHTML = html;"
        "var items = d.querySelector('#news-items');"
        "if (items) document.getElementById('news-items').innerHTML = items.innerHTML;"
        "});"
        "}"
        "setInterval(refreshNews, 60000);"
        "</script>"
    )

    return templates.TemplateResponse(
        request,
        "generic.html",
        {"page": "news", "title": "News", "content": html},
    )
