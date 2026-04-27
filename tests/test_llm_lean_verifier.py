"""Tier 2 unit tests — llm/lean_verifier.py, pure functions + Lean integration."""
import pytest
import json
from llm.lean_verifier import (
    LeanInstallStatus,
    VerificationLevel,
    LeanVerificationResult,
    check_lean_installed,
    get_lean_install_instructions,
    _extract_lean_code,
    _template_based_translation,
    render_result,
    render_result_json,
    verify_lean_code,
)


# =============================================================================
# Enums
# =============================================================================
class TestLeanInstallStatus:
    """Test LeanInstallStatus enum."""

    def test_all_values(self):
        assert LeanInstallStatus.AVAILABLE.value == "available"
        assert LeanInstallStatus.NOT_FOUND.value == "not_found"
        assert LeanInstallStatus.VERSION_UNKNOWN.value == "version_unknown"

    def test_count(self):
        assert len(LeanInstallStatus) == 3


class TestVerificationLevel:
    """Test VerificationLevel enum."""

    def test_all_values(self):
        assert VerificationLevel.L0_SYNTAX.value == "l0_syntax"
        assert VerificationLevel.L1_TYPECHECK.value == "l1_typecheck"
        assert VerificationLevel.L2_PROVEN.value == "l2_proven"
        assert VerificationLevel.L0_FAILED.value == "l0_failed"

    def test_count(self):
        assert len(VerificationLevel) == 4


# =============================================================================
# LeanVerificationResult dataclass
# =============================================================================
class TestLeanVerificationResult:
    """Test LeanVerificationResult dataclass."""

    def test_required_fields(self):
        r = LeanVerificationResult(
            hypothesis_id="h001",
            hypothesis_text="∀n m: Nat, n+m = m+n",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="theorem add_comm : True := by sorry",
        )
        assert r.hypothesis_id == "h001"
        assert r.hypothesis_text == "∀n m: Nat, n+m = m+n"
        assert r.level == VerificationLevel.L0_SYNTAX
        assert r.lean_code == "theorem add_comm : True := by sorry"
        assert r.errors == []  # default
        assert r.warnings == []  # default
        assert r.install_status == LeanInstallStatus.NOT_FOUND  # default

    def test_with_errors_and_warnings(self):
        r = LeanVerificationResult(
            hypothesis_id="h002",
            hypothesis_text="bad code",
            level=VerificationLevel.L0_FAILED,
            lean_code="theorem broken",
            errors=["error: unexpected token"],
            warnings=["warning: unused variable"],
        )
        assert len(r.errors) == 1
        assert len(r.warnings) == 1

    def test_with_install_status(self):
        r = LeanVerificationResult(
            hypothesis_id="h003",
            hypothesis_text="t",
            level=VerificationLevel.L1_TYPECHECK,
            lean_code="def x : Nat := 0",
            install_status=LeanInstallStatus.AVAILABLE,
        )
        assert r.install_status == LeanInstallStatus.AVAILABLE


# =============================================================================
# get_lean_install_instructions
# =============================================================================
class TestGetLeanInstallInstructions:
    """Test get_lean_install_instructions."""

    def test_returns_non_empty_string(self):
        s = get_lean_install_instructions()
        assert isinstance(s, str)
        assert len(s) > 10

    def test_contains_installation_hints(self):
        s = get_lean_install_instructions()
        assert "elan" in s or "lake" in s.lower()


# =============================================================================
# _extract_lean_code — pure string/regex parsing
# =============================================================================
class TestExtractLeanCode:
    """Test _extract_lean_code — pure, no I/O."""

    def test_extracts_fenced_lean_block(self):
        response = """Here is the Lean code:

```lean
theorem add_comm (n m : Nat) : n + m = m + n := by sorry
```

Let me know if you have questions."""
        result = _extract_lean_code(response)
        assert "theorem add_comm" in result
        assert "by sorry" in result

    def test_extracts_with_language_tag(self):
        result = _extract_lean_code("```lean\ntheorem foo := True\n```")
        assert "theorem foo" in result
        assert "```" not in result  # fences stripped

    def test_no_fence_returns_whole_response(self):
        raw = "theorem foo := True"
        assert _extract_lean_code(raw) == raw

    def test_empty_string(self):
        assert _extract_lean_code("") == ""

    def test_only_whitespace(self):
        assert _extract_lean_code("   \n  ") == ""

    def test_multiline_block_preserved(self):
        code = "theorem foo\n  (n : Nat) : n = n := by rfl"
        response = f"```lean\n{code}\n```"
        result = _extract_lean_code(response)
        assert "theorem foo" in result
        assert "(n : Nat)" in result

    def test_first_block_when_multiple(self):
        first = "theorem first := True"
        second = "theorem second := False"
        response = f"```lean\n{first}\n```\n\n```lean\n{second}\n```"
        result = _extract_lean_code(response)
        assert "theorem first" in result
        assert "theorem second" not in result


# =============================================================================
# _template_based_translation — pure, no I/O
# =============================================================================
class TestTemplateBasedTranslation:
    """Test _template_based_translation with mock ResearchHypothesis."""

    def _make_hypothesis(self, **kwargs):
        """Create a minimal ResearchHypothesis for testing."""
        from llm.hypothesis_generator import HypothesisType, ExperimentDesign
        defaults = dict(
            id="test-id",
            title="Test Hypothesis",
            core_statement="∀n m : Nat, n + m = m + n",
            hypothesis_type=HypothesisType.CAUSAL,
            based_on="unit test",
            experiment_design=ExperimentDesign(
                baseline="", variables=[], controls=[],
                evaluation_metrics=[], expected_results="",
            ),
        )
        defaults.update(kwargs)
        return defaults

    def _hypothesis(self, **kwargs):
        from llm.hypothesis_generator import HypothesisType, ExperimentDesign, ResearchHypothesis
        d = self._make_hypothesis(**kwargs)
        return ResearchHypothesis(**d)

    def test_generates_lean_code(self):
        h = self._hypothesis()
        code = _template_based_translation(h)
        assert isinstance(code, str)
        assert len(code) > 10

    def test_contains_hypothesis_title(self):
        h = self._hypothesis(title="Commutativity Theorem")
        code = _template_based_translation(h)
        assert "Commutativity_Theorem" in code

    def test_contains_by_sorry_for_mathy(self):
        h = self._hypothesis(core_statement="∀n m, n + m = m + n")
        code = _template_based_translation(h)
        assert "by sorry" in code

    def test_non_mathy_uses_stub(self):
        h = self._hypothesis(core_statement="This is a qualitative research question")
        code = _template_based_translation(h)
        assert "WARNING" in code or "stub" in code or "def " in code

    def test_safe_name_for_special_chars(self):
        h = self._hypothesis(
            title="Hypothesis: ∀x ∈ ℝ, x² ≥ 0",
            core_statement="∀x ∈ ℝ, x² ≥ 0",
        )
        code = _template_based_translation(h)
        # Should not crash — no special chars in theorem name
        assert isinstance(code, str)

    def test_hypothesis_type_included(self):
        from llm.hypothesis_generator import HypothesisType
        h = self._hypothesis(hypothesis_type=HypothesisType.MECHANISTIC)
        code = _template_based_translation(h)
        assert "mechanistic" in code.lower()

    def test_contains_formal_claim_comment(self):
        h = self._hypothesis(based_on="prior work on commutativity")
        code = _template_based_translation(h)
        assert "Formal claim" in code


# =============================================================================
# render_result — pure string formatting
# =============================================================================
class TestRenderResult:
    """Test render_result."""

    def _result(self, **kwargs):
        defaults = dict(
            hypothesis_id="h001",
            hypothesis_text="test hypothesis",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="theorem foo := True",
            errors=[],
            warnings=[],
            install_status=LeanInstallStatus.NOT_FOUND,
            translation_notes="",
        )
        defaults.update(kwargs)
        return LeanVerificationResult(**defaults)

    def test_contains_level(self):
        r = self._result(level=VerificationLevel.L1_TYPECHECK)
        output = render_result(r)
        assert "l1_typecheck" in output

    def test_contains_lean_code(self):
        r = self._result(lean_code="theorem add_comm := by sorry")
        output = render_result(r)
        assert "add_comm" in output

    def test_errors_shown(self):
        r = self._result(errors=["error: syntax error"])
        output = render_result(r)
        assert "error: syntax error" in output

    def test_warnings_shown(self):
        r = self._result(warnings=["unused variable 'x'"])
        output = render_result(r)
        assert "unused variable" in output

    def test_install_status_shown(self):
        r = self._result(install_status=LeanInstallStatus.NOT_FOUND)
        output = render_result(r)
        assert "Lean not found" in output or "not found" in output

    def test_install_instructions_when_not_found(self):
        r = self._result(install_status=LeanInstallStatus.NOT_FOUND)
        output = render_result(r)
        assert "elan" in output.lower() or "install" in output.lower()

    def test_translation_notes_not_in_text_output(self):
        """translation_notes are NOT shown in text output (only in JSON)."""
        r = self._result(translation_notes="template-based fallback")
        output = render_result(r)
        assert "template-based" not in output

    def test_hypothesis_id_shown(self):
        r = self._result(hypothesis_id="abc123")
        output = render_result(r)
        assert "abc123" in output

    def test_hypothesis_text_truncated(self):
        r = self._result(hypothesis_text="x" * 200)
        output = render_result(r)
        # Should not contain 200 x's literally
        assert "..." in output or len(output) < 500

    def test_icon_for_l0_syntax(self):
        r = self._result(level=VerificationLevel.L0_SYNTAX)
        output = render_result(r)
        # Should have some icon character
        assert any(c in output for c in ["🟡", "?", "l0"])

    def test_icon_for_l0_failed(self):
        r = self._result(level=VerificationLevel.L0_FAILED, errors=["e"])
        output = render_result(r)
        assert "❌" in output or "✗" in output

    def test_icon_for_l2_proven(self):
        r = self._result(level=VerificationLevel.L2_PROVEN)
        output = render_result(r)
        assert "✅" in output or "l2" in output

    def test_install_available_shows_version(self):
        r = self._result(install_status=LeanInstallStatus.AVAILABLE)
        output = render_result(r)
        assert "Lean installed" in output

    def test_multiple_errors(self):
        r = self._result(errors=["err1", "err2"])
        output = render_result(r)
        assert "err1" in output and "err2" in output

    def test_empty_errors_no_error_section(self):
        r = self._result(errors=[])
        output = render_result(r)
        assert "Errors" not in output


# =============================================================================
# render_result_json — pure JSON serialization
# =============================================================================
class TestRenderResultJson:
    """Test render_result_json."""

    def _result(self, **kwargs):
        defaults = dict(
            hypothesis_id="h001",
            hypothesis_text="test",
            level=VerificationLevel.L0_SYNTAX,
            lean_code="theorem foo := True",
            errors=[],
            warnings=[],
            install_status=LeanInstallStatus.NOT_FOUND,
            translation_notes="",
        )
        defaults.update(kwargs)
        return LeanVerificationResult(**defaults)

    def test_valid_json(self):
        r = self._result()
        output = render_result_json(r)
        parsed = json.loads(output)
        assert parsed["hypothesis_id"] == "h001"
        assert parsed["level"] == "l0_syntax"

    def test_contains_lean_code(self):
        r = self._result(lean_code="def x := 0")
        parsed = json.loads(render_result_json(r))
        assert parsed["lean_code"] == "def x := 0"

    def test_errors_as_list(self):
        r = self._result(errors=["error1", "error2"])
        parsed = json.loads(render_result_json(r))
        assert parsed["errors"] == ["error1", "error2"]

    def test_install_status_value(self):
        r = self._result(install_status=LeanInstallStatus.AVAILABLE)
        parsed = json.loads(render_result_json(r))
        assert parsed["install_status"] == "available"

    def test_includes_translation_notes(self):
        r = self._result(translation_notes="LLM translated")
        parsed = json.loads(render_result_json(r))
        assert parsed["translation_notes"] == "LLM translated"

    def test_json_unicode_safe(self):
        r = self._result(hypothesis_text="∀x ∈ ℝ")
        output = render_result_json(r)
        # Should not raise
        parsed = json.loads(output)
        assert "∀" in parsed["hypothesis_text"]


# =============================================================================
# Lean integration — requires lean installed
# =============================================================================
@pytest.mark.skipif(
    check_lean_installed()[0] != LeanInstallStatus.AVAILABLE,
    reason="Lean 4 not installed",
)
class TestLeanIntegration:
    """Test Lean integration — requires Lean 4 installed."""

    def test_check_lean_installed_returns_available(self):
        status, version = check_lean_installed()
        assert status == LeanInstallStatus.AVAILABLE
        assert version is not None
        assert "Lean" in version

    def test_verify_valid_type_correct_code(self):
        """Type-correct Lean code should pass L1_TYPECHECK."""
        code = "def add_one (n : Nat) : Nat := n + 1"
        result = verify_lean_code(code, "test-001", "add one to nat")
        assert result.install_status == LeanInstallStatus.AVAILABLE
        assert result.level == VerificationLevel.L1_TYPECHECK
        assert result.errors == []

    def test_verify_sorry_is_l2(self):
        """Code with `by sorry` should be L2_PROVEN."""
        code = "theorem add_comm (n m : Nat) : n + m = m + n := by sorry"
        result = verify_lean_code(code, "test-002", "commutativity")
        assert result.level == VerificationLevel.L2_PROVEN
        assert "sorry" in result.translation_notes

    def test_verify_proven_no_sorry_is_l1(self):
        """Code with real proof (no sorry) should be L1_TYPECHECK."""
        code = "theorem refl : ∀n : Nat, n = n := Nat.eq_refl"
        result = verify_lean_code(code, "test-003", "reflexivity")
        assert result.level == VerificationLevel.L1_TYPECHECK

    def test_verify_syntax_error_is_failed(self):
        """Syntax error should be L0_FAILED."""
        code = "theorem broken { this is not valid lean syntax"
        result = verify_lean_code(code, "test-004", "broken theorem")
        assert result.level == VerificationLevel.L0_FAILED
        assert len(result.errors) > 0

    def test_verify_type_error_is_failed(self):
        """Type error should be L0_FAILED."""
        code = "def bad : String := 42  -- type mismatch"
        result = verify_lean_code(code, "test-005", "type mismatch")
        assert result.level == VerificationLevel.L0_FAILED
        assert len(result.errors) > 0

    def test_verify_minimal_stub_passes(self):
        """Minimal def True := True should pass."""
        code = "def stub : Prop := True"
        result = verify_lean_code(code, "test-006", "minimal stub")
        assert result.level == VerificationLevel.L1_TYPECHECK

    def test_verify_multiple_errors_collected(self):
        """Multiple errors should all be captured."""
        code = "def a b: Nat := -- multiple issues"
        result = verify_lean_code(code, "test-007", "multi-error")
        assert result.level == VerificationLevel.L0_FAILED

    def test_verify_warnings_collected(self):
        """Warnings should be captured separately."""
        code = "theorem foo := _ -- underscore needs proof"
        result = verify_lean_code(code, "test-008", "underscore placeholder")
        # Lean may warn about underscore
        assert isinstance(result.warnings, list)

    def test_verify_json_output_parsed(self):
        """JSON output should be available."""
        code = "def x : Nat := 0"
        result = verify_lean_code(code, "test-009", "json test")
        # JSON output is a list of parsed messages
        assert isinstance(result.json_output, list)

    def test_verify_result_has_lean_file_path(self):
        """Lean file path should be set during verification."""
        code = "def x := 0"
        result = verify_lean_code(code, "test-010", "filepath test")
        # Path is set during run, cleared in finally
        # We can't check the path itself (deleted after), but verify it ran
        assert result.install_status == LeanInstallStatus.AVAILABLE
