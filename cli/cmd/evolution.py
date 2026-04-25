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

from cli._shared import print_success, print_error, print_info, print_header, Colors
from llm.evolution import get_evolution_memory, FeedbackType


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
    p.add_argument("--days", type=int, default=7, help="报告周期（天）")
    p.add_argument("--clear", "-c", action="store_true", help="清空所有进化数据")
    p.add_argument("--export", "-e", action="store_true", help="导出数据到 JSON")
    p.set_defaults(func=lambda a: evolution_main(
        show_stats=a.stats,
        show_patterns=a.patterns,
        show_feedback=a.feedback,
        show_report=a.report,
        report_days=a.days,
        clear=a.clear,
        export=a.export,
    ))


def evolution_main(
    show_stats: bool = False,
    show_patterns: bool = False,
    show_feedback: bool = False,
    show_report: bool = False,
    report_days: int = 7,
    clear: bool = False,
    export: bool = False,
):
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
        return 0

    # 导出数据
    if export:
        export_evolution_data(evo)
        return 0

    # 生成报告
    if show_report:
        show_learning_report(evo, days=report_days)
        return 0

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

    # 默认：显示完整仪表盘
    show_dashboard(evo)
    return 0


def show_dashboard(evo):
    """显示完整的 Evolution Dashboard."""
    stats = evo.get_stats()

    print_header("═" * 50)
    print_header("   AI Research OS — Evolution Dashboard")
    print_header("═" * 50)
    print()

    # 学习进度条
    progress = stats["learning_progress"]
    bar_len = 30
    filled = int(bar_len * progress)
    bar = "█" * filled + "░" * (bar_len - filled)

    print(f"  系统学习进度: [{bar}] {int(progress*100)}%")
    print()

    # 核心指标
    print_info("  📊 核心指标")
    print(f"    总反馈数:     {stats['total_feedback']}")
    print(f"    正面反馈:   {stats['positive_feedback']}  {render_star(stats.get('positive_rate', 0))}")
    print(f"    负面反馈:   {stats['negative_feedback']}")
    print(f"    进化事件:   {stats['total_events']}")
    print()

    # 模式统计
    print_info("  🧬 基因模式")
    print(f"    学习模式:   {stats['total_patterns']}")
    print(f"    可靠模式:   {stats['reliable_patterns']}  ⭐")
    print()

    # 进化阶段
    print_info("  🚀 进化阶段")
    stage = get_evolution_stage(stats)
    print(f"    当前阶段:   {stage}")
    print(f"    下一目标:   {get_next_goal(stats)}")
    print()

    # 快捷选项
    print_header("─" * 50)
    print_info("  详细视图:")
    print("    airos evolution --stats     # 统计详情")
    print("    airos evolution --patterns  # 所有模式")
    print("    airos evolution --feedback  # 反馈历史")
    print()


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
        print(f"    " + " | ".join(report.top_keywords[:5]))
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
    """显示详细统计."""
    stats = evo.get_stats()

    print_header("📊 进化统计详情")
    print()

    # 反馈趋势
    total = stats["total_feedback"]
    pos = stats["positive_feedback"]
    neg = stats["negative_feedback"]

    if total > 0:
        pos_rate = pos / total
        neg_rate = neg / total

        print_info("  用户满意度分布:")
        print(f"    满意 ████  {pos_rate*100:.1f}% ({pos})")
        print(f"    不满意 ████ {neg_rate*100:.1f}% ({neg})")
        print()

    # 事件类型分布
    print_info("  事件分布:")
    print(f"    总事件数: {stats['total_events']}")
    print()

    # 模式分析
    patterns = evo.get_all_patterns()
    if patterns:
        print_info("  模式效果排名 (Top 5):")

        # 按效果排序
        sorted_patterns = sorted(
            patterns,
            key=lambda p: p.get("effectiveness", 0),
            reverse=True
        )[:5]

        for i, p in enumerate(sorted_patterns, 1):
            eff = p.get("effectiveness", 0)
            total_att = p.get("success_count", 0) + p.get("failure_count", 0)
            print(f"    [{i}] {p['name'][:30]}")
            print(f"        成功率: {eff*100:.0f}% ({total_att}次尝试)")
    else:
        print("    暂无模式数据")


def show_patterns_view(evo):
    """显示所有学习到的模式."""
    patterns = evo.get_all_patterns()

    print_header("🧬 已学习的基因模式")
    print()

    if not patterns:
        print_info("  暂无学习到的模式")
        print_info("  使用 --chat 功能并提供反馈来积累模式")
        return

    reliable = [p for p in patterns if p.get("effectiveness", 0) >= 0.7]
    experimental = [p for p in patterns if p not in reliable]

    # 可靠模式
    if reliable:
        print_info("  ⭐ 可靠模式 (成功率 >70%):")
        for p in reliable:
            eff = p.get("effectiveness", 0)
            total = p.get("success_count", 0) + p.get("failure_count", 0)
            print(f"    • {p['name']}")
            print(f"      成功率: {eff*100:.0f}% | 尝试: {total}次")
        print()

    # 实验中模式
    if experimental:
        print_info("  🔬 实验中模式:")
        for p in experimental:
            eff = p.get("effectiveness", 0)
            total = p.get("success_count", 0) + p.get("failure_count", 0)
            print(f"    • {p['name']}")
            print(f"      成功率: {eff*100:.0f}% | 尝试: {total}次")


def show_feedback_view(evo):
    """显示最近反馈."""
    print_header("💬 最近反馈历史")
    print()

    try:
        with open(evo.feedback_file, encoding="utf-8") as f:
            lines = f.readlines()

        recent = lines[-20:] if len(lines) > 20 else lines

        if not recent:
            print_info("  暂无反馈")
            return

        for line in reversed(recent):
            line = line.strip()
            if not line:
                continue
            try:
                import json
                data = json.loads(line)
                fb_type = data.get("type", "")
                icon = "✅" if fb_type == "positive" else "❌" if fb_type == "negative" else "➖"
                query = data.get("query", "")[:40]
                timestamp = data.get("timestamp", "")[:10]
                print(f"  {icon} [{timestamp}] {query}...")
            except json.JSONDecodeError:
                continue
    except FileNotFoundError:
        print_info("  暂无反馈")


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
