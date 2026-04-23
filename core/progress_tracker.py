"""
Progress Tracker - 进度追踪系统
"""
from datetime import datetime
from typing import Dict, List


class ProgressTracker:
    """Track progress of research tasks."""
    
    def __init__(self):
        self.tasks = {}
    
    def add_task(self, task_id: str, description: str):
        self.tasks[task_id] = {
            "description": description,
            "status": "pending",
            "created": datetime.now().isoformat(),
            "completed": None
        }
    
    def complete_task(self, task_id: str):
        if task_id in self.tasks:
            self.tasks[task_id]["status"] = "completed"
            self.tasks[task_id]["completed"] = datetime.now().isoformat()
    
    def get_progress(self) -> float:
        total = len(self.tasks)
        if total == 0:
            return 0.0
        completed = sum(1 for t in self.tasks.values() if t["status"] == "completed")
        return completed / total * 100


_tracker = None

def get_tracker() -> ProgressTracker:
    global _tracker
    if _tracker is None:
        _tracker = ProgressTracker()
    return _tracker
