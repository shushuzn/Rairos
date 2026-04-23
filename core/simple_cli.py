"""
Simple CLI - 简化命令行界面

提供简单易用的命令行接口，降低使用门槛。
Inspired by "simplify complexity" philosophy from the tractor startup article.
"""
import sys
import argparse


class SimpleCLI:
    """
    简化版命令行工具
    
    提供最常用的功能，隐藏复杂性。
    """
    
    def __init__(self):
        self.parser = self._create_parser()
    
    def _create_parser(self) -> argparse.ArgumentParser:
        """创建参数解析器"""
        parser = argparse.ArgumentParser(
            description="📚 AI Research OS - 简单易用的论文研究工具",
            formatter_class=argparse.RawDescriptionHelpFormatter,
            epilog="""
示例:
  # 搜索论文
  python -m core.simple_cli search "machine learning"
  
  # 列出所有论文
  python -m core.simple_cli list
  
  # 查看论文状态
  python -m core.simple_cli status
  
  # 导入论文
  python -m core.simple_cli import 2301.001
  
  # 获取帮助
  python -m core.simple_cli help

快速开始:
  1. 使用 'search' 搜索感兴趣的论文
  2. 使用 'import' 导入论文
  3. 使用 'list' 查看已导入的论文
  4. 使用 'status' 查看统计信息
            """
        )
        
        subparsers = parser.add_subparsers(dest="command", help="可用命令")
        
        # Search command
        search_parser = subparsers.add_parser("search", help="🔍 搜索论文")
        search_parser.add_argument("query", help="搜索关键词")
        search_parser.add_argument("-n", "--limit", type=int, default=5, help="返回结果数量 (默认: 5)")
        
        # Import command
        import_parser = subparsers.add_parser("import", help="📥 导入论文")
        import_parser.add_argument("paper_id", help="论文ID (arXiv ID 或 DOI)")
        import_parser.add_argument("--source", choices=["arxiv", "doi"], default="arxiv", help="论文来源")
        
        # List command
        list_parser = subparsers.add_parser("list", help="📚 列出所有论文")
        list_parser.add_argument("-n", "--limit", type=int, default=20, help="显示数量 (默认: 20)")
        list_parser.add_argument("--status", choices=["all", "pending", "done"], default="all", help="按状态筛选")
        
        # Status command
        subparsers.add_parser("status", help="📊 查看状态")
        
        # Stats command
        subparsers.add_parser("stats", help="📈 查看统计")
        
        # Export command
        export_parser = subparsers.add_parser("export", help="💾 导出数据")
        export_parser.add_argument("format", choices=["json", "csv"], default="json", help="导出格式")
        
        # Help command
        subparsers.add_parser("help", help="❓ 显示帮助")
        
        return parser
    
    def run(self, args=None):
        """运行CLI"""
        args = self.parser.parse_args(args)
        
        if not args.command:
            self.parser.print_help()
            return 0
        
        if args.command == "search":
            return self._handle_search(args)
        elif args.command == "import":
            return self._handle_import(args)
        elif args.command == "list":
            return self._handle_list(args)
        elif args.command == "status":
            return self._handle_status(args)
        elif args.command == "stats":
            return self._handle_stats(args)
        elif args.command == "export":
            return self._handle_export(args)
        elif args.command == "help":
            self._show_help()
            return 0
        
        return 0
    
    def _handle_search(self, args):
        """处理搜索命令"""
        from db import Database
        
        print(f"\n🔍 搜索: {args.query}\n")
        
        db = Database()
        db.init()
        
        results, total = db.search_papers(query=args.query, limit=args.limit)
        
        if not results:
            print("❌ 未找到相关论文")
            print("💡 提示: 尝试使用更通用的关键词")
            return 1
        
        print(f"✅ 找到 {total} 个结果，显示前 {len(results)} 个:\n")
        
        for i, paper in enumerate(results, 1):
            print(f"{i}. {paper.title}")
            print(f"   作者: {paper.authors}")
            print(f"   来源: {paper.source}")
            print()
        
        print("💡 使用 'import <论文ID>' 导入论文")
        return 0
    
    def _handle_import(self, args):
        """处理导入命令"""
        from db import Database
        
        print(f"\n📥 正在导入: {args.paper_id}\n")
        
        db = Database()
        db.init()
        
        try:
            db.upsert_paper(args.paper_id, args.source)
            print(f"✅ 论文已导入: {args.paper_id}")
            print("💡 使用 'list' 查看所有论文")
            return 0
        except Exception as e:
            print(f"❌ 导入失败: {e}")
            return 1
    
    def _handle_list(self, args):
        """处理列出命令"""
        from db import Database
        
        print("\n📚 论文列表\n")
        
        db = Database()
        db.init()
        
        papers, total = db.list_papers(
            parse_status=args.status if args.status != "all" else None,
            limit=args.limit
        )
        
        if not papers:
            print("📭 还没有导入任何论文")
            print("💡 使用 'import <论文ID>' 导入论文")
            return 0
        
        print(f"共 {total} 篇论文:\n")
        
        for paper in papers:
            status_icon = "✅" if paper.parse_status == "done" else "⏳"
            print(f"{status_icon} {paper.uid}")
            print(f"   {paper.title[:60]}...")
            print()
        
        return 0
    
    def _handle_status(self, args):
        """处理状态命令"""
        from db import Database
        
        print("\n📊 系统状态\n")
        
        db = Database()
        db.init()
        
        s = db.get_stats()
        
        print(f"总论文数: {s['total_papers']}")
        print(f"  按来源: {', '.join(f'{k}={v}' for k, v in s['by_source'].items())}")
        print(f"  按状态: {', '.join(f'{k}={v}' for k, v in s['by_status'].items())}")
        
        print("\n队列状态:")
        print(f"  待处理: {s['queue_queued']}")
        print(f"  运行中: {s['queue_running']}")
        
        print("\n缓存:")
        print(f"  条目数: {s['cache_entries']}")
        
        return 0
    
    def _handle_stats(self, args):
        """处理统计命令"""
        from db import Database
        
        print("\n📈 详细统计\n")
        
        db = Database()
        db.init()
        
        s = db.get_stats()
        
        print("论文统计:")
        print(f"  总数: {s['total_papers']}")
        print("  来源分布:")
        for source, count in sorted(s['by_source'].items(), key=lambda x: -x[1]):
            pct = (count / s['total_papers'] * 100) if s['total_papers'] > 0 else 0
            print(f"    {source}: {count} ({pct:.1f}%)")
        
        print(f"\n去重记录: {s['dedup_records']}")
        
        return 0
    
    def _handle_export(self, args):
        """处理导出命令"""
        from db import Database
        import json
        
        print(f"\n💾 导出数据 ({args.format})\n")
        
        db = Database()
        db.init()
        
        fields, rows = db.export_papers(format=args.format)
        
        if args.format == "json":
            data = [dict(zip(fields, row)) for row in rows]
            print(json.dumps(data, indent=2, ensure_ascii=False)[:500])
        else:
            print(", ".join(fields))
            for row in rows[:5]:
                print(", ".join(str(v) for v in row))
            if len(rows) > 5:
                print(f"... (共 {len(rows)} 条)")
        
        return 0
    
    def _show_help(self):
        """显示帮助"""
        print("""
📚 AI Research OS - 简化命令行工具

🎯 核心命令:
  search <关键词>     🔍 搜索论文
  import <论文ID>    📥 导入论文
  list             📚 列出所有论文
  status            📊 查看系统状态
  stats             📈 查看详细统计
  export [格式]     💾 导出数据

📖 快速开始:
  1. 搜索论文:     search "machine learning"
  2. 导入论文:     import 2301.001
  3. 查看列表:     list
  4. 查看状态:     status

💡 小贴士:
  - 使用 --help 查看命令详情
  - list 支持 --status 筛选
  - export 支持 json 和 csv 格式
        """)


def main():
    """主函数"""
    cli = SimpleCLI()
    sys.exit(cli.run())


if __name__ == "__main__":
    main()
