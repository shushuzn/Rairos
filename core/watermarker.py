"""
WaterMarker - 水印系统
"""
from datetime import datetime
from typing import Dict, List


class WaterMarker:
    """添加水印标记系统"""

    def __init__(self):
        self.marks = []

    def add_mark(self, mark_type: str, content: str):
        self.marks.append({
            "type": mark_type,
            "content": content,
            "time": datetime.now().isoformat()
        })

    def get_marks(self) -> List[Dict]:
        return self.marks

    def clear(self):
        self.marks.clear()


_marker = None

def get_marker() -> WaterMarker:
    global _marker
    if _marker is None:
        _marker = WaterMarker()
    return _marker
