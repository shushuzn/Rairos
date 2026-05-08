"""LLM-Powered Research Route Planner.

A GPS for research — plans a concrete reading + experiment sequence from hypothesis to verdict.
Visualizes as a dependency graph, tracks progress, and re-plans on dead ends.
"""

from __future__ import annotations

import json
import time
import uuid
from dataclasses import asdict, dataclass, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


class StepType(Enum):
    READ_PAPER = "read_paper"
    RUN_EXPERIMENT = "run_experiment"
    COMPARE_METHODS = "compare_methods"
    WRITE_ANALYSIS = "write_analysis"
    SURVEY_BASELINES = "survey_baselines"
    CHECK_CONTRADICTION = "check_contradiction"
    REVISE_HYPOTHESIS = "revise_hypothesis"


class StepStatus(Enum):
    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    BLOCKED = "blocked"
    FAILED = "failed"
    SKIPPED = "skipped"


class PlanStatus(Enum):
    ACTIVE = "active"
    COMPLETED = "completed"
    ABANDONED = "abandoned"
    REVISED = "revised"


@dataclass
class PlanStep:
    """A single step in the research route."""

    step_id: str
    type: StepType
    description: str
    estimated_hours: float = 1.0
    dependencies: List[str] = field(default_factory=list)  # step_ids this depends on
    status: StepStatus = StepStatus.PENDING
    result: str = ""
    notes: str = ""
    created_at: float = field(default_factory=time.time)
    completed_at: float = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "step_id": self.step_id,
            "type": self.type.value,
            "description": self.description,
            "estimated_hours": self.estimated_hours,
            "dependencies": self.dependencies,
            "status": self.status.value,
            "result": self.result,
            "notes": self.notes,
            "created_at": datetime.fromtimestamp(self.created_at).isoformat(),
            "completed_at": datetime.fromtimestamp(self.completed_at).isoformat()
            if self.completed_at
            else "",
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "PlanStep":
        d = dict(d)
        d["type"] = StepType(d.pop("type"))
        d["status"] = StepStatus(d.pop("status", "pending"))
        return cls(**d)


@dataclass
class ResearchPlan:
    """A complete research route plan."""

    plan_id: str
    hypothesis: str
    goal: str  # what we're trying to determine
    steps: List[PlanStep] = field(default_factory=list)
    status: PlanStatus = PlanStatus.ACTIVE
    current_step_id: str = ""
    created_at: float = field(default_factory=time.time)
    updated_at: float = field(default_factory=time.time)
    completed_at: float = 0
    revision_count: int = 0
    parent_plan_id: str = ""  # if this is a revised plan

    def to_dict(self) -> Dict[str, Any]:
        return {
            "plan_id": self.plan_id,
            "hypothesis": self.hypothesis,
            "goal": self.goal,
            "steps": [s.to_dict() for s in self.steps],
            "status": self.status.value,
            "current_step_id": self.current_step_id,
            "created_at": datetime.fromtimestamp(self.created_at).isoformat(),
            "updated_at": datetime.fromtimestamp(self.updated_at).isoformat(),
            "completed_at": datetime.fromtimestamp(self.completed_at).isoformat()
            if self.completed_at
            else "",
            "revision_count": self.revision_count,
            "parent_plan_id": self.parent_plan_id,
        }

    @classmethod
    def from_dict(cls, d: Dict[str, Any]) -> "ResearchPlan":
        d = dict(d)
        d["status"] = PlanStatus(d.pop("status", "active"))
        d["steps"] = [PlanStep.from_dict(s) for s in d.get("steps", [])]
        return cls(**d)

    def get_step(self, step_id: str) -> Optional[PlanStep]:
        for s in self.steps:
            if s.step_id == step_id:
                return s
        return None

    def get_ready_steps(self) -> List[PlanStep]:
        """Steps whose dependencies are all completed."""
        completed_ids = {s.step_id for s in self.steps if s.status == StepStatus.COMPLETED}
        ready = []
        for s in self.steps:
            if s.status == StepStatus.PENDING:
                if all(dep in completed_ids for dep in s.dependencies):
                    ready.append(s)
        return ready

    def get_progress(self) -> Dict[str, Any]:
        total = len(self.steps)
        completed = sum(1 for s in self.steps if s.status == StepStatus.COMPLETED)
        failed = sum(1 for s in self.steps if s.status == StepStatus.FAILED)
        total_hours = sum(s.estimated_hours for s in self.steps)
        completed_hours = sum(
            s.estimated_hours for s in self.steps if s.status == StepStatus.COMPLETED
        )
        return {
            "total_steps": total,
            "completed": completed,
            "failed": failed,
            "pending": total - completed - failed,
            "progress_pct": round(completed / total * 100, 1) if total else 0,
            "estimated_hours": total_hours,
            "completed_hours": completed_hours,
        }


# ─── Storage ────────────────────────────────────────────────────────


def _get_plans_path() -> Path:
    path = Path.home() / ".ai_research_os" / "route_planner"
    path.mkdir(parents=True, exist_ok=True)
    return path


def _get_plans_index() -> Dict[str, str]:  # plan_id -> filename
    index_path = _get_plans_path() / "index.json"
    if index_path.exists():
        try:
            return json.loads(index_path.read_text(encoding="utf-8"))  # type: ignore[no-any-return]
        except Exception:
            pass
    return {}


def _save_index(index: Dict[str, str]) -> None:
    index_path = _get_plans_path() / "index.json"
    index_path.write_text(json.dumps(index, indent=2, ensure_ascii=False), encoding="utf-8")


def _get_plan_path(plan_id: str) -> Path:
    return _get_plans_path() / f"plan_{plan_id}.json"


# ─── Route Planner ───────────────────────────────────────────────────


class RoutePlanner:
    """Plans and tracks research routes from hypothesis to conclusion."""

    def __init__(self):
        pass

    def create_plan(
        self,
        hypothesis: str,
        goal: str,
        known_papers: Optional[List[Dict[str, str]]] = None,
        api_key: Optional[str] = None,
        base_url: Optional[str] = None,
        model: Optional[str] = None,
    ) -> ResearchPlan:
        """Create a new research route plan from a hypothesis.

        Args:
            hypothesis: The research hypothesis to investigate
            goal: What the plan should determine (e.g. "Does X outperform Y on Z?")
            known_papers: Optional list of {"arxiv_id": ..., "title": ...} already known
            api_key: LLM API key
            base_url: LLM API base URL
            model: Model name

        Returns:
            ResearchPlan with structured steps and dependencies
        """
        import os

        papers_context = ""
        if known_papers:
            papers_lines = []
            for p in known_papers[:10]:
                papers_lines.append(f"- {p.get('title', 'Unknown')} ({p.get('arxiv_id', '')})")
            papers_context = "\n\nKnown relevant papers:\n" + "\n".join(papers_lines)

        prompt = f"""You are a research strategy planner. Given a hypothesis and goal, create a concrete, dependency-ordered research plan.

HYPOTHESIS: {hypothesis}
GOAL: {goal}{papers_context}

Create a plan with 5-8 steps. Each step should be one of these types:
- read_paper: Read a specific paper or paper type
- run_experiment: Conduct an experiment or evaluation
- compare_methods: Systematically compare approaches
- write_analysis: Write up findings
- survey_baselines: Survey baseline methods before experiments
- check_contradiction: Check if existing results contradict the hypothesis
- revise_hypothesis: Revise the hypothesis based on findings

Respond ONLY with valid JSON (no markdown, no explanation):
{{
  "steps": [
    {{
      "type": "<step_type>",
      "description": "<specific, actionable description>",
      "estimated_hours": <float, e.g. 2.0>,
      "dependencies": [<step_id if this step depends on earlier ones, else empty array>]
    }}
  ]
}}

Rules:
- Steps must be in dependency order
- read_paper steps should specify what to look for (not just "read X")
- run_experiment steps should specify what metric to measure
- Every step must have a clear completion criterion
- Dependencies form a DAG — no circular dependencies
- Total estimated time should be 1-4 weeks of full-time work
- Include a "write_analysis" step at the end
- If the hypothesis could be disproven early, include a "check_contradiction" step"""

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        try:
            response = call_llm_chat_completions(
                base_url=base_url or os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
                api_key=api_key or os.getenv("OPENAI_API_KEY", ""),
                model=model or os.getenv("LLM_MODEL", "") or LLM_MODEL,
                system_prompt="You are a research strategy expert. Produce valid JSON only.",
                user_prompt=prompt,
            )

            parsed = json.loads(response.strip())
        except Exception:
            # Fallback to a simple default plan
            parsed = {
                "steps": [
                    {
                        "type": "survey_baselines",
                        "description": "Survey existing baseline methods for " + hypothesis[:60],
                        "estimated_hours": 4.0,
                        "dependencies": [],
                    },
                    {
                        "type": "read_paper",
                        "description": "Read 3 most relevant papers on " + hypothesis[:60],
                        "estimated_hours": 3.0,
                        "dependencies": [],
                    },
                    {
                        "type": "check_contradiction",
                        "description": "Check if any evidence contradicts the hypothesis",
                        "estimated_hours": 2.0,
                        "dependencies": ["step_1", "step_2"],
                    },
                    {
                        "type": "run_experiment",
                        "description": "Design and run experiments to test " + hypothesis[:60],
                        "estimated_hours": 8.0,
                        "dependencies": ["step_3"],
                    },
                    {
                        "type": "compare_methods",
                        "description": "Compare results against reported baselines",
                        "estimated_hours": 3.0,
                        "dependencies": ["step_4"],
                    },
                    {
                        "type": "write_analysis",
                        "description": "Write up findings and conclusions",
                        "estimated_hours": 4.0,
                        "dependencies": ["step_5"],
                    },
                ]
            }

        # Build plan with step IDs
        plan_id = str(uuid.uuid4())[:8]
        steps = []
        step_id_map = {}

        for i, s in enumerate(parsed.get("steps", [])):
            step_id = f"step_{i + 1}"
            step_id_map[step_id] = step_id
            dep_ids = []
            for dep in s.get("dependencies", []):
                dep_ids.append(dep)

            steps.append(
                PlanStep(
                    step_id=step_id,
                    type=StepType(s.get("type", "read_paper")),
                    description=s.get("description", ""),
                    estimated_hours=float(s.get("estimated_hours", 1.0)),
                    dependencies=dep_ids,
                )
            )

        plan = ResearchPlan(
            plan_id=plan_id,
            hypothesis=hypothesis,
            goal=goal,
            steps=steps,
        )

        self._save_plan(plan)
        return plan

    def revise_plan(
        self,
        plan_id: str,
        reason: str,
        api_key: Optional[str] = None,
    ) -> Optional[ResearchPlan]:
        """Revise an existing plan, incorporating completed steps and failures.

        Args:
            plan_id: Plan to revise
            reason: Why the plan needs revision (e.g. "experiment failed", "hypothesis contradicted")
            api_key: LLM API key

        Returns:
            New revised ResearchPlan
        """
        old_plan = self.get_plan(plan_id)
        if not old_plan:
            return None

        # Build context from completed/failed steps
        completed_context = []
        for s in old_plan.steps:
            if s.status == StepStatus.COMPLETED:
                result_str = f" → {s.result}" if s.result else ""
                completed_context.append(
                    f"- [{s.type.value}] {s.description}: COMPLETED{result_str}"
                )
            elif s.status == StepStatus.FAILED:
                notes_str = f" ({s.notes})" if s.notes else ""
                completed_context.append(f"- [{s.type.value}] {s.description}: FAILED{notes_str}")

        completed_text = (
            "\n".join(completed_context) if completed_context else "No steps completed yet."
        )

        # Determine next steps to replan
        ready = old_plan.get_ready_steps()
        pending = [s for s in old_plan.steps if s.status == StepStatus.PENDING]

        prompt = f"""You are a research strategy planner. A research plan needs revision.

ORIGINAL HYPOTHESIS: {old_plan.hypothesis}
GOAL: {old_plan.goal}
REVISION REASON: {reason}

COMPLETED STEPS:
{completed_text}

NEXT STEPS THAT WERE BLOCKED (dependencies not met):
{chr(10).join(f"- [{s.type.value}] {s.description}" for s in ready)}

PENDING STEPS:
{chr(10).join(f"- [{s.type.value}] {s.description} (depends on: {', '.join(s.dependencies)})" for s in pending)}

Create a revised plan that:
1. Keeps all completed steps as-is
2. Adjusts blocked/pending steps based on the revision reason
3. May add new steps to address the failure/contradiction
4. Maintains dependency integrity

Respond ONLY with valid JSON:
{{
  "steps": [
    {{
      "type": "<step_type>",
      "description": "<specific description>",
      "estimated_hours": <float>,
      "dependencies": [<step_id of completed steps this depends on>],
      "is_new": <true if this is a new step not in the original plan>
    }}
  ]
}}"""

        try:
            from llm.chat import call_llm_chat_completions
        except ImportError:
            from llm.client import call_llm_chat_completions

        import os

        try:
            response = call_llm_chat_completions(
                base_url=os.getenv("OPENAI_BASE_URL", "") or LLM_BASE_URL,
                api_key=api_key or os.getenv("OPENAI_API_KEY", ""),
                model=os.getenv("LLM_MODEL", "") or LLM_MODEL,
                system_prompt="You are a research strategy expert. Produce valid JSON only.",
                user_prompt=prompt,
            )
            parsed = json.loads(response.strip())
        except Exception:
            # Keep pending steps as-is
            parsed = {"steps": []}

        # Mark old plan as revised
        old_plan.status = PlanStatus.REVISED
        self._save_plan(old_plan)

        # Build new plan, preserving completed steps
        new_plan_id = str(uuid.uuid4())[:8]
        new_steps = []

        # Keep completed steps
        for s in old_plan.steps:
            if s.status == StepStatus.COMPLETED:
                new_steps.append(s)

        # Map old step IDs to new ones for dependency updates
        old_to_new = {s.step_id: s.step_id for s in new_steps}

        # Add new/revised steps
        for i, s_data in enumerate(parsed.get("steps", [])):
            new_step_id = f"step_{len(new_steps) + i + 1}"
            deps = []
            for dep in s_data.get("dependencies", []):
                if dep in old_to_new:
                    deps.append(dep)
                elif dep.startswith("step_") and dep not in old_to_new:
                    pass  # drop dependency on steps that don't exist

            new_steps.append(
                PlanStep(
                    step_id=new_step_id,
                    type=StepType(s_data.get("type", "read_paper")),
                    description=s_data.get("description", ""),
                    estimated_hours=float(s_data.get("estimated_hours", 1.0)),
                    dependencies=deps,
                )
            )

        new_plan = ResearchPlan(
            plan_id=new_plan_id,
            hypothesis=old_plan.hypothesis,
            goal=old_plan.goal,
            steps=new_steps,
            revision_count=old_plan.revision_count + 1,
            parent_plan_id=plan_id,
        )

        self._save_plan(new_plan)
        return new_plan

    # ── Plan CRUD ────────────────────────────────────────────────────

    def _save_plan(self, plan: ResearchPlan) -> None:
        index = _get_plans_index()
        index[plan.plan_id] = f"plan_{plan.plan_id}.json"
        _save_index(index)
        path = _get_plan_path(plan.plan_id)
        path.write_text(json.dumps(plan.to_dict(), indent=2, ensure_ascii=False), encoding="utf-8")

    def get_plan(self, plan_id: str) -> Optional[ResearchPlan]:
        path = _get_plan_path(plan_id)
        if not path.exists():
            return None
        try:
            d = json.loads(path.read_text(encoding="utf-8"))
            return ResearchPlan.from_dict(d)
        except Exception:
            return None

    def update_step(
        self,
        plan_id: str,
        step_id: str,
        status: StepStatus,
        result: str = "",
        notes: str = "",
    ) -> Optional[ResearchPlan]:
        """Update step status. Triggers re-evaluation of blocked steps."""
        plan = self.get_plan(plan_id)
        if not plan:
            return None

        step = plan.get_step(step_id)
        if not step:
            return None

        step.status = status
        if status == StepStatus.COMPLETED:
            step.completed_at = time.time()
        step.result = result
        step.notes = notes
        plan.updated_at = time.time()

        # Release blocked steps whose dependencies are now met
        completed_ids = {s.step_id for s in plan.steps if s.status == StepStatus.COMPLETED}
        for s in plan.steps:
            if s.status == StepStatus.BLOCKED:
                if all(dep in completed_ids for dep in s.dependencies):
                    s.status = StepStatus.PENDING

        # Check if plan is complete
        pending = sum(1 for s in plan.steps if s.status == StepStatus.PENDING)
        if pending == 0:
            plan.status = PlanStatus.COMPLETED
            plan.completed_at = time.time()

        self._save_plan(plan)
        return plan

    def list_plans(
        self, status: Optional[PlanStatus] = None, limit: int = 20
    ) -> List[ResearchPlan]:
        """List all plans, optionally filtered by status."""
        index = _get_plans_index()
        plans = []
        for plan_id in list(index.keys())[:limit]:
            plan = self.get_plan(plan_id)
            if plan and (status is None or plan.status == status):
                plans.append(plan)
        return sorted(plans, key=lambda p: p.updated_at, reverse=True)

    def delete_plan(self, plan_id: str) -> bool:
        """Delete a plan."""
        index = _get_plans_index()
        if plan_id not in index:
            return False
        path = _get_plan_path(plan_id)
        if path.exists():
            path.unlink()
        del index[plan_id]
        _save_index(index)
        return True
