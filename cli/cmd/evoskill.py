"""
EvoSkill CLI Command

Usage:
    airos evoskill init --task research_qa --dataset ./benchmark.csv
    airos evoskill run [--continue]
    airos evoskill eval
    airos evoskill diff
"""

import click
import sys
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from research_loop.evoskill_integration import EvoSkillPipeline
from cli._shared import print_success, print_error, print_info


def _build_evoskill_parser(subparsers):
    """Register evoskill subcommand."""
    p = subparsers.add_parser("evoskill", help="Benchmark-driven skill discovery")
    sub = p.add_subparsers(dest="evoskill_cmd", help="EvoSkill commands")

    # init command
    init_p = sub.add_parser("init", help="Initialize EvoSkill project")
    init_p.add_argument("--task", "-t", required=True, help="Task name")
    init_p.add_argument("--dataset", "-d", required=True, help="Path to benchmark CSV")
    init_p.add_argument("--harness", default="claude", help="Agent runtime")
    init_p.add_argument("--model", default="sonnet", help="Model to use")
    init_p.add_argument("--question-col", default="question", help="Question column name")
    init_p.add_argument("--answer-col", default="answer", help="Answer column name")
    init_p.add_argument("--category-col", default=None, help="Category column name")
    init_p.set_defaults(func=lambda a: evoskill_init.callback(
        task=a.task, dataset=a.dataset, harness=a.harness, model=a.model,
        question_col=a.question_col, answer_col=a.answer_col, category_col=a.category_col))

    # run command
    run_p = sub.add_parser("run", help="Run EvoSkill self-improvement loop")
    run_p.add_argument("--continue", "continue_mode", action="store_true", help="Resume from frontier")
    run_p.add_argument("--verbose", "-v", action="store_true", help="Show pass/fail details")
    run_p.set_defaults(func=lambda a: evoskill_run.callback(
        continue_mode=a.continue_mode, verbose=a.verbose))

    # eval command
    sub.add_parser("eval", help="Evaluate best program on validation set").set_defaults(
        func=lambda a: evoskill_eval.callback())

    # diff command
    diff_p = sub.add_parser("diff", help="Show diff between iterations")
    diff_p.add_argument("from_iter", type=int, nargs="?", default=None, help="Source iteration")
    diff_p.add_argument("to_iter", type=int, nargs="?", default=None, help="Target iteration")
    diff_p.set_defaults(func=lambda a: evoskill_diff.callback(
        from_iter=a.from_iter, to_iter=a.to_iter))

    # reset command
    sub.add_parser("reset", help="Reset all program branches").set_defaults(
        func=lambda a: evoskill_reset.callback())

    # is-available check
    p.set_defaults(func=lambda a: evoskill_status.callback())


@click.command("evoskill")
@click.argument("command", required=False, type=str)
@click.option("--task", "-t", help="Task name")
@click.option("--dataset", "-d", help="Path to benchmark CSV")
@click.option("--harness", default="claude", help="Agent runtime (claude, opencode, etc.)")
@click.option("--model", default="sonnet", help="Model to use")
@click.option("--question-col", default="question", help="Question column name")
@click.option("--answer-col", default="answer", help="Answer column name")
@click.option("--category-col", default=None, help="Category column name")
@click.option("--continue", "continue_mode", is_flag=True, help="Resume from frontier")
@click.option("--verbose", "-v", is_flag=True, help="Show pass/fail details")
def evoskill(
    command: str,
    task: str,
    dataset: str,
    harness: str,
    model: str,
    question_col: str,
    answer_col: str,
    category_col: str,
    continue_mode: bool,
    verbose: bool,
):
    """Benchmark-driven skill discovery with EvoSkill."""
    if command == "init" or task:
        evoskill_init.callback(task or "", dataset or "", harness, model, question_col, answer_col, category_col)
    elif command == "run":
        evoskill_run.callback(continue_mode, verbose)
    elif command == "eval":
        evoskill_eval.callback()
    elif command == "diff":
        evoskill_diff.callback(None, None)
    elif command == "reset":
        evoskill_reset.callback()
    else:
        evoskill_status.callback()


def evoskill_init(task: str, dataset: str, harness: str, model: str,
                  question_col: str, answer_col: str, category_col: str):
    """Initialize EvoSkill project."""
    print_info("Initializing EvoSkill project for task: " + task)

    try:
        pipeline = EvoSkillPipeline()

        if not pipeline.is_available():
            print_error("EvoSkill not available. Install with: pip install evoskill")
            sys.exit(1)

        result = pipeline.init(
            task=task,
            dataset_path=dataset,
            harness=harness,
            model=model,
            question_col=question_col,
            answer_col=answer_col,
            category_col=category_col,
        )

        print_success("Config created: " + result["config"])
        print_info("  Task: " + result["task"])
        print_info("\nNext: Edit .evoskill/task.md, then run: airos evoskill run")

    except Exception as e:
        print_error("Init failed: " + str(e))
        sys.exit(1)


def evoskill_run(continue_mode: bool, verbose: bool):
    """Run EvoSkill self-improvement loop."""
    print_info("Running EvoSkill self-improvement loop...")

    try:
        pipeline = EvoSkillPipeline()
        pipeline.run(continue_mode=continue_mode, verbose=verbose)
        print_success("Run completed")

    except FileNotFoundError:
        print_error("EvoSkill not initialized. Run: airos evoskill init")
        sys.exit(1)
    except Exception as e:
        print_error("Run failed: " + str(e))
        sys.exit(1)


def evoskill_eval():
    """Evaluate best program."""
    print_info("Evaluating...")

    try:
        pipeline = EvoSkillPipeline()
        pipeline.eval()
        print_success("Evaluation complete")

    except Exception as e:
        print_error("Eval failed: " + str(e))
        sys.exit(1)


def evoskill_diff(from_iter: int, to_iter: int):
    """Show diff between iterations."""
    try:
        pipeline = EvoSkillPipeline()
        diff = pipeline.show_diff(from_iter, to_iter)
        click.echo(diff)

    except Exception as e:
        print_error("Diff failed: " + str(e))
        sys.exit(1)


def evoskill_reset():
    """Reset all program branches."""
    try:
        pipeline = EvoSkillPipeline()
        pipeline.reset()
        print_success("Reset complete")

    except Exception as e:
        print_error("Reset failed: " + str(e))
        sys.exit(1)


def evoskill_status():
    """Check EvoSkill availability."""
    pipeline = EvoSkillPipeline()

    if pipeline.is_available():
        print_success("EvoSkill is available")
    else:
        print_error("EvoSkill not found")
        print_info("Install: pip install evoskill")
