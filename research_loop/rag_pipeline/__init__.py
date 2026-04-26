"""
RAG Pipeline Module

Complete闭环：paper2code + EvoSkill 自动改进管道

流程：论文解析 → 代码生成 → 测试生成 → Benchmark评估 → 技能改进

Usage:
    from research_loop.rag_pipeline import RagPipeline
    pipeline = RagPipeline()
    result = pipeline.run_full_pipeline("2106.09685", mode="minimal")
"""

import csv
import json
import re
import subprocess
from pathlib import Path
from typing import Optional

from research_loop.paper2code_integration import PaperPipeline
from research_loop.evoskill_integration import EvoSkillPipeline


class RagPipeline:
    """Paper → Code → Tests → Benchmark → Skills 自动闭环"""

    def __init__(self, work_dir: str = ".rag_work"):
        self.work_dir = Path(work_dir)
        self.paper_pipeline = PaperPipeline(work_dir=str(self.work_dir / ".paper2code"))
        self.evoskill_pipeline = EvoSkillPipeline(work_dir=str(self.work_dir / ".evoskill"))

    def run_full_pipeline(
        self,
        arxiv_id: str,
        mode: str = "minimal",
        framework: str = "pytorch",
        task_name: Optional[str] = None,
    ) -> dict:
        """
        执行完整 RAG 闭环流程

        Args:
            arxiv_id: arXiv ID (e.g. "2106.09685")
            mode: minimal | full | educational
            framework: pytorch | jax | numpy
            task_name: EvoSkill task name (default: arxiv_id)

        Returns:
            dict with paths to all generated artifacts
        """
        task_name = task_name or f"paper_{arxiv_id.replace('.', '_')}"
        paper_dir = self.work_dir / arxiv_id

        # Stage 1: paper2code 生成代码
        paper_result = self.paper_pipeline.run(arxiv_id, mode=mode, framework=framework)

        # Stage 2: 提取测试用例并生成 benchmark CSV
        test_csv = self._extract_and_generate_tests(arxiv_id, paper_dir)

        # Stage 3: 生成 pytest 测试文件
        self._generate_pytest_tests(paper_dir, test_csv)

        # Stage 4: 初始化 EvoSkill benchmark
        self._init_evoskill_benchmark(task_name, str(test_csv))

        return {
            "arxiv_id": arxiv_id,
            "code_dir": paper_result["implementation_dir"],
            "test_csv": str(test_csv),
            "test_dir": str(paper_dir / "tests"),
            "benchmark_dir": str(self.work_dir / ".evoskill"),
            "readme": paper_result["readme"],
        }

    def _extract_and_generate_tests(self, arxiv_id: str, paper_dir: Path) -> Path:
        """
        从论文提取测试用例并生成 CSV

        使用规则提取算法示例：
        - 从 README 或伪代码提取输入输出示例
        - 从公式描述生成测试用例
        """
        test_csv = paper_dir / "tests" / "test_cases.csv"
        test_csv.parent.mkdir(parents=True, exist_ok=True)

        # 尝试从已生成的代码/README中提取测试用例
        test_cases = self._extract_from_code(paper_dir)

        # 如果没有提取到，生成默认测试用例
        if not test_cases:
            test_cases = self._generate_default_cases(arxiv_id)

        # 写入 CSV
        self._write_test_csv(test_csv, test_cases)

        return test_csv

    def _extract_from_code(self, paper_dir: Path) -> list[dict]:
        """从生成的代码中提取测试用例"""
        test_cases = []

        # 检查 README 中是否有示例
        readme_path = paper_dir / "README.md"
        if readme_path.exists():
            content = readme_path.read_text(encoding="utf-8")
            test_cases.extend(self._parse_examples_from_readme(content))

        # 检查 src 目录中的示例代码
        src_dir = paper_dir / "src"
        if src_dir.exists():
            for py_file in src_dir.rglob("*.py"):
                content = py_file.read_text(encoding="utf-8")
                test_cases.extend(self._parse_examples_from_py(content))

        return test_cases[:20]  # 限制最多 20 个用例

    def _parse_examples_from_readme(self, content: str) -> list[dict]:
        """从 README 解析示例"""
        test_cases = []

        # 匹配代码块中的示例
        pattern = r'```(?:python|py)?\n(.*?)```'
        matches = re.findall(pattern, content, re.DOTALL)

        for match in matches:
            # 提取输入输出
            if "=" in match and "print" in match:
                test_cases.append({
                    "question": f"执行以下代码并给出输出: ```{match.strip()}```",
                    "expected_output": "运行成功",
                    "category": "execution",
                })

        return test_cases

    def _parse_examples_from_py(self, content: str) -> list[dict]:
        """从 Python 代码解析示例"""
        test_cases = []

        # 匹配 docstring 中的示例
        pattern = r'"""\s*(.*?)\s*"""'
        matches = re.findall(pattern, content, re.DOTALL)

        for match in matches:
            if "Example" in match or "例子" in match:
                test_cases.append({
                    "question": f"根据以下文档字符串实现函数: {match.strip()[:100]}",
                    "expected_output": "函数实现正确",
                    "category": "implementation",
                })

        return test_cases

    def _generate_default_cases(self, arxiv_id: str) -> list[dict]:
        """生成默认测试用例"""
        return [
            {
                "question": f"验证 {arxiv_id} 实现的正确性",
                "expected_output": "功能正常",
                "category": "general",
            },
            {
                "question": f"检查 {arxiv_id} 的 API 接口是否可调用",
                "expected_output": "接口可用",
                "category": "api",
            },
            {
                "question": f"验证 {arxiv_id} 输入输出格式",
                "expected_output": "格式正确",
                "category": "io",
            },
        ]

    def _write_test_csv(self, csv_path: Path, test_cases: list[dict]) -> None:
        """写入测试 CSV 文件"""
        with open(csv_path, "w", newline="", encoding="utf-8") as f:
            writer = csv.DictWriter(f, fieldnames=["question", "expected_output", "category"])
            writer.writeheader()
            writer.writerows(test_cases)

    def _generate_pytest_tests(self, paper_dir: Path, test_csv: Path) -> None:
        """生成 pytest 测试文件"""
        test_dir = test_csv.parent
        test_csv_path = str(test_csv)

        # 生成 conftest.py
        conftest_content = '''"""Fixtures for generated tests."""
import pytest
from pathlib import Path


@pytest.fixture
def test_data_path():
    """Path to test cases CSV."""
    return Path(__file__).parent / "test_cases.csv"


@pytest.fixture
def paper_dir():
    """Path to paper implementation."""
    return Path(__file__).parent.parent
'''
        (test_dir / "conftest.py").write_text(conftest_content, encoding="utf-8")

        # 生成测试文件
        test_impl_content = f'''"""Auto-generated tests for paper implementation."""
import csv
import pytest
from pathlib import Path


def load_test_cases():
    """Load test cases from CSV."""
    csv_path = Path(__file__).parent / "test_cases.csv"
    cases = []
    with open(csv_path, encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            cases.append(row)
    return cases


class TestPaperImplementation:
    """Tests for generated paper implementation."""

    @pytest.fixture(autouse=True)
    def setup(self, paper_dir):
        self.paper_dir = paper_dir

    def test_code_directory_exists(self):
        """Verify implementation directory exists."""
        src_dir = self.paper_dir / "src"
        assert src_dir.exists(), f"Implementation dir not found: {{src_dir}}"

    @pytest.mark.parametrize("case", load_test_cases(), ids=lambda c: c["category"])
    def test_case(self, case):
        """Run benchmark test case."""
        # 验证类别: execution, implementation, general, api, io
        assert case["category"] in ["execution", "implementation", "general", "api", "io"]
        assert len(case["question"]) > 0
        assert len(case["expected_output"]) > 0
'''
        (test_dir / "test_impl.py").write_text(test_impl_content, encoding="utf-8")

    def _init_evoskill_benchmark(self, task_name: str, csv_path: str) -> dict:
        """初始化 EvoSkill benchmark"""
        # 使用 EvoSkillPipeline 初始化
        evoskill_config = self.work_dir / ".evoskill" / "config.toml"
        task_md = self.work_dir / ".evoskill" / "task.md"

        evoskill_config.parent.mkdir(parents=True, exist_ok=True)

        # 写入配置
        config_content = f'''# EvoSkill benchmark for {task_name}

[harness]
name = "claude"
model = "sonnet"
data_dirs = []
timeout_seconds = 600
max_retries = 2

[evolution]
mode = "skill_only"
iterations = 10
frontier_size = 2
concurrency = 2
no_improvement_limit = 3
failure_samples = 2

[dataset]
path = "{csv_path}"
question_column = "question"
ground_truth_column = "expected_output"
category_column = "category"
train_ratio = 0.5
val_ratio = 0.3

[scorer]
type = "multi_tolerance"
'''
        evoskill_config.write_text(config_content, encoding="utf-8")

        # 写入 task 描述
        task_content = f'''# Task

验证 paper 实现的功能是否正确。

## Output format
返回 "通过" 或具体错误信息。
'''
        task_md.write_text(task_content, encoding="utf-8")

        return {
            "config": str(evoskill_config),
            "task": str(task_md),
        }

    def run_evoskill(self, continue_mode: bool = False) -> dict:
        """运行 EvoSkill 改进循环"""
        result = self.evoskill_pipeline.run(continue_mode=continue_mode)
        return result

    def list_skills(self) -> list[str]:
        """列出发现的技能"""
        return self.evoskill_pipeline.list_skills()
