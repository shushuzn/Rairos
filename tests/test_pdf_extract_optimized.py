"""Tests for optimized PDF extraction functionality."""
import pytest
from pathlib import Path
from pdf.extract import extract_pdf_text, extract_pdf_text_hybrid


@pytest.fixture
def sample_pdf_path():
    """Path to sample PDF file."""
    return Path("tests/fixtures/sample.pdf")


def test_extract_pdf_text(sample_pdf_path):
    """Test basic PDF text extraction."""
    if sample_pdf_path.exists():
        text = extract_pdf_text(sample_pdf_path)
        assert isinstance(text, str)
        assert len(text) > 0
    else:
        pytest.skip("Sample PDF not found")


def test_extract_pdf_text_hybrid(sample_pdf_path):
    """Test hybrid PDF text extraction."""
    if sample_pdf_path.exists():
        text = extract_pdf_text_hybrid(sample_pdf_path)
        assert isinstance(text, str)
        assert len(text) > 0
    else:
        pytest.skip("Sample PDF not found")


def test_extract_pdf_text_with_max_pages(sample_pdf_path):
    """Test PDF text extraction with max pages limit."""
    if sample_pdf_path.exists():
        text = extract_pdf_text(sample_pdf_path, max_pages=1)
        assert isinstance(text, str)
    else:
        pytest.skip("Sample PDF not found")
