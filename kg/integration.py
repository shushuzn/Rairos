"""Integration hooks for KGManager with existing ai_research_os flow."""

import json
import logging
import re
from typing import Any, Optional, Union
from pathlib import Path



logger = logging.getLogger(__name__)

_HAS_YAML = False
try:
    import yaml
    _HAS_YAML = True
except ImportError:
    pass


def _parse_yaml_frontmatter(text: str) -> dict:
    if text.startswith("---"):
        parts = text.split("---", 2)
        if len(parts) >= 3:
            try:
                return yaml.safe_load(parts[1]) or {}
            except Exception:
                pass
    return {}


def _parse_frontmatter_regex(text: str) -> dict:
    """Fallback: parse YAML frontmatter with regex when yaml module unavailable."""
    result = {}
    if not text.startswith("---"):
        return result
    parts = text.split("---", 2)
    if len(parts) < 3:
        return result
    body = parts[1]
    # Match key: value pairs
    for line in body.splitlines():
        line = line.strip()
        if not line or line.startswith("#"):
            continue
        m = re.match(r"^(\w+):\s*(.*)$", line)
        if m:
            key, val = m.group(1), m.group(2).strip()
            # Strip quotes
            val = val.strip("\"'")
            result[key] = val
    return result


def _read_pnote_frontmatter(pnote_path: Path | None) -> dict:
    if pnote_path is None or not pnote_path.exists():
        return {}
    try:
        text = pnote_path.read_text(encoding="utf-8")
        if _HAS_YAML:
            return _parse_yaml_frontmatter(text)
        return _parse_frontmatter_regex(text)
    except Exception:
        return {}


class KGIntegration:
    """Hooks to integrate KGManager into ai_research_os paper processing.

    Usage:
        kg = KGManager()
        integ = KGIntegration(kg)
        integ.on_paper_processed(paper_uid, pnote_path, cnote_paths, mnote_path)
        integ.on_citations_fetched(paper_uid, cited_uids, citing_uids)
        integ.on_mnote_created(mnote_path, member_paper_uids)
        integ.rebuild_from_papers_json("data/papers.json")
    """

    def __init__(self, kg_manager):
        self.kg = kg_manager

    def on_paper_processed(
        self,
        paper_uid: str,
        pnote_path: str | Path,
        cnote_paths: list[str | Path] | None = None,
        mnote_path: Optional[Union[str, Path]] = None,
        paper_title: Optional[str] = None,
        paper_authors: list[str] | None = None,
        paper_tags: list[str] | None = None,
        paper_year: Optional[int] = None,
    ):
        """Create nodes for a processed paper and its notes."""
        pnote_path = Path(pnote_path) if pnote_path else None

        if paper_title is None or paper_tags is None:
            meta = _read_pnote_frontmatter(pnote_path)
            paper_title = paper_title or meta.get("title", paper_uid)
            raw_tags = meta.get("tags", [])
            if isinstance(raw_tags, str):
                raw_tags = [t.strip() for t in raw_tags.split(",")]
            paper_tags = paper_tags or raw_tags
            raw_authors = meta.get("authors", [])
            if isinstance(raw_authors, str):
                raw_authors = [a.strip() for a in raw_authors.split(",")]
            paper_authors = paper_authors or raw_authors
            paper_year = paper_year or meta.get("year")

        paper_node_id = self.kg.add_node(
            "Paper", paper_uid, paper_title,
            authors=paper_authors, year=paper_year,
        )

        if pnote_path and pnote_path.exists():
            pnote_node_id = self.kg.add_node(
                "P-Note", paper_uid, f"P-Note: {paper_title}",
                path=str(pnote_path),
            )
            self.kg.add_edge(paper_node_id, pnote_node_id, "has_note")

        for tag in (paper_tags or []):
            tag_clean = tag.strip()
            if not tag_clean:
                continue
            tag_node_id = self.kg.upsert_node("Tag", tag_clean, tag_clean)
            self.kg.add_edge(paper_node_id, tag_node_id, "same_tag", weight=1.0)

            cnote_path = self._find_cnote_path(pnote_path, tag_clean) if pnote_path else None
            cnote_node_id = self.kg.add_node(
                "C-Note", f"{paper_uid}_{tag_clean}", f"C-Note: {tag_clean}",
                path=str(cnote_path) if cnote_path else None,
            )
            self.kg.add_edge(paper_node_id, cnote_node_id, "has_note")
            self.kg.add_edge(cnote_node_id, tag_node_id, "about_tag")

        if mnote_path:
            self._on_mnote_file_created(Path(mnote_path), paper_uid)

    def _find_cnote_path(self, pnote_path: Path, tag: str) -> Path | None:
        if pnote_path is None:
            return None
        notes_dir = pnote_path.parent
        for candidate in [
            notes_dir / f"{tag}_cnote.md",
            notes_dir / f"{tag}-cnote.md",
            notes_dir / f"cnote_{tag}.md",
        ]:
            if candidate.exists():
                return candidate
        return None

    def on_citations_fetched(
        self,
        paper_uid: str,
        cited_uids: list[str] | None = None,
        citing_uids: list[str] | None = None,
    ):
        """Create cite edges when citation data is fetched."""
        paper_node = self.kg.get_node_by_entity("Paper", paper_uid)
        paper_node_id = paper_node["id"] if paper_node else self.kg.add_node("Paper", paper_uid, paper_uid)

        for cited_uid in (cited_uids or []):
            cited_node = self.kg.get_node_by_entity("Paper", cited_uid)
            cited_node_id = cited_node["id"] if cited_node else self.kg.add_node("Paper", cited_uid, cited_uid)
            self.kg.add_edge(paper_node_id, cited_node_id, "cite", weight=1.0)

        for citing_uid in (citing_uids or []):
            citing_node = self.kg.get_node_by_entity("Paper", citing_uid)
            citing_node_id = citing_node["id"] if citing_node else self.kg.add_node("Paper", citing_uid, citing_uid)
            self.kg.add_edge(citing_node_id, paper_node_id, "cite", weight=1.0)

    def on_mnote_created(
        self,
        mnote_path: str | Path,
        member_paper_uids: list[str] | None = None,
    ):
        """Create M-Note node and in_comparison edges for all member papers."""
        self._on_mnote_file_created(Path(mnote_path), None, member_paper_uids)

    def _on_mnote_file_created(
        self,
        mnote_path: Path,
        primary_paper_uid: str | None,
        member_paper_uids: list[str] | None = None,
    ):
        if not mnote_path.exists():
            logger.warning(f"M-Note file not found: {mnote_path}")
            return

        meta = _read_pnote_frontmatter(mnote_path)
        tag_list = meta.get("tag", meta.get("tags", []))
        if isinstance(tag_list, str):
            tag = tag_list.strip()
        elif isinstance(tag_list, list):
            tag = str(tag_list[0]) if tag_list else "unknown"
        else:
            tag = "unknown"

        mnote_label = meta.get("title", f"M-Note: {tag}")
        mnote_node_id = self.kg.add_node(
            "M-Note", str(mnote_path), mnote_label,
            path=str(mnote_path), tag=tag,
        )
        tag_node_id = self.kg.upsert_node("Tag", tag, tag)
        self.kg.add_edge(mnote_node_id, tag_node_id, "about_tag")

        uids = member_paper_uids or meta.get("papers", []) or meta.get("members", [])
        if isinstance(uids, list) and uids and isinstance(uids[0], dict):
            uids = [p.get("uid") or p.get("id") for p in uids]
        uids = [uid for uid in uids if uid]
        if not uids and primary_paper_uid:
            uids = [primary_paper_uid]

        for uid in uids:
            pnode = self.kg.get_node_by_entity("Paper", uid)
            pnode_id = pnode["id"] if pnode else self.kg.add_node("Paper", uid, uid)
            self.kg.add_edge(pnode_id, mnote_node_id, "in_comparison", weight=1.0)

    def rebuild_from_papers_json(self, papers_json_path: str | Path):
        """Rebuild entire KG from the main papers.json database."""
        papers_json_path = Path(papers_json_path)
        if not papers_json_path.exists():
            logger.error(f"papers.json not found at {papers_json_path}")
            return

        data = json.loads(papers_json_path.read_text(encoding="utf-8"))
        papers = data.get("papers", {})
        citation_graph = data.get("citation_graph", {})

        for uid, paper_data in papers.items():
            tags = paper_data.get("tags", [])
            title = paper_data.get("title", uid)
            authors = paper_data.get("authors", [])
            year = paper_data.get("year")
            base = papers_json_path.parent
            pnote_path = base / "notes" / uid / f"{uid}_pnote.md"
            self.on_paper_processed(uid, pnote_path,
                paper_title=title, paper_tags=tags,
                paper_authors=authors, paper_year=year)

        for uid, citations in citation_graph.items():
            cited = citations.get("cited", [])
            citing = citations.get("citing", [])
            self.on_citations_fetched(uid, cited, citing)

        logger.info(f"KG rebuild complete: {len(papers)} papers processed")
