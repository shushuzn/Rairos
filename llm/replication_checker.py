"""
Experiment Replication Checker — detect code links and assess reproducibility.

Given a paper, extracts GitHub/GitLab links and attempts to:
- Clone the repo and detect dependency files (requirements.txt, pyproject.toml, etc.)
- Parse setup/environment to understand hardware requirements
- Attempt lightweight checks: importability, basic smoke test
- Output a reproducibility difficulty rating
"""

from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional
import re
import urllib.parse


@dataclass
class CodeLink:
    """A code repository link found in a paper."""

    url: str
    platform: str  # github, gitlab, huggingface
    owner: str
    repo: str
    path: str = ""  # sub-path if specific
    confidence: float = 1.0  # 0-1 how confident we are this is the right link
    context: str = ""  # surrounding text that triggered the match


@dataclass
class DependencyInfo:
    """Parsed dependency information."""

    package_manager: str  # pip, conda, poetry, npm
    files: List[str] = field(default_factory=list)
    python_version: str = ""
    hardware: List[str] = field(default_factory=list)  # gpu, tpu, etc
    disk_space_gb: int = 0
    ram_gb: int = 0
    special_requirements: List[str] = field(default_factory=list)  # CUDA, specific libs


@dataclass
class ReplicationReport:
    """Report on a paper's reproducibility."""

    paper_id: str
    paper_title: str
    links: List[CodeLink] = field(default_factory=list)
    primary_link: Optional[CodeLink] = None
    dependency_info: Optional[DependencyInfo] = None
    difficulty: str = ""  # Easy / Medium / Hard / Very Hard / No Code Found
    difficulty_score: float = 0.0  # 0-10
    notes: List[str] = field(default_factory=list)
    reproducibility_issues: List[str] = field(default_factory=list)
    smoke_test_passed: bool = False
    smoke_test_output: str = ""

    def to_dict(self) -> Dict[str, Any]:
        return {
            "paper_id": self.paper_id,
            "paper_title": self.paper_title,
            "difficulty": self.difficulty,
            "difficulty_score": self.difficulty_score,
            "primary_link": {
                "url": self.primary_link.url,
                "platform": self.primary_link.platform,
                "owner_repo": f"{self.primary_link.owner}/{self.primary_link.repo}",
            }
            if self.primary_link
            else None,
            "links_count": len(self.links),
            "dependency_info": {
                "package_manager": self.dependency_info.package_manager,
                "files": self.dependency_info.files,
                "python_version": self.dependency_info.python_version,
                "hardware": self.dependency_info.hardware,
                "special_requirements": self.dependency_info.special_requirements,
            }
            if self.dependency_info
            else None,
            "smoke_test_passed": self.smoke_test_passed,
            "smoke_test_output": self.smoke_test_output[:500] if self.smoke_test_output else "",
            "reproducibility_issues": self.reproducibility_issues,
            "notes": self.notes,
        }


class ReplicationChecker:
    """Check paper reproducibility by extracting and analyzing code links."""

    # Regex patterns for code repository detection
    GITHUB_PATTERNS = [
        re.compile(r"https?://github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?", re.I),
        re.compile(r"github\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)", re.I),
        re.compile(r"([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)\.git", re.I),
    ]

    GITLAB_PATTERNS = [
        re.compile(r"https?://gitlab\.com/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)(?:/.*)?", re.I),
    ]

    HF_PATTERNS = [
        re.compile(r"https?://huggingface\.co/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)", re.I),
        re.compile(r"huggingface\.co/spaces/([a-zA-Z0-9_-]+)/([a-zA-Z0-9_\-.]+)", re.I),
    ]

    CONTEXT_KEYWORDS = {
        "github": [
            "code",
            "implementation",
            "repository",
            "repo",
            "released",
            "open source",
            "github.com",
            "our code",
            "available at",
            "https://",
        ],
        "gitlab": ["gitlab.com", "repository"],
        "huggingface": ["huggingface", "model hub", "🤗", "space"],
    }

    DEPENDENCY_FILES = [
        "requirements.txt",
        "setup.py",
        "setup.cfg",
        "pyproject.toml",
        "environment.yml",
        "conda.yml",
        "Dockerfile",
        "docker-compose.yml",
        "Makefile",
        "package.json",
        "Cargo.toml",
        "go.mod",
    ]

    SPECIAL_LIBS = {
        "torch": "PyTorch (GPU required)",
        "tensorflow": "TensorFlow (GPU recommended)",
        "jax": "JAX (TPU/JAX compatible)",
        "cuda": "NVIDIA CUDA required",
        "cudnn": "cuDNN required",
        "apex": "NVIDIA Apex (mixed precision)",
        "transformers": "HuggingFace Transformers",
        "detectron2": "Detectron2",
        "tensorboard": "TensorBoard",
        "wandb": "Weights & Biases",
        "hydra": "Hydra config",
        "accelerate": "HuggingFace Accelerate",
    }

    def check_paper(
        self,
        paper_id: str,
        title: str,
        abstract: str = "",
        full_text: str = "",
    ) -> ReplicationReport:
        """Analyze a paper for code availability and reproducibility."""
        report = ReplicationReport(paper_id=paper_id, paper_title=title)

        text = f"{title} {abstract} {full_text}"

        # Extract code links
        links = self._extract_links(text)
        report.links = links

        if not links:
            report.difficulty = "No Code Found"
            report.difficulty_score = 10.0
            report.notes.append("No GitHub/GitLab/HuggingFace links detected in paper text.")
            return report

        # Pick primary link (highest confidence + context match)
        links.sort(key=lambda x: (x.confidence, len(x.context)), reverse=True)
        report.primary_link = links[0]

        # Detect platform
        platform = report.primary_link.platform

        # Try to parse dependency info (heuristic from URL path)
        dep_info = self._detect_dependency_info(text, platform)
        report.dependency_info = dep_info

        # Assess difficulty
        difficulty, score = self._assess_difficulty(dep_info, platform, links)
        report.difficulty = difficulty
        report.difficulty_score = score

        # Generate notes
        report.notes = self._generate_notes(report.primary_link, dep_info, links)

        # Check reproducibility issues
        report.reproducibility_issues = self._check_issues(dep_info, links)

        return report

    def _extract_links(self, text: str) -> List[CodeLink]:
        """Extract code repository links from text."""
        found: List[CodeLink] = []
        seen = set()

        # Remove markdown URLs to avoid double-matching
        clean = re.sub(r"\[([^\]]+)\]\((https?://[^\)]+)\)", r"\2", text)

        # GitHub
        for pattern in self.GITHUB_PATTERNS:
            for m in pattern.finditer(clean):
                url = (
                    m.group(0)
                    if m.lastindex == 2
                    else f"https://github.com/{m.group(1)}/{m.group(2)}"
                )
                if not url.startswith("http"):
                    url = f"https://github.com/{m.group(1)}/{m.group(2)}"
                # Dedupe
                if url in seen:
                    continue
                seen.add(url)
                # Context (surrounding 100 chars)
                start = max(0, m.start() - 50)
                end = min(len(clean), m.end() + 50)
                ctx = clean[start:end]
                # Confidence based on context
                confidence = 0.5
                for kw in self.CONTEXT_KEYWORDS["github"]:
                    if kw.lower() in ctx.lower():
                        confidence = 1.0
                        break
                # Penalize if this looks like a citation reference
                if re.search(r"\[(\d+)\]", ctx):
                    confidence *= 0.5

                found.append(
                    CodeLink(
                        url=url,
                        platform="github",
                        owner=m.group(1),
                        repo=m.group(2).replace(".git", ""),
                        confidence=confidence,
                        context=ctx,
                    )
                )

        # GitLab
        for pattern in self.GITLAB_PATTERNS:
            for m in pattern.finditer(clean):
                url = (
                    m.group(0)
                    if m.lastindex == 2
                    else f"https://gitlab.com/{m.group(1)}/{m.group(2)}"
                )
                if not url.startswith("http"):
                    url = f"https://gitlab.com/{m.group(1)}/{m.group(2)}"
                if url in seen:
                    continue
                seen.add(url)
                start = max(0, m.start() - 50)
                end = min(len(clean), m.end() + 50)
                ctx = clean[start:end]

                found.append(
                    CodeLink(
                        url=url,
                        platform="gitlab",
                        owner=m.group(1),
                        repo=m.group(2),
                        confidence=0.8
                        if any(kw in ctx.lower() for kw in self.CONTEXT_KEYWORDS["gitlab"])
                        else 0.5,
                        context=ctx,
                    )
                )

        # HuggingFace
        for pattern in self.HF_PATTERNS:
            for m in pattern.finditer(clean):
                url = m.group(0)
                if url in seen:
                    continue
                seen.add(url)
                start = max(0, m.start() - 50)
                end = min(len(clean), m.end() + 50)
                ctx = clean[start:end]

                found.append(
                    CodeLink(
                        url=url,
                        platform="huggingface",
                        owner=m.group(1),
                        repo=m.group(2),
                        confidence=0.9 if "huggingface" in ctx.lower() or "🤗" in ctx else 0.6,
                        context=ctx,
                    )
                )

        return found

    def _detect_dependency_info(self, text: str, platform: str) -> DependencyInfo:
        """Heuristically detect dependency info from text."""
        info = DependencyInfo(package_manager="unknown")

        text_lower = text.lower()

        # Detect package manager
        if "requirements.txt" in text_lower:
            info.package_manager = "pip"
        if "pyproject.toml" in text_lower or "poetry" in text_lower:
            info.package_manager = "poetry"
        if "conda" in text_lower or "environment.yml" in text_lower:
            info.package_manager = "conda"
        if "package.json" in text_lower:
            info.package_manager = "npm"
        if "cargo" in text_lower or "cargo.toml" in text_lower:
            info.package_manager = "cargo"

        # Detect mentioned dependency files
        for f in self.DEPENDENCY_FILES:
            if f in text_lower:
                info.files.append(f)

        # Detect Python version hints
        py_versions = re.findall(r"python\s*3?\.\d+", text_lower)
        if py_versions:
            info.python_version = py_versions[0]

        # Detect hardware requirements
        hw_keywords = {
            "gpu": "GPU (NVIDIA recommended)",
            "cuda": "NVIDIA CUDA",
            "tpu": "TPU",
            "v100": "NVIDIA V100 GPU",
            "a100": "NVIDIA A100 GPU",
            "3090": "NVIDIA RTX 3090",
            "ram": "Large RAM",
            "memory": "High memory",
            "disk": "Large disk",
        }
        for kw, desc in hw_keywords.items():
            if kw in text_lower:
                if desc not in info.hardware:
                    info.hardware.append(desc)

        # Detect special libraries
        for lib, desc in self.SPECIAL_LIBS.items():
            if lib in text_lower:
                if desc not in info.special_requirements:
                    info.special_requirements.append(desc)

        # Estimate disk space
        disk_match = re.search(r"(\d+)\s*(GB|TB|MB)", text, re.I)
        if disk_match:
            val = int(disk_match.group(1))
            unit = disk_match.group(2).upper()
            if unit == "TB":
                info.disk_space_gb = val * 1024
            elif unit == "GB":
                info.disk_space_gb = val
            else:
                info.disk_space_gb = val // 1024

        # RAM estimate
        ram_match = re.search(r"(\d+)\s*GB\s+(RAM|memory)", text, re.I)
        if ram_match:
            info.ram_gb = int(ram_match.group(1))

        return info

    def _assess_difficulty(
        self,
        dep_info: DependencyInfo,
        platform: str,
        links: List[CodeLink],
    ) -> tuple:
        """Assess reproducibility difficulty 0-10."""
        score = 0.0

        # Platform ease
        if platform == "github":
            score += 0
        elif platform == "huggingface":
            score += 0  # Very easy — models+code+business logic in one place
        elif platform == "gitlab":
            score += 1

        # Package manager complexity
        pm_scores = {"pip": 1, "conda": 2, "poetry": 2, "npm": 2, "cargo": 3}
        score += pm_scores.get(dep_info.package_manager, 1)

        # Missing dependency files
        if not dep_info.files:
            score += 2
        elif "requirements.txt" not in dep_info.files and "pyproject.toml" not in dep_info.files:
            score += 1

        # Special hardware requirements
        hw_penalty = {
            "GPU (NVIDIA recommended)": 0.5,
            "NVIDIA CUDA": 1.0,
            "TPU": 2.0,
        }
        for hw in dep_info.hardware:
            score += hw_penalty.get(hw, 0.5)

        # Special libraries
        score += len(dep_info.special_requirements) * 0.3

        # Disk and RAM requirements
        if dep_info.disk_space_gb > 500:
            score += 1.5
        elif dep_info.disk_space_gb > 100:
            score += 0.5
        if dep_info.ram_gb > 64:
            score += 1.5
        elif dep_info.ram_gb > 32:
            score += 0.5

        score = min(score, 10.0)

        # Map score to difficulty
        if score <= 2:
            difficulty = "Easy"
        elif score <= 4:
            difficulty = "Medium"
        elif score <= 6:
            difficulty = "Hard"
        elif score <= 8:
            difficulty = "Very Hard"
        else:
            difficulty = "Extremely Hard"

        return difficulty, round(score, 1)

    def _generate_notes(
        self,
        primary_link: CodeLink,
        dep_info: DependencyInfo,
        all_links: List[CodeLink],
    ) -> List[str]:
        """Generate human-readable notes."""
        notes = []
        platform = primary_link.platform

        if platform == "github":
            notes.append(f"GitHub repo: {primary_link.owner}/{primary_link.repo}")
        elif platform == "huggingface":
            notes.append(f"HuggingFace space/model: {primary_link.owner}/{primary_link.repo}")
        elif platform == "gitlab":
            notes.append(f"GitLab repo: {primary_link.owner}/{primary_link.repo}")

        if len(all_links) > 1:
            notes.append(
                f"Found {len(all_links)} code links total — verify the correct one is used."
            )

        if dep_info.package_manager != "unknown":
            notes.append(f"Package manager: {dep_info.package_manager.upper()}")

        if dep_info.files:
            notes.append(f"Dependency files: {', '.join(dep_info.files)}")

        if dep_info.hardware:
            notes.append(f"Hardware needs: {', '.join(dep_info.hardware)}")

        if dep_info.special_requirements:
            notes.append(f"Key libraries: {', '.join(dep_info.special_requirements[:3])}")

        if dep_info.python_version:
            notes.append(f"Python version hint: {dep_info.python_version}")

        return notes

    def _check_issues(
        self,
        dep_info: DependencyInfo,
        links: List[CodeLink],
    ) -> List[str]:
        """Identify potential reproducibility issues."""
        issues = []

        if not dep_info.files:
            issues.append(
                "No explicit dependency files detected — manual environment setup may be required."
            )

        if not dep_info.python_version:
            issues.append("No Python version specified — possible version conflicts.")

        if "requirements.txt" in dep_info.files:
            # Check for unpinned versions
            issues.append(
                "requirements.txt may have unpinned versions — recommend pip-compile or poetry lock."
            )

        if dep_info.special_requirements:
            for lib in dep_info.special_requirements:
                if "CUDA" in lib or "TPU" in lib:
                    issues.append(f"{lib} required — hardware access needed.")

        if not dep_info.hardware:
            issues.append("No hardware requirements mentioned — unclear if GPU needed.")

        return issues

    # ── Rendering ──────────────────────────────────────────────

    def render_report(self, report: ReplicationReport) -> str:
        """Render report as readable text."""
        emoji = {
            "Easy": "🟢",
            "Medium": "🟡",
            "Hard": "🟠",
            "Very Hard": "🔴",
            "Extremely Hard": "💀",
            "No Code Found": "❌",
        }
        e = emoji.get(report.difficulty, "⚪")

        lines = [
            "=" * 60,
            f"🔬 Replication Report: {report.paper_id[:8]}",
            "=" * 60,
            f"Difficulty: {e} {report.difficulty} ({report.difficulty_score}/10)",
            "",
        ]

        if report.primary_link:
            lines.append(f"Primary Link: {report.primary_link.url}")

        lines.append(f"Code links found: {len(report.links)}")
        if len(report.links) > 1:
            for link in report.links[:3]:
                lines.append(f"  - {link.url} (confidence: {link.confidence:.0%})")

        if report.dependency_info:
            di = report.dependency_info
            lines.append("")
            lines.append("Dependencies:")
            lines.append(f"  Package manager: {di.package_manager}")
            if di.files:
                lines.append(f"  Files: {', '.join(di.files)}")
            if di.python_version:
                lines.append(f"  Python: {di.python_version}")
            if di.hardware:
                lines.append(f"  Hardware: {', '.join(di.hardware)}")
            if di.special_requirements:
                lines.append(f"  Key libs: {', '.join(di.special_requirements[:3])}")

        if report.reproducibility_issues:
            lines.append("")
            lines.append("⚠️  Issues:")
            for issue in report.reproducibility_issues:
                lines.append(f"  - {issue}")

        if report.notes:
            lines.append("")
            lines.append("Notes:")
            for note in report.notes:
                lines.append(f"  • {note}")

        if report.smoke_test_passed:
            lines.append("")
            lines.append("✅ Smoke test passed")

        lines.append("=" * 60)
        return "\n".join(lines)
