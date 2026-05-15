     1|"""CLI command: insight — Manage key insight cards."""
     2|
     3|from __future__ import annotations
     4|
     5|
     6|import argparse
     7|
     8|from typing import List, cast
     9|
    10|
    11|from cli._shared import get_db, print_error, print_success
    12|
    13|from llm.insight_cards import InsightCard, InsightManager
    14|
    15|
    16|def _build_insight_parser(subparsers) -> argparse.ArgumentParser:
    17|    """Build the insight subcommand parser."""
# [LEGACY] Insight extractor — depends on llm/insight/

    18|
    19|    p = subparsers.add_parser(
    20|        "insight",
    21|        help="Manage key insight cards",
    22|        description="""Extract and manage key insights from papers.
    23|
    24|Examples:
    25|  airos insight add --paper p1 --content "BERT improves QA by 15%" --tags nlp,rag
    26|  airos insight search --query attention
    27|  airos insight rate --card i0001 --stars 4
    28|  airos insight like --card i0001
    29|  airos insight top
    30|  airos insight tag-cloud
    31|  airos insight quality-report --json
    32|  airos insight quality-report --watch --interval 60
    33|  airos insight alert --json
    34|  airos insight evolve --query "RAG"
    35|  airos insight recompute-credibility
    36|  airos insight eval-retrieval
    37|
    38|Valid --type values: finding, method, limitation, future_work
    39|Valid gap_type values: capability, method_limitation, exploration_gap,
    40|  unexplained_phenomenon, improvement, method_gap, embodied_planning,
    41|  cross_domain, theoretical, empirical""",
    42|    )
    43|
    44|    p.add_argument(
    45|        "action",
    46|        choices=[
    47|            "add",
    48|            "list",
    49|            "search",
    50|            "tag-cloud",
    51|            "export",
    52|            "rate",
    53|            "like",
    54|            "dislike",
    55|            "top",
    56|            "bottom",
    57|            "evolve",
    58|            "quality-report",
    59|            "recompute-credibility",
    60|            "archive-trendslop",
    61|            "eval-retrieval",
    62|            "alert",
    63|            "kg-bridge",
    64|            "promote",
    65|        ],
    66|        help="Action to perform",
    67|    )
    68|    p.add_argument("--paper", help="Paper ID")
    69|
    70|    p.add_argument("--content", help="Insight content")
    71|
    72|    p.add_argument(
    73|        "--type",
    74|        "-t",
    75|        choices=["finding", "method", "limitation", "future_work"],
    76|        default="finding",
    77|        help="Insight type",
    78|    )
    79|
    80|    p.add_argument("--tags", help="Comma-separated tags")
    81|
    82|    p.add_argument("--evidence", help="Evidence/paper reference")
    83|
    84|    p.add_argument("--query", "-q", help="Search query")
    85|
    86|    p.add_argument("--markdown", "-m", action="store_true", help="Output as Markdown")
    87|
    88|    p.add_argument("--collection", "-c", help="Collection ID to add to")
    89|
    90|    p.add_argument("--cite", help="Card ID to reference")
    91|
    92|    p.add_argument("--card", help="Card ID to rate/like/dislike")
    93|
    94|    p.add_argument("--stars", type=int, choices=[1, 2, 3, 4, 5], help="Star rating 1-5")
    95|
    96|    p.add_argument("--top-k", type=int, default=10, help="Number of top/bottom cards to show")
    97|
    98|    p.add_argument("--json", action="store_true", help="Output as JSON (for machine parsing)")
    99|
   100|    p.add_argument("--watch", action="store_true", help="Watch mode: continuously output report")
   101|
   102|    p.add_argument(
   103|        "--interval", type=int, default=30, help="Watch interval in seconds (default: 30)"
   104|    )
   105|
   106|    return p  # type: ignore[no-any-return]
   107|
   108|
   109|def _run_insight(args: argparse.Namespace) -> int:
   110|    """Run insight command."""
   111|
   112|    manager = InsightManager()
   113|
   114|    if args.action == "add":
   115|        if not args.paper or not args.content:
   116|            print_error("Usage: insight add --paper <pid> --content <text>")
   117|
   118|            return 1  # type: ignore[no-any-return]
   119|
   120|        tags = [t.strip() for t in args.tags.split(",")] if args.tags else []
   121|
   122|        card = manager.add_card(
   123|            paper_id=args.paper,
   124|            paper_title=args.paper,  # Will be updated if paper found
   125|            content=args.content,
   126|            insight_type=args.type,
   127|            tags=tags,
   128|            evidence=args.evidence or "",
   129|        )
   130|
   131|        # Try to get paper title
   132|
   133|        if args.paper:
   134|            db = get_db()
   135|
   136|            db.init()
   137|
   138|            paper = db.get_paper(args.paper) if hasattr(db, "get_paper") else None
   139|
   140|            if paper:
   141|                manager.update_card(card.card_id, tags=tags)  # Just update for now
   142|
   143|        print_success(f"Created insight card: {card.card_id}")
   144|
   145|        return 0  # type: ignore[no-any-return]
   146|
   147|    elif args.action == "list":
   148|        cards = manager.search_cards(
   149|            query=args.query,
   150|            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
   151|            insight_type=args.type if hasattr(args, "type") else None,
   152|        )
   153|
   154|        if args.markdown:
   155|            print(manager.render_markdown(cards))
   156|
   157|        else:
   158|            print(manager.render_text(cards))
   159|
   160|        return 0
   161|
   162|    elif args.action == "search":
   163|        cards = manager.search_cards(
   164|            query=args.query,
   165|            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
   166|        )
   167|
   168|        if args.markdown:
   169|            print(manager.render_markdown(cards))
   170|
   171|        else:
   172|            print(manager.render_text(cards))
   173|
   174|        return 0
   175|
   176|    elif args.action == "tag-cloud":
   177|        tag_cloud = manager.get_tag_cloud()
   178|
   179|        if not tag_cloud:
   180|            print("No tags found.")
   181|
   182|            return 0
   183|
   184|        print("📊 Tag Cloud\n")
   185|
   186|        max_count = max(tag_cloud.values()) if tag_cloud else 1
   187|
   188|        for tag, count in sorted(tag_cloud.items(), key=lambda x: -x[1])[:20]:
   189|            bar = "█" * int(count / max_count * 20)
   190|
   191|            print(f"  {tag:20} {count:3} {bar}")
   192|
   193|        return 0
   194|
   195|    elif args.action == "export":
   196|        cards = manager.search_cards(
   197|            query=args.query,
   198|            tags=[t.strip() for t in args.tags.split(",")] if args.tags else None,
   199|        )
   200|
   201|        if args.collection:
   202|            # Get cards from collection
   203|
   204|            collections = manager._load_collections()
   205|
   206|            for c in collections:
   207|                if c.get("collection_id") == args.collection:
   208|                    card_ids = c.get("card_ids", [])
   209|
   210|                    raw_cards = [manager.get_card(cid) for cid in card_ids]
   211|
   212|                    cards = cast(List[InsightCard], [c for c in raw_cards if c])
   213|
   214|                    break
   215|
   216|        output = manager.export_for_note(cards)
   217|
   218|        print(output)
   219|
   220|        return 0
   221|
   222|    elif args.action == "rate":
   223|        if not args.card or args.stars is None:
   224|            print_error("Usage: insight rate --card <id> --stars <1-5>")
   225|
   226|            return 1
   227|
   228|        ok = manager.rate_card(args.card, args.stars)
   229|
   230|        if ok:
   231|            card = manager.get_card(args.card)  # type: ignore[assignment]
   232|
   233|            stars = "★" * args.stars + "☆" * (5 - args.stars)
   234|
   235|            print_success(f"Rated {args.card}: {stars} ({args.stars}/5)")
   236|
   237|            # Bridge to EvolutionTracker
   238|
   239|            try:
   240|                from llm.insight import get_evolution_tracker
   241|
   242|                evo = get_evolution_tracker()
   243|
   244|                topic = card.paper_title if card else ""
   245|
   246|                evo.record_insight_feedback(
   247|                    topic=topic, insight_card_id=args.card, rating=args.stars, paper_title=topic
   248|                )
   249|
   250|            except Exception:
   251|                pass  # EvolutionTracker is optional
   252|
   253|        else:
   254|            print_error(f"Card not found: {args.card}")
   255|            return 1
   256|
   257|        return 0
   258|
   259|    elif args.action == "like":
   260|        if not args.card:
   261|            print_error("Usage: insight like --card <id>")
   262|
   263|            return 1
   264|
   265|        ok = manager.like_card(args.card)
   266|
   267|        if ok:
   268|            print_success(f"Liked {args.card} (★★★★★)")
   269|
   270|            try:
   271|                from llm.insight import get_evolution_tracker
   272|
   273|                evo = get_evolution_tracker()
   274|
   275|                card = manager.get_card(args.card)  # type: ignore[assignment]
   276|
   277|                topic = card.paper_title if card else ""
   278|
   279|                evo.record_insight_feedback(
   280|                    topic=topic, insight_card_id=args.card, rating=5, paper_title=topic
   281|                )
   282|
   283|            except Exception:
   284|                pass
   285|
   286|        else:
   287|            print_error(f"Card not found: {args.card}")
   288|            return 1
   289|
   290|        return 0
   291|
   292|    elif args.action == "dislike":
   293|        if not args.card:
   294|            print_error("Usage: insight dislike --card <id>")
   295|
   296|            return 1
   297|
   298|        ok = manager.dislike_card(args.card)
   299|
   300|        if ok:
   301|            print_success(f"Disliked {args.card} (★☆☆☆☆)")
   302|
   303|            try:
   304|                from llm.insight import get_evolution_tracker
   305|
   306|                evo = get_evolution_tracker()
   307|
   308|                card = manager.get_card(args.card)  # type: ignore[assignment]
   309|
   310|                topic = card.paper_title if card else ""
   311|
   312|                evo.record_insight_feedback(
   313|                    topic=topic, insight_card_id=args.card, rating=1, paper_title=topic
   314|                )
   315|
   316|            except Exception:
   317|                pass
   318|
   319|        else:
   320|            print_error(f"Card not found: {args.card}")
   321|            return 1
   322|
   323|        return 0
   324|
   325|    elif args.action == "top":
   326|        cards = manager.get_high_quality_cards(min_rating=4, min_scores=1)  # type: ignore[assignment]
   327|
   328|        if not cards:
   329|            print("No highly-rated cards yet. Rate some cards first!")
   330|
   331|            return 0
   332|
   333|        print(f"Top {min(args.top_k, len(cards))} Highest-Rated Insights:")
   334|
   335|        for c in cards[: args.top_k]:  # type: ignore[assignment]
   336|            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)  # type: ignore[attr-defined]
   337|
   338|            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")  # type: ignore[attr-defined]
   339|
   340|        return 0
   341|
   342|    elif args.action == "bottom":
   343|        cards = manager.get_low_quality_cards(max_rating=2, min_scores=1)  # type: ignore[assignment]
   344|
   345|        if not cards:
   346|            print("No low-rated cards yet.")
   347|
   348|            return 0
   349|
   350|        print(f"Lowest-Rated Insights ({len(cards)} total):")
   351|
   352|        for c in cards[: args.top_k]:  # type: ignore[assignment]
   353|            stars = "★" * c.quality_rating + "☆" * (5 - c.quality_rating)  # type: ignore[attr-defined]
   354|
   355|            print(f"  [{c.card_id}] {stars} ({c.usefulness_score:.2f}) {c.content[:60]}")  # type: ignore[attr-defined]
   356|
   357|        return 0
   358|
   359|    elif args.action == "evolve":
   360|        topic = args.query or ""
   361|        gap_type = args.tags or None
   362|        if not topic:
   363|            print_error("Usage: insight evolve --query <topic> [--tags <gap_type>]")
   364|            return 1
   365|        from llm.insight.evolution import InsightEvolution
   366|        from llm.insight.tracker import get_evolution_tracker
   367|
   368|        evo = InsightEvolution(tracker=get_evolution_tracker())  # type: ignore[assignment]
   369|        result = evo.evolve(topic=topic, gap_type=gap_type)  # type: ignore[attr-defined]
   370|        print(evo.render_summary(result))  # type: ignore[attr-defined]
   371|        return 0
   372|
   373|    elif args.action == "quality-report":
   374|        import json as _json
   375|
   376|        from llm.insight.tracker import get_evolution_tracker
   377|
   378|        tracker = get_evolution_tracker()
   379|
   380|        def _render_report(report):
   381|            if args.json:
   382|                print(_json.dumps(report, indent=2))
   383|            else:
   384|                print("=== Gene Pool Quality Report ===")
   385|                print(f"  Total capsules    : {report['total']}")
   386|                print(f"  Avg score        : {report['avg_score']}")
   387|                print(
   388|                    f"  Score dist       : high={report['score_distribution']['high (≥0.7)']} mid={report['score_distribution']['mid (0.4-0.7)']} low={report['score_distribution']['low (<0.4)']}"
   389|                )
   390|                print(f"  Credibility dist : {report['credibility_distribution']}")
   391|                print(
   392|                    f"  Trendslop        : {report['trendslop']['count']} ({report['trendslop']['pct']}) — {report['trendslop']['top_reasons']}"
   393|                )
   394|                print(
   395|                    f"  Feedback use     : high={report['feedback_distribution']['high_use (≥3)']} zero={report['feedback_distribution']['low_use (0)']}"
   396|                )
   397|                print(f"  At-risk (streak≥2): {report['at_risk_capsules']}")
   398|                print(f"  Top gap types    : {report['top_gap_types']}")
   399|
   400|        if args.watch:
   401|            import time
   402|
   403|            print("Watching GenePool health (Ctrl+C to stop)...", flush=True)
   404|            while True:
   405|                report = tracker.get_gene_pool_quality_report()
   406|                if "error" in report:
   407|                    print_error(f"quality-report: {report['error']}")
   408|                    return 1
   409|                ts = time.strftime("%Y-%m-%d %H:%M:%S")
   410|                if args.json:
   411|                    print(_json.dumps({"timestamp": ts, **report}))
   412|                else:
   413|                    print(f"\n[{ts}] === Gene Pool Quality Report ===")
   414|                    print(f"  Total capsules    : {report['total']}")
   415|                    print(f"  Avg score        : {report['avg_score']}")
   416|                    print(
   417|                        f"  Score dist       : high={report['score_distribution']['high (≥0.7)']} mid={report['score_distribution']['mid (0.4-0.7)']} low={report['score_distribution']['low (<0.4)']}"
   418|                    )
   419|                    print(f"  Credibility dist : {report['credibility_distribution']}")
   420|                    print(
   421|                        f"  Trendslop        : {report['trendslop']['count']} ({report['trendslop']['pct']})"
   422|                    )
   423|                    print(f"  At-risk (streak≥2): {report['at_risk_capsules']}")
   424|                time.sleep(args.interval)
   425|        else:
   426|            report = tracker.get_gene_pool_quality_report()
   427|            if "error" in report:
   428|                print_error(f"quality-report: {report['error']}")
   429|                return 1
   430|            _render_report(report)
   431|        return 0
   432|
   433|    elif args.action == "recompute-credibility":
   434|        from llm.insight.tracker import get_evolution_tracker
   435|
   436|        tracker = get_evolution_tracker()
   437|        result = tracker.recompute_credibility_all()
   438|        print(
   439|            f"Recomputed credibility: {result['updated']} updated, {result['errors']} errors (archived capsules skipped)"
   440|        )
   441|        return 0
   442|
   443|    elif args.action == "archive-trendslop":
   444|        from llm.insight.tracker import get_evolution_tracker
   445|
   446|        # type: ignore[assignment]
   447|        tracker = get_evolution_tracker()
   448|        capsules = tracker._load_capsules()
   449|        trendslop_capsules = [c for c in capsules if c.trendslop and c.status == "active"]
   450|        if not trendslop_capsules:
   451|            print("No active trendslop capsules to archive.")
   452|            return 0
   453|        archived = 0
   454|        for c in trendslop_capsules:  # type: ignore[assignment]
   455|            if tracker.archive_capsule(c.capsule_id):  # type: ignore[attr-defined]
   456|                archived += 1
   457|        print(f"Archived {archived} trendslop capsules out of {len(trendslop_capsules)} flagged.")
   458|        return 0
   459|
   460|    elif args.action == "eval-retrieval":
   461|        from llm.insight.tracker import get_evolution_tracker
   462|
   463|        tracker = get_evolution_tracker()
   464|        limit = getattr(args, "top_k", 50)
   465|        result = tracker.eval_retrieval(limit=limit)
   466|        if "error" in result:
   467|            print_error(f"eval-retrieval: {result['error']}")
   468|            return 1
   469|        print(f"=== Gene Pool Retrieval Eval (n={result['total']}) ===")
   470|        print(f"  recall@3 : {result['recall@3']}")
   471|        print(f"  recall@5 : {result['recall@5']}")
   472|        print(f"  MRR      : {result['mrr']}")
   473|        return 0
   474|
   475|    elif args.action == "alert":
   476|        import json as _json
   477|
   478|        from llm.insight.tracker import get_evolution_tracker
   479|
   480|        tracker = get_evolution_tracker()
   481|        report = tracker.get_gene_pool_quality_report()
   482|        if "error" in report:
   483|            print_error(f"alert: {report['error']}")
   484|            return 1
   485|        alerts = report.get("alerts", [])
   486|        if args.json:
   487|            print(
   488|                _json.dumps(
   489|                    {"total": report["total"], "alert_count": len(alerts), "alerts": alerts}
   490|                )
   491|            )
   492|            return 0
   493|        if not alerts:
   494|            print(f"No alerts (GenePool healthy: {report['total']} capsules)")
   495|            return 0
   496|        level_icons = {"critical": "🔴", "warning": "🟡", "info": "🔵"}
   497|        for alert in alerts:
   498|            icon = level_icons.get(alert["level"], "⚪")
   499|            print(f"{icon} [{alert['level'].upper()}] {alert['code']}: {alert['message']}")
   500|        return 0
   501|
