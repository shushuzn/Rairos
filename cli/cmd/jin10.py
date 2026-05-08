"""CLI command: jin10 — Jin10 Financial Data MCP Client.

Usage:
    airos jin10 quote XAUUSD           Real-time gold price
    airos jin10 kline XAUUSD 5m 20     K-line data
    airos jin10 flash                   Latest flash news
    airos jin10 search-flash 原油       Search flash news
    airos jin10 news                    Latest news list
    airos jin10 search-news 美联储      Search news
    airos jin10 news-detail <id>        News article detail
    airos jin10 calendar                Economic calendar
    airos jin10 symbols                 Supported symbols
"""

from __future__ import annotations

import argparse
import json
import sys

from cli._shared import Colors, print_error, print_success


def _build_jin10_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser(
        "jin10",
        help="Jin10 Financial Data (quotes, news, calendar)",
        description="Real-time financial data via Jin10 MCP service.",
    )

    sub = p.add_subparsers(dest="jin10_cmd", metavar=" COMMAND")

    # quote
    q = sub.add_parser("quote", help="Get real-time quote")
    q.add_argument("code", help="Symbol code (e.g. XAUUSD, USOIL)")
    q.set_defaults(func=_run_quote)

    # kline
    k = sub.add_parser("kline", help="Get K-line data")
    k.add_argument("code", help="Symbol code")
    k.add_argument("--time", "-t", type=int, default=1, help="Minutes: 1/5/15/60/240/1440")
    k.add_argument("--count", "-n", type=int, default=10, help="Number of candles")
    k.set_defaults(func=_run_kline)

    # flash
    f = sub.add_parser("flash", help="Latest flash news")
    f.add_argument("--cursor", help="Pagination cursor")
    f.set_defaults(func=_run_flash)

    # search-flash
    sf = sub.add_parser("search-flash", help="Search flash news")
    sf.add_argument("keyword", help="Search keyword")
    sf.set_defaults(func=_run_search_flash)

    # news
    n = sub.add_parser("news", help="Latest news list")
    n.add_argument("--cursor", help="Pagination cursor")
    n.set_defaults(func=_run_news)

    # search-news
    sn = sub.add_parser("search-news", help="Search news")
    sn.add_argument("keyword", help="Search keyword")
    sn.add_argument("--cursor", help="Pagination cursor")
    sn.set_defaults(func=_run_search_news)

    # news-detail
    nd = sub.add_parser("news-detail", help="Get news article detail")
    nd.add_argument("id", help="News article ID")
    nd.set_defaults(func=_run_news_detail)

    # calendar
    c = sub.add_parser("calendar", help="Economic calendar")
    c.set_defaults(func=_run_calendar)

    # symbols
    s = sub.add_parser("symbols", help="List supported symbols")
    s.set_defaults(func=_run_symbols)

    return p  # type: ignore[no-any-return]


def _run_quote(args) -> None:
    from llm.mcp_jin10 import quote as _quote  # type: ignore[attr-defined]

    try:
        raw = _quote(args.code)
    except Exception as e:
        print_error(f"Failed: {e}")
        sys.exit(1)

    data = raw.get("data", raw)
    print()
    print(f"  {Colors.CYAN}{data.get('name', args.code)} Quote{Colors.END}")
    print(f"  Time:   {data.get('time', '?')}")
    print(f"  Price:  {data.get('close', '?')}")
    print(f"  Open:   {data.get('open', '?')}")
    print(f"  High:   {data.get('high', '?')}")
    print(f"  Low:    {data.get('low', '?')}")
    print(f"  Volume: {data.get('volume', '?')}")
    print(f"  Change: {data.get('ups_price', '?')} ({data.get('ups_percent', '?')}%)")
    print()


def _run_kline(args) -> None:
    from llm.mcp_jin10 import kline as _kline  # type: ignore[attr-defined]

    try:
        data = _kline(args.code, args.time, args.count)  # type: ignore[attr-defined]
    except Exception as e:
        print_error(f"Failed: {e}")
        sys.exit(1)

    name = data.get("name", args.code)
    klines = data.get("klines", data.get("data", []))
    print(f"\n  {Colors.CYAN}{name} K-line ({args.time}){Colors.END}")
    print(f"  {'Time':<16} {'Open':>8} {'High':>8} {'Low':>8} {'Close':>8} {'Vol':>8}")
    print(f"  {'─' * 16} {'─' * 8} {'─' * 8} {'─' * 8} {'─' * 8} {'─' * 8}")
    for k in klines[: args.count]:
        print(
            f"  {str(k.get('time', ''))[:16]:<16} {k.get('open', ''):>8} {k.get('high', ''):>8} "
            f"{k.get('low', ''):>8} {k.get('close', ''):>8} {k.get('volume', ''):>8}"
        )
    print()


def _run_flash(args) -> None:
    from llm.mcp_jin10 import flash as _flash  # type: ignore[attr-defined]

    data = _flash(getattr(args, "cursor", ""))
    inner = data.get("data", data)
    items = inner.get("items", [])
    if isinstance(items, str):
        items = [items]
    nc = inner.get("next_cursor", "")
    hm = inner.get("has_more", False)

    print(f"\n  {Colors.CYAN}Flash News{Colors.END} ({len(items)} items)")
    if nc:
        print(f"  Next cursor: {nc}  |  More: {hm}")
    print()
    for item in items:
        if isinstance(item, str):
            print(f"  {item[:80]}")
        else:
            ts = str(item.get("time", ""))[:16]
            content = str(item.get("content", item.get("title", "")))[:80]
            print(f"  [{ts}] {content}")
    print()


def _run_search_flash(args) -> None:
    from llm.mcp_jin10 import search_flash as _sf  # type: ignore[attr-defined]

    raw = _sf(args.keyword)  # type: ignore[attr-defined]
    inner = (
        raw.get("data", raw)
        if isinstance(raw, dict)
        else {"items": raw if isinstance(raw, list) else []}
    )
    items = inner.get("items", [])
    if isinstance(items, str):
        items = [items]

    print(f"\n  {Colors.CYAN}Flash News: {args.keyword}{Colors.END} ({len(items)} results)")
    print()
    for item in items:
        if isinstance(item, str):
            print(f"  {item[:80]}")
        else:
            ts = str(item.get("time", ""))[:16]
            content = str(item.get("content", item.get("title", "")))[:80]
            print(f"  [{ts}] {content}")
    print()


def _run_news(args) -> None:
    from llm.mcp_jin10 import news_list as _nl  # type: ignore[attr-defined]

    data = _nl(getattr(args, "cursor", ""))  # type: ignore[attr-defined]
    items = data.get("items", data.get("data", []))
    nc = data.get("next_cursor", "")
    hm = data.get("has_more", False)

    print(f"\n  {Colors.CYAN}News{Colors.END} ({len(items)} items)")
    if nc:
        print(f"  Next cursor: {nc}  |  More: {hm}")
    print()
    for item in items:
        print(f"  [{item.get('id', '?')}] {str(item.get('title', ''))[:70]}")
        print(f"       {str(item.get('time', ''))[:16]} | {str(item.get('introduction', ''))[:60]}")
    print()


def _run_search_news(args) -> None:
    from llm.mcp_jin10 import search_news as _sn  # type: ignore[attr-defined]

    raw = _sn(args.keyword, getattr(args, "cursor", ""))  # type: ignore[attr-defined]
    inner = (
        raw.get("data", raw)
        if isinstance(raw, dict)
        else {"items": raw if isinstance(raw, list) else []}
    )
    items = inner.get("items", [])
    nc = inner.get("next_cursor", "")
    hm = inner.get("has_more", False)

    print(f"\n  {Colors.CYAN}News: {args.keyword}{Colors.END} ({len(items)} results)")
    if nc:
        print(f"  Next cursor: {nc}  |  More: {hm}")
    print()
    for item in items:
        if isinstance(item, str):
            print(f"  {item[:70]}")
        else:
            print(f"  [{item.get('id', '?')}] {str(item.get('title', ''))[:70]}")
            if item.get("time"):
                print(f"       {str(item.get('time', ''))[:16]}")
    print()


def _run_news_detail(args) -> None:
    from llm.mcp_jin10 import news_detail as _nd  # type: ignore[attr-defined]

    data = _nd(args.id)  # type: ignore[attr-defined]
    print(f"\n  {Colors.CYAN}{data.get('title', 'News Detail')}{Colors.END}")
    print(f"  ID: {data.get('id', '?')}  |  Time: {str(data.get('time', ''))[:16]}")
    print(f"  URL: {data.get('url', '')}")
    print(f"\n  {data.get('introduction', '')}")
    print(f"\n  {data.get('content', '')}")


def _run_calendar(args) -> None:  # type: ignore[attr-defined]
    from llm.mcp_jin10 import calendar as _cal  # type: ignore[attr-defined]

    raw = _cal()
    if isinstance(raw, dict):
        data = raw.get("data", raw.get("items", []))
    elif isinstance(raw, list):
        data = raw
    else:
        data = []

    print(f"\n  {Colors.CYAN}Economic Calendar{Colors.END} ({len(data or [])} items)")  # type: ignore[arg-type]
    print()
    for item in data or []:  # type: ignore[union-attr]
        print(
            f"  [{str(item.get('pub_time', ''))[:16]}] "
            f"{'⭐' * int(item.get('star', 0))} {item.get('title', '')}"
        )
        print(
            f"       Previous: {item.get('previous', '-')}  |  "
            f"Consensus: {item.get('consensus', '-')}  |  "
            f"Actual: {item.get('actual', '-')}"
        )
        if item.get("affect_txt"):
            print(f"       Impact: {item['affect_txt']}")
    print()


def _run_symbols(args) -> None:
    from llm.mcp_jin10 import symbols as _sym  # type: ignore[attr-defined]

    data = _sym()
    print(f"\n  {Colors.CYAN}Supported Symbols{Colors.END}")
    print()
    for s in data:
        print(f"  {s.get('code', '?'):<10} {s.get('name', '')}")
    print()
