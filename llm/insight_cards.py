"""
Key Insight Cards: Extract and manage research insights.
"""
from dataclasses import dataclass, field
from typing import List, Optional, Dict, Any
from datetime import datetime
from pathlib import Path
import json
import re


@dataclass
class InsightCard:
    """A key insight extracted from a paper."""
    card_id: str
    paper_id: str
    paper_title: str
    content: str
    insight_type: str = "finding"  # finding, method, limitation, future_work
    tags: List[str] = field(default_factory=list)
    evidence: str = ""
    page_ref: str = ""
    created_at: str = ""
    references: List[str] = field(default_factory=list)  # other card_ids


@dataclass
class InsightCollection:
    """A collection of insight cards around a topic."""
    collection_id: str
    title: str
    description: str = ""
    card_ids: List[str] = field(default_factory=list)
    tags: List[str] = field(default_factory=list)


class InsightManager:
    """Manage key insight cards."""

    def __init__(self, data_dir: Optional[Path] = None):
        self.data_dir = data_dir or Path.home() / ".ai_research_os"
        self.data_dir.mkdir(parents=True, exist_ok=True)
        self.cards_file = self.data_dir / "insight_cards.json"
        self.collections_file = self.data_dir / "insight_collections.json"

    def _load_cards(self) -> List[Dict]:
        if self.cards_file.exists():
            with open(self.cards_file, "r", encoding="utf-8") as f:
                return json.load(f)
        return []

    def _save_cards(self, data: List[Dict]) -> None:
        with open(self.cards_file, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def _load_collections(self) -> List[Dict]:
        if self.collections_file.exists():
            with open(self.collections_file, "r", encoding="utf-8") as f:
                return json.load(f)
        return []

    def _save_collections(self, data: List[Dict]) -> None:
        with open(self.collections_file, "w", encoding="utf-8") as f:
            json.dump(data, f, ensure_ascii=False, indent=2)

    def add_card(
        self,
        paper_id: str,
        paper_title: str,
        content: str,
        insight_type: str = "finding",
        tags: Optional[List[str]] = None,
        evidence: str = "",
        page_ref: str = "",
    ) -> InsightCard:
        """Add a new insight card."""
        data = self._load_cards()

        card_id = f"i{len(data) + 1:04d}"
        card = InsightCard(
            card_id=card_id,
            paper_id=paper_id,
            paper_title=paper_title,
            content=content,
            insight_type=insight_type,
            tags=tags or [],
            evidence=evidence,
            page_ref=page_ref,
            created_at=datetime.now().isoformat()[:10],
        )

        data.append({
            "card_id": card.card_id,
            "paper_id": card.paper_id,
            "paper_title": card.paper_title,
            "content": card.content,
            "insight_type": card.insight_type,
            "tags": card.tags,
            "evidence": card.evidence,
            "page_ref": card.page_ref,
            "created_at": card.created_at,
            "references": card.references,
        })

        self._save_cards(data)
        return card

    def get_card(self, card_id: str) -> Optional[InsightCard]:
        """Get a card by ID."""
        data = self._load_cards()
        for item in data:
            if item["card_id"] == card_id:
                return InsightCard(**item)
        return None

    def update_card(
        self,
        card_id: str,
        content: Optional[str] = None,
        tags: Optional[List[str]] = None,
        insight_type: Optional[str] = None,
    ) -> bool:
        """Update a card."""
        data = self._load_cards()
        for item in data:
            if item["card_id"] == card_id:
                if content is not None:
                    item["content"] = content
                if tags is not None:
                    item["tags"] = tags
                if insight_type is not None:
                    item["insight_type"] = insight_type
                self._save_cards(data)
                return True
        return False

    def add_reference(self, from_card_id: str, to_card_id: str) -> bool:
        """Add a reference from one card to another."""
        data = self._load_cards()
        for item in data:
            if item["card_id"] == from_card_id:
                if to_card_id not in item["references"]:
                    item["references"].append(to_card_id)
                    self._save_cards(data)
                return True
        return False

    def search_cards(
        self,
        query: Optional[str] = None,
        tags: Optional[List[str]] = None,
        insight_type: Optional[str] = None,
        paper_id: Optional[str] = None,
    ) -> List[InsightCard]:
        """Search cards by various criteria."""
        data = self._load_cards()
        results = []

        for item in data:
            # Filter by query
            if query:
                q = query.lower()
                if q not in item["content"].lower() and q not in item["paper_title"].lower():
                    continue

            # Filter by tags
            if tags:
                if not any(t in item["tags"] for t in tags):
                    continue

            # Filter by type
            if insight_type and item["insight_type"] != insight_type:
                continue

            # Filter by paper
            if paper_id and item["paper_id"] != paper_id:
                continue

            results.append(InsightCard(**item))

        # Sort by creation date
        results.sort(key=lambda x: x.created_at, reverse=True)
        return results

    def get_paper_cards(self, paper_id: str) -> List[InsightCard]:
        """Get all cards from a paper."""
        return self.search_cards(paper_id=paper_id)

    def get_tag_cloud(self) -> Dict[str, int]:
        """Get tag frequency."""
        data = self._load_cards()
        tags: Dict[str, int] = {}

        for item in data:
            for tag in item["tags"]:
                tags[tag] = tags.get(tag, 0) + 1

        return dict(sorted(tags.items(), key=lambda x: -x[1]))

    def create_collection(
        self,
        title: str,
        description: str = "",
        tags: Optional[List[str]] = None,
    ) -> InsightCollection:
        """Create a collection of cards."""
        collections = self._load_collections()

        collection_id = f"c{len(collections) + 1:04d}"
        collection = InsightCollection(
            collection_id=collection_id,
            title=title,
            description=description,
            tags=tags or [],
        )

        collections.append({
            "collection_id": collection.collection_id,
            "title": collection.title,
            "description": collection.description,
            "card_ids": collection.card_ids,
            "tags": collection.tags,
        })

        self._save_collections(collections)
        return collection

    def add_to_collection(self, collection_id: str, card_id: str) -> bool:
        """Add a card to a collection."""
        collections = self._load_collections()
        for item in collections:
            if item["collection_id"] == collection_id:
                if card_id not in item["card_ids"]:
                    item["card_ids"].append(card_id)
                    self._save_collections(collections)
                return True
        return False

    def extract_from_text(self, paper_id: str, paper_title: str, text: str) -> List[InsightCard]:
        """Extract potential insights from paper text using heuristics."""
        cards = []

        # Extract key claims (sentences with numbers and comparisons)
        patterns = [
            r"improved by\s+(\d+\.?\d*)%",
            r"achieved\s+(\d+\.?\d*)%",
            r"outperforms?\s+[\w\s]+by\s+(\d+\.?\d*)%",
            r"reduced\s+(\w+)\s+by\s+(\d+\.?\d*)%",
        ]

        for pattern in patterns:
            matches = re.finditer(pattern, text, re.IGNORECASE)
            for match in matches:
                context_start = max(0, match.start() - 100)
                context_end = min(len(text), match.end() + 100)
                context = text[context_start:context_end].strip()

                # Clean up whitespace
                context = re.sub(r'\s+', ' ', context)

                if len(context) > 20:
                    card = self.add_card(
                        paper_id=paper_id,
                        paper_title=paper_title,
                        content=f"Key finding: {match.group(0)} - {context[:200]}",
                        insight_type="finding",
                    )
                    cards.append(card)

        return cards

    def render_text(self, cards: List[InsightCard]) -> str:
        """Render cards as text."""
        if not cards:
            return "No insight cards found."

        lines = ["=" * 70, "💡 Key Insight Cards", "=" * 70, ""]

        type_icons = {
            "finding": "🎯",
            "method": "⚙️",
            "limitation": "⚠️",
            "future_work": "🔮",
        }

        for card in cards[:20]:
            icon = type_icons.get(card.insight_type, "💡")
            lines.append(f"{icon} [{card.card_id}] {card.insight_type.upper()}")
            lines.append(f"   Paper: {card.paper_title[:50]}")
            lines.append(f"   {card.content[:100]}")
            if card.tags:
                lines.append(f"   Tags: {', '.join(card.tags)}")
            lines.append("")

        lines.append(f"Total: {len(cards)} cards")
        lines.append("=" * 70)
        return "\n".join(lines)

    def render_markdown(self, cards: List[InsightCard]) -> str:
        """Render cards as Markdown."""
        lines = ["# Key Insight Cards\n"]

        if not cards:
            return "\n".join(lines) + "\nNo cards found."

        # Group by paper
        by_paper: Dict[str, List[InsightCard]] = {}
        for card in cards:
            if card.paper_id not in by_paper:
                by_paper[card.paper_id] = []
            by_paper[card.paper_id].append(card)

        for paper_id, paper_cards in by_paper.items():
            lines.append(f"## {paper_cards[0].paper_title[:60]}\n")
            lines.append(f"*From: {paper_id}*\n")

            for card in paper_cards:
                type_icon = {
                    "finding": "🎯",
                    "method": "⚙️",
                    "limitation": "⚠️",
                    "future_work": "🔮",
                }.get(card.insight_type, "💡")

                lines.append(f"### {type_icon} {card.insight_type.capitalize()}")

                lines.append(f"{card.content}\n")

                if card.evidence:
                    lines.append(f"> Evidence: {card.evidence}\n")

                if card.tags:
                    lines.append(f"*Tags: {', '.join(card.tags)}*\n")

        return "\n".join(lines)

    def export_for_note(self, cards: List[InsightCard]) -> str:
        """Export cards in a format suitable for notes."""
        lines = []

        for card in cards:
            lines.append(f"## {card.content[:80]}\n")
            lines.append(f"- Source: [[{card.paper_id}]]")
            lines.append(f"- Type: {card.insight_type}")
            if card.tags:
                lines.append(f"- Tags: {', '.join(['#' + t for t in card.tags])}")
            lines.append("")

        return "\n".join(lines)
