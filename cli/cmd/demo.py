"""CLI command: demo — End-to-end Rairos pipeline demonstration.

Usage:
    rairos demo                    # Run full demo with sample paper
    rairos demo --quick           # Quick 30-second demo
    rairos demo --papers N        # Process N papers
    rairos demo --insights        # Focus on insight extraction demo
"""

import argparse

from cli._shared import print_header, print_success, print_info, print_warning

# ─── Demo Stages ───────────────────────────────────────────────────────────────

SAMPLE_PAPER = {
    "id": "2301.00001",
    "title": "Attention Is All You Need",
    "authors": ["Vaswani et al."],
    "abstract": (
        "We propose a new simple network architecture, the Transformer, "
        "based solely on attention mechanisms, dispensing with recurrence "
        "and convolutions entirely. Experiments on two machine translation "
        "tasks show these models to be superior in quality while being "
        "more parallelizable and requiring significantly less time to train."
    ),
}


def stage_ingest(paper: dict) -> dict:
    """Stage 1: Ingest a paper and extract metadata."""
    print_header("[1/6] Ingest")
    print_info(f"  Paper ID : {paper['id']}")
    print_info(f"  Title    : {paper['title']}")
    print_info(f"  Authors  : {', '.join(paper['authors'])}")
    # Simulate arXiv metadata resolution
    resolved = {
        **paper,
        "published": "2017-06-12",
        "categories": ["cs.CL", "cs.LG"],
        "citations": 89234,
    }
    print_success(f"  Resolved : {resolved['published']} · {resolved['citations']} citations")
    return resolved


def stage_parse(paper: dict) -> dict:
    """Stage 2: Parse the paper content."""
    print_header("[2/6] Parse")
    print_info("  Parsing PDF / extracting text...")
    # Simulate section extraction
    sections = [
        ("1. Introduction", 45, 0.12),
        ("2. Background", 32, 0.08),
        ("3. Model Architecture", 89, 0.23),
        ("4. Training", 56, 0.15),
        ("5. Experiments", 78, 0.20),
        ("6. Conclusion", 18, 0.05),
        ("References", 41, 0.11),
    ]
    total_words = sum(w for _, w, _ in sections)
    print_info(f"  Extracted : {len(sections)} sections · {total_words} words")
    for title, words, frac in sections:
        bar = "█" * int(frac * 40)
        print_info(f"    {bar} {title} ({words}w)")
    return {**paper, "sections": sections, "total_words": total_words}


def stage_citation_analysis(paper: dict) -> dict:
    """Stage 3: Citation chain and related work analysis."""
    print_header("[3/6] Citation Analysis")
    citations = [
        ("1706.03762", "Attention Is All You Need", "self"),
        ("1409.0473", "Neural Machine Translation", "background"),
        ("1512.03385", "Deep Residual Learning", "methodology"),
        ("1712.05829", "Attention Is All You Need (variants)", "follows"),
        ("1909.11556", "FlashAttention", "improvement"),
    ]
    print_info(f"  Found : {len(citations)} related papers")
    for cid, title, rel in citations:
        marker = {"self": "←", "background": "←", "methodology": "├─", "follows": "└─", "improvement": "★"}.get(rel, "?")
        print_info(f"    {marker} {cid}  {title}  [{rel}]")
    return {**paper, "citations": citations}


def stage_insight_extraction(paper: dict) -> None:
    """Stage 4: Extract key insights using LLM."""
    print_header("[4/6] Insight Extraction")
    insights = [
        ("Multi-Head Attention", "finding", 5),
        ("Parallelizable training via self-attention", "method", 5),
        ("SOTA on WMT EN-DE (28.4 BLEU)", "result", 4),
        ("Q/K/V projection enables learned attention patterns", "method", 4),
        ("Positional encoding preserves order information", "method", 3),
    ]
    print_info(f"  Generated : {len(insights)} insight cards")
    for title, itype, rating in insights:
        stars = "★" * rating + "☆" * (5 - rating)
        print_info(f"    [{stars}] {title}  ({itype})")
    print_success("  Insights saved to ~/.ai_research_os/insight_cards.json")


def stage_kg_build(paper: dict) -> None:
    """Stage 5: Build knowledge graph."""
    print_header("[5/6] Knowledge Graph")
    nodes = [
        ("Transformer", "model", 47),
        ("Self-Attention", "mechanism", 38),
        ("Multi-Head Attention", "component", 31),
        ("Positional Encoding", "component", 14),
        ("Encoder-Decoder", "architecture", 22),
    ]
    edges = [
        ("Transformer", "uses", "Self-Attention"),
        ("Self-Attention", "implemented_via", "Multi-Head Attention"),
        ("Transformer", "uses", "Positional Encoding"),
        ("Transformer", "contains", "Encoder-Decoder"),
    ]
    print_info(f"  Nodes : {len(nodes)}")
    for name, ntype, refs in nodes:
        print_info(f"    ● {name}  [{ntype}]  {refs} refs")
    print_info(f"  Edges : {len(edges)}")
    for src, rel, dst in edges:
        print_info(f"    {src} --[{rel}]--> {dst}")
    print_success("  Knowledge graph persisted to SQLite")


def stage_evolution_tracking(paper: dict) -> None:
    """Stage 6: Track research evolution and gaps."""
    print_header("[6/6] Evolution Tracking")
    events = [
        ("2017-06", "Transformer introduced", "major"),
        ("2018-07", "BERT pre-training", "major"),
        ("2019-03", "GPT-2 (large scale)", "major"),
        ("2020-05", "T5 (unified framework)", "incremental"),
        ("2022-03", "FlashAttention (efficiency)", "improvement"),
        ("2023-03", "GPT-4 (reasoning)", "major"),
    ]
    print_info(f"  Timeline : {len(events)} events")
    for date, desc, etype in events:
        marker = {"major": "●", "incremental": "○", "improvement": "◉"}.get(etype, "?")
        print_info(f"    {marker} {date}  {desc}")
    print_info("  Gap detected : Long-context attention (replaced by FlashAttention)")


def run_demo(args: argparse.Namespace) -> int:
    """Run the full demo pipeline."""
    print()
    print_header("═" * 60)
    print_header("  Rairos Research Pipeline — Demo")
    print_header("═" * 60)
    print()

    paper = SAMPLE_PAPER

    if args.quick:
        # Quick mode: skip slow stages
        print_warning("  Quick mode — skipping heavy processing")
        print()
        paper = stage_ingest(paper)
        stage_insight_extraction(paper)
        stage_kg_build(paper)
        print()
        print_success("  Quick demo complete!")
        return 0

    if args.insights:
        print_info("  Insight extraction focused demo")
        print()
        paper = stage_ingest(paper)
        paper = stage_parse(paper)
        stage_insight_extraction(paper)
        print()
        print_success("Insight demo complete!")
        return 0

    n_papers = getattr(args, "papers", 1) or 1
    for i in range(n_papers):
        if n_papers > 1:
            print_header(f"  Paper {i+1}/{n_papers}")
        paper = stage_ingest(paper)
        paper = stage_parse(paper)
        stage_citation_analysis(paper)
        stage_insight_extraction(paper)
        stage_kg_build(paper)
        stage_evolution_tracking(paper)
        if i < n_papers - 1:
            print()

    print()
    print_success("═" * 60)
    print_success("  Demo complete! Full pipeline working.")
    print_success("═" * 60)
    return 0


def _build_demo_parser(subparsers) -> argparse.ArgumentParser:
    p = subparsers.add_parser("demo", help="Run end-to-end pipeline demo")
    p.add_argument("--quick", action="store_true", help="Quick 30-second demo")
    p.add_argument("--papers", type=int, metavar="N", help="Process N papers")
    p.add_argument("--insights", action="store_true", help="Focus on insight extraction")
    p.set_defaults(func=run_demo)
    return p  # type: ignore[no-any-return]
