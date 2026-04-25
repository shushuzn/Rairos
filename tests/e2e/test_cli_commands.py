"""End-to-end tests for CLI commands.

These tests verify that CLI command modules can be imported and initialized correctly.
"""
import pytest
import sys
from pathlib import Path

# Add project root to path
project_root = Path(__file__).parent.parent.parent
sys.path.insert(0, str(project_root))


class TestCLICommandImports:
    """Test that CLI commands can be imported and initialized."""

    def test_import_registry(self):
        """Test that _registry can be imported."""
        from cli._registry import main, SUBCOMMANDS
        assert "search" in SUBCOMMANDS
        assert "import" in SUBCOMMANDS
        assert "rag" in SUBCOMMANDS
        assert "visual" in SUBCOMMANDS

    def test_import_cmd_modules(self):
        """Test that command modules can be imported."""
        from cli.cmd.search import _build_search_parser
        from cli.cmd.import_ import _build_import_parser
        from cli.cmd.rag import _build_rag_parser
        from cli.cmd.visual import _build_visual_parser
        # All should be callable
        assert callable(_build_search_parser)
        assert callable(_build_import_parser)
        assert callable(_build_rag_parser)
        assert callable(_build_visual_parser)

    def test_research_loop_imports(self):
        """Test that research_loop modules can be imported."""
        from research_loop import PaperPipeline, RagPipeline
        from research_loop.core import run_research

        assert PaperPipeline is not None
        assert RagPipeline is not None
        assert callable(run_research)

    def test_visual_extractor_imports(self):
        """Test that visual extraction module imports correctly."""
        from pdf.visual import VisualExtractor, VisualContent
        assert VisualExtractor is not None
        assert VisualContent is not None

    def test_all_subcommands_available(self):
        """Test all subcommands are registered."""
        from cli._registry import SUBCOMMANDS

        expected = {
            "search", "list", "status", "queue", "cache", "dedup", "merge",
            "stats", "import", "export", "citations", "cite-graph", "cite-import",
            "cite-fetch", "cite-stats", "dedup-semantic", "research", "similar",
            "kg", "paper2code", "evoskill", "rag", "visual",
        }
        assert expected.issubset(SUBCOMMANDS)


class TestCLIParsers:
    """Test CLI parsers can be built."""

    def test_build_all_parsers(self):
        """Test that all parsers can be built without error."""
        import argparse
        from cli._registry import _build_all_parsers

        parser = argparse.ArgumentParser()
        subparsers = parser.add_subparsers()

        # Should not raise
        _build_all_parsers(subparsers)

        # Verify some subcommands exist
        assert "search" in subparsers.choices
        assert "import" in subparsers.choices
        assert "rag" in subparsers.choices
