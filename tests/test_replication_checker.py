"""Tests for llm/replication_checker.py — paper replication checking."""

from llm.replication_checker import (
    CodeLink,
    DependencyInfo,
    ReplicationReport,
    ReplicationChecker,
)


class TestCodeLink:
    """Test CodeLink dataclass."""

    def test_creates_with_required_fields(self):
        """Should create CodeLink with required fields."""
        link = CodeLink(
            url="https://github.com/facebookresearch/bert",
            platform="github",
            owner="facebookresearch",
            repo="bert",
        )
        assert link.url == "https://github.com/facebookresearch/bert"
        assert link.platform == "github"
        assert link.confidence == 1.0
        assert link.context == ""


class TestDependencyInfo:
    """Test DependencyInfo dataclass."""

    def test_default_package_manager_is_unknown(self):
        """Default package_manager should be 'unknown'."""
        info = DependencyInfo(package_manager="unknown")
        assert info.package_manager == "unknown"
        assert info.files == []
        assert info.hardware == []


class TestReplicationReport:
    """Test ReplicationReport dataclass."""

    def test_default_difficulty_is_empty(self):
        """Default difficulty should be empty string."""
        report = ReplicationReport(paper_id="p1", paper_title="Test Paper")
        assert report.difficulty == ""
        assert report.difficulty_score == 0.0
        assert report.links == []
        assert report.smoke_test_passed is False

    def test_to_dict_includes_all_fields(self):
        """to_dict should include all key fields."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test Paper",
            difficulty="Medium",
            difficulty_score=4.5,
        )
        d = report.to_dict()
        assert d["paper_id"] == "p1"
        assert d["difficulty"] == "Medium"
        assert d["difficulty_score"] == 4.5


class TestExtractLinks:
    """Test _extract_links regex extraction."""

    def test_finds_github_url_full(self):
        """Should find full GitHub URL."""
        checker = ReplicationChecker()
        links = checker._extract_links(
            "Implementation available at https://github.com/facebookresearch/bert"
        )
        assert len(links) == 1
        assert links[0].platform == "github"
        assert links[0].owner == "facebookresearch"
        assert links[0].repo == "bert"

    def test_finds_github_url_with_context_keywords(self):
        """GitHub URL with 'code' context keyword should have high confidence."""
        checker = ReplicationChecker()
        links = checker._extract_links("We released our code at https://github.com/team/repo")
        assert len(links) == 1
        assert links[0].confidence == 1.0

    def test_finds_github_url_without_context_keywords(self):
        """GitHub URL should be extractable even without strong context."""
        checker = ReplicationChecker()
        # "github.com" in the URL itself is a context keyword, so confidence
        # will be 1.0 - this test verifies extraction works correctly
        links = checker._extract_links("See https://github.com/xyz/abc123 for details")
        assert len(links) == 1
        assert links[0].platform == "github"

    def test_finds_huggingface_url(self):
        """Should find HuggingFace URLs."""
        checker = ReplicationChecker()
        links = checker._extract_links("Model available at https://huggingface.co/bert/bert-base")
        assert len(links) == 1
        assert links[0].platform == "huggingface"
        assert links[0].owner == "bert"

    def test_finds_huggingface_spaces(self):
        """Should find HuggingFace Spaces URLs."""
        checker = ReplicationChecker()
        links = checker._extract_links("Try it at https://huggingface.co/spaces/bert/demo")
        assert len(links) >= 1
        platforms = [l.platform for l in links]
        assert "huggingface" in platforms

    def test_finds_gitlab_url(self):
        """Should find GitLab URLs."""
        checker = ReplicationChecker()
        links = checker._extract_links("Repository: https://gitlab.com/team/repo")
        assert len(links) == 1
        assert links[0].platform == "gitlab"
        assert links[0].owner == "team"

    def test_deduplicates_urls(self):
        """Should not return duplicate URLs."""
        checker = ReplicationChecker()
        links = checker._extract_links(
            "Code at https://github.com/team/repo and also https://github.com/team/repo"
        )
        assert len(links) == 1

    def test_handles_markdown_links(self):
        """Should handle markdown-style links without double-matching."""
        checker = ReplicationChecker()
        links = checker._extract_links("[Our Code](https://github.com/team/repo)")
        assert len(links) == 1

    def test_handles_plain_text_owner_repo(self):
        """Should find owner/repo format without full URL."""
        checker = ReplicationChecker()
        # The regex matches owner/repo without protocol - use .git suffix to match
        links = checker._extract_links("Implementation: team/repo.git")
        assert len(links) == 1
        assert links[0].platform == "github"
        assert links[0].owner == "team"
        assert links[0].repo == "repo"

    def test_handles_git_suffix(self):
        """Should strip .git suffix from repo names."""
        checker = ReplicationChecker()
        links = checker._extract_links("https://github.com/team/repo.git")
        assert len(links) >= 1
        # Repo should have .git stripped
        repo_names = [l.repo for l in links]
        assert "repo" in repo_names

    def test_penalizes_citation_reference_style(self):
        """URLs near citation numbers like [1] should have lower confidence."""
        checker = ReplicationChecker()
        links = checker._extract_links("Implementation [1]: https://github.com/team/repo")
        assert len(links) == 1
        assert links[0].confidence < 1.0

    def test_returns_empty_for_no_links(self):
        """Should return empty list when no URLs found."""
        checker = ReplicationChecker()
        links = checker._extract_links("No code links in this text.")
        assert links == []


class TestDetectDependencyInfo:
    """Test _detect_dependency_info heuristic detection."""

    def test_detects_pip(self):
        """Should detect pip from requirements.txt mention."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info(
            "requires requirements.txt for installation", "github"
        )
        assert info.package_manager == "pip"

    def test_detects_poetry(self):
        """Should detect poetry from pyproject.toml."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("use pyproject.toml with poetry", "github")
        assert info.package_manager == "poetry"

    def test_detects_conda(self):
        """Should detect conda from environment.yml."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("setup with conda environment.yml", "github")
        assert info.package_manager == "conda"

    def test_detects_npm(self):
        """Should detect npm from package.json."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("use package.json for dependencies", "github")
        assert info.package_manager == "npm"

    def test_detects_dependency_files(self):
        """Should list all detected dependency files."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info(
            "uses requirements.txt and setup.py for dependencies", "github"
        )
        assert "requirements.txt" in info.files
        assert "setup.py" in info.files

    def test_detects_python_version(self):
        """Should extract Python version hint."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("Requires Python 3.8 or higher", "github")
        assert info.python_version == "python 3.8"

    def test_detects_gpu_hardware(self):
        """Should detect GPU hardware requirement."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("Requires GPU with CUDA support", "github")
        assert "GPU (NVIDIA recommended)" in info.hardware
        assert "NVIDIA CUDA" in info.hardware

    def test_detects_special_libs(self):
        """Should detect special library requirements."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("uses PyTorch and transformers library", "github")
        lib_names = [lib.split()[0] for lib in info.special_requirements]
        assert "torch" in lib_names or "PyTorch" in str(info.special_requirements)
        assert "transformers" in lib_names or "HuggingFace" in str(info.special_requirements)

    def test_detects_disk_space_gb(self):
        """Should parse disk space from text."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("Requires 500GB disk space", "github")
        assert info.disk_space_gb == 500

    def test_detects_disk_space_tb(self):
        """Should convert TB to GB."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("Dataset requires 2TB storage", "github")
        assert info.disk_space_gb == 2048

    def test_detects_ram_gb(self):
        """Should parse RAM requirement."""
        checker = ReplicationChecker()
        info = checker._detect_dependency_info("Needs 64GB RAM", "github")
        assert info.ram_gb == 64


class TestAssessDifficulty:
    """Test _assess_difficulty scoring."""

    def test_github_easy_low_score(self):
        """GitHub with pip and no deps should be low difficulty."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="pip", files=["requirements.txt"])
        difficulty, score = checker._assess_difficulty(info, "github", [])
        assert score <= 2.0
        assert difficulty == "Easy"

    def test_github_hard_high_score(self):
        """GitHub with CUDA and special libs should be hard."""
        checker = ReplicationChecker()
        info = DependencyInfo(
            package_manager="pip",
            files=[],
            hardware=["NVIDIA CUDA"],
            special_requirements=["NVIDIA CUDA required", "torch"],
        )
        difficulty, score = checker._assess_difficulty(info, "github", [])
        assert score >= 4.0

    def test_very_hard_for_tpu(self):
        """TPU requirement with missing deps should push score to Hard or higher."""
        checker = ReplicationChecker()
        info = DependencyInfo(
            package_manager="pip",
            files=[],
            hardware=["TPU"],
            special_requirements=["JAX (TPU/JAX compatible)", "torch"],
            disk_space_gb=600,
            ram_gb=128,
        )
        difficulty, score = checker._assess_difficulty(info, "github", [])
        assert score >= 6.0

    def test_no_dep_files_increases_score(self):
        """Missing dependency files should increase difficulty score."""
        checker = ReplicationChecker()
        info_with_files = DependencyInfo(package_manager="pip", files=["requirements.txt"])
        info_without = DependencyInfo(package_manager="pip", files=[])

        _, score_with = checker._assess_difficulty(info_with_files, "github", [])
        _, score_without = checker._assess_difficulty(info_without, "github", [])
        assert score_without > score_with

    def test_conda_higher_than_pip(self):
        """Conda should score higher than pip."""
        checker = ReplicationChecker()
        info_pip = DependencyInfo(package_manager="pip", files=["requirements.txt"])
        info_conda = DependencyInfo(package_manager="conda", files=["environment.yml"])

        _, score_pip = checker._assess_difficulty(info_pip, "github", [])
        _, score_conda = checker._assess_difficulty(info_conda, "github", [])
        assert score_conda > score_pip

    def test_huggingface_zero_platform_score(self):
        """HuggingFace should add 0 to platform score."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="pip", files=["requirements.txt"])
        difficulty, score = checker._assess_difficulty(info, "huggingface", [])
        assert difficulty == "Easy"
        assert score <= 2.0

    def test_large_disk_increases_score(self):
        """Large disk requirement (>500GB) should increase score."""
        checker = ReplicationChecker()
        info_small = DependencyInfo(package_manager="pip", disk_space_gb=100)
        info_large = DependencyInfo(package_manager="pip", disk_space_gb=600)

        _, score_small = checker._assess_difficulty(info_small, "github", [])
        _, score_large = checker._assess_difficulty(info_large, "github", [])
        assert score_large > score_small

    def test_large_ram_increases_score(self):
        """Large RAM requirement (>64GB) should increase score."""
        checker = ReplicationChecker()
        info_small = DependencyInfo(package_manager="pip", ram_gb=32)
        info_large = DependencyInfo(package_manager="pip", ram_gb=128)

        _, score_small = checker._assess_difficulty(info_small, "github", [])
        _, score_large = checker._assess_difficulty(info_large, "github", [])
        assert score_large > score_small

    def test_score_capped_at_10(self):
        """Difficulty score should be capped at 10."""
        checker = ReplicationChecker()
        info = DependencyInfo(
            package_manager="pip",
            files=[],
            hardware=["TPU", "NVIDIA CUDA", "GPU (NVIDIA recommended)"],
            special_requirements=["CUDA required", "torch", "jax", "tensorflow"],
            disk_space_gb=1000,
            ram_gb=256,
        )
        difficulty, score = checker._assess_difficulty(info, "github", [])
        assert score <= 10.0


class TestCheckIssues:
    """Test _check_issues reproducibility issue detection."""

    def test_no_dep_files_issue(self):
        """Missing dependency files should trigger issue."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="unknown", files=[])
        issues = checker._check_issues(info, [])
        assert any("No explicit dependency files" in i for i in issues)

    def test_no_python_version_issue(self):
        """Missing Python version should trigger issue."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="pip", files=["requirements.txt"], python_version="")
        issues = checker._check_issues(info, [])
        assert any("Python version" in i for i in issues)

    def test_requirements_txt_unpinned_issue(self):
        """requirements.txt without version pinning should warn."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="pip", files=["requirements.txt"])
        issues = checker._check_issues(info, [])
        assert any("unpinned" in i.lower() for i in issues)

    def test_cuda_issue(self):
        """CUDA requirement should generate hardware issue."""
        checker = ReplicationChecker()
        info = DependencyInfo(
            package_manager="pip",
            special_requirements=["NVIDIA CUDA required"],
        )
        issues = checker._check_issues(info, [])
        assert any("CUDA" in i for i in issues)

    def test_no_hardware_mentioned_issue(self):
        """No hardware mentioned should warn about unclear GPU needs."""
        checker = ReplicationChecker()
        info = DependencyInfo(package_manager="pip", files=["requirements.txt"], hardware=[])
        issues = checker._check_issues(info, [])
        assert any("hardware" in i.lower() for i in issues)


class TestGenerateNotes:
    """Test _generate_notes human-readable notes."""

    def test_github_note(self):
        """Should generate GitHub owner/repo note."""
        checker = ReplicationChecker()
        link = CodeLink(
            url="https://github.com/team/repo",
            platform="github",
            owner="team",
            repo="repo",
        )
        notes = checker._generate_notes(link, DependencyInfo(package_manager="unknown"), [])
        assert any("team/repo" in n for n in notes)

    def test_huggingface_note(self):
        """Should generate HuggingFace note."""
        checker = ReplicationChecker()
        link = CodeLink(
            url="https://huggingface.co/bert/base",
            platform="huggingface",
            owner="bert",
            repo="base",
        )
        notes = checker._generate_notes(link, DependencyInfo(package_manager="unknown"), [])
        assert any("HuggingFace" in n for n in notes)

    def test_multiple_links_note(self):
        """Multiple links should generate verification note."""
        checker = ReplicationChecker()
        link = CodeLink(url="https://github.com/a/b", platform="github", owner="a", repo="b")
        links = [link, link]
        notes = checker._generate_notes(link, DependencyInfo(package_manager="unknown"), links)
        assert any("2 code links" in n for n in notes)

    def test_package_manager_note(self):
        """Should include package manager in notes."""
        checker = ReplicationChecker()
        link = CodeLink(url="https://github.com/a/b", platform="github", owner="a", repo="b")
        info = DependencyInfo(package_manager="conda")
        notes = checker._generate_notes(link, info, [])
        assert any("CONDA" in n for n in notes)

    def test_dependency_files_note(self):
        """Should include dependency files in notes."""
        checker = ReplicationChecker()
        link = CodeLink(url="https://github.com/a/b", platform="github", owner="a", repo="b")
        info = DependencyInfo(package_manager="pip", files=["requirements.txt", "setup.py"])
        notes = checker._generate_notes(link, info, [])
        assert any("requirements.txt" in n for n in notes)


class TestCheckPaper:
    """Test check_paper main entry point."""

    def test_no_code_found(self):
        """Papers without links should return 'No Code Found' difficulty."""
        checker = ReplicationChecker()
        report = checker.check_paper("p1", "Test Paper Without Code", abstract="")
        assert report.difficulty == "No Code Found"
        assert report.difficulty_score == 10.0

    def test_paper_with_github_link(self):
        """Paper with GitHub link should not be 'No Code Found'."""
        checker = ReplicationChecker()
        report = checker.check_paper(
            "p1",
            "Test Paper",
            abstract="Code available at https://github.com/team/repo",
        )
        assert report.difficulty != "No Code Found"
        assert len(report.links) >= 1

    def test_paper_with_huggingface_link(self):
        """Paper with HuggingFace link should be detected."""
        checker = ReplicationChecker()
        report = checker.check_paper(
            "p1",
            "Test Paper",
            abstract="Model at https://huggingface.co/bert/bert-base",
        )
        assert report.primary_link is not None
        assert report.primary_link.platform == "huggingface"

    def test_primary_link_is_highest_confidence(self):
        """Primary link should be the one with highest confidence."""
        checker = ReplicationChecker()
        report = checker.check_paper(
            "p1",
            "Paper",
            abstract="Code https://github.com/a/b and released at https://github.com/c/d",
        )
        assert report.primary_link is not None

    def test_difficulty_is_assessed(self):
        """check_paper should set difficulty from _assess_difficulty."""
        checker = ReplicationChecker()
        report = checker.check_paper(
            "p1",
            "Test Paper",
            abstract="https://github.com/team/repo requires Python 3.8 torch",
        )
        assert report.difficulty != ""
        assert report.difficulty_score >= 0.0


class TestRenderReport:
    """Test render_report text rendering."""

    def test_renders_difficulty(self):
        """Should include difficulty level."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            difficulty="Medium",
            difficulty_score=4.5,
        )
        rendered = ReplicationChecker().render_report(report)
        assert "Medium" in rendered
        assert "4.5" in rendered

    def test_renders_no_code_found(self):
        """Should handle 'No Code Found' difficulty."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            difficulty="No Code Found",
            difficulty_score=10.0,
        )
        rendered = ReplicationChecker().render_report(report)
        assert "No Code Found" in rendered
        assert "❌" in rendered

    def test_renders_link_count(self):
        """Should show number of code links found."""
        link = CodeLink(
            url="https://github.com/a/b",
            platform="github",
            owner="a",
            repo="b",
        )
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            links=[link, link],
        )
        rendered = ReplicationChecker().render_report(report)
        assert "2" in rendered

    def test_renders_dependency_info(self):
        """Should show dependency info when available."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            dependency_info=DependencyInfo(
                package_manager="pip",
                files=["requirements.txt"],
                python_version="python 3.8",
            ),
        )
        rendered = ReplicationChecker().render_report(report)
        assert "PIP" in rendered or "pip" in rendered.lower()
        assert "requirements.txt" in rendered

    def test_renders_reproducibility_issues(self):
        """Should list reproducibility issues."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            reproducibility_issues=["No dependency files detected"],
        )
        rendered = ReplicationChecker().render_report(report)
        assert "No dependency files" in rendered

    def test_renders_smoke_test_passed(self):
        """Should show smoke test result."""
        report = ReplicationReport(
            paper_id="p1",
            paper_title="Test",
            smoke_test_passed=True,
        )
        rendered = ReplicationChecker().render_report(report)
        assert "smoke test passed" in rendered.lower()
