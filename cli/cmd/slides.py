"""
Paper → Slides CLI Command

Usage:
    airos slides 2106.09685                    # 单论文生成
    airos slides 2106.09685 1706.03762        # 多论文对比
    airos slides --list                        # 从数据库选择
    airos slides --interactive                 # 交互模式
"""

import click
import sys
import re
from pathlib import Path
from typing import Optional, List

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from cli._shared import print_success, print_error, print_info, print_warning
from llm.slides import PaperSlidesGenerator, SlidesConfig


def _build_slides_parser(subparsers):
    """Register slides subcommand."""
    p = subparsers.add_parser("slides", help="从论文自动生成演示文稿")
    p.add_argument("arxiv_ids", nargs="*", help="arXiv ID(s)，支持多个论文对比")
    p.add_argument("--list", "-l", action="store_true", help="从本地库选择论文")
    p.add_argument("--interactive", "-i", action="store_true", help="交互模式")
    p.add_argument("--format", "-f", default="pptx",
                   type=click.Choice(["pptx", "md", "html"]),
                   help="输出格式 (默认 pptx)")
    p.add_argument("--template", "-t", default="academic",
                   type=click.Choice(["academic", "minimal", "modern"]),
                   help="幻灯片模板")
    p.add_argument("--slides", "-s", type=int, default=10,
                   help="幻灯片数量 (默认 10)")
    p.add_argument("--output", "-o", type=click.Path(), default=None,
                   help="输出路径")
    p.add_argument("--include-notes", action="store_true",
                   help="包含演讲者备注")
    p.add_argument("--lang", default="zh",
                   type=click.Choice(["zh", "en", "bilingual"]),
                   help="输出语言")
    p.set_defaults(func=lambda a: slides_main(
        a.arxiv_ids, a.list, a.interactive, a.format, a.template,
        a.slides, a.output, a.include_notes, a.lang
    ))


def slides_main(
    arxiv_ids: List[str],
    use_list: bool,
    interactive: bool,
    fmt: str,
    template: str,
    num_slides: int,
    output_path: Optional[str],
    include_notes: bool,
    lang: str,
):
    """Main slides generation entry point."""
    try:
        from db.database import Database
    except ImportError:
        print_error("数据库模块不可用")
        sys.exit(1)

    db = Database()

    # 交互模式
    if interactive or (not arxiv_ids and not use_list):
        interactive_mode(db)
        return

    # 从列表选择
    papers_to_process = []
    if use_list:
        papers_to_process = select_papers_from_db(db)
    else:
        # 解析 arXiv ID
        for aid in arxiv_ids:
            match = re.search(r"(\d+\.\d+)", aid)
            if match:
                papers_to_process.append(match.group(1))

    if not papers_to_process:
        print_error("没有选择任何论文")
        return

    # 生成幻灯片
    generator = PaperSlidesGenerator(db)
    config = SlidesConfig(
        template=template,
        num_slides=num_slides,
        output_format=fmt,
        output_path=output_path,
        include_notes=include_notes,
        language=lang,
    )

    try:
        result = generator.generate(papers_to_process, config)
        print_success(f"幻灯片已生成: {result['output_path']}")
        print_info(f"  幻灯片数: {result['slide_count']}")
        print_info(f"  论文数: {result['paper_count']}")
    except Exception as e:
        print_error(f"生成失败: {e}")
        sys.exit(1)


def interactive_mode(db):
    """交互式选择论文并生成幻灯片."""
    print_info("=== Paper → Slides 交互模式 ===")
    print_info("输入论文编号（逗号分隔），或 'q' 退出:\n")

    # 获取本地论文
    papers = db.get_all_papers(limit=50)
    if not papers:
        print_error("本地数据库为空，先导入一些论文")
        return

    for i, paper in enumerate(papers, 1):
        title = paper.get("title", "Untitled")[:50]
        year = paper.get("published", "")[:4] or "?"
        print(f"  [{i}] {year} - {title}")

    print()
    choice = input("选择: ").strip()

    if choice.lower() in ("q", "quit", "exit"):
        return

    try:
        indices = [int(x.strip()) - 1 for x in choice.split(",")]
        selected = [papers[i] for i in indices if 0 <= i < len(papers)]
        paper_ids = [p["uid"] or p.get("arxiv_id", "") for p in selected]

        generator = PaperSlidesGenerator(db)
        result = generator.generate(paper_ids, SlidesConfig())
        print_success(f"\n生成成功: {result['output_path']}")
    except (ValueError, IndexError) as e:
        print_error(f"选择无效: {e}")


def select_papers_from_db(db) -> List[str]:
    """从数据库选择论文."""
    papers = db.get_all_papers(limit=20)
    if not papers:
        print_error("没有找到本地论文")
        return []

    print_info("可用的论文:")
    for i, paper in enumerate(papers, 1):
        title = paper.get("title", "Untitled")[:40]
        print(f"  [{i}] {title}")

    choice = input("\n选择 (逗号分隔): ").strip()
    try:
        indices = [int(x.strip()) - 1 for x in choice.split(",")]
        return [papers[i]["uid"] or papers[i].get("arxiv_id", "")
                for i in indices if 0 <= i < len(papers)]
    except (ValueError, IndexError):
        print_error("无效选择")
        return []


# Click 命令入口 (直接运行)
@click.command("slides")
@click.argument("arxiv_ids", nargs=-1, type=str)
@click.option("--list", "-l", is_flag=True, help="从本地库选择")
@click.option("--interactive", "-i", is_flag=True, help="交互模式")
@click.option("--format", "-f", default="pptx", type=click.Choice(["pptx", "md", "html"]))
@click.option("--template", "-t", default="academic")
@click.option("--slides", "-s", type=int, default=10)
@click.option("--output", "-o", type=str, default=None)
@click.option("--include-notes", is_flag=True)
@click.option("--lang", default="zh", type=click.Choice(["zh", "en", "bilingual"]))
def slides(
    arxiv_ids: tuple,
    list: bool,
    interactive: bool,
    format: str,
    template: str,
    slides: int,
    output: str,
    include_notes: bool,
    lang: str,
):
    """从论文自动生成演示文稿."""
    slides_main(
        list(arxiv_ids), list, interactive,
        format, template, slides, output, include_notes, lang
    )


def _run_slides(args) -> int:
    """Run slides command from argparse args."""
    return slides_main(
        args.arxiv_ids or [],
        args.list,
        args.interactive,
        args.format,
        args.template,
        args.slides,
        args.output,
        args.include_notes,
        args.lang
    )


if __name__ == "__main__":
    slides()
