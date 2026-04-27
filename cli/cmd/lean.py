"""CLI command: lean — Verify research hypotheses with Lean 4 theorem prover."""
from __future__ import annotations

import argparse
import sys

from llm.lean_verifier import (
    LeanVerificationResult,
    LeanInstallStatus,
    VerificationLevel,
    check_lean_installed,
    get_lean_install_instructions,
    translate_hypothesis_to_lean,
    verify_lean_code,
    render_result,
    render_result_json,
)
from llm.hypothesis_generator import ResearchHypothesis, HypothesisType, ExperimentDesign


def _build_lean_parser(subparsers) -> argparse.ArgumentParser:
    """Build the lean subcommand parser."""
    p = subparsers.add_parser(
        "lean",
        help="Verify research hypotheses with Lean 4 theorem prover",
        description="Translate research hypotheses into Lean 4 code and verify them.",
    )
    p.add_argument(
        "hypothesis_text",
        nargs="?",
        default=None,
        help="Research hypothesis as natural language text",
    )
    p.add_argument(
        "--hypothesis-id",
        type=str,
        default=None,
        help="Hypothesis ID to load from experiment tracker",
    )
    p.add_argument(
        "--no-llm",
        action="store_true",
        help="Skip LLM translation, use templates only",
    )
    p.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    p.add_argument(
        "--check-install",
        action="store_true",
        help="Just check if Lean is installed",
    )
    p.add_argument(
        "--model", "-M",
        type=str,
        default=None,
        help="LLM model to use",
    )
    p.add_argument(
        "--code-only",
        action="store_true",
        help="Only output the generated Lean code, no verification",
    )
    return p


def _run_lean(args: argparse.Namespace) -> int:
    """Run Lean verification command."""
    # Check install mode
    if args.check_install:
        status, version = check_lean_installed()
        if status == LeanInstallStatus.AVAILABLE:
            print(f"✅ Lean installed: {version}")
            return 0
        else:
            print("❌ Lean not found")
            print(get_lean_install_instructions())
            return 1

    # Build hypothesis
    hypothesis = _build_hypothesis(args)
    if not hypothesis:
        print("❌ No hypothesis provided. Use positional arg or --hypothesis-id")
        return 1

    # Generate Lean code
    lean_code, notes = translate_hypothesis_to_lean(
        hypothesis,
        use_llm=not args.no_llm,
        model=args.model,
    )

    if args.code_only:
        print(lean_code)
        return 0

    # Verify
    result = verify_lean_code(
        lean_code=lean_code,
        hypothesis_id=hypothesis.id,
        hypothesis_text=hypothesis.core_statement,
    )
    result.translation_notes = notes

    # Render
    if args.json:
        print(render_result_json(result))
    else:
        print(render_result(result))

    return 0 if result.level != VerificationLevel.L0_FAILED else 1


def _build_hypothesis(args: argparse.Namespace):
    """Build a ResearchHypothesis from CLI args."""
    hypothesis_id = args.hypothesis_id or "cli-input"
    hypothesis_text = args.hypothesis_text or ""

    # Try to load from experiment tracker
    if args.hypothesis_id:
        try:
            from llm.experiment_tracker import ExperimentTracker
            tracker = ExperimentTracker()
            exps = tracker.list_experiments()
            found = [e for e in exps if e.hypothesis_id == args.hypothesis_id]
            if found:
                exp = found[0]
                return ResearchHypothesis(
                    id=args.hypothesis_id,
                    title=exp.name,
                    core_statement=exp.name,
                    hypothesis_type=HypothesisType.EXPLORATORY,
                    based_on="loaded from experiment tracker",
                    experiment_design=ExperimentDesign(
                        baseline="",
                        variables=[],
                        controls=[],
                        evaluation_metrics=[],
                        expected_results="",
                    ),
                )
            else:
                print(f"⚠ Hypothesis ID '{args.hypothesis_id}' not found in tracker")
                return None
        except Exception:
            pass

    # Build from text
    if hypothesis_text:
        return ResearchHypothesis(
            id=hypothesis_id,
            title=hypothesis_text[:40],
            core_statement=hypothesis_text,
            hypothesis_type=HypothesisType.EXPLORATORY,
            based_on="user input",
            experiment_design=ExperimentDesign(
                baseline="",
                variables=[],
                controls=[],
                evaluation_metrics=[],
                expected_results="",
            ),
        )

    return None
