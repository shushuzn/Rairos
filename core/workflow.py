"""
Workflow Automation - 工作流自动化
"""
from typing import Callable



class Workflow:
    """自动化工作流"""
    
    def __init__(self, name: str):
        self.name = name
        self.steps = []
    
    def add_step(self, func: Callable, description: str):
        self.steps.append((func, description))
    
    def run(self):
        for func, desc in self.steps:
            print(f"运行: {desc}")
            func()


_workflows = {}


def register_workflow(name: str, workflow: Workflow):
    _workflows[name] = workflow


def get_workflow(name: str) -> Workflow:
    return _workflows.get(name)
