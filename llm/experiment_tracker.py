"""
Experiment Tracker: Track experiments for research roadmaps.
"""
import json, uuid
from dataclasses import dataclass, asdict, field
from datetime import datetime
from enum import Enum
from pathlib import Path
from typing import List, Optional, Dict, Any

class ExperimentStatus(Enum):
    RUNNING = "running"
    COMPLETED = "completed"
    FAILED = "failed"
    CANCELLED = "cancelled"

@dataclass
class Metric:
    name: str
    value: float
    unit: str = ""
    timestamp: str = ""

@dataclass
class Experiment:
    id: str
    name: str
    description: str = ""
    roadmap_milestone: str = ""
    hypothesis_id: str = ""  # Links experiment back to the hypothesis it validated
    config: Dict = field(default_factory=dict)
    results: Dict = field(default_factory=dict)
    metrics: List = field(default_factory=list)
    status: str = "running"
    created_at: str = ""
    completed_at: str = ""
    artifacts: List = field(default_factory=list)
    tags: List = field(default_factory=list)

    def __post_init__(self):
        if not self.created_at:
            self.created_at = datetime.now().isoformat()

    def to_dict(self): return asdict(self)

    @classmethod
    def from_dict(cls, data):
        data = data.copy()
        if 'metrics' in data:
            data['metrics'] = [Metric(**m) if isinstance(m, dict) else m for m in data['metrics']]
        return cls(**data)

class ExperimentTracker:
    def __init__(self, data_dir=None):
        p = Path(data_dir or Path.home() / ".ai_research_os" / "experiments")
        p.mkdir(parents=True, exist_ok=True)
        self.f = p / "experiments.json"

    def _load(self):
        if not self.f.exists(): return []
        try:
            with open(self.f) as f:
                return [Experiment.from_dict(e) for e in json.load(f)]
        except: return []

    def _save(self, exps):
        with open(self.f, 'w') as f:
            json.dump([e.to_dict() for e in exps], f, ensure_ascii=False, indent=2)

    def run(self, name, description="", roadmap_milestone="", hypothesis_id="", config=None, tags=None):
        exps = self._load()
        e = Experiment(id=str(uuid.uuid4())[:8], name=name, description=description,
                      roadmap_milestone=roadmap_milestone, hypothesis_id=hypothesis_id,
                      config=config or {}, tags=tags or [])
        exps.append(e)
        self._save(exps)
        return e

    def get(self, eid):
        for e in self._load():
            if e.id == eid: return e

    def list_experiments(self, status=None, milestone=None, tag=None):
        exps = self._load()
        if status: exps = [e for e in exps if e.status == status]
        if milestone: exps = [e for e in exps if e.roadmap_milestone == milestone]
        if tag: exps = [e for e in exps if tag in e.tags]
        return sorted(exps, key=lambda x: -datetime.fromisoformat(x.created_at).timestamp())

    def complete(self, eid, results=None):
        exps = self._load()
        for e in exps:
            if e.id == eid:
                e.status = "completed"
                e.completed_at = datetime.now().isoformat()
                if results: e.results = results
                self._save(exps)

                # Record VALIDATED event so gap sorting learns from experiment outcomes
                if e.hypothesis_id:
                    try:
                        from llm.insight_evolution import EvolutionTracker, ExplorationAction
                        ev = EvolutionTracker()
                        ev.record_event(
                            topic="; ".join(e.tags) if e.tags else e.name,
                            action=ExplorationAction.VALIDATED,
                            hypothesis_id=e.hypothesis_id,
                            gap_type=e.config.get("hypothesis_type", ""),
                        )
                    except Exception:
                        pass  # Non-fatal — experiment tracker works without evolution

                return e

    def fail(self, eid, error=""):
        exps = self._load()
        for e in exps:
            if e.id == eid:
                e.status = "failed"
                e.completed_at = datetime.now().isoformat()
                if error: e.results["error"] = error
                self._save(exps)

                # Record REJECTED event so gap sorting learns from failed experiments
                if e.hypothesis_id:
                    try:
                        from llm.insight_evolution import EvolutionTracker, ExplorationAction
                        ev = EvolutionTracker()
                        ev.record_event(
                            topic="; ".join(e.tags) if e.tags else e.name,
                            action=ExplorationAction.REJECTED,
                            hypothesis_id=e.hypothesis_id,
                            gap_type=e.config.get("hypothesis_type", ""),
                        )
                    except Exception:
                        pass  # Non-fatal — experiment tracker works without evolution

                return e

    def add_metric(self, eid, name, value, unit=""):
        exps = self._load()
        for e in exps:
            if e.id == eid:
                e.metrics.append(Metric(name=name, value=value, unit=unit))
                self._save(exps)
                return e

    def compare(self, exp_ids, metric_names=None):
        exps = [self.get(eid) for eid in exp_ids]
        exps = [e for e in exps if e]
        if not exps: return {"error": "No experiments found"}
        if not metric_names:
            metric_names = list(set(m.name for e in exps for m in e.metrics))
        rows = []
        for e in exps:
            row = {"id": e.id, "name": e.name, "status": e.status}
            for mn in metric_names:
                for m in e.metrics:
                    if m.name == mn: row[mn] = m.value; break
            rows.append(row)
        return {"metrics": metric_names, "experiments": rows}

    def delete(self, eid):
        exps = self._load()
        n = len(exps)
        exps = [e for e in exps if e.id != eid]
        if len(exps) < n: self._save(exps); return True
        return False

    def render_list(self, exps, verbose=False):
        if not exps: return "No experiments found."
        lines, icons = [], {"running":"⚡","completed":"✓","failed":"✗"}
        for e in exps:
            lines.append(f"{icons.get(e.status,'?')} [{e.id}] {e.name} ({e.status})")
            if e.roadmap_milestone: lines.append(f"  Milestone: {e.roadmap_milestone}")
            if verbose and e.metrics:
                lines.append(f"  Metrics: " + ", ".join(f"{m.name}={m.value}" for m in e.metrics))
        return chr(10).join(lines)

    def render_compare(self, comp):
        if "error" in comp: return f"Error: {comp['error']}"
        lines = ["## Experiment Comparison", "", "| Exp | " + " | ".join(comp["metrics"]) + " |", "|---| " + "|---".join([""]*len(comp["metrics"]))]
        for r in comp["experiments"]:
            vals = [str(r.get(m, "-")) for m in comp["metrics"]]
            lines.append(f"| {r['name'][:15]} | " + " | ".join(vals) + " |")
        return chr(10).join(lines)
