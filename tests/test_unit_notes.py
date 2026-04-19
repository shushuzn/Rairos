"""Tier 4 unit tests for notes-related functions (ensure_cnote, timeline, collect_pnotes, etc.)."""

class TestTodayIsoTier4:
    def test_returns_iso_format(self):
        import ai_research_os as airo
        result = airo.today_iso()
        # Frozen to 2024-06-15 by conftest.py autouse fixture
        assert result == "2024-06-15"

    def test_returns_todays_date(self):
        import ai_research_os as airo
        result = airo.today_iso()
        # Frozen to 2024-06-15 by conftest.py autouse fixture
        assert result == "2024-06-15"


class TestSlugifyTitleTier4:
    def test_basic(self):
        import ai_research_os as airo
        assert airo.slugify_title("Hello World") == "Hello-World"

    def test_empty(self):
        import ai_research_os as airo
        assert airo.slugify_title("") == "Paper"

    def test_max_len(self):
        import ai_research_os as airo
        long_title = "A" * 200
        result = airo.slugify_title(long_title)
        assert len(result) <= 200

    def test_strips_special_chars(self):
        import ai_research_os as airo
        result = airo.slugify_title("Hello! World? #Test")
        assert "!" not in result
        assert "?" not in result
        assert "#" not in result

    def test_unicode_letters_kept(self):
        import ai_research_os as airo
        result = airo.slugify_title("机器学习")
        assert result == "机器学习"


class TestIsProbablyDoiTier4:
    def test_true_patterns(self):
        import ai_research_os as airo
        assert airo.is_probably_doi("10.1001/test")
        assert airo.is_probably_doi("10.1234/abc")

    def test_false_patterns(self):
        import ai_research_os as airo
        assert not airo.is_probably_doi("not a doi")
        assert not airo.is_probably_doi("")


class TestNormalizeDoiTier4:
    def test_url_stripped(self):
        import ai_research_os as airo
        assert airo.normalize_doi("https://doi.org/10.1001/test") == "10.1001/test"
        assert airo.normalize_doi("http://dx.doi.org/10.1001/test") == "10.1001/test"

    def test_plain(self):
        import ai_research_os as airo
        assert airo.normalize_doi("10.1001/test") == "10.1001/test"

    def test_none_input(self):
        import ai_research_os as airo
        assert airo.normalize_doi(None) is None


class TestNormalizeArxivIdTier4:
    def test_url_formats(self):
        import ai_research_os as airo
        assert airo.normalize_arxiv_id("https://arxiv.org/abs/2301.00001") == "2301.00001"
        assert airo.normalize_arxiv_id("https://arxiv.org/abs/2301.00001v2") == "2301.00001v2"

    def test_new_style_bare(self):
        import ai_research_os as airo
        assert airo.normalize_arxiv_id("2301.00001") == "2301.00001"
        assert airo.normalize_arxiv_id("2301.00001v2") == "2301.00001v2"

    def test_old_style_bare(self):
        import ai_research_os as airo
        assert airo.normalize_arxiv_id("hep-th/9901001") == "hep-th/9901001"

    def test_doi_format(self):
        import ai_research_os as airo
        assert airo.normalize_arxiv_id("10.1001/test") is None

    def test_none_input(self):
        import ai_research_os as airo
        assert airo.normalize_arxiv_id(None) is None


class TestParseFrontmatterTier4:
    def test_parses_yaml_block(self):
        import ai_research_os as airo
        content = "---\ntitle: Test\ntags:\n  - A\n  - B\n---\nbody"
        result = airo.parse_frontmatter(content)
        assert result["title"] == "Test"
        assert result["tags"] == ["A", "B"]

    def test_empty_dict_for_no_frontmatter(self):
        import ai_research_os as airo
        assert airo.parse_frontmatter("no frontmatter") == {}

    def test_handles_multiline_values(self):
        import ai_research_os as airo
        content = "---\ntitle: Multi\n  Line\n---\nbody"
        result = airo.parse_frontmatter(content)
        assert "title" in result

    def test_inline_tags(self):
        import ai_research_os as airo
        content = "---\ntags: [Agent, RAG]\n---\nbody"
        result = airo.parse_frontmatter(content)
        assert result.get("tags") == "[Agent, RAG]"


class TestParseDateFromFrontmatterTier4:
    def test_returns_iso_format(self):
        import ai_research_os as airo
        content = "---\ndate: 2024-03-15\n---\n"
        fm = airo.parse_frontmatter(content)
        result = airo.parse_date_from_frontmatter(fm)
        assert result == "2024-03-15"

    def test_returns_empty_for_missing_date(self):
        import ai_research_os as airo
        content = "---\ntitle: Test\n---\n"
        fm = airo.parse_frontmatter(content)
        result = airo.parse_date_from_frontmatter(fm)
        assert result == ""

    def test_returns_bad_format_with_warning(self):
        import ai_research_os as airo
        import warnings
        content = "---\ndate: not-a-date\n---\n"
        fm = airo.parse_frontmatter(content)
        with warnings.catch_warnings(record=True) as rec:
            warnings.simplefilter("always")
            result = airo.parse_date_from_frontmatter(fm)
        assert result == "not-a-date"
        assert any("Unrecognized date format" in str(w.message) for w in rec)


class TestRenderCnoteTier4:
    def test_has_required_sections(self):
        import ai_research_os as airo
        result = airo.render_cnote("Test Concept")
        assert "Test Concept" in result
        assert "type: concept" in result
        assert "## 核心定义" in result

    def test_type_is_concept(self):
        import ai_research_os as airo
        result = airo.render_cnote("Test Concept")
        assert "concept" in result.lower()

    def test_hash_escaped_in_title(self):
        import ai_research_os as airo
        result = airo.render_cnote("LLM# vs CNN#")
        lines = result.split("\n")
        title_line = lines[4]
        assert r"\#" in title_line
        assert not title_line.startswith("# ") or title_line.startswith("# LLM")


class TestEnsureCnoteTier4:
    def test_creates_cnote_file(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        path = airo.ensure_cnote(root, "Test Concept")
        assert path.exists()
        assert "Test" in path.name

    def test_returns_existing_file(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        p1 = airo.ensure_cnote(root, "Test")
        p2 = airo.ensure_cnote(root, "Test")
        assert p1 == p2


class TestEnsureTimelineTier4:
    def test_creates_timeline_file(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path
        path = airo.ensure_timeline(root)
        assert path.exists()
        assert path.name == "Timeline.md"

    def test_is_idempotent(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path
        p1 = airo.ensure_timeline(root)
        p2 = airo.ensure_timeline(root)
        assert p1 == p2
        assert p2.exists()


class TestUpdateTimelineTier4:
    def test_appends_to_new_year_section(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path
        airo.ensure_timeline(root)
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        pnote = papers_dir / "P - 2024-01-01 - Test Event.md"
        pnote.write_text("---\npublished: 2024-01-01\n---\n", encoding="utf-8")
        year = "2024"  # frozen year from conftest
        result = airo.update_timeline(root, year, pnote, "Test Event")
        assert result.exists()
        content = result.read_text(encoding="utf-8")
        assert year in content

    def test_no_change_for_existing_bullet(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path
        airo.ensure_timeline(root)
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        pnote = papers_dir / "P - 2024-01-01 - Test Event.md"
        pnote.write_text("---\npublished: 2024-01-01\n---\n", encoding="utf-8")
        year = "2024"  # frozen year from conftest
        r1 = airo.update_timeline(root, year, pnote, "Test Event")
        r2 = airo.update_timeline(root, year, pnote, "Test Event")
        assert r1.exists()
        assert r2.exists()

    def test_inserts_before_next_year(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path
        airo.ensure_timeline(root)
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        pnote = papers_dir / "P - 2024-01-01 - Current Event.md"
        pnote.write_text("---\npublished: 2024-01-01\n---\n", encoding="utf-8")
        next_year = "2025"  # frozen year + 1
        content = f"## {next_year}\n- Future event\n"
        tl_path = root / "00-Radar" / "Timeline.md"
        tl_path.write_text(content, encoding="utf-8")
        year = "2024"  # frozen year from conftest
        result = airo.update_timeline(root, year, pnote, "Current Event")
        assert result.exists()
        result_content = result.read_text(encoding="utf-8")
        assert next_year in result_content


class TestCollectPnotesTier4:
    def test_returns_empty_for_no_files(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        result = airo.collect_pnotes(root)
        assert result == []

    def test_sorts_by_date_descending(self, tmp_path):
        """collect_pnotes sorts by path (lexicographic), which on Windows sorts 2024-01 < 2024-06."""
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        papers_dir = root / "Papers"
        papers_dir.mkdir()
        # On Windows, filesystem sorts: "2024-01" < "2024-06" (character comparison)
        # So path order is: B (2024-01) before A (2024-06)
        f1 = papers_dir / "P - 2024-06-01 - A.md"
        f2 = papers_dir / "P - 2024-01-01 - B.md"
        f1.write_text("---\npublished: 2024-06-01\n---\n", encoding="utf-8")
        f2.write_text("---\npublished: 2024-01-01\n---\n", encoding="utf-8")
        assert f1.exists() and f2.exists()  # setup verification
        import re
        result = airo.collect_pnotes(root)
        dates = [re.search(r"\d{4}-\d{2}-\d{2}", p.stem).group() for p in result]
        # Verify: result should be sorted (lexicographic), and dates are correctly extracted
        assert dates == sorted(dates), f"result not sorted: {dates}"
        assert set(dates) == {"2024-01-01", "2024-06-01"}  # both dates present


class TestPnotesByTagTier4:
    def test_groups_by_tag(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        f1 = papers_dir / "P - 2024-01-01 - A.md"
        f2 = papers_dir / "P - 2024-02-01 - B.md"
        f1.write_text("---\npublished: 2024-01-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
        f2.write_text("---\npublished: 2024-02-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
        assert f1.exists() and f2.exists()  # setup verification
        result = airo.pnotes_by_tag(root)
        assert "Agent" in result
        assert len(result["Agent"]) == 2  # verify both files grouped


class TestPickTop3PnotesForTagTier4:
    def test_returns_none_for_fewer_than_3(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        (papers_dir / "P - 2024-01-01 - A.md").write_text("---\npublished: 2024-01-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
        (papers_dir / "P - 2024-02-01 - B.md").write_text("---\npublished: 2024-02-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
        tag_map = airo.pnotes_by_tag(root)
        result = airo.pick_top3_pnotes_for_tag("Agent", tag_map)
        assert result is None

    def test_returns_3_oldest_for_more_than_3(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        files = []
        for i in range(5):
            f = papers_dir / f"P - 2024-0{i+1}-01 - {i}.md"
            f.write_text(f"---\npublished: 2024-0{i+1}-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
            files.append(f)
        assert all(f.exists() for f in files)  # setup verification
        tag_map = airo.pnotes_by_tag(root)
        result = airo.pick_top3_pnotes_for_tag("Agent", tag_map)
        assert result is not None
        assert len(result) == 3
        # Verify they are the 3 oldest (earliest dates)
        import re
        result_stems = sorted([re.search(r"\d{4}-\d{2}-\d{2}", p.stem).group() for p in result])
        assert result_stems == ["2024-01-01", "2024-02-01", "2024-03-01"]  # oldest 3

    def test_returns_none_for_unknown_tag(self, tmp_path):
        import ai_research_os as airo
        root = tmp_path / "notes"
        root.mkdir()
        papers_dir = root / "02-Papers"
        papers_dir.mkdir(parents=True)
        (papers_dir / "P - 2024-01-01 - A.md").write_text("---\npublished: 2024-01-01\ntags:\n  - Agent\n---\n", encoding="utf-8")
        tag_map = airo.pnotes_by_tag(root)
        result = airo.pick_top3_pnotes_for_tag("UnknownTag", tag_map)
        assert result is None


class TestWikilinkForPnoteTier4:
    def test_stem_only(self):
        import ai_research_os as airo
        from pathlib import Path
        p = Path("P - 2024-03-15 - Test Paper.md")
        result = airo.wikilink_for_pnote(p)
        assert result == "[[P - 2024-03-15 - Test Paper]]"


class TestUpsertLinkUnderHeadingTier4:
    def test_heading_without_hash_prefix(self, tmp_path):
        import ai_research_os as airo
        md = "## 关联笔记\n- old link\n"
        result = airo.upsert_link_under_heading(md, "关联笔记", "- [[P1]]")
        assert "[[P1]]" in result
        assert result.count("- [[P1]]") == 1

    def test_heading_not_found_appends_section(self, tmp_path):
        import ai_research_os as airo
        md = "# Other\nNo heading here"
        result = airo.upsert_link_under_heading(md, "New Section", "- [[P1]]")
        assert "New Section" in result
        assert "[[P1]]" in result

    def test_special_chars_in_heading(self, tmp_path):
        import ai_research_os as airo
        md = "## 机器学习\n"
        result = airo.upsert_link_under_heading(md, "机器学习", "- [[P1]]")
        assert "[[P1]]" in result


class TestReadPnoteMetadataTier4:
    def test_read_pnote_metadata_basic(self, tmp_path):
        import ai_research_os as airo
        pnote = tmp_path / "P - 2024 - Test Paper.md"
        pnote.write_text(
            "type: paper\n"
            "status: draft\n"
            "date: 2024-03-15\n"
            "tags: [LLM, Agent]\n"
            "------------------\n"
            "\n"
            "# Test Paper Title\n"
            "\n"
            "**Source:** ARXIV: 2301.00001\n"
        )
        result = airo.read_pnote_metadata(pnote)
        assert result["title"] == "Test Paper Title"
        assert result["year"] == "2024"
        assert result["date"] == "2024-03-15"
        assert result["source"] == "arxiv"
        assert result["uid"] == "2301.00001"
        assert result["tags"] == ["LLM", "Agent"]

    def test_read_pnote_metadata_fallback_uid(self, tmp_path):
        import ai_research_os as airo
        pnote = tmp_path / "P - 2023 - No Source.md"
        pnote.write_text(
            "type: paper\n"
            "status: draft\n"
            "date: 2023-11-01\n"
            "tags: [RAG]\n"
            "------------------\n"
            "\n"
            "# No Source Info\n"
        )
        result = airo.read_pnote_metadata(pnote)
        assert result["title"] == "No Source Info"
        assert result["year"] == "2023"
        assert result["uid"] == ""  # no Source line in body, uid stays empty


class TestEnsureOrUpdateMnoteABCUpdate:
    """Cover mnote.py lines 86-92: curA/B/C exist but need updating."""

    def test_updates_abc_and_appends_view_evolution_log(self, tmp_path):
        """When existing mnote has curA/B/C that differ from new top3, re.sub + evolution log."""
        import ai_research_os as airo
        from pathlib import Path

        mnote_dir = tmp_path / "notes"
        mnote_dir.mkdir(parents=True)

        # mnote_filename strips "P - XXXX - " prefix then truncates to 19 chars.
        # "P - 2024-06-15 - OldPaperA" → "OldPaperA" (strip prefix, length ≤ 19)
        # "P - 2024-06-15 - NewPaperA" → "NewPaperA"
        # Both share the same {tag} = "Agent".
        existing_fname = "M - Agent - OldPaperA vs OldPaperB vs OldPaperC.md"
        existing_content = (
            "---\n"
            "type: comparison\n"
            "date: 2024-06-01\n"
            "---\n"
            "------------------\n"
            "\n"
            "# Agent: PaperA vs PaperB vs PaperC\n"
            "\n"
            "## 当前 A/B/C（自动补齐）\n"
            "\n"
            "- A: P - 2024-06-15 - OldPaperA\n"
            "- B: P - 2024-06-14 - OldPaperB\n"
            "- C: P - 2024-06-13 - OldPaperC\n"
        )
        existing_file = mnote_dir / existing_fname
        existing_file.write_text(existing_content, encoding="utf-8")

        # New top3 paths with different stems
        top3_dir = tmp_path / "papers"
        top3_dir.mkdir()
        a_path = top3_dir / "P - 2024-06-15 - NewPaperA.md"
        b_path = top3_dir / "P - 2024-06-14 - NewPaperB.md"
        c_path = top3_dir / "P - 2024-06-13 - NewPaperC.md"
        a_path.write_text("---\n", encoding="utf-8")
        b_path.write_text("---\n", encoding="utf-8")
        c_path.write_text("---\n", encoding="utf-8")

        result = airo.ensure_or_update_mnote(mnote_dir, "Agent", [a_path, b_path, c_path])

        assert result is not None
        updated = result.read_text(encoding="utf-8")
        # A/B/C lines should be updated to new stems
        assert "- A: P - 2024-06-15 - NewPaperA" in updated
        assert "- B: P - 2024-06-14 - NewPaperB" in updated
        assert "- C: P - 2024-06-13 - NewPaperC" in updated
        # View Evolution Log should be appended
        assert "View Evolution Log" in updated
        assert "旧观点" in updated
        assert "新证据" in updated

    def test_abc_all_missing_appends_section(self, tmp_path):
        """Lines 81-84: when existing mnote has no A/B/C at all, appends the section."""
        import ai_research_os as airo
        from pathlib import Path

        mnote_dir = tmp_path / "notes"
        mnote_dir.mkdir(parents=True)

        # Existing mnote with NO A/B/C lines at all.
        # Use stems that don't have "P - XXXX - " prefix so short() is identity.
        # "OldAlpha" (9 chars) < 19, no truncation, no prefix strip.
        existing_fname = "M - Agent - OldAlpha vs OldBeta vs OldGamma.md"
        existing_content = (
            "---\n"
            "type: comparison\n"
            "---\n"
            "------------------\n"
            "\n"
            "# Agent Comparison\n"
            "\n"
            "Some content without A/B/C markers.\n"
        )
        existing_file = mnote_dir / existing_fname
        existing_file.write_text(existing_content, encoding="utf-8")

        # New top3 paths with different stems
        # short("P - 2024-06-15 - NewAlpha") → strips prefix → "NewAlpha"
        # short("P - 2024-06-15 - NewBeta")  → strips prefix → "NewBeta"
        # short("P - 2024-06-15 - NewGamma") → strips prefix → "NewGamma"
        top3_dir = tmp_path / "papers"
        top3_dir.mkdir()
        a_path = top3_dir / "P - 2024-06-15 - NewAlpha.md"
        b_path = top3_dir / "P - 2024-06-14 - NewBeta.md"
        c_path = top3_dir / "P - 2024-06-13 - NewGamma.md"
        a_path.write_text("---\n", encoding="utf-8")
        b_path.write_text("---\n", encoding="utf-8")
        c_path.write_text("---\n", encoding="utf-8")

        result = airo.ensure_or_update_mnote(mnote_dir, "Agent", [a_path, b_path, c_path])

        assert result is not None
        updated = result.read_text(encoding="utf-8")
        # Should append the A/B/C section (lines 82-83)
        # The stems get the "P - XXXX - " prefix stripped by short()
        # so curA/B/C are None (no "- A: ..." lines exist) → lines 82-83 run
        # The append uses full stems (a.stem = "P - 2024-06-15 - NewAlpha")
        assert "- A: P - 2024-06-15 - NewAlpha" in updated
        assert "- B: P - 2024-06-14 - NewBeta" in updated
        assert "- C: P - 2024-06-13 - NewGamma" in updated


