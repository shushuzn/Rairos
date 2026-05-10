"""Tests for research_loop/paper_parser.py PaperContent dataclass."""


class TestPaperContent:
    """Test PaperContent from research_loop/paper_parser.py."""

    def _paper_content(self):
        from research_loop.paper_parser import PaperContent
        return PaperContent

    def test_init_attributes(self):
        PC = self._paper_content()
        pc = PC(arxiv_id="2301.00001", title="Test Paper")
        assert pc.arxiv_id == "2301.00001"
        assert pc.title == "Test Paper"

    def test_optional_fields_default(self):
        PC = self._paper_content()
        pc = PC(arxiv_id="2301.00001", title="Test")
        assert pc.authors == []
        assert pc.abstract == ""
        assert pc.equations == []
        assert pc.claims == []
        assert pc.hyperparameters == {}
        assert pc.datasets == []
        assert pc.methods == []
        assert pc.categories == []
        assert pc.algorithm_fingerprint == ""

    def test_all_fields(self):
        PC = self._paper_content()
        pc = PC(
            arxiv_id="2301.00001",
            title="Test Paper",
            authors=["Alice", "Bob"],
            abstract="This is a test.",
            published="2023-01-01",
            updated="2023-01-02",
            algorithm_descriptions=["Alg1"],
            equations=["E=mc^2"],
            claims=["Claim 1"],
            hyperparameters={"lr": "0.001"},
            datasets=["ImageNet"],
            methods=["Method A"],
            categories=["cs.AI"],
            algorithm_fingerprint="abc123",
        )
        assert pc.arxiv_id == "2301.00001"
        assert pc.authors == ["Alice", "Bob"]
        assert pc.abstract == "This is a test."
        assert pc.equations == ["E=mc^2"]
        assert pc.hyperparameters == {"lr": "0.001"}
        assert pc.algorithm_fingerprint == "abc123"

    def test_repr_includes_arxiv_id(self):
        PC = self._paper_content()
        pc = PC(arxiv_id="2301.12345", title="Test")
        r = repr(pc)
        assert "2301.12345" in r
        assert "Test" in r
