"""
Evolution Dashboard CLI Command

Usage:
    airos evolution              # 显示仪表盘
    airos evolution --stats     # 显示统计信息
    airos evolution --patterns  # 显示学习到的模式
    airos evolution --feedback # 显示最近反馈
    airos evolution --report    # 生成学习报告
    airos evolution --clear    # 清空数据
    airos evolution --export   # 导出进化数据
"""

import argparse
import sys
from pathlib import Path

# Add parent to path for imports
sys.path.insert(0, str(Path(__file__).parent.parent.parent))

from cli._shared import print_success, print_info, print_header
from cli.warp import WarpBlocks
from llm.evolution import get_evolution_memory


def _build_evolution_parser(subparsers):
    """Register evolution subcommand."""
    p = subparsers.add_parser(
        "evolution",
        help="Evolution Dashboard — 查看系统学习进度"
    )
    p.add_argument("--stats", "-s", action="store_true", help="显示统计信息")
    p.add_argument("--patterns", "-p", action="store_true", help="显示学习到的模式")
    p.add_argument("--feedback", "-f", action="store_true", help="显示最近反馈")
    p.add_argument("--report", "-r", action="store_true", help="生成学习报告")
    p.add_argument("--sessions", action="store_true", help="显示研究会话")
    p.add_argument("--days", type=int, default=7, help="报告周期（天）")
    p.add_argument("--clear", "-c", action="store_true", help="清空所有进化数据")
    p.add_argument("--export", "-e", action="store_true", help="导出数据到 JSON")
    p.set_defaults(func=lambda a: evolution_main(
        show_stats=a.stats,
        show_patterns=a.patterns,
        show_feedback=a.feedback,
        show_report=a.report,
        show_sessions=getattr(a, 'sessions', False),
        report_days=a.days,
        clear=a.clear,
        export=a.export,
    ))


def evolution_main(
    show_stats: bool = False,
    show_patterns: bool = False,
    show_feedback: bool = False,
    show_report: bool = False,
    show_sessions: bool = False,
    report_days: int = 7,
    clear: bool = False,
    export: bool = False,
) -> int:
    """Main evolution dashboard entry point."""
    evo = get_evolution_memory()

    # 清空数据
    if clear:
        confirm = input("确认清空所有进化数据？(y/N): ").strip().lower()
        if confirm == "y":
            evo.clear()
            print_success("已清空所有进化数据")
        else:
            print_info("已取消")
        return 0  # type: ignore[no-any-return]

    # 导出数据
    if export:
        export_evolution_data(evo)
        return 0  # type: ignore[no-any-return]

    # 生成报告
    if show_report:
        show_learning_report(evo, days=report_days)
        return 0  # type: ignore[no-any-return]

    # 显示统计
    if show_stats:
        show_stats_view(evo)
        return 0

    # 显示模式
    if show_patterns:
        show_patterns_view(evo)
        return 0

    # 显示反馈
    if show_feedback:
        show_feedback_view(evo)
        return 0

    # 显示会话
    if show_sessions:
        show_sessions_view()
        return 0

    # 默认：显示完整仪表盘
    show_dashboard(evo)
    return 0


def show_sessions_view():
    """显示研究会话列表."""
    from llm.research_session import get_session_tracker

    print_header("📚 研究会话历史")
    print()

    tracker = get_session_tracker()
    sessions = tracker.get_recent_sessions(days=30, limit=10)

    if not sessions:
        print_info("  暂无研究会话记录")
        print_info("  使用 --chat 功能开始研究会话")
        return

    for session in sessions:
        print(tracker.render_session_tree(session))
        print()


def show_dashboard(evo):
    """显示完整的 Evolution Dashboard — Warp 风格."""
    from rich.console import Console

    stats = evo.get_stats()
    c = Console()

    # Title
    c.rule("[bold #FF8272] AI Research OS — Evolution Dashboard [/]")
    c.print()

    # Progress bar section
    progress = stats["learning_progress"]
    bar_len = 24
    filled = int(bar_len * progress)
    bar = "█" * filled + "░" * (bar_len - filled)
    pct = int(progress * 100)
    status = "[#B4FA72]● Learning[/]" if pct < 50 else "[#B4FA72]✓ Evolving[/]"

    print(WarpBlocks.section(
        f"System Progress {pct}%",
        f"[#A5D5FE]{bar}[/]  {status}",
        f"Events: {stats['total_events']}  |  Patterns: {stats['total_patterns']}",
        width=60
    ))
    c.print()

    # Core metrics table
    pos_rate = stats.get('positive_rate', 0)
    rate_str = f"[#B4FA72]{render_star(pos_rate)} {pos_rate*100:.0f}%[/]"
    neg = stats['negative_feedback']
    pos = stats['positive_feedback']
    rows = [
        ["Total Feedback", str(stats['total_feedback']), "[#A5D5FE]Overall[/]"],
        ["Positive", str(pos), rate_str],
        ["Negative", str(neg), "[#FF5555]Needs attention[/]" if neg > pos else "[#8E8E8E]Balanced[/]"],
        ["Patterns", str(stats['total_patterns']), "[#B4FA72]✓ Evolved[/]"],
        ["Reliable", str(stats['reliable_patterns']), "[#D0D1FE]★ Stable[/]"],
    ]
    c.print(WarpBlocks.table(
        ["Metric", "Value", "Status"],
        rows,
        title="Core Metrics"
    ))
    c.print()

    # Evolution stage
    stage = get_evolution_stage(stats)
    goal = get_next_goal(stats)
    print(WarpBlocks.section(
        "Evolution Stage",
        f"[#FF8272]Stage:[/] {stage}",
        f"[#A5D5FE]Next:[/] {goal}",
        width=60
    ))
    c.print()

    # Quick links
    print(WarpBlocks.section(
        "Quick Actions",
        "[#A5D5FE]--stats[/]    Detailed statistics",
        "[#A5D5FE]--patterns[/]  View all patterns",
        "[#A5D5FE]--feedback[/] Feedback history",
        "[#A5D5FE]--report[/]   Learning report",
        width=60
    ))
    c.print()


def show_learning_report(evo, days: int = 7):
    """显示学习报告."""
    from llm.evolution_report import generate_evolution_report

    print_header("📊 学习报告")
    print()

    report = generate_evolution_report(days=days)

    if report.total_queries == 0:
        print_info("  暂无数据")
        print("  使用 --chat 功能并提供反馈来积累学习数据")
        print()
        print("  建议:")
        print("    airos chat '什么是 transformer？'")
        print("    airos evolution --report --days 30")
        return

    # 周期信息
    print(f"  📅 周期: {report.period_start[:10]} ~ {report.period_end[:10]}")
    print(f"  💬 总问答: {report.total_queries} | 满意率: {report.positive_rate * 100:.1f}%")
    print()

    # 热门论文
    if report.top_papers:
        print_info("  📚 热门论文:")
        for i, p in enumerate(report.top_papers[:3], 1):
            print(f"    {i}. {p.title[:40]}")
            print(f"       引用 {p.positive_count} 次 | Boost: {p.boost_score:.2f}")
        print()

    # 关键词
    if report.top_keywords:
        print_info("  🔑 关注热点:")
        print("    " + " | ".join(report.top_keywords[:5]))
        print()

    # 探索建议
    if report.questions_to_explore:
        print_info("  💡 建议探索:")
        for q in report.questions_to_explore[:3]:
            print(f"    • {q}")
        print()

    # 进化阶段
    print_info(f"  📍 {report.evolution_stage}")
    print(f"     {report.progress_towards_next}")
    print()

    # 保存选项
    print("  使用 --export 导出完整报告")


def show_stats_view(evo):
    """显示详细统计 — Warp 风格."""
    from rich.console import Console

    stats = evo.get_stats()
    c = Console()

    c.rule("[bold #FF8272] Evolution Statistics [/]")
    c.print()

    # Feedback distribution
    total = stats["total_feedback"]
    pos = stats["positive_feedback"]
    neg = stats["negative_feedback"]

    if total > 0:
        pos_rate = pos / total
        neg_rate = neg / total

        # Mini bar charts
        bar_len = 16
        pos_bar = "█" * int(bar_len * pos_rate) + "░" * (bar_len - int(bar_len * pos_rate))
        neg_bar = "█" * int(bar_len * neg_rate) + "░" * (bar_len - int(bar_len * neg_rate))

        rows = [
            ["[#B4FA72]Satisfied[/]", f"[#B4FA72]{pos_bar}[/]  {pos_rate*100:.0f}% ({pos})"],
            ["[#FF5555]Unsatisfied[/]", f"[#FF5555]{neg_bar}[/]  {neg_rate*100:.0f}% ({neg})"],
            ["[#A5D5FE]Total[/]", f"[#A5D5FE]{total}[/]"],
        ]
        c.print(WarpBlocks.table(["Metric", "Distribution"], rows, title="User Satisfaction"))
        c.print()

    # Event distribution
    rows = [
        ["[#FF8272]Total Events[/]", str(stats["total_events"])],
        ["[#A5D5FE]Patterns[/]", str(stats["total_patterns"])],
        ["[#B4FA72]Reliable[/]", str(stats["reliable_patterns"])],
    ]
    c.print(WarpBlocks.table(["Event Type", "Count"], rows, title="Event Distribution"))
    c.print()

    # Pattern analysis
    patterns = evo.get_all_patterns()
    if patterns:
        sorted_patterns = sorted(
            patterns,
            key=lambda p: p.get("effectiveness", 0),
            reverse=True
        )[:8]

        rows = []
        for i, p in enumerate(sorted_patterns, 1):
            eff = p.get("effectiveness", 0)
            total_att = p.get("success_count", 0) + p.get("failure_count", 0)
            eff_str = f"[#B4FA72]{eff*100:.0f}%[/]" if eff >= 0.7 else f"[#FEFDC2]{eff*100:.0f}%[/]" if eff >= 0.4 else f"[#FF5555]{eff*100:.0f}%[/]"
            rows.append([str(i), p['name'][:28], eff_str, str(total_att)])

        c.print(WarpBlocks.table(
            ["#", "Pattern", "Effectiveness", "Attempts"],
            rows,
            title="Top Patterns"
        ))
    else:
        c.print(WarpBlocks.panel("Patterns", "No pattern data yet"))
    c.print()


def show_patterns_view(evo):
    """显示所有学习到的模式 — Warp 风格."""
    from rich.console import Console

    patterns = evo.get_all_patterns()
    c = Console()

    c.rule("[bold #FF8272]  🧬 基因模式库  [/]")
    c.print()

    if not patterns:
        print(WarpBlocks.panel(
            "Pattern Library",
            "[#8E8E8E]暂无学习到的模式\n\n使用 --chat 功能并提供反馈来积累模式[/]"
        ))
        return

    reliable = [p for p in patterns if p.get("effectiveness", 0) >= 0.7]
    experimental = [p for p in patterns if p not in reliable]

    # 可靠模式
    if reliable:
        rows = []
        for p in reliable:
            eff = p.get("effectiveness", 0)
            total = p.get("success_count", 0) + p.get("failure_count", 0)
            eff_str = f"[#B4FA72]{eff*100:.0f}%[/]"
            rows.append(["⭐", p['name'][:30], eff_str, f"[#A5D5FE]{total}[/]"])

        c.print(WarpBlocks.table(
            ["", "Pattern", "Effectiveness", "Attempts"],
            rows,
            title="Reliable Patterns (>70%)"
        ))
        c.print()

    # 实验中模式
    if experimental:
        rows = []
        for p in experimental:
            eff = p.get("effectiveness", 0)
            total = p.get("success_count", 0) + p.get("failure_count", 0)
            eff_pct = eff * 100
            if eff_pct >= 50:
                eff_str = f"[#FEFDC2]{eff_pct:.0f}%[/]"
            else:
                eff_str = f"[#FF5555]{eff_pct:.0f}%[/]"
            rows.append(["🔬", p['name'][:30], eff_str, f"[#A5D5FE]{total}[/]"])

        c.print(WarpBlocks.table(
            ["", "Pattern", "Effectiveness", "Attempts"],
            rows,
            title="Experimental Patterns"
        ))


def show_feedback_view(evo):
    """显示最近反馈 — Warp 风格."""
    import json
    from rich.console import Console

    c = Console()
    c.rule("[bold #FF8272]  💬 反馈历史  [/]")
    c.print()

    try:
        with open(evo.feedback_file, encoding="utf-8") as f:
            lines = f.readlines()
    except FileNotFoundError:
        print(WarpBlocks.panel(
            "Feedback History",
            "[#8E8E8E]暂无反馈记录\n\n使用 --chat 功能并提供反馈来积累数据[/]"
        ))
        return

    recent = lines[-20:] if len(lines) > 20 else lines

    if not recent:
        print(WarpBlocks.panel(
            "Feedback History",
            "[#8E8E8E]暂无反馈记录\n\n使用 --chat 功能并提供反馈来积累数据[/]"
        ))
        return

    rows = []
    for line in reversed(recent):
        line = line.strip()
        if not line:
            continue
        try:
            data = json.loads(line)
            fb_type = data.get("type", "")
            icon = "[#B4FA72]✅[/]" if fb_type == "positive" else "[#FF5555]❌[/]" if fb_type == "negative" else "[#8E8E8E]➖[/]"
            query = data.get("query", "")[:38]
            timestamp = data.get("timestamp", "")[:10]
            rows.append([icon, f"[#A5D5FE][{timestamp}][/]", query])
        except json.JSONDecodeError:
            continue

    if rows:
        c.print(WarpBlocks.table(
            ["", "Time", "Query"],
            rows,
            title=f"Recent Feedback ({len(rows)} entries)"
        ))


def export_evolution_data(evo):
    """导出进化数据."""
    import json
    from datetime import datetime

    patterns = evo.get_all_patterns()
    stats = evo.get_stats()

    export_data = {
        "exported_at": datetime.now().isoformat(),
        "stats": stats,
        "patterns": patterns,
    }

    output_path = Path(f"evolution_export_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json")
    output_path.write_text(json.dumps(export_data, indent=2, ensure_ascii=False), encoding="utf-8")

    print_success(f"数据已导出: {output_path}")


def get_evolution_stage(stats: dict) -> str:
    """根据统计判断进化阶段."""
    total = stats["total_feedback"]
    reliable = stats["reliable_patterns"]

    if total == 0:
        return "🌱 种子期 — 等待首次反馈"
    elif total < 10:
        return "🌿 萌芽期 — 正在学习"
    elif reliable < 3:
        return "🌳 成长期 — 积累模式"
    elif reliable < 5:
        return "🌲 成熟期 — 优化提升"
    else:
        return "🚀 进化期 — 系统正在进化"


def get_next_goal(stats: dict) -> str:
    """获取下一个目标."""
    reliable = stats["reliable_patterns"]

    if reliable < 1:
        return "收集 3+ 反馈，产出首个可靠模式"
    elif reliable < 3:
        return "积累 10+ 反馈，强化现有模式"
    elif reliable < 5:
        return "扩展模式库，覆盖更多场景"
    else:
        return "系统已具备自进化能力"


def render_star(rate: float) -> str:
    """渲染星级."""
    stars = int(rate * 5)
    return "⭐" * stars + "☆" * (5 - stars)


# CLI 入口
def _run_evolution(args) -> int:
    """Run evolution command from argparse args."""
    return evolution_main(
        show_stats=args.stats,
        show_patterns=args.patterns,
        show_feedback=args.feedback,
        show_report=getattr(args, 'report', False),
        show_sessions=getattr(args, 'sessions', False),
        report_days=getattr(args, 'days', 7),
        clear=args.clear,
        export=args.export,
    )


if __name__ == "__main__":
    import argparse
    p = argparse.ArgumentParser()
    sp = p.add_subparsers()
    _build_evolution_parser(sp)
    args = p.parse_args()
    evolution_main(
        show_stats=args.stats,
        show_patterns=args.patterns,
        show_feedback=args.feedback,
        clear=args.clear,
        export=args.export,
    )
