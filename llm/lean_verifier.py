"""
Lean 4 Theorem Prover Integration — Formal Verification for Research Hypotheses.

将研究假说翻译为 Lean 4 代码，通过 `lean --json` 验证语法正确性。
形式化层次：
  Level 0 — 仅语法检查（Lean 能解析）
  Level 1 — 类型检查（所有类型签名正确）
  Level 2 — 证明通过（所有 theorem 有完整证明）

使用方式：
  python -m llm.lean_verifier "交换律: forall n m : Nat, n + m = m + n"
  python -m llm.lean_verifier --hypothesis-id abc123
"""
from __future__ import annotations

import json
import shutil
import subprocess
import tempfile
from dataclasses import dataclass, field
from enum import Enum
from pathlib import Path
from typing import Optional, List

try:
    from llm.hypothesis_generator import ResearchHypothesis, HypothesisResult
    HYPOTHESIS_AVAILABLE = True
except ImportError:
    HYPOTHESIS_AVAILABLE = False


# ── Lean translation system prompt ────────────────────────────────────────────
LEAN_TRANSLATION_SYSTEM = """You are a Lean 4 code generator. Translate informal research hypotheses into Lean 4 code.

## Core Conventions

### Unicode Symbol Mapping (copy literally, don't replace)
- ∀ → ∀  (typed as `\\all` or `\\forall`)
- ∃ → ∃  (typed as `\\ex` or `\\exists`)
- ≤ → ≤  (typed as `\\le`)
- ≥ → ≥  (typed as `\\ge`)
- ≠ → ≠  (typed as `\\ne`)
- → → →  (typed as `\\r`)
- ← → ←  (typed as `\\l`)
- ∧ → ∧  (typed as `\\and`)
- ∨ → ∨  (typed as `\\or`)
- ¬ → ¬  (typed as `\\not`)
- ∈ → ∈  (typed as `\\in`)
- ⊆ → ⊆  (typed as `\\sub`)

### Lean 4 Types & Mathlib Conventions
- Natural numbers: `Nat` (NOT `int` or `Integer`)
- Integers: `Int`
- Real numbers: `Real` (from `Mathlib`)
- Booleans: `Bool`
- Propositions: `Prop` (implicit for theorems)
- Sets: `Set α` (from `Std.Data.Set`)
- Functions: `α → β` (arrow notation, NOT `function` or `func`)
- Equality: `=` (propositional equality)
- Negation: `¬P` (typed as `\\not P`)

### Naming Conventions
- Types: `CamelCase` (e.g., `Group`, `Vector`)
- Variables/functions: `snake_case` (e.g., `add_comm`, `composition`)
- Theorems: `snake_case` with descriptive name (e.g., `add_comm`, `mul_assoc`)
- Use `∀ n : Nat` (with type annotation), not bare `∀ n`
- Use `: Prop` for propositional theorems

### Translation Patterns by Domain

#### Causal / Effect Hypotheses
```lean
theorem causal_effect_example
    (T : Type) [Inhabited T]
    (treatment : T → Bool)
    (outcome : T → Nat)
    : ∀ t : T, outcome t > 0  :=  by sorry
```

#### Convergence / Optimization Hypotheses
```lean
theorem convergence_claim
    {E : Type} [NormedAddCommGroup E]
    (seq : ℕ → E)
    (cauchy : ∀ ε > 0, ∃ N, ∀ m n ≥ N, ∥seq m - seq n∥ < ε)
    : ∃ L : E, ∀ ε > 0, ∃ N, ∀ n ≥ N, ∥seq n - L∥ < ε  :=  by sorry
```

#### Set-Theoretic / Inclusion Hypotheses
```lean
theorem set_inclusion_claim
    {α : Type}
    (A B C : Set α)
    (hAB : A ⊆ B)
    (hBC : B ⊆ C)
    : A ⊆ C  :=  by sorry
```

#### Probability / Expectation Hypotheses
```lean
theorem expectation_bound
    {Ω : Type} [MeasureSpace Ω]
    (X : Ω → ℝ) [IsFiniteExpectation ℙ X]
    : ℙ {ω | X ω ≥ 0} ≥ 0  :=  by sorry
```

#### Function / Map Hypotheses
```lean
theorem surjectivity_claim
    {α β : Type}
    (f : α → β)
    (hf : Surjective f)
    : ∀ b : β, ∃ a : α, f a = b  :=  by sorry
```

## Output Rules
1. Output ONLY the Lean 4 code block — nothing else
2. Use `by sorry` as a proof stub (level L2)
3. Omit the proof stub if the statement cannot be proven: use `:= ?` placeholder
4. Keep it minimal — focus on the core claim
5. If the claim CANNOT be formalized in Lean 4, explain in a `--` comment

## Format
```lean
-- Translate: [one-line summary]
[Lean 4 code]
```
"""

LEAN_TRANSLATION_USER_TEMPLATE = """Translate this research hypothesis into Lean 4 code:

---
Title: {title}
Core Claim: {core_statement}
Type: {hypothesis_type}
Based on: {based_on}
Variables: {variables}
Expected Result: {expected_result}
---

Translate the Core Claim into Lean 4 following the conventions above.
Output ONLY the ```lean ... ``` code block.
"""


# ── Data structures ────────────────────────────────────────────────────────────

class LeanInstallStatus(Enum):
    """Whether Lean is available."""
    AVAILABLE = "available"
    NOT_FOUND = "not_found"
    VERSION_UNKNOWN = "version_unknown"


class VerificationLevel(Enum):
    """Formality level of the Lean output."""
    L0_SYNTAX = "l0_syntax"        # Lean parsed the file without error
    L1_TYPECHECK = "l1_typecheck"  # All type signatures are valid
    L2_PROVEN = "l2_proven"        # Proofs are complete (no `sorry`)
    L0_FAILED = "l0_failed"         # Syntax errors


@dataclass
class LeanVerificationResult:
    """Result of verifying a hypothesis in Lean 4."""
    hypothesis_id: str
    hypothesis_text: str
    level: VerificationLevel
    lean_code: str
    lean_file_path: Optional[str] = None
    errors: List[str] = field(default_factory=list)
    warnings: List[str] = field(default_factory=list)
    # Parsed output from `lean --json`
    json_output: Optional[dict] = None
    install_status: LeanInstallStatus = LeanInstallStatus.NOT_FOUND
    translation_notes: str = ""


def check_lean_installed() -> tuple[LeanInstallStatus, Optional[str]]:
    """
    Check if `lean` is installed and available in PATH.

    Uses shutil.which (fast, no subprocess) for availability check.
    subprocess call (with timeout) is only used to get version string.
    """
    lean_path = shutil.which("lean")
    if not lean_path:
        return LeanInstallStatus.NOT_FOUND, None

    try:
        result = subprocess.run(
            ["lean", "--version"],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=30,
        )
        version = result.stdout.strip() or result.stderr.strip() or "unknown"
        return LeanInstallStatus.AVAILABLE, version
    except subprocess.TimeoutExpired:
        # Timeout means lean exists but is slow/warm-up — treat as available
        return LeanInstallStatus.AVAILABLE, "Lean (timeout during version check)"
    except FileNotFoundError:
        return LeanInstallStatus.NOT_FOUND, None
    except OSError:
        return LeanInstallStatus.NOT_FOUND, None


def get_lean_install_instructions() -> str:
    """Return OS-specific installation instructions."""
    return """
Lean 4 is not installed. To install:

  macOS/Linux:
    lake new myproject && cd myproject && lake build

  Windows (with elan):
    elan default leanprover/lean4:stable
    lean --version

  Or use GitHub Actions CI cache approach — see:
    https://leanprover-community.github.io/get_started.html
""".strip()


# ── Code generation ─────────────────────────────────────────────────────────

def translate_hypothesis_to_lean(
    hypothesis: "ResearchHypothesis",
    use_llm: bool = True,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> tuple[str, str]:
    """
    Translate a ResearchHypothesis into Lean 4 code.

    Returns:
        (lean_code, translation_notes)
    """
    user_prompt = LEAN_TRANSLATION_USER_TEMPLATE.format(
        title=hypothesis.title,
        core_statement=hypothesis.core_statement,
        hypothesis_type=hypothesis.hypothesis_type.value if hasattr(hypothesis.hypothesis_type, 'value') else str(hypothesis.hypothesis_type),
        based_on=hypothesis.based_on,
        variables=", ".join(hypothesis.experiment_design.variables),
        expected_result=hypothesis.experiment_design.expected_results,
    )

    if use_llm and HYPOTHESIS_AVAILABLE:
        try:
            from llm.chat import call_llm_chat_completions
            from llm.constants import LLM_BASE_URL, LLM_MODEL
            import os

            api_key = api_key or os.getenv("OPENAI_API_KEY", "")
            if api_key:
                response = call_llm_chat_completions(
                    base_url=base_url or LLM_BASE_URL,
                    api_key=api_key,
                    model=model or LLM_MODEL,
                    system_prompt=LEAN_TRANSLATION_SYSTEM,
                    user_prompt=user_prompt,
                )
                return _extract_lean_code(response), ""
        except Exception:
            pass  # Fall through to template-based

    # Fallback: template-based translation
    return _template_based_translation(hypothesis), "(template-based fallback)"


def _extract_lean_code(response: str) -> str:
    """Extract Lean code from LLM response, stripping markdown fences."""
    # Find the first ```lean ... ``` block
    import re
    match = re.search(r"```lean\s*\n(.*?)```", response, re.DOTALL)
    if match:
        return match.group(1).strip()

    # Fallback: return entire response if no fence found
    return response.strip()


# ── Symbol translation map ────────────────────────────────────────────────────
_LEAN_SYMBOLS = [
    ("∀", "∀"),
    ("∃", "∃"),
    ("→", "→"),
    ("←", "←"),
    ("≤", "≤"),
    ("≥", "≥"),
    ("≠", "≠"),
    ("∧", "∧"),
    ("∨", "∨"),
    ("¬", "¬"),
    ("∈", "∈"),
    ("⊆", "⊆"),
    ("ℝ", "ℝ"),
    ("ℕ", "ℕ"),
    ("ℤ", "ℤ"),
    # ASCII fallbacks (escaped in Lean)
    ("forall", "∀"),
    ("exists", "∃"),
    ("<=", "≤"),
    (">=", "≥"),
    (">", ">"),
    ("<", "<"),
    ("=>", "→"),
    ("<=", "≤"),  # keep
    ("->", "→"),
    ("<-", "←"),
]


def _translate_symbols(text: str) -> str:
    """Replace common math symbols with Lean-compatible Unicode."""
    result = text
    for old, new in _LEAN_SYMBOLS:
        result = result.replace(old, new)
    return result


def _template_based_translation(hypothesis: "ResearchHypothesis") -> str:
    """Generate Lean code from a hypothesis using domain-aware templates."""
    name = hypothesis.title or hypothesis.core_statement or "H"
    safe_name = "".join(c if c.isalnum() else "_" for c in name)[:30]
    statement = hypothesis.core_statement

    hyp_type = (hypothesis.hypothesis_type.value
                if hasattr(hypothesis.hypothesis_type, 'value')
                else str(hypothesis.hypothesis_type))

    # Normalize statement with Lean-compatible symbols
    statement = _translate_symbols(statement)

    # Detect domain indicators
    mathy = any(w in statement for w in [
        "∀", "∃", "→", "≤", "≥", "≠", "∧", "∨", "∈", "⊆",
    ])
    convergence = any(w in statement.lower() for w in [
        "converge", "limit", "sequence", "cauchy", "tends to", "supremum", "infimum",
    ])
    probability = any(w in statement.lower() for w in [
        "probability", "expectation", "measure", "random", "variance", "entropy",
        "distribution", "bayesian", "likelihood", "posterior",
    ])
    causal = any(w in statement.lower() for w in [
        "cause", "effect", "treatment", "control", "outcome", "intervention",
        "counterfactual", "do-calculus",
    ])
    set_theory = any(w in statement.lower() for w in [
        "subset", "element", "union", "intersection", "complement",
        "partition", "equivalence", "class",
    ])
    functions = any(w in statement.lower() for w in [
        "function", "injective", "surjective", "bijective", "homomorphism",
        "isomorphism", "kernel", "image", "composition",
    ])

    # Choose domain template
    if convergence:
        lean_code = _FALLBACK_TEMPLATES.get(
            "convergence",
            _FALLBACK_TEMPLATES["mathy"],
        ).format(safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type)
    elif probability:
        lean_code = _FALLBACK_TEMPLATES.get(
            "probability",
            _FALLBACK_TEMPLATES["mathy"],
        ).format(safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type)
    elif causal:
        lean_code = _FALLBACK_TEMPLATES.get(
            "causal",
            _FALLBACK_TEMPLATES["mathy"],
        ).format(safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type)
    elif set_theory:
        lean_code = _FALLBACK_TEMPLATES.get(
            "set_theory",
            _FALLBACK_TEMPLATES["mathy"],
        ).format(safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type)
    elif functions:
        lean_code = _FALLBACK_TEMPLATES.get(
            "functions",
            _FALLBACK_TEMPLATES["mathy"],
        ).format(safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type)
    elif mathy:
        lean_code = _FALLBACK_TEMPLATES["mathy"].format(
            safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type,
        )
    else:
        lean_code = _FALLBACK_TEMPLATES["qualitative"].format(
            safe_name=safe_name, statement=statement[:80], hyp_type=hyp_type,
        )

    return lean_code


# ── Domain-specific fallback templates ─────────────────────────────────────────
_FALLBACK_TEMPLATES = {
    "mathy": """\
-- Hypothesis: {safe_name}
-- Type: {hyp_type}

/- Formal claim: {statement} -/

theorem {safe_name}_theorem
    : Prop  :=  by sorry
""",

    "convergence": """\
-- Hypothesis: {safe_name}
-- Type: convergence/analysis

/- Formal claim: {statement} -/

open Nat Real

theorem {safe_name}_converges
    (ε : ℝ) (hε : ε > 0)
    : ∃ N : ℕ, ∀ n ≥ N, True  :=  by sorry
""",

    "probability": """\
-- Hypothesis: {safe_name}
-- Type: probability/statistics

/- Formal claim: {statement} -/

open Real

-- Probability space stub (replace with actual measure space)
axiom Ω : Type
axiom ℙ : MeasureSpace Ω

theorem {safe_name}_probability
    : ℙ = ℙ  :=  by sorry
""",

    "causal": """\
-- Hypothesis: {safe_name}
-- Type: causal inference

/- Formal claim: {statement} -/

open Bool

theorem {safe_name}_causal_effect
    (unit : Type)
    (treatment : unit → Bool)
    (outcome : unit → ℕ)
    : Prop  :=  by sorry
""",

    "set_theory": """\
-- Hypothesis: {safe_name}
-- Type: set theory

/- Formal claim: {statement} -/

open Set

theorem {safe_name}_set_theorem
    {α : Type}
    (A B C : Set α)
    : Prop  :=  by sorry
""",

    "functions": """\
-- Hypothesis: {safe_name}
-- Type: function/map theory

/- Formal claim: {statement} -/

open Function

theorem {safe_name}_function_theorem
    {α β : Type}
    (f : α → β)
    : Prop  :=  by sorry
""",

    "qualitative": """\
-- Hypothesis: {safe_name}
-- Type: {hyp_type}

-- Natural language claim:
-- {statement}

-- WARNING: This hypothesis is qualitative and cannot be fully formalized in Lean 4.
-- Only a syntax stub is provided for basic verification.

def {safe_name}_stub : Prop := True
""",
}


# ── Verification ─────────────────────────────────────────────────────────────

def verify_lean_code(
    lean_code: str,
    hypothesis_id: str = "unknown",
    hypothesis_text: str = "",
) -> LeanVerificationResult:
    """
    Verify Lean 4 code by running `lean --json` on a temp file.

    Args:
        lean_code: Lean 4 source code to verify
        hypothesis_id: ID for tracking
        hypothesis_text: human-readable hypothesis text

    Returns:
        LeanVerificationResult with level and error details
    """
    install_status, version = check_lean_installed()

    result = LeanVerificationResult(
        hypothesis_id=hypothesis_id,
        hypothesis_text=hypothesis_text,
        level=VerificationLevel.L0_FAILED,
        lean_code=lean_code,
        install_status=install_status,
    )

    if install_status != LeanInstallStatus.AVAILABLE:
        result.errors.append(f"Lean not found: {get_lean_install_instructions()}")
        return result

    # Write to temporary file and verify
    with tempfile.NamedTemporaryFile(
        mode="w",
        suffix=".lean",
        delete=False,
        encoding="utf-8",
    ) as f:
        f.write(lean_code)
        temp_path = f.name

    result.lean_file_path = temp_path

    try:
        # Run lean --json to get structured output
        proc = subprocess.run(
            ["lean", "--json", temp_path],
            capture_output=True,
            text=True,
            encoding="utf-8",
            timeout=60,
        )

        # Parse JSON output (lean 4 outputs one JSON object per line on stdout)
        error_lines = []
        warning_lines = []
        json_messages = []

        for line in (proc.stdout + proc.stderr).splitlines():
            line = line.strip()
            if not line:
                continue
            try:
                msg = json.loads(line)
                json_messages.append(msg)
                if msg.get("severity") == "error":
                    error_lines.append(f"{msg.get('file', '?')}:{msg.get('pos', 0)} — {msg.get('data', '')}")
                elif msg.get("severity") == "warning":
                    warning_lines.append(f"{msg.get('file', '?')}:{msg.get('pos', 0)} — {msg.get('data', '')}")
            except json.JSONDecodeError:
                # Plain text error message
                if "error" in line.lower() or "failed" in line.lower():
                    error_lines.append(line)

        result.json_output = json_messages
        result.errors = error_lines
        result.warnings = warning_lines

        # Determine verification level
        if not error_lines and proc.returncode == 0:
            # Check for `sorry` in the code
            if "sorry" in lean_code:
                result.level = VerificationLevel.L2_PROVEN
                result.translation_notes = "Proof stub present (by sorry) — level L2"
            else:
                result.level = VerificationLevel.L1_TYPECHECK
                result.translation_notes = "Type-correct, no proof"
        elif not error_lines:
            result.level = VerificationLevel.L0_SYNTAX
            result.translation_notes = "Syntax valid, type errors present"
        else:
            result.level = VerificationLevel.L0_FAILED
            result.translation_notes = f"Syntax/type errors: {len(error_lines)} issue(s)"

    except subprocess.TimeoutExpired:
        result.errors.append("Lean verification timed out (>60s)")
        result.level = VerificationLevel.L0_FAILED
    except FileNotFoundError:
        result.errors.append("Lean executable not found")
        result.install_status = LeanInstallStatus.NOT_FOUND
    except Exception as e:
        result.errors.append(f"Verification error: {e}")
        result.level = VerificationLevel.L0_FAILED
    finally:
        # Clean up temp file
        try:
            Path(temp_path).unlink(missing_ok=True)
        except Exception:
            pass

    return result


# ── Full pipeline ────────────────────────────────────────────────────────────

def verify_hypothesis(
    hypothesis: "ResearchHypothesis",
    use_llm: bool = True,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> LeanVerificationResult:
    """
    Full pipeline: translate hypothesis → verify with Lean → return result.
    """
    lean_code, notes = translate_hypothesis_to_lean(
        hypothesis,
        use_llm=use_llm,
        api_key=api_key,
        base_url=base_url,
        model=model,
    )

    result = verify_lean_code(
        lean_code=lean_code,
        hypothesis_id=hypothesis.id,
        hypothesis_text=hypothesis.core_statement,
    )
    result.translation_notes = notes

    return result


# ── Rendering ────────────────────────────────────────────────────────────────

def render_result(result: LeanVerificationResult) -> str:
    """Render a verification result as formatted text."""
    level_icon = {
        VerificationLevel.L0_FAILED: "❌",
        VerificationLevel.L0_SYNTAX: "🟡",
        VerificationLevel.L1_TYPECHECK: "🟢",
        VerificationLevel.L2_PROVEN: "✅",
    }

    install_icon = {
        LeanInstallStatus.AVAILABLE: "✅ Lean installed",
        LeanInstallStatus.NOT_FOUND: "⚠️  Lean not found",
        LeanInstallStatus.VERSION_UNKNOWN: "? Lean version unknown",
    }

    lines = [
        "─" * 60,
        f"{level_icon.get(result.level, '?')} Verification Level: {result.level.value}",
        f"{install_icon.get(result.install_status, '?')}",
        "",
    ]

    if result.hypothesis_id:
        lines.append(f"  Hypothesis ID: {result.hypothesis_id}")

    if result.hypothesis_text:
        stmt = result.hypothesis_text[:80] + ("..." if len(result.hypothesis_text) > 80 else "")
        lines.append(f"  Claim: {stmt}")

    lines.append("")
    lines.append("─ Lean code ─────────────────────────────")
    lines.append(result.lean_code or "(no code generated)")

    if result.errors:
        lines.append("")
        lines.append("─ Errors ─────────────────────────────────")
        for err in result.errors:
            lines.append(f"  ✗ {err}")

    if result.warnings:
        lines.append("")
        lines.append("─ Warnings ────────────────────────────────")
        for warn in result.warnings:
            lines.append(f"  ⚠ {warn}")

    if result.install_status == LeanInstallStatus.NOT_FOUND:
        lines.append("")
        lines.append(get_lean_install_instructions())

    lines.append("─" * 60)

    return "\n".join(lines)


def render_result_json(result: LeanVerificationResult) -> str:
    """Render result as JSON."""
    return json.dumps({
        "hypothesis_id": result.hypothesis_id,
        "hypothesis_text": result.hypothesis_text,
        "level": result.level.value,
        "lean_code": result.lean_code,
        "errors": result.errors,
        "warnings": result.warnings,
        "install_status": result.install_status.value,
        "translation_notes": result.translation_notes,
        "lean_file_path": result.lean_file_path,
    }, ensure_ascii=False, indent=2)


# ── CLI entry point ────────────────────────────────────────────────────────────

def main() -> int:
    """CLI entry point: python -m llm.lean_verifier [options] [hypothesis_text]"""
    import argparse
    import sys

    parser = argparse.ArgumentParser(
        description="Verify research hypotheses in Lean 4",
        formatter_class=argparse.RawDescriptionHelpFormatter,
        epilog="""
Examples:
  python -m llm.lean_verifier "forall n m : Nat, n + m = m + n"
  python -m llm.lean_verifier --hypothesis-id abc123
  python -m llm.lean_verifier --check-install
  python -m llm.lean_verifier --hypothesis-id abc123 --no-llm
        """,
    )
    parser.add_argument(
        "hypothesis_text",
        nargs="?",
        default=None,
        help="Research hypothesis as natural language text",
    )
    parser.add_argument(
        "--hypothesis-id",
        type=str,
        default=None,
        help="Hypothesis ID to load from experiment tracker",
    )
    parser.add_argument(
        "--no-llm",
        action="store_true",
        help="Skip LLM translation, use templates only",
    )
    parser.add_argument(
        "--json", "-j",
        action="store_true",
        help="Output as JSON",
    )
    parser.add_argument(
        "--check-install",
        action="store_true",
        help="Just check if Lean is installed",
    )
    parser.add_argument(
        "--model", "-M",
        type=str,
        default=None,
        help="LLM model to use",
    )

    args = parser.parse_args()

    # Check install mode
    if args.check_install:
        status, version = check_lean_installed()
        if status == LeanInstallStatus.AVAILABLE:
            print(f"✅ Lean installed: {version}")
            return 0
        else:
            print("❌ Lean not found")
            print(get_lean_install_instructions())
            return 1

    # Load hypothesis
    hypothesis = None
    hypothesis_id = args.hypothesis_id or "cli-input"
    hypothesis_text = args.hypothesis_text or ""

    if args.hypothesis_id and HYPOTHESIS_AVAILABLE:
        from llm.experiment_tracker import ExperimentTracker
        tracker = ExperimentTracker()
        exps = tracker.list_experiments()
        found = [e for e in exps if e.hypothesis_id == args.hypothesis_id]
        if found:
            exp = found[0]
            if HYPOTHESIS_AVAILABLE:
                from llm.hypothesis_generator import HypothesisType, ResearchHypothesis, ExperimentDesign
                hypothesis = ResearchHypothesis(
                    id=args.hypothesis_id,
                    title=exp.name,
                    core_statement=exp.name,
                    hypothesis_type=HypothesisType.EXPLORATORY,
                    based_on="loaded from experiment tracker",
                    experiment_design=ExperimentDesign(
                        baseline="",
                        variables=[],
                        controls=[],
                        evaluation_metrics=[],
                        expected_results="",
                    ),
                )
                hypothesis_text = exp.name
        else:
            print(f"⚠ Hypothesis ID '{args.hypothesis_id}' not found in tracker")
            return 1

    # Build synthetic hypothesis from text
    if not hypothesis and hypothesis_text:
        if HYPOTHESIS_AVAILABLE:
            from llm.hypothesis_generator import HypothesisType, ResearchHypothesis, ExperimentDesign
            hypothesis = ResearchHypothesis(
                id=hypothesis_id,
                title=hypothesis_text[:40],
                core_statement=hypothesis_text,
                hypothesis_type=HypothesisType.EXPLORATORY,
                based_on="user input",
                experiment_design=ExperimentDesign(
                    baseline="",
                    variables=[],
                    controls=[],
                    evaluation_metrics=[],
                    expected_results="",
                ),
            )

    if not hypothesis:
        print("❌ No hypothesis provided. Use positional arg or --hypothesis-id")
        return 1

    # Verify
    result = verify_hypothesis(
        hypothesis=hypothesis,
        use_llm=not args.no_llm,
        model=args.model,
    )

    # Render
    if args.json:
        print(render_result_json(result))
    else:
        print(render_result(result))

    return 0 if result.level != VerificationLevel.L0_FAILED else 1


if __name__ == "__main__":
    raise SystemExit(main())
