# AI Research OS 使用指南

## 📖 目录

- [快速开始](#快速开始)
- [基础用法](#基础用法)
- [进阶功能](#进阶功能)
- [性能优化](#性能优化)
- [常见问题](#常见问题)

---

## 🚀 快速开始

### 1. 搜索论文

```bash
# 使用简化CLI搜索
python -m core.simple_cli search "machine learning"

# 使用高级搜索
python cli.py search "LLM agent"

# 搜索arXiv
python cli.py search "deep learning" --source arxiv
```

### 2. 导入论文

```bash
# 导入arXiv论文
python -m core.simple_cli import 2301.001

# 导入DOI论文
python -m core.simple_cli import "10.1234/example" --source doi
```

### 3. 查看状态

```bash
# 查看系统状态
python -m core.simple_cli status

# 查看详细统计
python -m core.simple_cli stats

# 列出所有论文
python -m core.simple_cli list
```

---

## 📚 基础用法

### 搜索论文

```python
from db import Database

db = Database()
db.init()

# 搜索论文
results, total = db.search_papers(
    query="machine learning",
    limit=10
)

for paper in results:
    print(f"标题: {paper.title}")
    print(f"作者: {paper.authors}")
    print(f"来源: {paper.source}")
    print()
```

### 导入论文

```python
# 导入单篇论文
db.upsert_paper("2301.001", "arxiv")

# 批量导入
paper_ids = [
    "2301.001",
    "2301.002",
    "2301.003"
]

for pid in paper_ids:
    db.upsert_paper(pid, "arxiv")
```

### 查看论文列表

```python
# 列出所有论文
papers, total = db.list_papers(
    limit=50,
    parse_status="done"  # 可选: "pending", "done"
)

print(f"共 {total} 篇论文")

for paper in papers:
    print(f"- {paper.title[:50]}...")
```

---

## 🎯 进阶功能

### 使用性能分析

```python
from core.profiler import get_profiler, profile

# 获取性能分析器
profiler = get_profiler()
profiler.enable()

# 分析函数性能
@profile("my_function")
def my_function():
    # 你的代码
    pass

# 生成性能报告
print(profiler.get_report())
```

### 使用API速率限制

```python
from core.rate_limiter import get_rate_limit_manager, RateLimitConfig

# 获取速率限制管理器
manager = get_rate_limit_manager()

# 为API端点设置限流
manager.wait_for_endpoint(
    "arxiv",
    RateLimitConfig(requests_per_minute=30)
)

# 检查是否可以调用
if manager.can_call_endpoint("arxiv"):
    # 执行API调用
    pass
```

### 使用智能缓存

```python
from core.smart_cache import get_smart_cache

# 获取智能缓存
cache = get_smart_cache()

# 存储数据
cache.set("key", {"data": "value"}, ttl=3600)

# 获取数据
data = cache.get("key")

# 查看缓存统计
stats = cache.get_stats()
print(f"命中率: {stats['hit_rate_percent']:.1f}%")
```

### 使用资源监控

```python
from core.resource_monitor import get_resource_monitor

# 获取资源监控器
monitor = get_resource_monitor()

# 收集资源统计
stats = monitor.collect_stats()
print(f"CPU: {stats.cpu_percent:.1f}%")
print(f"内存: {stats.memory_percent:.1f}%")
print(f"磁盘: {stats.disk_percent:.1f}%")

# 生成资源报告
print(monitor.get_resource_report())
```

---

## ⚡ 性能优化

### 1. 数据库优化

```python
from db.optimize import apply_database_optimizations, get_database_stats

# 应用数据库优化
db = Database()
applied = apply_database_optimizations(db)

print("已应用的优化:")
for opt in applied:
    print(f"  - {opt}")

# 查看数据库统计
stats = get_database_stats(db)
print(f"总论文数: {stats['papers_count']}")
print(f"数据库大小: {stats['database_size_mb']:.2f} MB")
```

### 2. 缓存优化

```python
from core.smart_cache import get_smart_cache

cache = get_smart_cache()

# 设置合适的缓存大小
# 默认500MB，可根据需要调整

# 查看缓存使用情况
stats = cache.get_stats()
print(f"缓存命中率: {stats['hit_rate_percent']:.1f}%")
print(f"缓存条目: {stats['total_entries']}")
print(f"缓存大小: {stats['total_size_mb']:.2f} MB")
```

### 3. API调用优化

```python
from core.rate_limiter import get_rate_limit_manager

manager = get_rate_limit_manager()

# 查看各端点的使用统计
all_stats = manager.get_all_stats()
for endpoint, stats in all_stats.items():
    print(f"{endpoint}: {stats['total_requests']} 请求")
```

---

## 🔧 常见问题

### Q: 如何设置API密钥？

```bash
# 方式1: 环境变量
export OPENAI_API_KEY="your-api-key"

# 方式2: 命令行参数
python cli.py research --api-key "your-api-key"
```

### Q: 如何查看详细的错误信息？

```python
from core.exceptions import format_error_message

try:
    # 你的代码
    pass
except Exception as e:
    print(format_error_message(e))
```

### Q: 如何清理缓存？

```python
from core.smart_cache import get_smart_cache

cache = get_smart_cache()
cache.clear()  # 清空所有缓存

# 或者清理过期条目
cache.cleanup_expired()
```

### Q: 如何导出论文数据？

```bash
# JSON格式
python -m core.simple_cli export json > papers.json

# CSV格式
python -m core.simple_cli export csv > papers.csv
```

### Q: 如何监控性能？

```python
from core.profiler import get_profiler, profile_block

profiler = get_profiler()

with profile_block("my_operation"):
    # 执行操作
    pass

# 查看性能报告
print(profiler.get_report())
```

---

## 📊 使用示例

### 示例1: 批量导入论文

```python
from db import Database
from core.rate_limiter import get_rate_limit_manager

db = Database()
db.init()

manager = get_rate_limit_manager()

# arXiv论文ID列表
arxiv_ids = [
    "2301.001",
    "2301.002",
    "2301.003",
    "2301.004",
    "2301.005"
]

for paper_id in arxiv_ids:
    # 等待速率限制
    manager.wait_for_endpoint("arxiv")
    
    # 导入论文
    try:
        db.upsert_paper(paper_id, "arxiv")
        print(f"✓ 已导入: {paper_id}")
    except Exception as e:
        print(f"✗ 导入失败: {paper_id} - {e}")
```

### 示例2: 分析论文趋势

```python
from db import Database
from collections import Counter

db = Database()
db.init()

# 获取所有论文
papers, total = db.list_papers(limit=1000)

# 统计来源分布
source_counter = Counter(p.source for p in papers)
print("论文来源分布:")
for source, count in source_counter.most_common():
    pct = (count / total * 100) if total > 0 else 0
    print(f"  {source}: {count} ({pct:.1f}%)")
```

### 示例3: 性能监控脚本

```python
from core.resource_monitor import get_resource_monitor
from core.profiler import get_profiler

monitor = get_resource_monitor()
profiler = get_profiler()

# 收集初始状态
initial_stats = monitor.collect_stats()

# 执行操作
profiler.enable()

# ... 执行你的代码 ...

# 生成综合报告
print("=== 资源使用报告 ===")
print(monitor.get_resource_report())

print("\n=== 性能分析报告 ===")
print(profiler.get_report())
```

---

## 🎓 更多资源

- [项目GitHub](https://github.com/shushuzn/ai_research_os)
- [API文档](docs/api.md)
- [开发指南](docs/development.md)
- [更新日志](CHANGELOG.md)

---

**提示**: 使用 `python -m core.simple_cli help` 查看所有可用命令！
