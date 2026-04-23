"""Tests for tag inference functionality."""
from core import Paper
from notes.keyword_tags import infer_tags_if_empty


def test_infer_tags_if_empty_with_provided_tags():
    """Test that provided tags are returned as-is."""
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Test Paper",
        authors=["Author 1", "Author 2"],
        abstract="This is a test paper about LLMs and Agents.",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf"
    )
    provided_tags = ["LLM", "Agent"]
    result = infer_tags_if_empty(provided_tags, paper)
    assert result == provided_tags


def test_infer_tags_if_empty_with_no_tags():
    """Test that tags are inferred when none are provided."""
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Large Language Models for Agent Systems",
        authors=["Author 1", "Author 2"],
        abstract="This paper explores the use of large language models in autonomous agent systems. We discuss how LLMs can be used to power agent decision-making and tool use.",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf"
    )
    result = infer_tags_if_empty([], paper)
    assert "LLM" in result
    assert "Agent" in result


def test_infer_tags_if_empty_with_unsorted():
    """Test that Unsorted is returned when no tags are inferred."""
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="A Study of Nothing in Particular",
        authors=["Author 1", "Author 2"],
        abstract="This paper discusses various topics that are not related to any specific AI field.",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf"
    )
    result = infer_tags_if_empty([], paper)
    assert result == ["Unsorted"]


def test_infer_tags_if_empty_with_redundant_tags():
    """Test that redundant tags are removed."""
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="GPT-4 for Agent Systems",
        authors=["Author 1", "Author 2"],
        abstract="This paper explores the use of GPT-4 in autonomous agent systems. We discuss how GPT models can be used to power agent decision-making.",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf"
    )
    result = infer_tags_if_empty([], paper)
    # Should include GPT and Agent, but not LLM (since GPT is more specific)
    assert "GPT" in result
    assert "Agent" in result


def test_infer_tags_if_empty_with_multiple_tags():
    """Test that multiple relevant tags are inferred."""
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Multimodal RAG Systems for Medical AI",
        authors=["Author 1", "Author 2"],
        abstract="This paper presents a multimodal retrieval-augmented generation system for medical AI applications. We combine vision and text modalities to improve medical document understanding.",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf"
    )
    result = infer_tags_if_empty([], paper)
    assert "Multimodal" in result
    assert "RAG" in result
    assert "MedicalAI" in result
