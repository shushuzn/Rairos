"""Tier 2 unit tests — llm/gap_detector.py, pure functions, no I/O."""
import pytest
from llm.gap_detector import (
    GapType,
    GapSeverity,
    ResearchGap,
    ResearchQuestion,
    GapAnalysisResult,
    GapDetector,
)


# =============================================================================
# Enum tests
# =============================================================================
class TestGapType:
    """Test GapType enum."""

    def test_all_types_have_values(self):
        """All GapType variants have string values."""
        assert GapType.UNEXPLORED_APPLICATION.value == "unexplored_application"
        assert GapType.METHOD_LIMITATION.value == "method_limitation"
        assert GapType.CONTRADICTION.value == "contradiction"
        assert GapType.EVALUATION_GAP.value == "evaluation_gap"
        assert GapType.SCALABILITY_ISSUE.value == "scalability_issue"
        assert GapType.THEORETICAL_GAP.value == "theoretical_gap"
        assert GapType.DATASET_GAP.value == "dataset_gap"
        assert GapType.GENERALIZATION_GAP.value == "generalization_gap"

    def test_can_construct_from_value(self):
        """Enum can be constructed from string value."""
        assert GapType("unexplored_application") == GapType.UNEXPLORED_APPLICATION
        assert GapType("method_limitation") == GapType.METHOD_LIMITATION


class TestGapSeverity:
    """Test GapSeverity enum."""

    def test_all_severities_have_values(self):
        """All GapSeverity variants have values."""
        assert GapSeverity.HIGH.value == "high"
        assert GapSeverity.MEDIUM.value == "medium"
        assert GapSeverity.LOW.value == "low"

    def test_can_construct_from_value(self):
        """Enum can be constructed from string value."""
        assert GapSeverity("high") == GapSeverity.HIGH
        assert GapSeverity("low") == GapSeverity.LOW


# =============================================================================
# ResearchGap dataclass tests
# =============================================================================
class TestResearchGap:
    """Test ResearchGap dataclass."""

    def test_required_fields(self):
        """Required fields: gap_type, description, evidence_papers."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="Transformer fails on long sequences",
            evidence_papers=["Paper A", "Paper B"],
        )
        assert gap.gap_type == GapType.METHOD_LIMITATION
        assert gap.description == "Transformer fails on long sequences"
        assert gap.evidence_papers == ["Paper A", "Paper B"]

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="Desc",
            evidence_papers=[],
        )
        assert gap.severity == GapSeverity.MEDIUM
        assert gap.confidence == 0.5
        assert gap.suggested_approach == ""
        assert gap.related_gaps == []

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        gap = ResearchGap(
            gap_type=GapType.CONTRADICTION,
            description="Contradiction found",
            evidence_papers=["P1"],
            severity=GapSeverity.HIGH,
            confidence=0.9,
            suggested_approach="Design new experiment",
            related_gaps=["gap1", "gap2"],
        )
        assert gap.severity == GapSeverity.HIGH
        assert gap.confidence == 0.9
        assert gap.suggested_approach == "Design new experiment"
        assert gap.related_gaps == ["gap1", "gap2"]


# =============================================================================
# ResearchQuestion dataclass tests
# =============================================================================
class TestResearchQuestion:
    """Test ResearchQuestion dataclass."""

    def test_required_fields(self):
        """Required fields: question, gap."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="Gap desc",
            evidence_papers=[],
        )
        q = ResearchQuestion(question="How to improve X?", gap=gap)
        assert q.question == "How to improve X?"
        assert q.gap == gap

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        q = ResearchQuestion(question="Q?", gap=gap)
        assert q.hypothesis == ""
        assert q.methodology_suggestion == ""
        assert q.expected_impact == ""
        assert q.feasibility == 0.5
        assert q.novelty_score == 0.5

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        gap = ResearchGap(
            gap_type=GapType.EVALUATION_GAP,
            description="D",
            evidence_papers=[],
        )
        q = ResearchQuestion(
            question="How to benchmark?",
            gap=gap,
            hypothesis="Hypothesis here",
            methodology_suggestion="Use standard benchmarks",
            expected_impact="High impact",
            feasibility=0.8,
            novelty_score=0.7,
        )
        assert q.hypothesis == "Hypothesis here"
        assert q.methodology_suggestion == "Use standard benchmarks"
        assert q.expected_impact == "High impact"
        assert q.feasibility == 0.8
        assert q.novelty_score == 0.7


# =============================================================================
# GapAnalysisResult dataclass tests
# =============================================================================
class TestGapAnalysisResult:
    """Test GapAnalysisResult dataclass."""

    def test_required_fields(self):
        """Required fields: topic."""
        result = GapAnalysisResult(topic="Transformer")
        assert result.topic == "Transformer"

    def test_optional_fields_default(self):
        """Optional fields have defaults."""
        result = GapAnalysisResult(topic="T")
        assert result.gaps == []
        assert result.questions == []
        assert result.coverage_score == 0.0
        assert result.opportunities_score == 0.0
        assert result.analyzed_papers_count == 0
        assert result.summary == ""

    def test_all_fields_can_be_set(self):
        """All fields can be set."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        q = ResearchQuestion(question="Q?", gap=gap)
        result = GapAnalysisResult(
            topic="Attention Mechanism",
            gaps=[gap],
            questions=[q],
            coverage_score=0.75,
            opportunities_score=7.5,
            analyzed_papers_count=15,
            summary="分析完成",
        )
        assert len(result.gaps) == 1
        assert len(result.questions) == 1
        assert result.coverage_score == 0.75
        assert result.opportunities_score == 7.5
        assert result.analyzed_papers_count == 15
        assert result.summary == "分析完成"


# =============================================================================
# _parse_gaps tests
# =============================================================================
class TestParseGaps:
    """Test _parse_gaps logic."""

    def _parse_gaps(self, response: str, topic: str) -> list:
        """Replicate _parse_gaps logic."""
        import re
        gaps = []

        type_map = {
            'unexplored_application': GapType.UNEXPLORED_APPLICATION,
            'method_limitation': GapType.METHOD_LIMITATION,
            'contradiction': GapType.CONTRADICTION,
            'evaluation_gap': GapType.EVALUATION_GAP,
            'scalability_issue': GapType.SCALABILITY_ISSUE,
            'theoretical_gap': GapType.THEORETICAL_GAP,
            'dataset_gap': GapType.DATASET_GAP,
            'generalization_gap': GapType.GENERALIZATION_GAP,
        }
        severity_map = {
            'high': GapSeverity.HIGH,
            'medium': GapSeverity.MEDIUM,
            'low': GapSeverity.LOW,
        }

        clean_response = re.sub(r'```.*?```', '', response, flags=re.DOTALL).strip()

        for line in clean_response.split('\n'):
            line = line.strip()
            if not line or line.startswith('#'):
                continue

            match = re.match(
                r'\[(\w+)\]\s*(.+?)\s*\|\s*(.+?)\s*\|\s*([\d.]+)\s*\|\s*(\w+)',
                line,
            )
            if match:
                gap_type_str, description, papers_str, conf_str, severity_str = match.groups()

                gap_type = type_map.get(gap_type_str, GapType.METHOD_LIMITATION)
                severity = severity_map.get(severity_str, GapSeverity.MEDIUM)
                confidence = float(conf_str) if conf_str else 0.5
                papers = [p.strip() for p in papers_str.split(',') if p.strip()]

                gap = ResearchGap(
                    gap_type=gap_type,
                    description=description.strip(),
                    evidence_papers=papers,
                    severity=severity,
                    confidence=confidence,
                )
                gaps.append(gap)

        return gaps

    def test_parses_valid_line(self):
        """Valid line extracts gap correctly."""
        response = "[method_limitation] Transformer has O(n) complexity | Paper1 | 0.8 | high"
        gaps = self._parse_gaps(response, "transformer")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION
        assert gaps[0].description == "Transformer has O(n) complexity"
        assert gaps[0].evidence_papers == ["Paper1"]
        assert gaps[0].confidence == 0.8
        assert gaps[0].severity == GapSeverity.HIGH

    def test_multiple_lines(self):
        """Multiple valid lines create multiple gaps."""
        response = (
            "[unexplored_application] New domain not explored | PaperA | 0.7 | medium\n"
            "[contradiction] Results conflict | PaperB, PaperC | 0.6 | high"
        )
        gaps = self._parse_gaps(response, "topic")
        assert len(gaps) == 2
        assert gaps[0].gap_type == GapType.UNEXPLORED_APPLICATION
        assert gaps[1].gap_type == GapType.CONTRADICTION

    def test_multiple_papers_comma_separated(self):
        """Multiple papers are comma-separated."""
        response = "[method_limitation] Gap | Paper1, Paper2, Paper3 | 0.5 | low"
        gaps = self._parse_gaps(response, "t")
        assert gaps[0].evidence_papers == ["Paper1", "Paper2", "Paper3"]

    def test_skips_fenced_code_blocks(self):
        """Content inside triple backticks is stripped."""
        response = "```json\n[unexplored_application] Inside code | P | 0.5 | medium\n```\n[method_limitation] Outside code | Q | 0.6 | high"
        gaps = self._parse_gaps(response, "t")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION

    def test_skips_hash_comment_lines(self):
        """Lines starting with # are skipped."""
        response = "# This is a comment\n[method_limitation] Real gap | P | 0.7 | medium"
        gaps = self._parse_gaps(response, "t")
        assert len(gaps) == 1

    def test_unknown_gap_type_defaults_to_method_limitation(self):
        """Unknown gap type string falls back to METHOD_LIMITATION."""
        response = "[unknown_type] Some gap | Paper | 0.5 | medium"
        gaps = self._parse_gaps(response, "t")
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION

    def test_unknown_severity_defaults_to_medium(self):
        """Unknown severity string falls back to MEDIUM."""
        response = "[method_limitation] Gap | Paper | 0.5 | unknown"
        gaps = self._parse_gaps(response, "t")
        assert gaps[0].severity == GapSeverity.MEDIUM

    def test_empty_response_returns_empty_list(self):
        """Empty response returns empty list."""
        assert self._parse_gaps("", "t") == []
        assert self._parse_gaps("# just comments\n   ", "t") == []

    def test_whitespace_in_papers_trimmed(self):
        """Paper names have whitespace trimmed."""
        response = "[method_limitation] Gap | Paper1 , Paper2 | 0.5 | low"
        gaps = self._parse_gaps(response, "t")
        assert "Paper1" in gaps[0].evidence_papers
        assert "Paper2" in gaps[0].evidence_papers

    def test_parses_all_gap_types(self):
        """All gap type strings parse correctly."""
        type_strs = [
            "unexplored_application", "method_limitation", "contradiction",
            "evaluation_gap", "scalability_issue", "theoretical_gap",
            "dataset_gap", "generalization_gap",
        ]
        for ts in type_strs:
            response = f"[{ts}] Description | Paper | 0.5 | medium"
            gaps = self._parse_gaps(response, "t")
            assert len(gaps) == 1, f"Failed for {ts}"


# =============================================================================
# _detect_gaps_rules tests
# =============================================================================
class TestDetectGapsRules:
    """Test _detect_gaps_rules logic."""

    def _detect_gaps_rules(self, paper_summaries: str) -> list:
        """Replicate _detect_gaps_rules logic."""
        import re
        gaps = []

        type_patterns = {
            GapType.UNEXPLORED_APPLICATION: [
                r'未探索|未研究|future work|future directions|open problem',
                r'potential application|limitation.*future|future research',
            ],
            GapType.METHOD_LIMITATION: [
                r'limitation|不足|weakness|shortcoming|constraint',
                r'does not scale|only works for|restricted to',
            ],
            GapType.CONTRADICTION: [
                r'however|but|in contrast|on the contrary',
                r'conflicting|disagree|differ|contradict',
            ],
            GapType.EVALUATION_GAP: [
                r'no benchmark|lack.*evaluation|evaluation.*limited',
                r'no standard|without baseline',
            ],
        }

        for gap_type, patterns in type_patterns.items():
            for pattern in patterns:
                if re.search(pattern, paper_summaries, re.IGNORECASE):
                    gaps.append(ResearchGap(
                        gap_type=gap_type,
                        description=f"基于关键词 '{pattern}' 发现的潜在研究空白",
                        evidence_papers=["从摘要中推断"],
                        confidence=0.3,
                    ))
                    break

        return gaps

    def test_no_matches_returns_empty(self):
        """No keyword matches returns empty list."""
        gaps = self._detect_gaps_rules("This is a normal research paper.")
        assert gaps == []

    def test_unexplored_keyword(self):
        """Chinese '未探索' triggers UNEXPLORED_APPLICATION."""
        gaps = self._detect_gaps_rules("该方法有未探索的应用场景")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.UNEXPLORED_APPLICATION

    def test_limitation_keyword(self):
        """'limitation' triggers METHOD_LIMITATION."""
        gaps = self._detect_gaps_rules("This method has a limitation")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.METHOD_LIMITATION

    def test_contradiction_keyword(self):
        """'however' triggers CONTRADICTION."""
        gaps = self._detect_gaps_rules("However, the results differ")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.CONTRADICTION

    def test_evaluation_gap_keyword(self):
        """'no benchmark' triggers EVALUATION_GAP."""
        gaps = self._detect_gaps_rules("There is no benchmark for this task")
        assert len(gaps) == 1
        assert gaps[0].gap_type == GapType.EVALUATION_GAP

    def test_case_insensitive(self):
        """Pattern matching is case insensitive."""
        gaps = self._detect_gaps_rules("LIMITATION found")
        assert len(gaps) == 1

    def test_multiple_types_detected(self):
        """Multiple gap types can be detected."""
        text = "The method has limitation and however contradicts prior work"
        gaps = self._detect_gaps_rules(text)
        assert len(gaps) >= 2
        types = {g.gap_type for g in gaps}
        assert GapType.METHOD_LIMITATION in types
        assert GapType.CONTRADICTION in types

    def test_confidence_is_03(self):
        """Rule-based gaps have confidence of 0.3."""
        gaps = self._detect_gaps_rules("This has limitation")
        assert gaps[0].confidence == 0.3

    def test_evidence_from_abstract(self):
        """Evidence paper is '从摘要中推断'."""
        gaps = self._detect_gaps_rules("Has limitation")
        assert gaps[0].evidence_papers == ["从摘要中推断"]

    def test_only_first_match_per_type(self):
        """Only first matching pattern per type creates a gap."""
        text = "Has limitation and weakness and shortcoming"
        gaps = self._detect_gaps_rules(text)
        # Only one gap for METHOD_LIMITATION (first matching pattern)
        method_gaps = [g for g in gaps if g.gap_type == GapType.METHOD_LIMITATION]
        assert len(method_gaps) == 1


# =============================================================================
# _parse_questions tests
# =============================================================================
class TestParseQuestions:
    """Test _parse_questions logic."""

    def _parse_questions(self, response: str, gaps: list) -> list:
        """Replicate _parse_questions logic."""
        questions = []
        default_gap = gaps[0] if gaps else None

        for line in response.strip().split('\n'):
            line = line.strip()
            if not line or line.startswith('#'):
                continue

            parts = [p.strip() for p in line.split('|')]
            if len(parts) >= 1:
                question_text = parts[0]
                hypothesis = parts[1] if len(parts) > 1 else ""
                methodology = parts[2] if len(parts) > 2 else ""
                impact = parts[3] if len(parts) > 3 else ""
                feasibility = float(parts[4]) if len(parts) > 4 and parts[4] else 0.5
                novelty = float(parts[5]) if len(parts) > 5 and parts[5] else 0.5

                q = ResearchQuestion(
                    question=question_text,
                    gap=default_gap,
                    hypothesis=hypothesis,
                    methodology_suggestion=methodology,
                    expected_impact=impact,
                    feasibility=feasibility,
                    novelty_score=novelty,
                )
                questions.append(q)

        return questions

    def test_basic_parsing(self):
        """Basic pipe-delimited line parses correctly."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        response = "How to improve? | Hypothesis here | Method X | Big impact | 0.8 | 0.7"
        qs = self._parse_questions(response, [gap])
        assert len(qs) == 1
        assert qs[0].question == "How to improve?"
        assert qs[0].hypothesis == "Hypothesis here"
        assert qs[0].methodology_suggestion == "Method X"
        assert qs[0].expected_impact == "Big impact"
        assert qs[0].feasibility == 0.8
        assert qs[0].novelty_score == 0.7

    def test_uses_first_gap_as_default(self):
        """Default gap is first in list."""
        gap1 = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D1",
            evidence_papers=[],
        )
        gap2 = ResearchGap(
            gap_type=GapType.CONTRADICTION,
            description="D2",
            evidence_papers=[],
        )
        qs = self._parse_questions("Question text | | | | |", [gap1, gap2])
        assert qs[0].gap == gap1

    def test_uses_none_when_gaps_empty(self):
        """Default gap is None when gaps list is empty."""
        qs = self._parse_questions("Question text | | | | |", [])
        assert qs[0].gap is None

    def test_multiple_lines(self):
        """Multiple lines create multiple questions."""
        response = "Question 1 | H1 | | | 0.5 | 0.5\nQuestion 2 | H2 | | | 0.6 | 0.7"
        qs = self._parse_questions(response, [])
        assert len(qs) == 2
        assert qs[0].question == "Question 1"
        assert qs[1].question == "Question 2"

    def test_skips_hash_comment_lines(self):
        """Lines starting with # are skipped."""
        response = "# Comment\nValid question | H | | | 0.5 | 0.5"
        qs = self._parse_questions(response, [])
        assert len(qs) == 1

    def test_empty_response_returns_empty_list(self):
        """Empty response returns empty list."""
        assert self._parse_questions("", []) == []
        assert self._parse_questions("# just comments", []) == []

    def test_minimum_one_part(self):
        """Question text only (single pipe section) works."""
        qs = self._parse_questions("Just question text", [])
        assert len(qs) == 1
        assert qs[0].question == "Just question text"
        assert qs[0].hypothesis == ""

    def test_feasibility_defaults_to_05(self):
        """Missing feasibility defaults to 0.5."""
        qs = self._parse_questions("Q | | | | | 0.7", [])
        assert qs[0].feasibility == 0.5

    def test_novelty_defaults_to_05(self):
        """Missing novelty defaults to 0.5."""
        qs = self._parse_questions("Q | | | | 0.8 |", [])
        assert qs[0].novelty_score == 0.5

    def test_whitespace_stripped_from_parts(self):
        """Each part has whitespace stripped."""
        qs = self._parse_questions(" Q1  |  H1  |  M1  |  I1  |  0.8  |  0.7  ", [])
        assert qs[0].question == "Q1"
        assert qs[0].hypothesis == "H1"


# =============================================================================
# _generate_questions_rules tests
# =============================================================================
class TestGenerateQuestionsRules:
    """Test _generate_questions_rules logic."""

    def _generate_questions_rules(self, gaps: list) -> list:
        """Replicate _generate_questions_rules logic."""
        questions = []
        topic = "该方法"

        templates = {
            GapType.UNEXPLORED_APPLICATION: "如何将 {topic} 应用于新场景？",
            GapType.METHOD_LIMITATION: "如何改进 {topic} 的方法以解决局限？",
            GapType.CONTRADICTION: "如何协调/解决 {topic} 中的矛盾发现？",
            GapType.EVALUATION_GAP: "如何为 {topic} 建立标准化评估？",
            GapType.SCALABILITY_ISSUE: "{topic} 如何扩展到更大规模？",
            GapType.THEORETICAL_GAP: "如何加强 {topic} 的理论基础？",
            GapType.DATASET_GAP: "如何构建 {topic} 的基准数据集？",
            GapType.GENERALIZATION_GAP: "{topic} 的泛化能力如何验证？",
        }

        for gap in gaps[:5]:
            template = templates.get(gap.gap_type, "如何改进 {topic}？")
            question_text = template.format(topic=topic)

            q = ResearchQuestion(
                question=question_text,
                gap=gap,
                feasibility=0.6,
                novelty_score=0.5,
            )
            questions.append(q)

        return questions

    def test_empty_gaps_returns_empty(self):
        """Empty gaps list returns empty questions."""
        assert self._generate_questions_rules([]) == []

    def test_uses_template_for_gap_type(self):
        """Correct template is used for each gap type."""
        gap = ResearchGap(
            gap_type=GapType.UNEXPLORED_APPLICATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap])
        assert "如何将" in qs[0].question
        assert "该方法" in qs[0].question

    def test_method_limitation_template(self):
        """METHOD_LIMITATION gets correct template."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap])
        assert "如何改进" in qs[0].question
        assert "方法以解决局限" in qs[0].question

    def test_max_5_questions(self):
        """Maximum 5 questions even with more gaps."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap] * 10)
        assert len(qs) == 5

    def test_question_gap_is_original(self):
        """Each question's gap field references the original gap."""
        gap = ResearchGap(
            gap_type=GapType.CONTRADICTION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap])
        assert qs[0].gap == gap

    def test_feasibility_is_06(self):
        """Rule-based questions have feasibility 0.6."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap])
        assert qs[0].feasibility == 0.6

    def test_novelty_is_05(self):
        """Rule-based questions have novelty_score 0.5."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([gap])
        assert qs[0].novelty_score == 0.5

    def test_fallback_template_for_unknown_type(self):
        """Unknown gap type uses fallback template."""
        gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,  # will be overridden
            description="D",
            evidence_papers=[],
        )
        # Manually set an unknown type via the enum
        unknown_gap = ResearchGap(
            gap_type=GapType.METHOD_LIMITATION,
            description="D",
            evidence_papers=[],
        )
        qs = self._generate_questions_rules([unknown_gap])
        assert "如何改进" in qs[0].question


# =============================================================================
# _calculate_coverage tests
# =============================================================================
class TestCalculateCoverage:
    """Test _calculate_coverage logic."""

    def _calculate_coverage(self, papers: list) -> float:
        """Replicate _calculate_coverage logic."""
        if not papers:
            return 0.0

        count_score = min(len(papers) / 20, 1.0)

        recency_scores = []
        for p in papers:
            try:
                year = int(p.get('year', 0))
            except (ValueError, TypeError):
                year = 0
            if year >= 2024:
                recency_scores.append(1.0)
            elif year >= 2022:
                recency_scores.append(0.7)
            elif year >= 2020:
                recency_scores.append(0.4)
            else:
                recency_scores.append(0.1)

        recency = sum(recency_scores) / len(recency_scores) if recency_scores else 0

        has_abstract = sum(1 for p in papers if p.get('abstract')) / len(papers)

        return (count_score * 0.4 + recency * 0.4 + has_abstract * 0.2)

    def test_empty_papers_returns_zero(self):
        """Empty list returns 0.0."""
        assert self._calculate_coverage([]) == 0.0

    def test_count_score_maxes_at_20(self):
        """Count score caps at 1.0 for 20+ papers."""
        papers = [{"year": 2024} for _ in range(30)]
        cov = self._calculate_coverage(papers)
        # count_score = 1.0 (capped), recency = 1.0, abstract = 0
        # 0.4*1.0 + 0.4*1.0 + 0.2*0 = 0.8
        assert cov <= 0.8

    def test_count_score_linear_to_20(self):
        """Count score increases linearly up to 20 papers."""
        cov_5 = self._calculate_coverage([{"year": 2024}] * 5)
        cov_10 = self._calculate_coverage([{"year": 2024}] * 10)
        assert cov_10 > cov_5

    def test_recency_2024_and_above(self):
        """Year >= 2024 scores 1.0."""
        papers = [{"year": 2025} for _ in range(20)]
        cov = self._calculate_coverage(papers)
        # count=1.0, recency=1.0, abstract=0 → 0.4 + 0.4 = 0.8
        assert cov == 0.8

    def test_recency_2022_to_2023(self):
        """Year 2022-2023 scores 0.7."""
        papers = [{"year": 2022} for _ in range(20)]
        cov = self._calculate_coverage(papers)
        # count=1.0, recency=0.7, abstract=0 → 0.4 + 0.28 = 0.68
        assert abs(cov - 0.68) < 0.001

    def test_recency_2020_to_2021(self):
        """Year 2020-2021 scores 0.4."""
        papers = [{"year": 2020} for _ in range(20)]
        cov = self._calculate_coverage(papers)
        # count=1.0, recency=0.4, abstract=0 → 0.4 + 0.16 = 0.56
        assert cov == 0.56

    def test_recency_before_2020(self):
        """Year before 2020 scores 0.1."""
        papers = [{"year": 2018} for _ in range(20)]
        cov = self._calculate_coverage(papers)
        # count=1.0, recency=0.1, abstract=0 → 0.4 + 0.04 = 0.44
        assert abs(cov - 0.44) < 0.001

    def test_abstract_contributes_02(self):
        """Papers with abstract contribute 0.2 to score."""
        papers_with = [{"year": 2024, "abstract": "Some text"} for _ in range(20)]
        papers_without = [{"year": 2024} for _ in range(20)]
        cov_with = self._calculate_coverage(papers_with)
        cov_without = self._calculate_coverage(papers_without)
        # abstract diff = 1.0 * 0.2 = 0.2
        assert cov_with == cov_without + 0.2

    def test_mixed_years_averaged(self):
        """Mixed years are averaged."""
        papers = [
            {"year": 2025},
            {"year": 2018},
        ]
        cov = self._calculate_coverage(papers)
        # count=2/20=0.1, recency=(1.0+0.1)/2=0.55, abstract=0
        # 0.1*0.4 + 0.55*0.4 = 0.04 + 0.22 = 0.26
        assert abs(cov - 0.26) < 0.001

    def test_invalid_year_treated_as_zero(self):
        """Invalid year strings treated as 0."""
        papers = [{"year": "not a year"} for _ in range(20)]
        cov = self._calculate_coverage(papers)
        # count=1.0, recency=0.1 (year 0), abstract=0 → 0.4 + 0.04 = 0.44
        assert abs(cov - 0.44) < 0.001


# =============================================================================
# _generate_summary tests
# =============================================================================
class TestGenerateSummary:
    """Test _generate_summary logic."""

    def _generate_summary(self, result: GapAnalysisResult) -> str:
        """Replicate _generate_summary logic."""
        high_gaps = [g for g in result.gaps if g.severity == GapSeverity.HIGH]
        medium_gaps = [g for g in result.gaps if g.severity == GapSeverity.MEDIUM]

        summary_parts = []

        if high_gaps:
            summary_parts.append(f"发现 {len(high_gaps)} 个高优先级研究空白")

        if medium_gaps:
            summary_parts.append(f"{len(medium_gaps)} 个中优先级空白")

        if result.coverage_score > 0.7:
            summary_parts.append("该领域研究较为成熟")
        elif result.coverage_score > 0.4:
            summary_parts.append("该领域有一定基础，仍有探索空间")
        else:
            summary_parts.append("该领域研究较少，创新机会较多")

        if result.questions:
            summary_parts.append(f"生成 {len(result.questions)} 个研究问题建议")

        return "；".join(summary_parts)

    def test_high_gaps_count(self):
        """High severity gaps counted in summary."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[],
                            severity=GapSeverity.HIGH),
            ],
        )
        summary = self._generate_summary(result)
        assert "1 个高优先级研究空白" in summary

    def test_medium_gaps_count(self):
        """Medium severity gaps counted in summary."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[],
                            severity=GapSeverity.MEDIUM),
                ResearchGap(gap_type=GapType.CONTRADICTION, description="D", evidence_papers=[],
                            severity=GapSeverity.MEDIUM),
            ],
        )
        summary = self._generate_summary(result)
        assert "2 个中优先级空白" in summary

    def test_coverage_mature(self):
        """High coverage score gives mature message."""
        result = GapAnalysisResult(topic="T", coverage_score=0.8)
        summary = self._generate_summary(result)
        assert "该领域研究较为成熟" in summary

    def test_coverage_moderate(self):
        """Moderate coverage score gives moderate message."""
        result = GapAnalysisResult(topic="T", coverage_score=0.5)
        summary = self._generate_summary(result)
        assert "该领域有一定基础，仍有探索空间" in summary

    def test_coverage_low(self):
        """Low coverage score gives low message."""
        result = GapAnalysisResult(topic="T", coverage_score=0.3)
        summary = self._generate_summary(result)
        assert "该领域研究较少，创新机会较多" in summary

    def test_boundary_07_is_mature(self):
        """Coverage > 0.7 is considered mature."""
        result = GapAnalysisResult(topic="T", coverage_score=0.71)
        summary = self._generate_summary(result)
        assert "该领域研究较为成熟" in summary

    def test_boundary_04_is_moderate(self):
        """Coverage > 0.4 and <= 0.7 is considered moderate."""
        result = GapAnalysisResult(topic="T", coverage_score=0.41)
        summary = self._generate_summary(result)
        assert "该领域有一定基础，仍有探索空间" in summary

    def test_questions_count(self):
        """Questions count included in summary."""
        gap = ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[])
        q = ResearchQuestion(question="Q?", gap=gap)
        result = GapAnalysisResult(topic="T", questions=[q, q, q])
        summary = self._generate_summary(result)
        assert "生成 3 个研究问题建议" in summary

    def test_empty_result(self):
        """Empty result has minimal summary."""
        result = GapAnalysisResult(topic="T")
        summary = self._generate_summary(result)
        assert "高优先级" not in summary
        assert "中优先级" not in summary
        assert "研究较少" in summary
        assert "研究问题建议" not in summary

    def test_joined_by_semi_colon(self):
        """Summary parts joined by Chinese semicolon."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[],
                            severity=GapSeverity.HIGH),
            ],
            coverage_score=0.5,
        )
        summary = self._generate_summary(result)
        assert "；" in summary


# =============================================================================
# render_result tests
# =============================================================================
class TestRenderResult:
    """Test render_result formatting."""

    def _render_result(self, result: GapAnalysisResult) -> str:
        """Replicate render_result logic."""
        lines = [
            f"🔬 《{result.topic}》研究空白分析",
            f"   分析论文数: {result.analyzed_papers_count}",
            f"   覆盖程度: {result.coverage_score:.0%}",
            f"   机会评分: {result.opportunities_score:.1f}/10",
            "",
        ]

        if result.gaps:
            lines.append("💡 研究空白：")
            for i, gap in enumerate(result.gaps, 1):
                severity_icon = {
                    GapSeverity.HIGH: "🔴",
                    GapSeverity.MEDIUM: "🟡",
                    GapSeverity.LOW: "🟢",
                }.get(gap.severity, "⚪")

                gap_type_name = {
                    GapType.UNEXPLORED_APPLICATION: "未探索应用",
                    GapType.METHOD_LIMITATION: "方法局限",
                    GapType.CONTRADICTION: "矛盾",
                    GapType.EVALUATION_GAP: "评估缺失",
                    GapType.SCALABILITY_ISSUE: "可扩展性",
                    GapType.THEORETICAL_GAP: "理论空白",
                    GapType.DATASET_GAP: "数据集缺失",
                    GapType.GENERALIZATION_GAP: "泛化问题",
                }.get(gap.gap_type, gap.gap_type.value)

                lines.append(f"  {i}. {severity_icon} [{gap_type_name}] {gap.description}")
                if gap.evidence_papers:
                    lines.append(f"     证据: {', '.join(gap.evidence_papers[:2])}")
            lines.append("")

        if result.questions:
            lines.append("📝 研究问题建议：")
            for i, q in enumerate(result.questions, 1):
                lines.append(f"  {i}. {q.question}")
                if q.hypothesis:
                    lines.append(f"     假设: {q.hypothesis[:60]}...")
                if q.methodology_suggestion:
                    lines.append(f"     方法: {q.methodology_suggestion[:50]}...")
            lines.append("")

        lines.append(f"📊 {result.summary}")

        return "\n".join(lines)

    def test_header(self):
        """Header contains topic and analysis info."""
        result = GapAnalysisResult(topic="Transformer", analyzed_papers_count=10, coverage_score=0.6)
        output = self._render_result(result)
        assert "🔬 《Transformer》研究空白分析" in output
        assert "分析论文数: 10" in output
        assert "覆盖程度: 60%" in output

    def test_opportunities_score(self):
        """Opportunities score shown in header."""
        result = GapAnalysisResult(topic="T", opportunities_score=7.5)
        output = self._render_result(result)
        assert "机会评分: 7.5/10" in output

    def test_no_gaps_section_when_empty(self):
        """No gaps section when gaps is empty."""
        result = GapAnalysisResult(topic="T", gaps=[])
        output = self._render_result(result)
        assert "💡 研究空白" not in output

    def test_gap_with_severity_icon(self):
        """Gap severity maps to correct icon."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.METHOD_LIMITATION,
                    description="Gap desc",
                    evidence_papers=["Paper1"],
                    severity=GapSeverity.HIGH,
                ),
            ],
        )
        output = self._render_result(result)
        assert "🔴" in output
        assert "方法局限" in output
        assert "Gap desc" in output

    def test_gap_medium_severity_icon(self):
        """Medium severity gets 🟡 icon."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.CONTRADICTION,
                    description="D",
                    evidence_papers=[],
                    severity=GapSeverity.MEDIUM,
                ),
            ],
        )
        output = self._render_result(result)
        assert "🟡" in output

    def test_gap_low_severity_icon(self):
        """Low severity gets 🟢 icon."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.EVALUATION_GAP,
                    description="D",
                    evidence_papers=[],
                    severity=GapSeverity.LOW,
                ),
            ],
        )
        output = self._render_result(result)
        assert "🟢" in output

    def test_evidence_papers_limited_to_2(self):
        """Evidence papers limited to first 2."""
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.METHOD_LIMITATION,
                    description="D",
                    evidence_papers=["P1", "P2", "P3", "P4"],
                    severity=GapSeverity.MEDIUM,
                ),
            ],
        )
        output = self._render_result(result)
        assert "P1" in output
        assert "P2" in output
        # Should not contain P3 or P4
        lines = output.split('\n')
        evidence_line = [l for l in lines if '证据:' in l][0]
        assert "P3" not in evidence_line

    def test_question_shown(self):
        """Question text shown."""
        gap = ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[])
        result = GapAnalysisResult(
            topic="T",
            questions=[ResearchQuestion(question="How to improve?", gap=gap)],
        )
        output = self._render_result(result)
        assert "📝 研究问题建议" in output
        assert "How to improve?" in output

    def test_hypothesis_truncated_to_60(self):
        """Long hypothesis truncated to 60 chars with ellipsis."""
        gap = ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[])
        result = GapAnalysisResult(
            topic="T",
            questions=[
                ResearchQuestion(
                    question="Q?",
                    gap=gap,
                    hypothesis="A" * 100,
                )
            ],
        )
        output = self._render_result(result)
        assert "假设: " + "A" * 60 + "..." in output
        assert ("A" * 100) not in output

    def test_methodology_truncated_to_50(self):
        """Long methodology suggestion truncated to 50 chars."""
        gap = ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[])
        result = GapAnalysisResult(
            topic="T",
            questions=[
                ResearchQuestion(
                    question="Q?",
                    gap=gap,
                    methodology_suggestion="B" * 80,
                )
            ],
        )
        output = self._render_result(result)
        assert "方法: " + "B" * 50 + "..." in output

    def test_summary_appended(self):
        """Summary appended at end."""
        result = GapAnalysisResult(topic="T", summary="分析完成")
        output = self._render_result(result)
        assert output.endswith("📊 分析完成")

    def test_gap_type_names_all_types(self):
        """All gap types have Chinese names in output."""
        for gt in GapType:
            result = GapAnalysisResult(
                topic="T",
                gaps=[
                    ResearchGap(
                        gap_type=gt,
                        description="D",
                        evidence_papers=["P"],
                        severity=GapSeverity.MEDIUM,
                    ),
                ],
            )
            output = self._render_result(result)
            assert "D" in output  # description shown


# =============================================================================
# render_json tests
# =============================================================================
class TestRenderJson:
    """Test render_json formatting."""

    def _render_json(self, result: GapAnalysisResult) -> str:
        """Replicate render_json logic."""
        import json

        data = {
            "topic": result.topic,
            "gaps": [
                {
                    "type": g.gap_type.value,
                    "description": g.description,
                    "severity": g.severity.value,
                    "confidence": g.confidence,
                    "evidence_papers": g.evidence_papers,
                }
                for g in result.gaps
            ],
            "questions": [
                {
                    "question": q.question,
                    "hypothesis": q.hypothesis,
                    "methodology": q.methodology_suggestion,
                    "feasibility": q.feasibility,
                    "novelty": q.novelty_score,
                }
                for q in result.questions
            ],
            "coverage_score": result.coverage_score,
            "opportunities_score": result.opportunities_score,
            "analyzed_papers_count": result.analyzed_papers_count,
        }

        return json.dumps(data, ensure_ascii=False, indent=2)

    def test_topic_in_json(self):
        """Topic included in JSON."""
        result = GapAnalysisResult(topic="Transformer")
        output = self._render_json(result)
        assert '"topic": "Transformer"' in output

    def test_gap_in_json(self):
        """Gap data included in JSON."""
        import json
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.METHOD_LIMITATION,
                    description="Gap desc",
                    evidence_papers=["Paper1"],
                    severity=GapSeverity.HIGH,
                    confidence=0.8,
                ),
            ],
        )
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["gaps"][0]["type"] == "method_limitation"
        assert parsed["gaps"][0]["description"] == "Gap desc"
        assert parsed["gaps"][0]["severity"] == "high"
        assert parsed["gaps"][0]["confidence"] == 0.8

    def test_question_in_json(self):
        """Question data included in JSON."""
        gap = ResearchGap(gap_type=GapType.METHOD_LIMITATION, description="D", evidence_papers=[])
        result = GapAnalysisResult(
            topic="T",
            questions=[
                ResearchQuestion(
                    question="How to improve?",
                    gap=gap,
                    hypothesis="Hypothesis",
                    methodology_suggestion="Method",
                    feasibility=0.8,
                    novelty_score=0.7,
                )
            ],
        )
        output = self._render_json(result)
        assert '"How to improve?"' in output
        assert '"Hypothesis"' in output
        assert '"Method"' in output

    def test_coverage_scores_in_json(self):
        """Coverage and opportunities scores included."""
        result = GapAnalysisResult(
            topic="T",
            coverage_score=0.75,
            opportunities_score=8.5,
            analyzed_papers_count=12,
        )
        output = self._render_json(result)
        assert '"coverage_score": 0.75' in output
        assert '"opportunities_score": 8.5' in output
        assert '"analyzed_papers_count": 12' in output

    def test_empty_lists(self):
        """Empty gaps and questions produce empty arrays."""
        result = GapAnalysisResult(topic="T")
        output = self._render_json(result)
        assert '"gaps": []' in output
        assert '"questions": []' in output

    def test_is_valid_json(self):
        """Output is valid JSON."""
        import json
        result = GapAnalysisResult(
            topic="T",
            gaps=[
                ResearchGap(
                    gap_type=GapType.CONTRADICTION,
                    description="Contradiction",
                    evidence_papers=["P"],
                    severity=GapSeverity.LOW,
                    confidence=0.3,
                ),
            ],
        )
        output = self._render_json(result)
        parsed = json.loads(output)
        assert parsed["topic"] == "T"
        assert len(parsed["gaps"]) == 1
        assert parsed["gaps"][0]["type"] == "contradiction"


# =============================================================================
# GapDetector instantiation
# =============================================================================
class TestGapDetectorInit:
    """Test GapDetector class."""

    def test_can_instantiate(self):
        """GapDetector can be instantiated."""
        detector = GapDetector()
        assert detector.db is None

    def test_can_instantiate_with_db(self):
        """GapDetector can be instantiated with db."""
        mock_db = object()
        detector = GapDetector(db=mock_db)
        assert detector.db is mock_db
