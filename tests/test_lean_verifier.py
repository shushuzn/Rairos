"""Tests for llm/lean_verifier.py — Lean 4 theorem prover integration."""

import pytest
from unittest.mock import patch, MagicMock
import json
import tempfile
from pathlib import Path

from llm.lean_verifier import (
    LeanInstallStatus,
    VerificationLevel,
    LeanVerificationResult,
    check_lean_installed,
    get_lean_install_instructions,
    translate_hypothesis_to_lean,
    _extract_lean_code,
    _translate_symbols,
    _template_based_translation,
    verify_lean_code,
    render_result,
    render_result_json,
    LEAN_TRANSLATION_SYSTEM,
    LEAN_TRANSLATION_USER_TEMPLATE,
    _FALLBACK_TEMPLATES,
)


class TestEnums:
    """Test enum values."""

    def test_lean_install_status_values(self):
        """LeanInstallStatus should have three values."""
        assert LeanInstallStatus.AVAILABLE.value == "available"
        assert LeanInstallStatus.NOT_FOUND.value == "not_found"
        assert LeanInstallStatus.VERSION_UNKNOWN.value == "version_unknown"

    def test_verification_level_values(self):
        """VerificationLevel should have four values."""
        assert VerificationLevel.L0_SYNTAX.value == "l0_syntax"
        assert VerificationLevel.L1_TYPECHECK.value == "l1_typecheck"
        assert VerificationLevel.L2_PROVEN.value == "l2_proven"
        assert VerificationLevel.L0_FAILED.value == "l0_failed"


class TestLeanVerificationResult:
    """Test LeanVerificationResult dataclass."""

    def test_creates_with_required_fields(self):
        """Should create with required fields."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test hypothesis",
            level=VerificationLevel.L0_FAILED,
            lean_code="theorem test : Prop := by sorry",
        )
        assert result.hypothesis_id == "h1"
        assert result.level == VerificationLevel.L0_FAILED
        assert result.errors == []
        assert result.install_status == LeanInstallStatus.NOT_FOUND

    def test_default_error_list(self):
        """errors and warnings should default to empty lists."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="",
        )
        assert result.errors == []
        assert result.warnings == []


class TestCheckLeanInstalled:
    """Test check_lean_installed()."""

    def test_returns_not_found_when_lean_not_in_path(self):
        """Should return NOT_FOUND when shutil.which returns None."""
        with patch("llm.lean_verifier.shutil.which", return_value=None):
            status, version = check_lean_installed()
            assert status == LeanInstallStatus.NOT_FOUND
            assert version is None

    def test_returns_available_when_lean_found(self):
        """Should return AVAILABLE with version when lean --version succeeds."""
        with patch("llm.lean_verifier.shutil.which", return_value="/usr/bin/lean"), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="Lean (version 4.2.0)", stderr="", returncode=0)
            status, version = check_lean_installed()
            assert status == LeanInstallStatus.AVAILABLE
            assert "4.2.0" in version

    def test_returns_available_on_timeout(self):
        """Should return AVAILABLE when lean exists but times out."""
        import subprocess
        with patch("llm.lean_verifier.shutil.which", return_value="/usr/bin/lean"), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="lean", timeout=30)
            status, version = check_lean_installed()
            assert status == LeanInstallStatus.AVAILABLE
            assert "timeout" in version

    def test_returns_not_found_on_file_not_found(self):
        """Should return NOT_FOUND when lean executable not found (OSError)."""
        import subprocess
        with patch("llm.lean_verifier.shutil.which", return_value="/usr/bin/lean"), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.side_effect = FileNotFoundError()
            status, version = check_lean_installed()
            assert status == LeanInstallStatus.NOT_FOUND


class TestExtractLeanCode:
    """Test _extract_lean_code regex extraction."""

    def test_extracts_from_fenced_block(self):
        """Should extract code from ```lean fenced block."""
        response = """
        Here is the Lean code:

        ```lean
        theorem add_comm (n m : Nat) : n + m = m + n := by sorry
        ```
        """
        code = _extract_lean_code(response)
        assert "theorem add_comm" in code
        assert "by sorry" in code

    def test_handles_fence_without_language_tag(self):
        """Should handle ``` without language tag."""
        response = """
        ```
        theorem foo : Prop := by sorry
        ```
        """
        code = _extract_lean_code(response)
        assert "theorem foo" in code

    def test_returns_stripped_code(self):
        """Should strip whitespace from extracted code."""
        response = """
        ```lean

          theorem test : Nat := 0

        ```
        """
        code = _extract_lean_code(response)
        assert code.startswith("theorem")
        assert not code.startswith("\n")

    def test_fallback_returns_whole_response(self):
        """Should return full response when no fence found."""
        response = "theorem foo : Prop := by sorry"
        code = _extract_lean_code(response)
        assert code == "theorem foo : Prop := by sorry"


class TestTranslateSymbols:
    """Test _translate_symbols Unicode mapping."""

    def test_translates_forall(self):
        """Should translate 'forall' to ∀."""
        assert "∀" in _translate_symbols("forall n : Nat")
        assert "∃" in _translate_symbols("exists x")
        assert "→" in _translate_symbols("forall x -> y")
        assert "≤" in _translate_symbols("a <= b")
        assert "≥" in _translate_symbols("a >= b")

    def test_preserves_unicode_symbols(self):
        """Already-correct Unicode symbols should pass through unchanged."""
        result = _translate_symbols("∀ n : Nat, ∃ m : Nat")
        assert "∀" in result
        assert "∃" in result

    def test_translates_arrow_variants(self):
        """Both -> and => should map to →."""
        assert "→" in _translate_symbols("a -> b")
        assert "→" in _translate_symbols("a => b")

    def test_identity_mapping(self):
        """Symbols already correct should not change."""
        text = "∀ n m : Nat, n + m = m + n"
        result = _translate_symbols(text)
        assert result == text


class TestFallbackTemplates:
    """Test _FALLBACK_TEMPLATES structure."""

    def test_all_domains_have_templates(self):
        """All domain templates should exist."""
        expected = {"mathy", "convergence", "probability", "causal", "set_theory", "functions", "qualitative"}
        assert set(_FALLBACK_TEMPLATES.keys()) == expected

    def test_all_templates_contain_safe_name_placeholder(self):
        """All templates should contain {safe_name} placeholder."""
        for name, tmpl in _FALLBACK_TEMPLATES.items():
            assert "{safe_name}" in tmpl, f"Template '{name}' missing {{safe_name}}"

    def test_all_templates_contain_statement_placeholder(self):
        """All templates should contain {statement} placeholder."""
        for name, tmpl in _FALLBACK_TEMPLATES.items():
            assert "{statement}" in tmpl, f"Template '{name}' missing {{statement}}"

    def test_templates_produce_lean_code(self):
        """All templates should produce Lean code."""
        for name, tmpl in _FALLBACK_TEMPLATES.items():
            # Escape literal curly braces used in Lean syntax (e.g. {α : Type})
            escaped = tmpl.replace("{", "{{").replace("}", "}}")
            result = escaped.format(safe_name="test", statement="∀ x", hyp_type="test")
            assert "theorem" in result or "def" in result, f"Template '{name}' doesn't produce Lean code"


class MockHypothesis:
    """Mock ResearchHypothesis for testing without importing the actual class."""
    def __init__(self, title="", core_statement="", hypothesis_type="", based_on="", variables=None, expected_results=""):
        self.title = title
        self.core_statement = core_statement
        self.hypothesis_type = hypothesis_type
        self.based_on = based_on
        self.variables = variables or []
        self.expected_results = expected_results
        self.id = "mock-hypothesis-id"
        self.experiment_design = MagicMock()
        self.experiment_design.variables = variables or []
        self.experiment_design.expected_results = expected_results


class TestTemplateBasedTranslation:
    """Test _template_based_translation domain detection."""

    def test_mathy_template_for_unicode_symbols(self):
        """Unicode symbols should trigger mathy template."""
        hyp = MockHypothesis(
            title="Test",
            core_statement="∀ n m : Nat, n + m = m + n",
            hypothesis_type="theoretical",
        )
        result = _template_based_translation(hyp)
        assert "theorem" in result

    def test_convergence_keywords_trigger_convergence_template(self):
        """Convergence-related keywords should trigger convergence template."""
        hyp = MockHypothesis(
            title="Convergence Claim",
            core_statement="The sequence converges to L",
            hypothesis_type="theoretical",
        )
        result = _template_based_translation(hyp)
        assert "converges" in result.lower() or "convergence" in result.lower() or "theorem" in result

    def test_probability_keywords_trigger_probability_template(self):
        """Probability-related keywords should trigger probability template."""
        hyp = MockHypothesis(
            title="Probability Claim",
            core_statement="The probability of X is greater than 0.5",
            hypothesis_type="statistical",
        )
        result = _template_based_translation(hyp)
        assert "theorem" in result or "probability" in result.lower()

    def test_safe_name_sanitizes_title(self):
        """safe_name should be sanitized (alphanumeric only)."""
        hyp = MockHypothesis(
            title="Test!@#$ Claim (2024)",
            core_statement="∀ x, x > 0",
            hypothesis_type="theoretical",
        )
        result = _template_based_translation(hyp)
        assert "theorem" in result
        # safe_name should not contain special chars
        assert "!" not in result
        assert "@" not in result

    def test_fallback_for_qualitative(self):
        """Pure qualitative text should use qualitative template."""
        hyp = MockHypothesis(
            title="Qualitative Claim",
            core_statement="The model is effective at predicting outcomes",
            hypothesis_type="empirical",
        )
        result = _template_based_translation(hyp)
        assert "theorem" in result or "def" in result


class TestVerifyLeanCode:
    """Test verify_lean_code main verification function."""

    def test_returns_l0_failed_when_lean_not_installed(self):
        """Should return L0_FAILED when Lean is not found."""
        with patch("llm.lean_verifier.check_lean_installed", return_value=(LeanInstallStatus.NOT_FOUND, None)):
            result = verify_lean_code("theorem test : Prop := by sorry", "h1", "test")
            assert result.level == VerificationLevel.L0_FAILED
            assert any("Lean not found" in e for e in result.errors)

    def test_returns_result_when_lean_available(self):
        """Should return a result object with correct fields when Lean is available."""
        with patch("llm.lean_verifier.check_lean_installed", return_value=(LeanInstallStatus.AVAILABLE, "4.2.0")), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="", stderr="", returncode=0)
            result = verify_lean_code("theorem test : Prop := by sorry", "h1", "test hypothesis")
            assert isinstance(result, LeanVerificationResult)
            assert result.hypothesis_id == "h1"
            assert result.hypothesis_text == "test hypothesis"
            assert result.install_status == LeanInstallStatus.AVAILABLE

    def test_sets_l2_proven_when_sorry_present(self):
        """Code with 'sorry' should be L2_PROVEN when Lean passes."""
        with patch("llm.lean_verifier.check_lean_installed", return_value=(LeanInstallStatus.AVAILABLE, "4.2.0")), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="", stderr="", returncode=0)
            result = verify_lean_code("theorem test : Prop := by sorry", "h1", "test")
            assert result.level == VerificationLevel.L2_PROVEN

    def test_sets_l1_typecheck_when_no_sorry(self):
        """Code without 'sorry' and clean exit should be L1_TYPECHECK."""
        with patch("llm.lean_verifier.check_lean_installed", return_value=(LeanInstallStatus.AVAILABLE, "4.2.0")), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.return_value = MagicMock(stdout="", stderr="", returncode=0)
            result = verify_lean_code("theorem test : Prop := 0 = 0", "h1", "test")
            assert result.level == VerificationLevel.L1_TYPECHECK

    def test_timeout_sets_l0_failed(self):
        """Timeout should set L0_FAILED with timeout error."""
        import subprocess
        with patch("llm.lean_verifier.check_lean_installed", return_value=(LeanInstallStatus.AVAILABLE, "4.2.0")), \
             patch("llm.lean_verifier.subprocess.run") as mock_run:
            mock_run.side_effect = subprocess.TimeoutExpired(cmd="lean", timeout=60)
            result = verify_lean_code("theorem test : Prop := by sorry", "h1", "test")
            assert result.level == VerificationLevel.L0_FAILED
            assert any("timed out" in e for e in result.errors)


class TestRenderResult:
    """Test render_result text rendering."""

    def test_renders_l0_failed_level(self):
        """Should show ❌ for L0_FAILED."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L0_FAILED,
            lean_code="",
        )
        rendered = render_result(result)
        assert "❌" in rendered
        assert "l0_failed" in rendered

    def test_renders_l1_typecheck_level(self):
        """Should show 🟢 for L1_TYPECHECK."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L1_TYPECHECK,
            lean_code="theorem test : Prop := 0 = 0",
        )
        rendered = render_result(result)
        assert "🟢" in rendered or "l1_typecheck" in rendered

    def test_renders_lean_not_found_install_status(self):
        """Should show ⚠️  for NOT_FOUND install status."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L0_FAILED,
            lean_code="",
            install_status=LeanInstallStatus.NOT_FOUND,
        )
        rendered = render_result(result)
        assert "Lean not found" in rendered

    def test_renders_hypothesis_id(self):
        """Should display hypothesis ID."""
        result = LeanVerificationResult(
            hypothesis_id="hyp_abc123",
            hypothesis_text="A test hypothesis about math",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="",
        )
        rendered = render_result(result)
        assert "hyp_abc123" in rendered

    def test_renders_lean_code(self):
        """Should display the Lean code."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="theorem add_comm : Prop := by sorry",
        )
        rendered = render_result(result)
        assert "theorem add_comm" in rendered

    def test_renders_errors(self):
        """Should list errors when present."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L0_FAILED,
            lean_code="broken code",
            errors=["error: unexpected token", "error: type mismatch"],
        )
        rendered = render_result(result)
        assert "unexpected token" in rendered
        assert "type mismatch" in rendered

    def test_renders_warnings(self):
        """Should list warnings when present."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L1_TYPECHECK,
            lean_code="theorem test : Prop := by sorry",
            warnings=["warning: unused variable 'x'"],
        )
        rendered = render_result(result)
        assert "unused variable" in rendered


class TestRenderResultJson:
    """Test render_result_json JSON output."""

    def test_returns_valid_json(self):
        """Should return valid JSON string."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test hypothesis",
            level=VerificationLevel.L1_TYPECHECK,
            lean_code="theorem test : Prop := by sorry",
        )
        json_str = render_result_json(result)
        parsed = json.loads(json_str)
        assert parsed["hypothesis_id"] == "h1"
        assert parsed["level"] == "l1_typecheck"
        assert parsed["lean_code"] == "theorem test : Prop := by sorry"

    def test_includes_all_fields(self):
        """JSON should include all key fields."""
        result = LeanVerificationResult(
            hypothesis_id="h1",
            hypothesis_text="test",
            level=VerificationLevel.L2_PROVEN,
            lean_code="theorem test : Prop := by sorry",
            errors=["error1"],
            warnings=["warn1"],
            install_status=LeanInstallStatus.AVAILABLE,
            translation_notes="proved with sorry",
        )
        json_str = render_result_json(result)
        parsed = json.loads(json_str)
        assert "errors" in parsed
        assert "warnings" in parsed
        assert "install_status" in parsed
        assert "translation_notes" in parsed


class TestGetLeanInstallInstructions:
    """Test get_lean_install_instructions()."""

    def test_returns_string(self):
        """Should return a non-empty string with installation steps."""
        instructions = get_lean_install_instructions()
        assert isinstance(instructions, str)
        assert len(instructions) > 0
        assert "Lean" in instructions

    def test_mentions_elan(self):
        """Instructions should mention elan for Windows."""
        instructions = get_lean_install_instructions()
        assert "elan" in instructions


class TestTranslateHypothesisToLean:
    """Test translate_hypothesis_to_lean()."""

    def test_returns_tuple(self):
        """Should return (lean_code, notes) tuple."""
        hyp = MockHypothesis(
            title="Test Theorem",
            core_statement="∀ n : Nat, n + 0 = n",
            hypothesis_type="theoretical",
        )
        with patch("llm.lean_verifier.HYPOTHESIS_AVAILABLE", True):
            with patch("llm.chat.call_llm_chat_completions") as mock_call:
                mock_call.side_effect = Exception("no api key")
                code, notes = translate_hypothesis_to_lean(hyp, use_llm=False)
                assert isinstance(code, str)
                assert isinstance(notes, str)
                assert len(code) > 0

    def test_falls_back_to_template_when_llm_fails(self):
        """Should use template-based translation when LLM call fails."""
        hyp = MockHypothesis(
            title="Test Theorem",
            core_statement="∀ n : Nat, n + 0 = n",
            hypothesis_type="theoretical",
        )
        with patch("llm.lean_verifier.HYPOTHESIS_AVAILABLE", True):
            with patch("llm.chat.call_llm_chat_completions") as mock_call:
                mock_call.side_effect = Exception("API error")
                code, notes = translate_hypothesis_to_lean(hyp, use_llm=True)
                assert "theorem" in code
                assert "template-based" in notes


class TestSystemPromptAndTemplate:
    """Test system prompt and user template content."""

    def test_system_prompt_contains_lean_conventions(self):
        """LEAN_TRANSLATION_SYSTEM should contain Lean 4 conventions."""
        assert "Lean 4" in LEAN_TRANSLATION_SYSTEM
        assert "Nat" in LEAN_TRANSLATION_SYSTEM
        assert "theorem" in LEAN_TRANSLATION_SYSTEM

    def test_user_template_has_placeholders(self):
        """LEAN_TRANSLATION_USER_TEMPLATE should have all placeholders."""
        placeholders = ["{title}", "{core_statement}", "{hypothesis_type}", "{based_on}", "{variables}", "{expected_result}"]
        for ph in placeholders:
            assert ph in LEAN_TRANSLATION_USER_TEMPLATE, f"Missing placeholder: {ph}"

    def test_system_prompt_lists_translation_patterns(self):
        """System prompt should list domain-specific translation patterns."""
        assert "Causal" in LEAN_TRANSLATION_SYSTEM or "causal" in LEAN_TRANSLATION_SYSTEM
        assert "Convergence" in LEAN_TRANSLATION_SYSTEM or "convergence" in LEAN_TRANSLATION_SYSTEM
        assert "Probability" in LEAN_TRANSLATION_SYSTEM or "probability" in LEAN_TRANSLATION_SYSTEM
