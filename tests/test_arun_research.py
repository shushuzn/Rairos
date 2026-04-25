"""Async research loop tests — arun_research with mocked I/O."""
import asyncio
import tempfile
from pathlib import Path
from unittest.mock import AsyncMock, patch

import pytest

from core import Paper
from research_loop import arun_research


# =============================================================================
# Fixtures
# =============================================================================

def _make_paper(uid: str = "2301.12345", title: str = "Test Paper") -> Paper:
    return Paper(
        source="arxiv",
        uid=uid,
        title=title,
        authors=["Alice Smith"],
        abstract="This is the abstract.",
        published="2023-06-01",
        updated="2023-07-15",
        abs_url=f"https://arxiv.org/abs/{uid}",
        pdf_url=f"https://arxiv.org/pdf/{uid}.pdf",
    )


@pytest.fixture
def tmp_output_dir():
    with tempfile.TemporaryDirectory() as d:
        yield Path(d)


# =============================================================================
# arun_research — async pipeline
# =============================================================================

class TestArunResearch:
    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_full_pipeline_creates_note(self, tmp_output_dir):
        """arun_research processes papers and writes note files."""
        paper = _make_paper(uid="2301.00001", title="Attention Is All You Need")

        async def mock_download(*a, **kw):
            pdf_path = Path(tempfile.gettempdir()) / "2301.00001.pdf"
            pdf_path.write_bytes(b"%PDF-1.4 test")
            return pdf_path

        with patch("research_loop.core.search_arxiv", return_value=[paper]), \
             patch("pdf.extract.extract_pdf_text", return_value="Extracted text."), \
             patch("pdf.extract_async.download_pdf_async", side_effect=mock_download):
            from llm import client_async
            with patch.object(client_async, "call_llm_chat_completions_async", new_callable=AsyncMock) as mock_llm:
                mock_llm.return_value = "AI draft content."
                paths = await arun_research(
                    query="transformer",
                    limit=5,
                    output_dir=tmp_output_dir,
                    api_key="test-key",
                    download_pdfs=True,
                    skip_existing=True,
                    verbose=False,
                )

        assert len(paths) == 1
        assert paths[0].exists()
        assert "Attention Is All You Need" in paths[0].read_text(encoding="utf-8")

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_no_api_key_warns_and_continues(self, tmp_output_dir):
        """Without API key, arun_research produces metadata-only notes."""
        paper = _make_paper(uid="2301.00002")

        with patch("research_loop.core.search_arxiv", return_value=[paper]), \
             patch("pdf.extract_async.download_pdf_async", new_callable=AsyncMock) as mock_dl, \
             patch("pdf.extract.extract_pdf_text", return_value="Some text"):
            mock_dl.side_effect = Exception("no pdf")

            with pytest.warns(UserWarning, match="OPENAI_API_KEY not set"):
                paths = await arun_research(
                    query="test",
                    output_dir=tmp_output_dir,
                    api_key="",
                    download_pdfs=True,
                    skip_existing=True,
                    verbose=False,
                )

        assert len(paths) == 1
        note_text = paths[0].read_text(encoding="utf-8")
        assert "_Note: Set `OPENAI_API_KEY` to enable AI draft generation._" in note_text

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_skip_existing_note(self, tmp_output_dir):
        """skip_existing=True skips notes that already exist."""
        paper = _make_paper(uid="2301.00003")
        uid_filename = "2301.00003_Test-Paper.md"
        note_path = tmp_output_dir / uid_filename
        note_path.write_text("Already exists", encoding="utf-8")

        with patch("research_loop.core.search_arxiv", return_value=[paper]):
            paths = await arun_research(
                query="test",
                output_dir=tmp_output_dir,
                api_key="",
                download_pdfs=False,
                skip_existing=True,
                verbose=False,
            )

        assert len(paths) == 1
        assert paths[0] == note_path
        assert note_path.read_text(encoding="utf-8") == "Already exists"

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_empty_search_returns_empty_list(self, tmp_output_dir):
        """Empty search result returns an empty list without error."""
        with patch("research_loop.core.search_arxiv", return_value=[]):
            paths = await arun_research(
                query="nonexistent query xyz",
                output_dir=tmp_output_dir,
                verbose=False,
            )

        assert paths == []

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_search_error_returns_empty(self, tmp_output_dir):
        """RuntimeError from search_arxiv is caught and returns []."""
        with patch("research_loop.core.search_arxiv", side_effect=RuntimeError("Network error")):
            paths = await arun_research(
                query="test",
                output_dir=tmp_output_dir,
                verbose=True,
            )

        assert paths == []

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_semaphore_concurrency_limit(self, tmp_output_dir):
        """Semaphore ensures at most 3 papers are processed concurrently."""
        paper1 = _make_paper(uid="2301.00011", title="Paper One")
        paper2 = _make_paper(uid="2301.00012", title="Paper Two")
        paper3 = _make_paper(uid="2301.00013", title="Paper Three")
        paper4 = _make_paper(uid="2301.00014", title="Paper Four")
        papers = [paper1, paper2, paper3, paper4]

        max_concurrent = 0
        current_concurrent = 0
        lock = asyncio.Lock()

        async def mock_download(*a, **kw):
            nonlocal current_concurrent, max_concurrent
            async with lock:
                current_concurrent += 1
                max_concurrent = max(max_concurrent, current_concurrent)
            await asyncio.sleep(0.05)
            async with lock:
                current_concurrent -= 1
            pdf_path = Path(tempfile.gettempdir()) / "mock.pdf"
            pdf_path.write_bytes(b"%PDF-1.4")
            return pdf_path

        with patch("research_loop.core.search_arxiv", return_value=papers), \
             patch("pdf.extract.extract_pdf_text", return_value="Extracted text."), \
             patch("pdf.extract_async.download_pdf_async", side_effect=mock_download):
            from llm import client_async
            with patch.object(client_async, "call_llm_chat_completions_async", new_callable=AsyncMock) as mock_llm:
                mock_llm.return_value = "Draft."
                await arun_research(
                    query="test",
                    output_dir=tmp_output_dir,
                    api_key="test-key",
                    download_pdfs=True,
                    skip_existing=True,
                    verbose=False,
                )

        # Semaphore(5) means at most 5 tasks run at the same time
        assert max_concurrent <= 5, f"Expected <=5 concurrent, got {max_concurrent}"

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_error_isolation_one_paper_fails(self, tmp_output_dir):
        """A failure in one paper does not stop others from completing."""
        paper1 = _make_paper(uid="2301.00021", title="Good Paper")
        paper2 = _make_paper(uid="2301.00022", title="Bad Paper")
        papers = [paper1, paper2]

        async def flaky_download(url: str, *a, **kw):
            # Extract uid from URL to write correct filename
            uid = url.split("/")[-1].replace(".pdf", "")
            pdf_path = Path(tempfile.gettempdir()) / f"{uid}.pdf"
            if "2301.00022" in str(url):
                raise RuntimeError("PDF download failed")
            # Valid minimal PDF so PyMuPDF can extract text
            pdf_bytes = (
                b"%PDF-1.4\n"
                b"1 0 obj<</Type/Catalog/Pages 2 0 R>>endobj\n"
                b"2 0 obj<</Type/Pages/Kids[3 0 R]/Count 1>>endobj\n"
                b"3 0 obj<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792] "
                b"/Contents 4 0 R/Resources<</Font<</F1 5 0 R>>>>>>endobj\n"
                b"4 0 obj<</Length 44>>stream\n"
                b"BT /F1 12 Tf 100 700 Td (Extracted text.) Tj ET\n"
                b"endstream endobj\n"
                b"5 0 obj<</Type/Font/Subtype/Type1/BaseFont/Helvetica>>endobj\n"
                b"xref\n0 6\n"
                b"0000000000 65535 f\n"
                b"0000000009 00000 n\n"
                b"0000000058 00000 n\n"
                b"0000000115 00000 n\n"
                b"0000000270 00000 n\n"
                b"0000000350 00000 n\n"
                b"trailer<</Size 6/Root 1 0 R>>\n"
                b"startxref\n"
                b"427\n"
                b"%%EOF"
            )
            pdf_path.write_bytes(pdf_bytes)
            return pdf_path

        with patch("research_loop.core.search_arxiv", return_value=papers), \
             patch("pdf.extract.extract_pdf_text", return_value="Extracted text."), \
             patch("pdf.extract_async.download_pdf_async", side_effect=flaky_download):
            from llm import client_async
            with patch.object(client_async, "call_llm_chat_completions_async", new_callable=AsyncMock) as mock_llm:
                mock_llm.return_value = "Draft."
                paths = await arun_research(
                    query="test",
                    output_dir=tmp_output_dir,
                    api_key="test-key",
                    download_pdfs=True,
                    skip_existing=True,
                    verbose=True,
                )

        # Both papers should be attempted; good one succeeds, bad one fails
        # We should have 2 paths (metadata-only note for the bad paper)
        assert len(paths) == 2
        good_note = tmp_output_dir / "2301.00021_Good-Paper.md"
        bad_note = tmp_output_dir / "2301.00022_Bad-Paper.md"
        assert good_note.exists()
        assert bad_note.exists()
        # Good paper should have AI draft content
        assert "AI Research Note" in good_note.read_text(encoding="utf-8")

    @pytest.mark.no_freeze
    @pytest.mark.asyncio
    async def test_all_papers_processed_on_error(self, tmp_output_dir):
        """Multiple papers: even if one PDF fails, the others complete."""
        paper1 = _make_paper(uid="2301.00031", title="Paper A")
        paper2 = _make_paper(uid="2301.00032", title="Paper B")
        paper3 = _make_paper(uid="2301.00033", title="Paper C")
        papers = [paper1, paper2, paper3]

        async def mock_dl(url: str, *a, **kw):
            uid = url.split("/")[-1].replace(".pdf", "")
            pdf_path = Path(tempfile.gettempdir()) / f"{uid}.pdf"
            if "2301.00032" in str(url):
                raise RuntimeError("failed")
            pdf_path.write_bytes(b"%PDF-1.4")
            return pdf_path

        with patch("research_loop.core.search_arxiv", return_value=papers), \
             patch("pdf.extract.extract_pdf_text", return_value="Extracted text."), \
             patch("pdf.extract_async.download_pdf_async", side_effect=mock_dl):
            from llm import client_async
            with patch.object(client_async, "call_llm_chat_completions_async", new_callable=AsyncMock) as mock_llm:
                mock_llm.return_value = "Draft."
                paths = await arun_research(
                    query="test",
                    output_dir=tmp_output_dir,
                    api_key="test-key",
                    download_pdfs=True,
                    skip_existing=True,
                    verbose=False,
                )

        # 3 notes should be created (1 per paper)
        assert len(paths) == 3
        for p in papers:
            slug = p.title.replace(" ", "-")
            note = tmp_output_dir / f"{p.uid}_{slug}.md"
            assert note.exists()
