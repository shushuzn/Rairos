"""Tests for renderers module functionality."""
from core import Paper


def test_render_pnote_basic():
    """Test basic P-note rendering functionality."""
    from renderers.pnote import render_pnote
    
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Test Paper Title",
        authors=["Author 1", "Author 2"],
        abstract="This is a test abstract.",
        published="2023-01-01",
        updated="2023-01-02",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="https://arxiv.org/pdf/2301.001.pdf",
        primary_category="cs.AI"
    )
    
    result = render_pnote(
        p=paper,
        tags=["LLM", "Agent"],
        extracted_sections_md="# Introduction\nTest content",
    )
    
    # Check that key elements are present
    assert "type: paper" in result
    assert "Test Paper Title" in result
    assert "ARXIV: 2301.001" in result  # Note: source is uppercased in render
    assert "Author 1, Author 2" in result


def test_render_pnote_with_tags():
    """Test P-note rendering with tags."""
    from renderers.pnote import render_pnote
    
    paper = Paper(
        source="doi",
        uid="10.1234/test",
        title="DOI Paper",
        authors=["Author"],
        abstract="Abstract",
        published="2023-06-15",
        updated="2023-06-15",
        abs_url="https://doi.org/10.1234/test",
        pdf_url="",
    )
    
    tags = ["RAG", "Evaluation", "Safety"]
    result = render_pnote(
        p=paper,
        tags=tags,
        extracted_sections_md="",
    )
    
    assert "tags: [RAG, Evaluation, Safety]" in result


def test_render_pnote_empty_authors():
    """Test P-note rendering with empty authors."""
    from renderers.pnote import render_pnote
    
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Anonymous Paper",
        authors=[],
        abstract="Abstract",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="",
    )
    
    result = render_pnote(
        p=paper,
        tags=[],
        extracted_sections_md="",
    )
    
    assert "Unknown" in result
    assert "Anonymous Paper" in result


def test_render_pnote_with_ai_draft():
    """Test P-note rendering with AI draft."""
    from renderers.pnote import render_pnote
    
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="AI Draft Paper",
        authors=["Author"],
        abstract="Abstract",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="",
    )
    
    ai_draft = "## AI Draft\nThis is the AI generated draft."
    result = render_pnote(
        p=paper,
        tags=["Test"],
        extracted_sections_md="",
        ai_draft_md=ai_draft,
    )
    
    assert "ai_generated: true" in result or "rubric: draft-ai" in result
    assert "AI Draft" in result


def test_render_pnote_frontmatter_format():
    """Test P-note frontmatter format."""
    from renderers.pnote import render_pnote
    
    paper = Paper(
        source="arxiv",
        uid="2301.001",
        title="Frontmatter Test",
        authors=["Author"],
        abstract="Abstract",
        published="2023-01-01",
        updated="2023-01-01",
        abs_url="https://arxiv.org/abs/2301.001",
        pdf_url="",
    )
    
    result = render_pnote(
        p=paper,
        tags=["Tag1"],
        extracted_sections_md="",
    )
    
    # Check frontmatter format
    assert "---" in result
    assert "type: paper" in result
    assert "status: draft" in result
    assert "date:" in result
    assert "tags:" in result
