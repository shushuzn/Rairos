"""Quick test"""
from core.progress_tracker import get_tracker
from core.watermarker import get_marker

# 测试进度追踪
tracker = get_tracker()
tracker.add_task("task1", "测试任务")
tracker.complete_task("task1")
print(f"进度: {tracker.get_progress():.1f}%")

# 测试水印
marker = get_marker()
marker.add_mark("info", "测试水印")
print(f"水印数量: {len(marker.get_marks())}")

print("✅ 优化完成")
