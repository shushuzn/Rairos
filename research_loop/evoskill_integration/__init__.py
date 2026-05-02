"""
EvoSkill Integration Module

Bridges ai-research-os with EvoSkill for benchmark-driven skill discovery.
EvoSkill automatically creates and improves AI skills based on benchmark data.

Usage:
    from research_loop.evoskill_integration import EvoSkillPipeline
    pipeline = EvoSkillPipeline()
    pipeline.init(task="my_research_task")
    result = pipeline.run()
"""

import subprocess
import shutil
from pathlib import Path
from typing import Optional


class EvoSkillPipeline:
    """EvoSkill wrapper for benchmark-driven skill discovery."""

    def __init__(self, work_dir: str = ".evoskill"):
        self.work_dir = Path(work_dir)
        self.evoskill_cli = shutil.which("evoskill")
        self.evoskill_skill = Path.home() / ".claude" / "skills" / "evoskill"

    def is_available(self) -> bool:
        """Check if evoskill CLI or skill is available."""
        if self.evoskill_cli:
            return True
        if self.evoskill_skill.exists():
            return True
        return False

    def init(
        self,
        task: str,
        dataset_path: str,
        harness: str = "claude",
        model: str = "sonnet",
        question_col: str = "question",
        answer_col: str = "answer",
        category_col: Optional[str] = None,
    ) -> dict:
        """
        Initialize EvoSkill project configuration.

        Args:
            task: Task name/identifier
            dataset_path: Path to CSV benchmark file
            harness: Agent runtime (claude, opencode, codex, goose, openhands)
            model: Model to use
            question_col: CSV column for questions
            answer_col: CSV column for expected answers
            category_col: Optional CSV column for categories

        Returns:
            dict with paths to generated config files
        """
        self.work_dir.mkdir(exist_ok=True)
        config_path = self.work_dir / "config.toml"
        task_path = self.work_dir / "task.md"

        self._write_config(config_path, {
            "task": task,
            "dataset_path": dataset_path,
            "harness": harness,
            "model": model,
            "question_col": question_col,
            "answer_col": answer_col,
            "category_col": category_col,
        })

        return {
            "config": str(config_path),
            "task": str(task_path),
            "work_dir": str(self.work_dir),
        }

    def _write_config(self, config_path: Path, params: dict) -> None:
        """Write EvoSkill config.toml."""
        category_section = '\ncategory_column = "' + str(params["category_col"]) + '"' if params["category_col"] else ""

        content = """# EvoSkill project configuration for {task}

[harness]
name = "{harness}"
model = "{model}"
data_dirs = []
timeout_seconds = 1200
max_retries = 3

[evolution]
mode = "skill_only"
iterations = 20
frontier_size = 3
concurrency = 4
no_improvement_limit = 5
failure_samples = 3

[dataset]
path = "{dataset_path}"
question_column = "{question_col}"
ground_truth_column = "{answer_col}"{category_section}
train_ratio = 0.18
val_ratio = 0.12

[scorer]
type = "multi_tolerance"
""".format(
            task=params["task"],
            dataset_path=params["dataset_path"],
            harness=params["harness"],
            model=params["model"],
            question_col=params["question_col"],
            answer_col=params["answer_col"],
            category_section=category_section,
        )
        config_path.write_text(content, encoding="utf-8")

    def run(self, continue_mode: bool = False, verbose: bool = False) -> dict:
        """
        Run the EvoSkill self-improvement loop.

        Args:
            continue_mode: Resume from existing frontier
            verbose: Show per-sample pass/fail results

        Returns:
            dict with run results
        """
        if not self.is_available():
            raise FileNotFoundError(
                "EvoSkill not found. Install with:\n"
                "  pip install evoskill\n"
                "Or run: evoskill init in your project directory"
            )

        cmd = ["evoskill", "run"]
        if continue_mode:
            cmd.append("--continue")
        if verbose:
            cmd.append("--verbose")

        subprocess.run(cmd, check=True)

        return {
            "status": "completed",
            "frontier": str(self.work_dir / "frontier"),
        }

    def eval(self) -> dict:
        """Evaluate the best program on validation set."""
        subprocess.run(["evoskill", "eval"], check=True)
        return {"status": "evaluated"}

    def list_skills(self) -> list[str]:
        """List discovered skills."""
        result = subprocess.run(
            ["evoskill", "skills"],
            capture_output=True,
            text=True,
            check=True,
        )
        skills = [line.strip() for line in result.stdout.strip().split("\n") if line.strip()]
        return skills

    def show_diff(self, from_iter: Optional[int] = None, to_iter: Optional[int] = None) -> str:
        """
        Show diff between iterations.

        Args:
            from_iter: Source iteration (default: baseline)
            to_iter: Target iteration (default: current best)

        Returns:
            Diff output as string
        """
        cmd = ["evoskill", "diff"]
        if from_iter is not None and to_iter is not None:
            cmd.extend([str(from_iter), str(to_iter)])

        result = subprocess.run(cmd, capture_output=True, text=True, check=True)
        return result.stdout

    def reset(self) -> None:
        """Reset all program branches and start fresh."""
        subprocess.run(["evoskill", "reset"], check=True)
