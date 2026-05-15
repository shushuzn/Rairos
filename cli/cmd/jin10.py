     1|"""CLI command: jin10 — Jin10 Financial Data MCP Client.
     2|
     3|Usage:
     4|    airos jin10 quote XAUUSD           Real-time gold price
     5|    airos jin10 kline XAUUSD 5m 20     K-line data
     6|    airos jin10 flash                   Latest flash news
     7|    airos jin10 search-flash 原油       Search flash news
     8|    airos jin10 news                    Latest news list
     9|    airos jin10 search-news 美联储      Search news
    10|    airos jin10 news-detail <id>        News article detail
    11|    airos jin10 calendar                Economic calendar
    12|    airos jin10 symbols                 Supported symbols
    13|"""
# [LEGACY] Jin10 financial news CLI — unique external API integration

    14|
    15|from __future__ import annotations
    16|
    17|import argparse
    18|
    19|import sys
    20|
    21|from cli._shared import Colors, print_error
    22|
    23|
    24|def _build_jin10_parser(subparsers) -> argparse.ArgumentParser:
    25|    p = subparsers.add_parser(
    26|        "jin10",
    27|        help="Jin10 Financial Data (quotes, news, calendar)",
    28|        description="Real-time financial data via Jin10 MCP service.",
    29|    )
    30|
    31|    sub = p.add_subparsers(dest="jin10_cmd", metavar=" COMMAND")
    32|
    33|    # quote
    34|    q = sub.add_parser("quote", help="Get real-time quote")
    35|    q.add_argument("code", help="Symbol code (e.g. XAUUSD, USOIL)")
    36|    q.set_defaults(func=_run_quote)
    37|
    38|    # kline
    39|    k = sub.add_parser("kline", help="Get K-line data")
    40|    k.add_argument("code", help="Symbol code")
    41|    k.add_argument("--time", "-t", type=int, default=1, help="Minutes: 1/5/15/60/240/1440")
    42|    k.add_argument("--count", "-n", type=int, default=10, help="Number of candles")
    43|    k.set_defaults(func=_run_kline)
    44|
    45|    # flash
    46|    f = sub.add_parser("flash", help="Latest flash news")
    47|    f.add_argument("--cursor", help="Pagination cursor")
    48|    f.set_defaults(func=_run_flash)
    49|
    50|    # search-flash
    51|    sf = sub.add_parser("search-flash", help="Search flash news")
    52|    sf.add_argument("keyword", help="Search keyword")
    53|    sf.set_defaults(func=_run_search_flash)
    54|
    55|    # news
    56|    n = sub.add_parser("news", help="Latest news list")
    57|    n.add_argument("--cursor", help="Pagination cursor")
    58|    n.set_defaults(func=_run_news)
    59|
    60|    # search-news
    61|    sn = sub.add_parser("search-news", help="Search news")
    62|    sn.add_argument("keyword", help="Search keyword")
    63|    sn.add_argument("--cursor", help="Pagination cursor")
    64|    sn.set_defaults(func=_run_search_news)
    65|
    66|    # news-detail
    67|    nd = sub.add_parser("news-detail", help="Get news article detail")
    68|    nd.add_argument("id", help="News article ID")
    69|    nd.set_defaults(func=_run_news_detail)
    70|
    71|    # calendar
    72|    c = sub.add_parser("calendar", help="Economic calendar")
    73|    c.set_defaults(func=_run_calendar)
    74|
    75|    # symbols
    76|    s = sub.add_parser("symbols", help="List supported symbols")
    77|    s.set_defaults(func=_run_symbols)
    78|
    79|    return p  # type: ignore[no-any-return]
    80|
    81|
    82|def _run_quote(args) -> None:
    83|    from llm.mcp_jin10 import quote as _quote  # type: ignore[attr-defined]
    84|
    85|    try:
    86|        raw = _quote(args.code)
    87|    except Exception as e:
    88|        print_error(f"Failed: {e}")
    89|        sys.exit(1)
    90|
    91|    data = raw.get("data", raw)
    92|    print()
    93|    print(f"  {Colors.CYAN}{data.get('name', args.code)} Quote{Colors.END}")
    94|    print(f"  Time:   {data.get('time', '?')}")
    95|    print(f"  Price:  {data.get('close', '?')}")
    96|    print(f"  Open:   {data.get('open', '?')}")
    97|    print(f"  High:   {data.get('high', '?')}")
    98|    print(f"  Low:    {data.get('low', '?')}")
    99|    print(f"  Volume: {data.get('volume', '?')}")
   100|    print(f"  Change: {data.get('ups_price', '?')} ({data.get('ups_percent', '?')}%)")
   101|    print()
   102|
   103|
   104|def _run_kline(args) -> None:
   105|    from llm.mcp_jin10 import kline as _kline  # type: ignore[attr-defined]
   106|
   107|    try:
   108|        data = _kline(args.code, args.time, args.count)  # type: ignore[attr-defined]
   109|    except Exception as e:
   110|        print_error(f"Failed: {e}")
   111|        sys.exit(1)
   112|
   113|    name = data.get("name", args.code)
   114|    klines = data.get("klines", data.get("data", []))
   115|    print(f"\n  {Colors.CYAN}{name} K-line ({args.time}){Colors.END}")
   116|    print(f"  {'Time':<16} {'Open':>8} {'High':>8} {'Low':>8} {'Close':>8} {'Vol':>8}")
   117|    print(f"  {'─' * 16} {'─' * 8} {'─' * 8} {'─' * 8} {'─' * 8} {'─' * 8}")
   118|    for k in klines[: args.count]:
   119|        print(
   120|            f"  {str(k.get('time', ''))[:16]:<16} {k.get('open', ''):>8} {k.get('high', ''):>8} "
   121|            f"{k.get('low', ''):>8} {k.get('close', ''):>8} {k.get('volume', ''):>8}"
   122|        )
   123|    print()
   124|
   125|
   126|def _run_flash(args) -> None:
   127|    from llm.mcp_jin10 import flash as _flash  # type: ignore[attr-defined]
   128|
   129|    data = _flash(getattr(args, "cursor", ""))
   130|    inner = data.get("data", data)
   131|    items = inner.get("items", [])
   132|    if isinstance(items, str):
   133|        items = [items]
   134|    nc = inner.get("next_cursor", "")
   135|    hm = inner.get("has_more", False)
   136|
   137|    print(f"\n  {Colors.CYAN}Flash News{Colors.END} ({len(items)} items)")
   138|    if nc:
   139|        print(f"  Next cursor: {nc}  |  More: {hm}")
   140|    print()
   141|    for item in items:
   142|        if isinstance(item, str):
   143|            print(f"  {item[:80]}")
   144|        else:
   145|            ts = str(item.get("time", ""))[:16]
   146|            content = str(item.get("content", item.get("title", "")))[:80]
   147|            print(f"  [{ts}] {content}")
   148|    print()
   149|
   150|
   151|def _run_search_flash(args) -> None:
   152|    from llm.mcp_jin10 import search_flash as _sf  # type: ignore[attr-defined]
   153|
   154|    raw = _sf(args.keyword)  # type: ignore[attr-defined]
   155|    inner = (
   156|        raw.get("data", raw)
   157|        if isinstance(raw, dict)
   158|        else {"items": raw if isinstance(raw, list) else []}
   159|    )
   160|    items = inner.get("items", [])
   161|    if isinstance(items, str):
   162|        items = [items]
   163|
   164|    print(f"\n  {Colors.CYAN}Flash News: {args.keyword}{Colors.END} ({len(items)} results)")
   165|    print()
   166|    for item in items:
   167|        if isinstance(item, str):
   168|            print(f"  {item[:80]}")
   169|        else:
   170|            ts = str(item.get("time", ""))[:16]
   171|            content = str(item.get("content", item.get("title", "")))[:80]
   172|            print(f"  [{ts}] {content}")
   173|    print()
   174|
   175|
   176|def _run_news(args) -> None:
   177|    from llm.mcp_jin10 import news_list as _nl  # type: ignore[attr-defined]
   178|
   179|    data = _nl(getattr(args, "cursor", ""))  # type: ignore[attr-defined]
   180|    items = data.get("items", data.get("data", []))
   181|    nc = data.get("next_cursor", "")
   182|    hm = data.get("has_more", False)
   183|
   184|    print(f"\n  {Colors.CYAN}News{Colors.END} ({len(items)} items)")
   185|    if nc:
   186|        print(f"  Next cursor: {nc}  |  More: {hm}")
   187|    print()
   188|    for item in items:
   189|        print(f"  [{item.get('id', '?')}] {str(item.get('title', ''))[:70]}")
   190|        print(f"       {str(item.get('time', ''))[:16]} | {str(item.get('introduction', ''))[:60]}")
   191|    print()
   192|
   193|
   194|def _run_search_news(args) -> None:
   195|    from llm.mcp_jin10 import search_news as _sn  # type: ignore[attr-defined]
   196|
   197|    raw = _sn(args.keyword, getattr(args, "cursor", ""))  # type: ignore[attr-defined]
   198|    inner = (
   199|        raw.get("data", raw)
   200|        if isinstance(raw, dict)
   201|        else {"items": raw if isinstance(raw, list) else []}
   202|    )
   203|    items = inner.get("items", [])
   204|    nc = inner.get("next_cursor", "")
   205|    hm = inner.get("has_more", False)
   206|
   207|    print(f"\n  {Colors.CYAN}News: {args.keyword}{Colors.END} ({len(items)} results)")
   208|    if nc:
   209|        print(f"  Next cursor: {nc}  |  More: {hm}")
   210|    print()
   211|    for item in items:
   212|        if isinstance(item, str):
   213|            print(f"  {item[:70]}")
   214|        else:
   215|            print(f"  [{item.get('id', '?')}] {str(item.get('title', ''))[:70]}")
   216|            if item.get("time"):
   217|                print(f"       {str(item.get('time', ''))[:16]}")
   218|    print()
   219|
   220|
   221|def _run_news_detail(args) -> None:
   222|    from llm.mcp_jin10 import news_detail as _nd  # type: ignore[attr-defined]
   223|
   224|    data = _nd(args.id)  # type: ignore[attr-defined]
   225|    print(f"\n  {Colors.CYAN}{data.get('title', 'News Detail')}{Colors.END}")
   226|    print(f"  ID: {data.get('id', '?')}  |  Time: {str(data.get('time', ''))[:16]}")
   227|    print(f"  URL: {data.get('url', '')}")
   228|    print(f"\n  {data.get('introduction', '')}")
   229|    print(f"\n  {data.get('content', '')}")
   230|
   231|
   232|def _run_calendar(args) -> None:  # type: ignore[attr-defined]
   233|    from llm.mcp_jin10 import calendar as _cal  # type: ignore[attr-defined]
   234|
   235|    raw = _cal()
   236|    if isinstance(raw, dict):
   237|        data = raw.get("data", raw.get("items", []))
   238|    elif isinstance(raw, list):
   239|        data = raw
   240|    else:
   241|        data = []
   242|
   243|    print(f"\n  {Colors.CYAN}Economic Calendar{Colors.END} ({len(data or [])} items)")  # type: ignore[arg-type]
   244|    print()
   245|    for item in data or []:  # type: ignore[union-attr]
   246|        print(
   247|            f"  [{str(item.get('pub_time', ''))[:16]}] "
   248|            f"{'⭐' * int(item.get('star', 0))} {item.get('title', '')}"
   249|        )
   250|        print(
   251|            f"       Previous: {item.get('previous', '-')}  |  "
   252|            f"Consensus: {item.get('consensus', '-')}  |  "
   253|            f"Actual: {item.get('actual', '-')}"
   254|        )
   255|        if item.get("affect_txt"):
   256|            print(f"       Impact: {item['affect_txt']}")
   257|    print()
   258|
   259|
   260|def _run_symbols(args) -> None:
   261|    from llm.mcp_jin10 import symbols as _sym  # type: ignore[attr-defined]
   262|
   263|    data = _sym()
   264|    print(f"\n  {Colors.CYAN}Supported Symbols{Colors.END}")
   265|    print()
   266|    for s in data:
   267|        print(f"  {s.get('code', '?'):<10} {s.get('name', '')}")
   268|    print()
   269|
