"""
RAG Pipeline CLI Command

Usage:
    airos rag 2106.09685 --mode minimal
    airos rag run-full <arxiv_id> [--continue]
    airos rag gen-tests <arxiv_id>
    airos rag init-benchmark <csv_path>
"""

import click
import sys
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from research_loop.rag_pipeline import RagPipeline
from cli._shared import print_success, print_error, print_info


def _build_rag_parser(subparsers):
    """Register rag subcommand."""
    p = subparsers.add_parser("rag", help="RAG闭环: paper2code + EvoSkill")
    sub = p.add_subparsers(dest="rag_cmd", help="RAG commands")

    # run-full command
    run_p = sub.add_parser("run-full", help="执行完整 RAG 闭环")
    run_p.add_argument("arxiv_id", help="arXiv ID")
    run_p.add_argument("--mode", "-m", default="minimal",
                       choices=["minimal", "full", "educational"])
    run_p.add_argument("--framework", "-f", default="pytorch",
                       choices=["pytorch", "jax", "numpy"])
    run_p.add_argument("--task", "-t", default=None, help="EvoSkill task name")
    run_p.set_defaults(func=lambda a: rag_run_full.callback(
        arxiv_id=a.arxiv_id, mode=a.mode, framework=a.framework, task_name=a.task))

    # gen-tests command
    gen_p = sub.add_parser("gen-tests", help="从论文生成测试用例")
    gen_p.add_argument("arxiv_id", help="arXiv ID")
    gen_p.set_defaults(func=lambda a: rag_gen_tests.callback(arxiv_id=a.arxiv_id))

    # init-benchmark command
    init_p = sub.add_parser("init-benchmark", help="初始化 EvoSkill benchmark")
    init_p.add_argument("csv_path", help="测试用例 CSV 路径")
    init_p.add_argument("--task", "-t", required=True, help="Task name")
    init_p.set_defaults(func=lambda a: rag_init_benchmark.callback(
        csv_path=a.csv_path, task_name=a.task))

    # run-evoskill command
    evo_p = sub.add_parser("run-evoskill", help="运行 EvoSkill 改进")
    evo_p.add_argument("--continue", "continue_mode", action="store_true",
                       help="从 frontier 继续")
    evo_p.set_defaults(func=lambda a: rag_run_evoskill.callback(
        continue_mode=a.continue_mode))

    # list-skills command
    sub.add_parser("list-skills", help="列出发现的技能").set_defaults(
        func=lambda a: rag_list_skills.callback())

    p.set_defaults(func=lambda a: rag_status.callback())


@click.command("rag")
@click.argument("arxiv_id", required=False, type=str)
@click.option("--mode", "-m", default="minimal",
              type=click.Choice(["minimal", "full", "educational"]),
              help="Implementation mode")
@click.option("--framework", "-f", default="pytorch",
              type=click.Choice(["pytorch", "jax", "numpy"]),
              help="Deep learning framework")
@click.option("--task", "-t", default=None, help="EvoSkill task name")
def rag(arxiv_id: str, mode: str, framework: str, task_name: str):
    """RAG闭环: paper2code + EvoSkill 自动改进管道"""
    if arxiv_id:
        rag_run_full.callback(arxiv_id, mode, framework, task_name)
    else:
        rag_status.callback()


def rag_run_full(arxiv_id: str, mode: str, framework: str, task_name: str):
    """执行完整 RAG 闭环流程"""
    import re

    # Clean arxiv ID from URL if needed
    match = re.search(r"(\d+\.\d+)", arxiv_id)
    if match:
        arxiv_id = match.group(1)

    print_info(f"Starting RAG pipeline for arXiv: {arxiv_id}")

    try:
        pipeline = RagPipeline()
        result = pipeline.run_full_pipeline(
            arxiv_id=arxiv_id,
            mode=mode,
            framework=framework,
            task_name=task_name,
        )

        print_success(f"RAG pipeline completed!")
        print_info(f"  Code: {result['code_dir']}")
        print_info(f"  Test CSV: {result['test_csv']}")
        print_info(f"  Test dir: {result['test_dir']}")
        print_info(f"  Benchmark: {result['benchmark_dir']}")
        print_info("\nNext: Run 'airos rag run-evoskill' to start skill improvement")

    except FileNotFoundError as e:
        print_error(f"Error: {e}")
        print_info("Hint: Ensure paper2code skill is installed")
        sys.exit(1)
    except Exception as e:
        print_error(f"RAG pipeline failed: {e}")
        sys.exit(1)


def rag_gen_tests(arxiv_id: str):
    """从论文生成测试用例"""
    print_info(f"Generating tests for arXiv: {arxiv_id}")

    try:
        pipeline = RagPipeline()
        test_csv = pipeline._extract_and_generate_tests(
            arxiv_id,
            pipeline.work_dir / arxiv_id
        )
        pipeline._generate_pytest_tests(
            pipeline.work_dir / arxiv_id,
            test_csv
        )

        print_success(f"Tests generated: {test_csv}")

    except Exception as e:
        print_error(f"Test generation failed: {e}")
        sys.exit(1)


def rag_init_benchmark(csv_path: str, task_name: str):
    """初始化 EvoSkill benchmark"""
    print_info(f"Initializing benchmark for task: {task_name}")

    try:
        pipeline = RagPipeline()
        result = pipeline._init_evoskill_benchmark(task_name, csv_path)

        print_success(f"Benchmark initialized!")
        print_info(f"  Config: {result['config']}")
        print_info(f"  Task: {result['task']}")
        print_info("\nNext: Run 'airos rag run-evoskill'")

    except Exception as e:
        print_error(f"Benchmark init failed: {e}")
        sys.exit(1)


def rag_run_evoskill(continue_mode: bool):
    """运行 EvoSkill 改进循环"""
    print_info("Running EvoSkill improvement loop...")

    try:
        pipeline = RagPipeline()
        result = pipeline.run_evoskill(continue_mode=continue_mode)
        print_success("EvoSkill run completed")

    except Exception as e:
        print_error(f"EvoSkill run failed: {e}")
        sys.exit(1)


def rag_list_skills():
    """列出发现的技能"""
    try:
        pipeline = RagPipeline()
        skills = pipeline.list_skills()

        if skills:
            print_info("Discovered skills:")
            for skill in skills:
                print(f"  - {skill}")
        else:
            print_info("No skills discovered yet")

    except Exception as e:
        print_error(f"List skills failed: {e}")
        sys.exit(1)


def rag_status():
    """检查 RAG pipeline 状态"""
    try:
        pipeline = RagPipeline()

        print_info("RAG Pipeline Status:")
        print_info(f"  Work dir: {pipeline.work_dir}")

        # Check paper2code
        if pipeline.paper_pipeline.is_available():
            print_success("  paper2code: available")
        else:
            print_error("  paper2code: not found")

        # Check EvoSkill
        if pipeline.evoskill_pipeline.is_available():
            print_success("  EvoSkill: available")
        else:
            print_error("  EvoSkill: not found")

    except Exception as e:
        print_error(f"Status check failed: {e}")
