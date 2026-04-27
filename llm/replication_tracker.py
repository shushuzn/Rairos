"""
Replication Tracker: Track paper replication attempts.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from datetime import datetime
from pathlib import Path

from llm.tracker_base import JsonFileStore


@dataclass
class ReplicationAttempt:
    """A replication attempt."""
    attempt_id: str
    paper_id: str
    paper_title: str
    status: str = "in_progress"  # in_progress, success, failed, partial
    attempt_date: str = ""
    environment: Dict[str, str] = field(default_factory=dict)
    results: Dict[str, Any] = field(default_factory=dict)
    differences: List[str] = field(default_factory=list)
    notes: str = ""
    config: Dict[str, Any] = field(default_factory=dict)


@dataclass
class ReplicationReport:
    """A replication report."""
    attempt: ReplicationAttempt
    summary: str = ""
    methodology: str = ""
    findings: List[str] = field(default_factory=list)
    recommendations: List[str] = field(default_factory=list)


class ReplicationTracker(JsonFileStore):
    """Track paper replication attempts."""

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or Path.home() / ".ai_research_os"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.data_file = self.data_dir / "replications.json"

    def _post_load(self, raw: List[dict]) -> List[dict]:
        return raw

    def _pre_save(self, data: List[dict]) -> List[dict]:
        return data

    def create_attempt(
        self,
        paper_id: str,
        paper_title: str,
        config: Optional[Dict] = None,
    ) -> ReplicationAttempt:
        """Create a new replication attempt."""
        data = self._load()

        attempt_id = f"{paper_id}_{len(data) + 1}"
        attempt = ReplicationAttempt(
            attempt_id=attempt_id,
            paper_id=paper_id,
            paper_title=paper_title,
            attempt_date=datetime.now().isoformat()[:10],
            config=config or {},
        )

        data.append({
            "attempt_id": attempt.attempt_id,
            "paper_id": attempt.paper_id,
            "paper_title": attempt.paper_title,
            "status": attempt.status,
            "attempt_date": attempt.attempt_date,
            "environment": attempt.environment,
            "results": attempt.results,
            "differences": attempt.differences,
            "notes": attempt.notes,
            "config": attempt.config,
        })

        self._save(data)
        return attempt

    def update_attempt(
        self,
        attempt_id: str,
        status: Optional[str] = None,
        results: Optional[Dict] = None,
        differences: Optional[List[str]] = None,
        notes: Optional[str] = None,
        environment: Optional[Dict] = None,
    ) -> bool:
        """Update a replication attempt."""
        data = self._load()

        for item in data:
            if item["attempt_id"] == attempt_id:
                if status is not None:
                    item["status"] = status
                if results is not None:
                    item["results"].update(results)
                if differences is not None:
                    item["differences"].extend(differences)
                if notes is not None:
                    item["notes"] = notes
                if environment is not None:
                    item["environment"].update(environment)
                self._save(data)
                return True
        return False

    def get_attempt(self, attempt_id: str) -> Optional[ReplicationAttempt]:
        """Get a replication attempt by ID."""
        data = self._load()

        for item in data:
            if item["attempt_id"] == attempt_id:
                return ReplicationAttempt(**item)
        return None

    def get_paper_attempts(self, paper_id: str) -> List[ReplicationAttempt]:
        """Get all attempts for a paper."""
        data = self._load()
        attempts = []

        for item in data:
            if item["paper_id"] == paper_id:
                attempts.append(ReplicationAttempt(**item))

        return attempts

    def get_all_attempts(self, status: Optional[str] = None) -> List[ReplicationAttempt]:
        """Get all replication attempts."""
        data = self._load()
        attempts = []

        for item in data:
            if status is None or item["status"] == status:
                attempts.append(ReplicationAttempt(**item))

        # Sort by date descending
        attempts.sort(key=lambda x: x.attempt_date, reverse=True)
        return attempts

    def get_statistics(self) -> Dict[str, Any]:
        """Get replication statistics."""
        data = self._load()

        total = len(data)
        success = sum(1 for d in data if d["status"] == "success")
        failed = sum(1 for d in data if d["status"] == "failed")
        partial = sum(1 for d in data if d["status"] == "partial")
        in_progress = sum(1 for d in data if d["status"] == "in_progress")

        return {
            "total": total,
            "success": success,
            "failed": failed,
            "partial": partial,
            "in_progress": in_progress,
            "success_rate": (success / total * 100) if total > 0 else 0,
        }

    def generate_report(self, attempt_id: str) -> Optional[ReplicationReport]:
        """Generate a replication report."""
        attempt = self.get_attempt(attempt_id)
        if not attempt:
            return None

        report = ReplicationReport(attempt=attempt)

        # Generate summary based on status
        if attempt.status == "success":
            report.summary = f"Successfully replicated '{attempt.paper_title}'"
        elif attempt.status == "failed":
            report.summary = f"Failed to replicate '{attempt.paper_title}'"
            if attempt.differences:
                report.findings.append(
                    f"Found {len(attempt.differences)} key differences"
                )
        elif attempt.status == "partial":
            report.summary = f"Partially replicated '{attempt.paper_title}'"
            report.findings.append("Some results matched, others diverged")

        # Add methodology
        report.methodology = self._format_methodology(attempt)

        # Add recommendations
        report.recommendations = self._generate_recommendations(attempt)

        return report

    def _format_methodology(self, attempt: ReplicationAttempt) -> str:
        """Format methodology from config and environment."""
        parts = ["## Methodology\n"]

        if attempt.config:
            parts.append("### Configuration")
            for k, v in attempt.config.items():
                parts.append(f"- {k}: {v}")

        if attempt.environment:
            parts.append("\n### Environment")
            for k, v in attempt.environment.items():
                parts.append(f"- {k}: {v}")

        return "\n".join(parts)

    def _generate_recommendations(self, attempt: ReplicationAttempt) -> List[str]:
        """Generate recommendations based on results."""
        recs = []

        if attempt.status == "failed":
            recs.append("Review original paper's experimental setup in detail")
            recs.append("Check for missing implementation details or hyperparameters")

            if not attempt.environment.get("gpu"):
                recs.append("Consider using GPU for resource-intensive experiments")

        if attempt.differences:
            recs.append("Document all deviations from original methodology")
            recs.append("Analyze which differences most affected results")

        if attempt.results:
            recs.append("Compare specific metrics quantitatively when possible")

        return recs

    def render_text(self, attempts: List[ReplicationAttempt]) -> str:
        """Render attempts as text list."""
        if not attempts:
            return "No replication attempts recorded."

        lines = ["=" * 70, "🔬 Replication Tracker", "=" * 70, ""]

        stats = self.get_statistics()
        lines.append(f"Total: {stats['total']} | Success: {stats['success']} | "
                    f"Failed: {stats['failed']} | Partial: {stats['partial']} | "
                    f"In Progress: {stats['in_progress']}")
        lines.append("")

        for attempt in attempts[:20]:
            status_icon = {
                "success": "✅",
                "failed": "❌",
                "partial": "⚠️",
                "in_progress": "⏳",
            }.get(attempt.status, "❓")

            lines.append(f"{status_icon} [{attempt.attempt_id}]")
            lines.append(f"   Paper: {attempt.paper_title[:50]}")
            lines.append(f"   Date: {attempt.attempt_date} | Status: {attempt.status}")

            if attempt.results:
                metrics = list(attempt.results.keys())[:3]
                lines.append(f"   Results: {', '.join(metrics)}")

            if attempt.differences:
                lines.append(f"   Differences: {len(attempt.differences)}")

            lines.append("")

        lines.append("=" * 70)
        return "\n".join(lines)

    def render_markdown(self, attempts: List[ReplicationAttempt]) -> str:
        """Render attempts as Markdown."""
        lines = ["# Replication Tracker\n"]

        stats = self.get_statistics()
        lines.append(f"| Metric | Value |")
        lines.append(f"|--------|-------|")
        lines.append(f"| Total | {stats['total']} |")
        lines.append(f"| Success | {stats['success']} |")
        lines.append(f"| Failed | {stats['failed']} |")
        lines.append(f"| Partial | {stats['partial']} |")
        lines.append(f"| Success Rate | {stats['success_rate']:.1f}% |")
        lines.append("")

        if attempts:
            lines.append("## Recent Attempts\n")
            lines.append("| Paper | Status | Date |")
            lines.append("|-------|--------|------|")

            for attempt in attempts[:10]:
                title = attempt.paper_title[:40] + "..." if len(attempt.paper_title) > 40 else attempt.paper_title
                lines.append(f"| {title} | {attempt.status} | {attempt.attempt_date} |")

        return "\n".join(lines)
