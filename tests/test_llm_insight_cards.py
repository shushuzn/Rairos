"""Tier 2 unit tests — llm/insight_cards.py, pure functions, no I/O."""
import pytest
from llm.insight_cards import InsightCard, InsightCollection, InsightManager


def card(card_id="i001", paper_id="p001", paper_title="Test Paper",
         content="Test insight content", insight_type="finding",
         tags=None, evidence="", created_at="", references=None):
    return InsightCard(
        card_id=card_id, paper_id=paper_id, paper_title=paper_title,
        content=content, insight_type=insight_type, tags=tags or [],
        evidence=evidence, page_ref="", created_at=created_at,
        references=references or [],
    )


def cards_from_dicts(items):
    """Convert dicts to InsightCard list (simulate _load_cards)."""
    return [InsightCard(**d) for d in items]


# =============================================================================
# Dataclass tests
# =============================================================================
class TestInsightCard:
    """Test InsightCard dataclass."""

    def test_required_fields(self):
        card = InsightCard(
            card_id="i0001", paper_id="p001",
            paper_title="Attention Is All You Need",
            content="Multi-head attention outperforms single-head",
        )
        assert card.card_id == "i0001"
        assert card.paper_id == "p001"
        assert card.paper_title == "Attention Is All You Need"
        assert card.content == "Multi-head attention outperforms single-head"

    def test_optional_fields_default(self):
        c = card(card_id="i", paper_id="p", paper_title="T", content="C")
        assert c.insight_type == "finding"
        assert c.tags == []
        assert c.evidence == ""
        assert c.page_ref == ""
        assert c.created_at == ""
        assert c.references == []

    def test_all_fields_can_be_set(self):
        c = card(
            card_id="full", paper_id="p", paper_title="T", content="C",
            insight_type="method", tags=["transformer", "nlp"],
            evidence="Table 1", created_at="2026-01-01",
            references=["i0001"],
        )
        assert c.insight_type == "method"
        assert c.tags == ["transformer", "nlp"]
        assert c.evidence == "Table 1"
        assert c.created_at == "2026-01-01"
        assert c.references == ["i0001"]


class TestInsightCollection:
    """Test InsightCollection dataclass."""

    def test_required_fields(self):
        col = InsightCollection(collection_id="c0001", title="Transformer Research")
        assert col.collection_id == "c0001"
        assert col.title == "Transformer Research"

    def test_optional_fields_default(self):
        col = InsightCollection(collection_id="c", title="T")
        assert col.description == ""
        assert col.card_ids == []
        assert col.tags == []


# =============================================================================
# render_text tests — test the actual method on real objects
# =============================================================================
class TestRenderText:
    """Test InsightManager.render_text."""

    manager = InsightManager()

    def test_empty_returns_placeholder(self):
        output = self.manager.render_text([])
        assert output == "No insight cards found."

    def test_header_border_and_title(self):
        output = self.manager.render_text([card()])
        assert "=" * 70 in output
        assert "💡 Key Insight Cards" in output

    def test_finding_icon(self):
        output = self.manager.render_text([card(insight_type="finding")])
        assert "🎯 [i001] FINDING" in output

    def test_method_icon(self):
        output = self.manager.render_text([card(insight_type="method")])
        assert "⚙️ [i001] METHOD" in output

    def test_limitation_icon(self):
        output = self.manager.render_text([card(insight_type="limitation")])
        assert "⚠️ [i001] LIMITATION" in output

    def test_future_work_icon(self):
        output = self.manager.render_text([card(insight_type="future_work")])
        assert "🔮 [i001] FUTURE_WORK" in output

    def test_unknown_type_uses_default_icon(self):
        output = self.manager.render_text([card(insight_type="unknown_type")])
        assert "💡 [i001] UNKNOWN_TYPE" in output

    def test_paper_title_truncated_to_50(self):
        long_title = "A" * 60
        output = self.manager.render_text([card(paper_title=long_title)])
        assert "A" * 50 in output
        assert ("A" * 51) not in output

    def test_content_truncated_to_100(self):
        long_content = "X" * 120
        output = self.manager.render_text([card(content=long_content)])
        assert ("X" * 100) in output
        assert ("X" * 101) not in output

    def test_tags_shown(self):
        output = self.manager.render_text([card(tags=["transformer", "attention"])])
        assert "Tags: transformer, attention" in output

    def test_no_tags_when_empty(self):
        output = self.manager.render_text([card(tags=[])])
        assert "Tags:" not in output

    def test_max_20_cards(self):
        # range(1, 31) gives i=1..30 → IDs "i0001".."i0030"
        cards = [card(card_id=f"i{i:04d}") for i in range(1, 31)]
        output = self.manager.render_text(cards)
        assert "i0001" in output
        assert "i0020" in output
        assert "i0021" not in output

    def test_total_count(self):
        cards = [card(card_id=f"i{i:04d}") for i in range(5)]
        output = self.manager.render_text(cards)
        assert "Total: 5 cards" in output

    def test_multiple_cards(self):
        cards = [
            card(card_id="i001", insight_type="finding", content="First insight"),
            card(card_id="i002", insight_type="method", content="Second insight"),
        ]
        output = self.manager.render_text(cards)
        assert "i001" in output and "i002" in output
        assert "FINDING" in output and "METHOD" in output


# =============================================================================
# render_markdown tests — test the actual method on real objects
# =============================================================================
class TestRenderMarkdown:
    """Test InsightManager.render_markdown."""

    manager = InsightManager()

    def test_empty_returns_base_header(self):
        output = self.manager.render_markdown([])
        assert "# Key Insight Cards" in output
        assert "No cards found." in output

    def test_header_present(self):
        output = self.manager.render_markdown([card()])
        assert "# Key Insight Cards" in output

    def test_paper_section_title(self):
        c = card(paper_id="p1", paper_title="My Paper Title")
        output = self.manager.render_markdown([c])
        assert "## My Paper Title" in output
        assert "*From: p1*" in output

    def test_paper_title_truncated_to_60(self):
        long_title = "B" * 70
        output = self.manager.render_markdown([card(paper_title=long_title)])
        assert ("B" * 60) in output
        assert ("B" * 61) not in output

    def test_multiple_cards_same_paper_grouped(self):
        # Cards from same paper_id are grouped together under one ## heading.
        # Cards from different paper_ids get separate sections.
        cards = [
            card(paper_id="p1", paper_title="Paper Alpha", card_id="i001", content="Card 1"),
            card(paper_id="p1", paper_title="Paper Alpha", card_id="i002", content="Card 2"),
            card(paper_id="p2", paper_title="Paper Beta", card_id="i003", content="Card 3"),
        ]
        output = self.manager.render_markdown(cards)
        # p1 → one "Paper Alpha" section, p2 → one "Paper Beta" section
        assert output.count("## Paper Alpha") == 1
        assert output.count("## Paper Beta") == 1
        assert "Card 1" in output and "Card 2" in output
        assert "Card 3" in output

    def test_evidence_shown(self):
        output = self.manager.render_markdown([card(evidence="Table 1 results")])
        assert "> Evidence: Table 1 results" in output

    def test_no_evidence_section_when_empty(self):
        output = self.manager.render_markdown([card(evidence="")])
        assert "Evidence:" not in output

    def test_tags_shown(self):
        output = self.manager.render_markdown([card(tags=["nlp", "transformer"])])
        assert "*Tags: nlp, transformer*" in output

    def test_no_tags_section_when_empty(self):
        output = self.manager.render_markdown([card(tags=[])])
        assert "Tags:" not in output

    def test_finding_capitalized(self):
        output = self.manager.render_markdown([card(insight_type="finding")])
        assert "### 🎯 Finding" in output

    def test_method_capitalized(self):
        output = self.manager.render_markdown([card(insight_type="method")])
        assert "### ⚙️ Method" in output

    def test_limitation_capitalized(self):
        output = self.manager.render_markdown([card(insight_type="limitation")])
        assert "### ⚠️ Limitation" in output

    def test_future_work_capitalized(self):
        output = self.manager.render_markdown([card(insight_type="future_work")])
        assert "### 🔮 Future_work" in output

    def test_unknown_type_uses_default_icon(self):
        output = self.manager.render_markdown([card(insight_type="unknown")])
        assert "### 💡 Unknown" in output


# =============================================================================
# export_for_note tests — test the actual method on real objects
# =============================================================================
class TestExportForNote:
    """Test InsightManager.export_for_note."""

    manager = InsightManager()

    def test_content_as_header(self):
        output = self.manager.export_for_note([card(content="Attention breakthrough")])
        assert "## Attention breakthrough" in output

    def test_content_truncated_to_80(self):
        long_content = "X" * 100
        output = self.manager.export_for_note([card(content=long_content)])
        assert ("X" * 80) in output
        assert ("X" * 81) not in output

    def test_paper_id_as_wiki_link(self):
        output = self.manager.export_for_note([card(paper_id="p123")])
        assert "- Source: [[p123]]" in output

    def test_insight_type_shown(self):
        output = self.manager.export_for_note([card(insight_type="method")])
        assert "- Type: method" in output

    def test_tags_as_hashtags(self):
        output = self.manager.export_for_note([card(tags=["nlp", "transformer"])])
        assert "- Tags: #nlp, #transformer" in output

    def test_no_tags_when_empty(self):
        output = self.manager.export_for_note([card(tags=[])])
        assert "Tags:" not in output

    def test_multiple_cards(self):
        cards = [
            card(paper_id="p001", content="First insight"),
            card(paper_id="p002", content="Second insight"),
        ]
        output = self.manager.export_for_note(cards)
        assert "## First insight" in output
        assert "## Second insight" in output
        assert "[[p001]]" in output
        assert "[[p002]]" in output


# =============================================================================
# search_cards tests — replicate pure filtering logic
# =============================================================================
class TestSearchCardsFilter:
    """Test search_cards filtering logic (replicated for isolation)."""

    def _search_cards(self, data, query=None, tags=None, insight_type=None, paper_id=None):
        """Replicate search_cards filtering logic."""
        results = []
        for item in data:
            if query:
                q = query.lower()
                if q not in item["content"].lower() and q not in item["paper_title"].lower():
                    continue
            if tags:
                if not any(t in item["tags"] for t in tags):
                    continue
            if insight_type and item["insight_type"] != insight_type:
                continue
            if paper_id and item["paper_id"] != paper_id:
                continue
            results.append(item)
        results.sort(key=lambda x: x["created_at"], reverse=True)
        return results

    def _d(self, card_id="i001", paper_id="p001", paper_title="T",
            content="C", insight_type="finding", tags=None, created_at="2026-01-01"):
        return {
            "card_id": card_id, "paper_id": paper_id, "paper_title": paper_title,
            "content": content, "insight_type": insight_type, "tags": tags or [],
            "evidence": "", "page_ref": "", "created_at": created_at, "references": [],
        }

    def test_no_filters_returns_all(self):
        data = [self._d(card_id="i001"), self._d(card_id="i002")]
        assert len(self._search_cards(data)) == 2

    def test_filter_by_query_in_content(self):
        data = [
            self._d(card_id="i001", content="Transformer attention"),
            self._d(card_id="i002", content="CNN classifier"),
        ]
        result = self._search_cards(data, query="transformer")
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_filter_by_query_in_title(self):
        data = [
            self._d(card_id="i001", paper_title="BERT paper"),
            self._d(card_id="i002", paper_title="CNN study"),
        ]
        result = self._search_cards(data, query="bert")
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_query_case_insensitive(self):
        data = [self._d(card_id="i001", content="Deep Learning")]
        result = self._search_cards(data, query="DEEP")
        assert len(result) == 1

    def test_query_not_matching(self):
        data = [self._d(card_id="i001", content="transformer")]
        result = self._search_cards(data, query="cnn")
        assert len(result) == 0

    def test_filter_by_single_tag(self):
        data = [
            self._d(card_id="i001", tags=["nlp"]),
            self._d(card_id="i002", tags=["cv"]),
        ]
        result = self._search_cards(data, tags=["nlp"])
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_filter_by_tags_any_match(self):
        data = [
            self._d(card_id="i001", tags=["nlp", "transformer"]),
            self._d(card_id="i002", tags=["cv"]),
        ]
        result = self._search_cards(data, tags=["transformer"])
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_filter_by_multiple_tags(self):
        data = [
            self._d(card_id="i001", tags=["nlp"]),
            self._d(card_id="i002", tags=["cv"]),
        ]
        result = self._search_cards(data, tags=["nlp", "cv"])
        assert len(result) == 2

    def test_filter_by_insight_type(self):
        data = [
            self._d(card_id="i001", insight_type="finding"),
            self._d(card_id="i002", insight_type="method"),
        ]
        result = self._search_cards(data, insight_type="method")
        assert len(result) == 1 and result[0]["card_id"] == "i002"

    def test_filter_by_paper_id(self):
        data = [
            self._d(card_id="i001", paper_id="p001"),
            self._d(card_id="i002", paper_id="p002"),
        ]
        result = self._search_cards(data, paper_id="p001")
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_sort_by_created_at_descending(self):
        data = [
            self._d(card_id="i001", created_at="2026-01-01"),
            self._d(card_id="i002", created_at="2026-04-01"),
            self._d(card_id="i003", created_at="2026-02-01"),
        ]
        result = self._search_cards(data)
        assert result[0]["card_id"] == "i002"
        assert result[1]["card_id"] == "i003"
        assert result[2]["card_id"] == "i001"

    def test_combined_filters(self):
        data = [
            self._d(card_id="i001", paper_id="p001", insight_type="finding",
                    tags=["nlp"], content="transformer"),
            self._d(card_id="i002", paper_id="p001", insight_type="method",
                    tags=["nlp"], content="bert"),
            self._d(card_id="i003", paper_id="p002", insight_type="finding",
                    tags=["nlp"], content="cnn"),
        ]
        result = self._search_cards(data, paper_id="p001", insight_type="finding")
        assert len(result) == 1 and result[0]["card_id"] == "i001"

    def test_empty_created_at_sorted_last(self):
        """Empty string < any date in descending order — items with dates come first."""
        data = [
            self._d(card_id="i001", created_at="2026-01-01"),
            self._d(card_id="i002", created_at=""),
        ]
        result = self._search_cards(data)
        # Descending: "2026-01-01" > "" so i001 comes first
        assert result[0]["card_id"] == "i001"


# =============================================================================
# get_tag_cloud logic tests — replicate pure function
# =============================================================================
class TestTagCloud:
    """Test get_tag_cloud logic (replicated for isolation)."""

    def _get_tag_cloud(self, data):
        """Replicate get_tag_cloud."""
        tags = {}
        for item in data:
            for tag in item["tags"]:
                tags[tag] = tags.get(tag, 0) + 1
        return dict(sorted(tags.items(), key=lambda x: -x[1]))

    def test_counts_tags(self):
        data = [
            {"tags": ["nlp"]},
            {"tags": ["nlp"]},
            {"tags": ["cv"]},
        ]
        cloud = self._get_tag_cloud(data)
        assert cloud["nlp"] == 2
        assert cloud["cv"] == 1

    def test_sorted_by_frequency_descending(self):
        data = [
            {"tags": ["cv"]},
            {"tags": ["nlp"]},
            {"tags": ["nlp"]},
            {"tags": ["nlp"]},
        ]
        cloud = self._get_tag_cloud(data)
        keys = list(cloud.keys())
        assert keys[0] == "nlp"
        assert keys[1] == "cv"

    def test_empty_list(self):
        assert self._get_tag_cloud([]) == {}

    def test_no_tags(self):
        assert self._get_tag_cloud([{"tags": []}]) == {}


# =============================================================================
# InsightManager instantiation
# =============================================================================
class TestInsightManagerInit:
    """Test InsightManager class."""

    def test_can_instantiate(self):
        manager = InsightManager()
        assert manager is not None
        assert manager.data_dir is not None

    def test_has_expected_methods(self):
        manager = InsightManager()
        assert hasattr(manager, "render_text")
        assert hasattr(manager, "render_markdown")
        assert hasattr(manager, "export_for_note")
        assert hasattr(manager, "search_cards")
        assert hasattr(manager, "get_tag_cloud")
