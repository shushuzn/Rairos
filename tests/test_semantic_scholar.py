"""Tests for parsers/semantic_scholar.py — singleton client behavior."""

from __future__ import annotations

from unittest.mock import MagicMock, patch

import pytest


def test_get_client_returns_singleton():
    """Multiple calls to _get_client() return the same instance."""
    import parsers.semantic_scholar as s2

    # Reset singleton to ensure clean state
    s2._client = None

    with patch("parsers.semantic_scholar.httpx.Client") as mock_client_cls:
        mock_instance = MagicMock()
        mock_client_cls.return_value = mock_instance

        client1 = s2._get_client()
        client2 = s2._get_client()

        assert client1 is client2, "_get_client() must return the same instance"
        assert mock_client_cls.call_count == 1, "httpx.Client must be instantiated only once"


def test_search_semantic_scholar_returns_s2paper_list(monkeypatch):
    """search_semantic_scholar returns a list of S2Paper objects."""
    import parsers.semantic_scholar as s2

    s2._client = None  # Reset

    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {
        "data": [
            {
                "paperId": "paper1",
                "title": "Attention Is All You Need",
                "abstract": "We propose Transformer.",
                "authors": [{"name": "Vaswani"}],
                "year": 2017,
                "venue": "NeurIPS",
                "citationCount": 50000,
                "openAccessPdf": None,
            }
        ]
    }

    mock_client = MagicMock()
    mock_client.get.return_value = mock_response
    monkeypatch.setattr(s2, "_get_client", lambda: mock_client)

    results = s2.search_semantic_scholar("transformer", max_results=1)

    assert len(results) == 1
    assert results[0].title == "Attention Is All You Need"
    assert results[0].paper_id == "paper1"


def test_get_paper_by_id_returns_s2paper(monkeypatch):
    """get_paper_by_id returns an S2Paper."""
    import parsers.semantic_scholar as s2

    s2._client = None

    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {
        "paperId": "abc123",
        "title": "BERT",
        "abstract": "Pre-training of deep bidirectional Transformers.",
        "authors": [{"name": "Devlin"}],
        "year": 2018,
        "venue": "NAACL",
        "citationCount": 80000,
        "openAccessPdf": None,
    }

    mock_client = MagicMock()
    mock_client.get.return_value = mock_response
    monkeypatch.setattr(s2, "_get_client", lambda: mock_client)

    result = s2.get_paper_by_id("abc123")

    assert result is not None
    assert result.paper_id == "abc123"
    assert result.title == "BERT"


def test_get_citations_returns_list(monkeypatch):
    """get_citations returns a list of S2Paper."""
    import parsers.semantic_scholar as s2

    s2._client = None

    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {
        "data": [
            {
                "citingPaper": {
                    "paperId": "cite1",
                    "title": "ALBERT",
                    "abstract": "A Lite BERT.",
                    "authors": [{"name": "Lan"}],
                    "year": 2019,
                    "venue": "ICLR",
                    "citationCount": 10000,
                    "openAccessPdf": None,
                }
            }
        ]
    }

    mock_client = MagicMock()
    mock_client.get.return_value = mock_response
    monkeypatch.setattr(s2, "_get_client", lambda: mock_client)

    results = s2.get_citations("abc123")

    assert len(results) == 1
    assert results[0].title == "ALBERT"


def test_get_references_returns_list(monkeypatch):
    """get_references returns a list of S2Paper."""
    import parsers.semantic_scholar as s2

    s2._client = None

    mock_response = MagicMock()
    mock_response.status_code = 200
    mock_response.json.return_value = {
        "data": [
            {
                "referencedPaper": {
                    "paperId": "ref1",
                    "title": "Attention",
                    "abstract": "Old attention paper.",
                    "authors": [{"name": "Bahdanau"}],
                    "year": 2015,
                    "venue": "ICLR",
                    "citationCount": 30000,
                    "openAccessPdf": None,
                }
            }
        ]
    }

    mock_client = MagicMock()
    mock_client.get.return_value = mock_response
    monkeypatch.setattr(s2, "_get_client", lambda: mock_client)

    results = s2.get_references("abc123")

    assert len(results) == 1
    assert results[0].title == "Attention"


def test_s2paper_to_paper():
    """S2Paper.to_paper() produces a valid Paper."""
    from parsers.semantic_scholar import S2Paper

    s2paper = S2Paper(
        {
            "paperId": "xyz",
            "title": "Test Paper",
            "abstract": "Abstract text",
            "authors": [{"name": "Smith"}],
            "year": 2020,
            "venue": "CVPR",
            "citationCount": 100,
            "openAccessPdf": {"url": "https://example.com/pdf"},
            "externalIds": {"ArXiv": "arXiv:1234.5678", "DOI": "10.1234/test"},
        }
    )

    paper = s2paper.to_paper()

    assert paper.title == "Test Paper"
    assert paper.uid == "arXiv:1234.5678"
    assert paper.pdf_url == "https://example.com/pdf"
    assert paper.authors == ["Smith"]
    assert paper.doi == "10.1234/test"
