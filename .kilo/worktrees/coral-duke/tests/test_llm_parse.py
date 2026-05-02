"""Tier 2 unit tests — llm/parse.py, pure functions, no I/O."""
from llm.parse import (
    _parse_rubric,
    _parse_rubric_json,
    _parse_sections,
    extract_rubric_scores,
    parse_ai_pnote_draft,
)


# =============================================================================
# _parse_rubric_json — malformed JSON fallback
# =============================================================================
class TestParseRubricJson:
    def test_parses_valid_json(self):
        result = _parse_rubric_json('{"novelty": 3, "leverage": 4}')
        assert result["novelty"] == 3
        assert result["leverage"] == 4

    def test_strips_trailing_comma(self):
        # LLM mistake: trailing comma before closing brace
        result = _parse_rubric_json('{"novelty": 3, "leverage": 4, }')
        assert result["novelty"] == 3
        assert result["leverage"] == 4

    def test_falls_back_to_regex_when_json_totally_broken(self):
        # No recognisable keys at all
        result = _parse_rubric_json("totally broken no keys here")
        assert result == {}

    def test_extracts_overall_from_json(self):
        result = _parse_rubric_json('{"novelty": 5, "overall": "Strong Accept"}')
        assert result["novelty"] == 5
        assert result["overall"] == "Strong Accept"

    def test_returns_empty_dict_for_completely_broken_json(self):
        # No recognisable keys at all
        result = _parse_rubric_json("this is not json at all")
        assert result == {}

    def test_extracts_all_score_keys(self):
        raw = '{"novelty": 1, "leverage": 2, "evidence": 3, "cost": 4, "moat": 5, "adoption": 3}'
        result = _parse_rubric_json(raw)
        assert result["novelty"] == 1
        assert result["leverage"] == 2
        assert result["evidence"] == 3
        assert result["cost"] == 4
        assert result["moat"] == 5
        assert result["adoption"] == 3


# =============================================================================
# _parse_rubric — fallback chain
# =============================================================================
class TestParseRubric:
    def test_extracts_from_xml_block(self):
        raw = '''<!--
<RUBRIC>
{"novelty": 4, "leverage": 5, "evidence": 3}
</RUBRIC>
-->'''
        result = _parse_rubric(raw)
        assert result["novelty"] == 4
        assert result["leverage"] == 5
        assert result["evidence"] == 3

    def test_falls_back_to_json_object_anywhere_in_text(self):
        raw = 'Some text {"novelty": 2, "leverage": 3} more text'
        result = _parse_rubric(raw)
        assert result["novelty"] == 2
        assert result["leverage"] == 3

    def test_falls_back_to_individual_score_lines(self):
        raw = "Novelty (1-5): 3\nLeverage: 4\nEvidence: 5"
        result = _parse_rubric(raw)
        assert result["novelty"] == 3
        assert result["leverage"] == 4
        assert result["evidence"] == 5

    def test_handles_score_with_parenthetical_range(self):
        # "novelty: 3 (1-5)" → should extract 3
        raw = "novelty: 3 (1-5)\nleverage: 4 (1-5)\n"
        result = _parse_rubric(raw)
        assert result["novelty"] == 3
        assert result["leverage"] == 4

    def test_handles_score_with_score_in_parentheses_after_keyword(self):
        # "* Novelty (1-5): 5" — closing paren before colon
        raw = "* Novelty (1-5): 5\n* Leverage (1-5): 4\n"
        result = _parse_rubric(raw)
        assert result["novelty"] == 5
        assert result["leverage"] == 4

    def test_handles_single_quoted_json_values(self):
        # LLM sometimes emits single-quoted JSON: "novelty": '3'
        raw = '{"novelty": \'3\', "leverage": "4"}'
        result = _parse_rubric(raw)
        assert result["novelty"] == 3
        assert result["leverage"] == 4

    def test_extracts_overall_judgment(self):
        raw = "novelty: 3\nOverall judgment: This is a strong paper accept"
        result = _parse_rubric(raw)
        assert result["overall"] == "This is a strong paper accept"

    def test_extracts_overall_judgment_chinese_colon(self):
        raw = "novelty: 3\nOverall judgment：Borderline Accept"
        result = _parse_rubric(raw)
        assert result["overall"] == "Borderline Accept"

    def test_returns_empty_dict_when_nothing_found(self):
        raw = "This text has no rubric scores at all"
        result = _parse_rubric(raw)
        assert result == {}

    def test_mixed_block_and_fallback_prefers_block(self):
        # XML block is parsed first (primary path)
        raw = '''<!--
<RUBRIC>
{"novelty": 9}
</RUBRIC>
-->
novelty: 3 (1-5)
'''
        result = _parse_rubric(raw)
        assert result["novelty"] == 9  # block value wins, not fallback

    def test_case_insensitive_key_matching(self):
        raw = '"NOVELTY": 3\n"LEVERAGE": 4'
        result = _parse_rubric(raw)
        assert result["novelty"] == 3
        assert result["leverage"] == 4

    def test_only_first_score_per_key_is_used(self):
        # After finding first occurrence, should not keep searching
        raw = "novelty: 3\nnovelty: 9\nleverage: 4"
        result = _parse_rubric(raw)
        assert result["novelty"] == 3  # first value wins
        assert result["leverage"] == 4


# =============================================================================
# _parse_sections — no headings
# =============================================================================
class TestParseSections:
    def test_empty_dict_when_no_headings(self):
        result = _parse_sections("No headings here just plain text")
        assert result == {}

    def test_empty_input(self):
        result = _parse_sections("")
        assert result == {}

    def test_single_heading(self):
        raw = "## 1. 背景\nSome content here\nwith multiple lines"
        result = _parse_sections(raw)
        assert "## 1. 背景" in result
        assert result["## 1. 背景"] == "Some content here\nwith multiple lines"

    def test_multiple_sections_split_correctly(self):
        raw = """## 1. 背景
Background content

## 2. 核心问题
Problem content

## 3. 方法结构
Method content"""
        result = _parse_sections(raw)
        assert "## 1. 背景" in result
        assert "## 2. 核心问题" in result
        assert "## 3. 方法结构" in result
        assert "Background content" in result["## 1. 背景"]
        assert "Problem content" in result["## 2. 核心问题"]
        assert "Method content" in result["## 3. 方法结构"]

    def test_extracts_subsection_as_separate_key(self):
        raw = """## 3. 方法结构
Parent content

### 3.1 架构拆解
Architecture content"""
        result = _parse_sections(raw)
        assert "## 3. 方法结构" in result
        assert "## 3.1 架构拆解" in result
        # Parent should NOT contain subsection content
        assert "Architecture content" not in result["## 3. 方法结构"]
        assert "Parent content" in result["## 3. 方法结构"]
        assert result["## 3.1 架构拆解"] == "Architecture content"

    def test_multiple_subsections_extracted(self):
        raw = """## 5. 实验分析
Parent analysis

### 5.1 数据集
Dataset content

### 5.2 基线对比
Baseline content

### 5.3 消融实验
Ablation content"""
        result = _parse_sections(raw)
        assert "## 5. 实验分析" in result
        assert "## 5.1 数据集" in result
        assert "## 5.2 基线对比" in result
        assert "## 5.3 消融实验" in result
        assert "Dataset content" in result["## 5.1 数据集"]
        assert "Baseline content" in result["## 5.2 基线对比"]
        # Subsections stripped from parent
        assert "Dataset content" not in result["## 5. 实验分析"]

    def test_heading_stripping_from_content(self):
        # The heading line itself should not appear in content
        raw = "## 1. 背景\nLine after heading"
        result = _parse_sections(raw)
        assert result["## 1. 背景"] == "Line after heading"

    def test_subsection_content_with_newlines(self):
        raw = """## 3. 方法结构
Parent

### 3.1 架构拆解
Line 1
Line 2
Line 3"""
        result = _parse_sections(raw)
        assert result["## 3.1 架构拆解"] == "Line 1\nLine 2\nLine 3"


# =============================================================================
# extract_rubric_scores
# =============================================================================
class TestExtractRubricScores:
    def test_returns_only_valid_scores(self):
        rubric = {
            "novelty": 3,
            "leverage": 6,   # out of range
            "evidence": 1,
            "cost": 0,       # out of range
            "moat": 4,
            "adoption": 5,
        }
        result = extract_rubric_scores(rubric)
        assert result == {"novelty": 3, "evidence": 1, "moat": 4, "adoption": 5}

    def test_returns_empty_dict_for_empty_rubric(self):
        assert extract_rubric_scores({}) == {}

    def test_ignores_non_integer_values(self):
        rubric = {
            "novelty": "three",   # string, not int
            "leverage": 4.5,      # float, not int
            "evidence": 3,
        }
        result = extract_rubric_scores(rubric)
        assert result == {"evidence": 3}

    def test_boundary_values_1_and_5_accepted(self):
        rubric = {"novelty": 1, "leverage": 5}
        result = extract_rubric_scores(rubric)
        assert result == {"novelty": 1, "leverage": 5}

    def test_ignores_overall_string_score(self):
        rubric = {"novelty": 3, "overall": "Strong Accept"}
        result = extract_rubric_scores(rubric)
        assert "overall" not in result
        assert result["novelty"] == 3


# =============================================================================
# parse_ai_pnote_draft — integration
# =============================================================================
class TestParseAiPnoteDraft:
    def test_returns_sections_rubric_and_raw(self):
        raw = """## 1. 背景
Background text

## 2. 核心问题
Problem text

<!--
<RUBRIC>
{"novelty": 4, "leverage": 5}
</RUBRIC>
-->
"""
        sections, rubric, raw_md = parse_ai_pnote_draft(raw)
        assert isinstance(sections, dict)
        assert isinstance(rubric, dict)
        assert isinstance(raw_md, str)
        assert raw_md == raw
        assert rubric["novelty"] == 4
        assert rubric["leverage"] == 5

    def test_raw_returned_unchanged(self):
        raw = "## 1. 背景\nContent"
        _, _, raw_md = parse_ai_pnote_draft(raw)
        assert raw_md is raw  # same object, not a copy

    def test_empty_rubric_when_no_scores(self):
        raw = "## 1. 背景\nJust content"
        _, rubric, _ = parse_ai_pnote_draft(raw)
        assert rubric == {}
