"""CLI command: hypothesize — Generate research hypotheses from gaps."""
from __future__ import annotations

import argparse

from cli._shared import get_db, print_info
from llm.hypothesis_generator import HypothesisGenerator


def _build_hypothesize_parser(subparsers) -> argparse.ArgumentParser:
    """Build the hypothesize subcommand parser."""
    p = subparsers.add_parser(
        "hypothesize",
        help="Generate testable research hypotheses from gaps",
        description="Generate research hypotheses with experiment designs and risk assessments.",
    )
    p.add_argument(
        "topic",
        nargs="?",
        default=None,
        help="Research topic for hypothesis generation",
    )
    p.add_argument(
        "--gap", "-g",
        type=str,
        default="",
        help="Gap context from gap analysis",
    )
    p.add_argument(
        "--trend", "-t",
        type=str,
        default="",
        help="Trend context from trend analysis",
    )
    p.add_argument(
        "--story", "-s",
        type=str,
        default="",
        help="Story context from story weaving",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Disable LLM enhancement",
    )
    p.add_argument(
        "--creative",
        action="store_true",
        help="Generate creative cross-domain hypotheses",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--model", "-M",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--top", "-n",
        type=int,
        default=5,
        help="Number of hypotheses to generate (default: 5)",
    )
    p.add_argument(
        "--lean",
        action="store_true",
        help="Verify each hypothesis with Lean 4 theorem prover",
    )
    p.add_argument(
        "--no-llm-verify",
        action="store_true",
        help="Skip LLM translation in Lean verification (template only)",
    )
    p.add_argument(
        "--validate",
        type=str,
        default=None,
        dest="validate_id",
        metavar="HYPOTHESIS_ID",
        help="Validate a hypothesis by ID: show experiment results and verdict",
    )
    p.add_argument(
        "--list", "-l",
        action="store_true",
        dest="list_hypotheses",
        help="List all hypotheses with their verdict status",
    )
    return p


def _run_hypothesize(args: argparse.Namespace) -> int:
    """Run hypothesis generation command."""
    db = get_db()
    db.init()

    # Validate mode: check hypothesis status
    if args.validate_id:
        return _run_validate_hypothesis(args.validate_id)

    # List mode: show all hypotheses with verdicts
    if args.list_hypotheses:
        return _run_list_hypotheses()

    generator = HypothesisGenerator(db=db)

    if not args.topic:
        print("❌ 请提供 topic")
        return 1

    print_info(f"🎯 Generating hypotheses for: {args.topic}")

    result = generator.generate(
        topic=args.topic,
        gap_context=args.gap,
        trend_context=args.trend,
        story_context=args.story,
        use_llm=not args.no_llm,
        model=args.model,
        creative=args.creative,
    )

    # Optional: Lean 4 verification
    lean_results = {}
    if args.lean:
        lean_results = _verify_hypotheses_with_lean(result.hypotheses, not args.no_llm_verify, args.model)

    if args.json:
        print(generator.render_json(result))
    else:
        print()
        print(generator.render_result(result))
        if lean_results:
            print(_render_lean_results(lean_results))

    return 0


def _run_list_hypotheses() -> int:
    """List all hypotheses with their verdict status."""
    from llm.insight_evolution import EvolutionTracker
    from llm.experiment_tracker import ExperimentTracker

    ev = EvolutionTracker()
    tracker = ExperimentTracker()

    # Collect all hypothesis_ids from events
    events = ev.get_recent_events(limit=10000)
    hypothesis_ids = set()
    hypothesis_topics = {}  # hypothesis_id -> topic (from hypothesized events)
    for e in events:
        if e.hypothesis_id:
            hypothesis_ids.add(e.hypothesis_id)
            if hasattr(e.action, 'value') and e.action.value == 'hypothesized':
                hypothesis_topics[e.hypothesis_id] = e.topic or e.gap_title or 'unknown'

    if not hypothesis_ids:
        print("No hypotheses found. Run `airos hypothesize <topic>` to generate some.")
        return 0

    # Group experiments by hypothesis_id to get names
    experiments = tracker.list_experiments()
    exp_by_hid = {}
    for e in experiments:
        if e.hypothesis_id:
            if e.hypothesis_id not in exp_by_hid:
                exp_by_hid[e.hypothesis_id] = []
            exp_by_hid[e.hypothesis_id].append(e)

    print(f"Research Hypotheses ({len(hypothesis_ids)} total)\n")
    print(f"  {'Hypothesis ID':<12} {'Status':<14} {'Detail':<40} Experiments")
    print(f"  {'-'*12} {'-'*14} {'-'*40} {'-'*10}")

    # Sort by status: VALIDATED first, then REJECTED, MIXED, INCONCLUSIVE
    status_order = {'VALIDATED': 0, 'REJECTED': 1, 'MIXED': 2, 'INCONCLUSIVE': 3}

    rows = []
    for hid in sorted(hypothesis_ids):
        evts = ev.get_hypothesis_events(hid)
        verdict, detail = _compute_verdict(evts)
        name = exp_by_hid.get(hid, [None])[0].name if exp_by_hid.get(hid) else ''
        n_exp = len(exp_by_hid.get(hid, []))
        rows.append((status_order.get(verdict, 99), verdict, detail, name, hid, n_exp))

    rows.sort()

    for _, verdict, detail, name, hid, n_exp in rows:
        icon = {"VALIDATED": "✅", "REJECTED": "❌", "MIXED": "⚠", "INCONCLUSIVE": "○"}.get(verdict, "?")
        name_short = (name[:37] + "...") if len(name) > 40 else name
        detail_short = (detail[:37] + "...") if len(detail) > 40 else detail
        print(f"  {icon} {verdict:<12} {name_short:<40} {n_exp}  [{hid}]")

    print()
    return 0


def _run_validate_hypothesis(hypothesis_id: str) -> int:
    """Validate a hypothesis by checking linked experiment outcomes."""
    from llm.insight_evolution import EvolutionTracker, ExplorationAction
    from llm.experiment_tracker import ExperimentTracker

    ev = EvolutionTracker()
    tracker = ExperimentTracker()

    events = ev.get_hypothesis_events(hypothesis_id)
    verdict, detail = _compute_verdict(events)

    print(f"🎯 Hypothesis: {hypothesis_id}")
    print()
    print(f"   verdict: {verdict}")
    print(f"  detail:  {detail}")
    print()

    # Show experiment outcomes
    experiments = tracker.list_experiments()
    linked = [e for e in experiments if e.hypothesis_id == hypothesis_id]

    if linked:
        print(f"  linked experiments: {len(linked)}")
        for e in linked:
            icon = {"running": "⚡", "completed": "✓", "failed": "✗"}.get(e.status, "?")
            print(f"    {icon} [{e.id}] {e.name} ({e.status})")
            if e.results:
                for k, v in e.results.items():
                    print(f"       {k}: {v}")
    else:
        print("  no linked experiments found")

    print()

    # Show event timeline
    if events:
        print(f"  event timeline:")
        for evt in events:
            icon = _action_icon(evt.action)
            print(f"    {icon} {evt.action.value} — {evt.topic or '(no topic)'}")

    return 0


def _compute_verdict(events):
    """Compute VALIDATED / REJECTED / INCONCLUSIVE from events."""
    if not events:
        return "INCONCLUSIVE", "no experiments recorded"

    action_vals = {e.action.value if hasattr(e.action, 'value') else str(e.action) for e in events}
    has_completed = "validated" in action_vals
    has_failed = "rejected" in action_vals

    if has_completed and has_failed:
        return "MIXED", "both validated and rejected experiments exist"
    if has_completed:
        return "VALIDATED", "all experiments succeeded"
    if has_failed:
        return "REJECTED", "all experiments failed"
    return "INCONCLUSIVE", "no completed experiments yet"


def _action_icon(action):
    val = action.value if hasattr(action, 'value') else str(action)
    return {
        "validated": "✅",
        "rejected": "❌",
        "hypothesized": "💡",
        "viewed": "👁",
        "accepted": "👍",
        "expanded": "📖",
    }.get(val, "•")


# ── Lean 4 integration ────────────────────────────────────────────────────────

def _verify_hypotheses_with_lean(
    hypotheses,
    use_llm: bool,
    model: str | None,
) -> dict:
    """Verify each hypothesis with Lean 4. Returns {hypothesis_id: LeanVerificationResult}."""
    from llm.lean_verifier import verify_hypothesis, LeanInstallStatus

    results = {}
    for h in hypotheses:
        try:
            result = verify_hypothesis(h, use_llm=use_llm, model=model)
            results[h.id] = result
        except Exception:
            pass  # Skip on any error — don't break hypothesis generation
    return results


def _render_lean_results(results: dict) -> str:
    """Render Lean verification results as a formatted block."""
    if not results:
        return ""

    install_status, _ = _get_lean_install_status()
    lines = [
        "─" * 60,
        "🔬 Lean 4 形式化验证",
        "─" * 60,
    ]

    if install_status == "not_found":
        lines.append("⚠️  Lean 4 未安装 — 跳过形式化验证")
        lines.append("  安装: elan default leanprover/lean4:stable")
        lines.append("─" * 60)
        return "\n".join(lines)

    for h_id, result in results.items():
        icon = {
            "l2_proven": "✅",
            "l1_typecheck": "🟢",
            "l0_syntax": "🟡",
            "l0_failed": "❌",
        }.get(result.level.value, "?")

        lines.append(f"{icon} [{h_id}] {result.level.value}")
        if result.translation_notes:
            lines.append(f"    {result.translation_notes}")
        if result.errors:
            lines.append(f"    错误: {result.errors[0][:80]}")

    lines.append("─" * 60)
    return "\n".join(lines)


def _get_lean_install_status():
    """Return (status_str, version) for Lean installation."""
    from llm.lean_verifier import check_lean_installed, LeanInstallStatus
    status, version = check_lean_installed()
    status_str = {
        LeanInstallStatus.AVAILABLE: "available",
        LeanInstallStatus.NOT_FOUND: "not_found",
        LeanInstallStatus.VERSION_UNKNOWN: "unknown",
    }[status]
    return status_str, version
