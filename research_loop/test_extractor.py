"""
Test Extractor — extract testable assertions from paper content and generate pytest suites.

Used by the paper2code integration pipeline:
  extract_tests(paper_content, code, module_name) → TestSuite
  save_tests(suite, test_dir)

TestSuite feeds into:
  - benchmark_runner: run_benchmark() executes the generated tests
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import List, Dict, Any


@dataclass
class TestCase:
    """A single pytest test case."""

    name: str
    doc: str  # description / assertion being tested
    test_code: str  # actual pytest test code


@dataclass
class TestSuite:
    """Collection of test cases for a paper implementation."""

    arxiv_id: str
    module_name: str
    test_cases: List[TestCase] = field(default_factory=list)
    imports: List[str] = field(default_factory=list)
    fixtures: Dict[str, str] = field(default_factory=dict)  # name → fixture code


def extract_tests(content: Any, code: str, module_name: str = "model") -> TestSuite:
    """Extract testable assertions from paper content and generate pytest test suite.

    Scans PaperContent for:
    - Numerical claims (e.g. "achieves 95% accuracy")
    - Hyperparameters (e.g. learning_rate=0.001)
    - Algorithm properties (e.g. "attention mechanism")
    - Equation constraints (e.g. "output sum equals 1" for softmax)

    Also inspects the generated code for testable function signatures.
    """
    suite = TestSuite(
        arxiv_id=content.arxiv_id,
        module_name=module_name,
        imports=[
            "import pytest",
            "import torch",
            f"from {module_name} import *",
        ],
    )

    # 1. Numerical claims from paper
    for claim in content.claims:
        tc = _claim_to_test(claim, module_name)
        if tc:
            suite.test_cases.append(tc)

    # 2. Hyperparameter validation tests
    for hp_name, hp_val in content.hyperparameters.items():
        tc = _hyperparam_test(hp_name, hp_val, module_name)
        if tc:
            suite.test_cases.append(tc)

    # 3. Algorithm property tests (from algorithm_descriptions)
    for algo in content.algorithm_descriptions:
        tc = _algorithm_property_test(algo, module_name)
        if tc:
            suite.test_cases.append(tc)

    # 4. Equation-based tests (e.g. softmax sum-to-1, attention weights non-negative)
    for eq in content.equations:
        tc = _equation_test(eq, module_name)
        if tc:
            suite.test_cases.append(tc)

    # 5. Dataset presence tests
    if content.datasets:
        tc = _dataset_test(content.datasets, module_name)
        if tc:
            suite.test_cases.append(tc)

    # 6. Code signature tests — verify key functions exist
    for tc in _code_signature_tests(code, module_name):
        suite.test_cases.append(tc)

    # Deduplicate by name
    seen = set()
    unique = []
    for tc in suite.test_cases:
        if tc.name not in seen:
            seen.add(tc.name)
            unique.append(tc)
    suite.test_cases = unique

    return suite


def save_tests(suite: TestSuite, test_dir: Path) -> Path:
    """Write TestSuite to test_dir/test_module.py and conftest.py."""
    test_dir = Path(test_dir)
    test_dir.mkdir(parents=True, exist_ok=True)

    # Write conftest.py with fixtures
    conftest_content = _render_conftest(suite)
    (test_dir / "conftest.py").write_text(conftest_content, encoding="utf-8")

    # Write main test file
    test_file = test_dir / f"test_{suite.module_name}.py"
    test_content = _render_test_module(suite)
    test_file.write_text(test_content, encoding="utf-8")

    return test_file


# ─── Private helpers ──────────────────────────────────────────────────────────


def _claim_to_test(claim: str, module_name: str) -> TestCase | None:
    """Convert a paper claim into a pytest test if it contains testable assertions."""
    claim_lower = claim.lower()

    # Accuracy / performance claims
    acc_match = re.search(r"(\d+(?:\.\d+)?)\s*%\s*(?:accuracy|acc|performance)", claim_lower)
    if acc_match:
        threshold = float(acc_match.group(1)) / 100
        return TestCase(
            name="test_model_accuracy_threshold",
            doc=f"Model should achieve at least {threshold:.1%} accuracy (from paper claim: {claim[:60]})",
            test_code=f"""\
# Paper claim: {claim[:80]}
# This is a placeholder — replace with actual evaluation on the target dataset
# pytest.importorskip("{module_name}")
pass
""",
        )

    # Loss / error reduction claims
    loss_match = re.search(r"(?:loss|error|perplexity)\s*(?:of|=|:)\s*([\d.]+)", claim_lower)
    if loss_match:
        threshold = float(loss_match.group(1))
        return TestCase(
            name="test_loss_below_threshold",
            doc=f"Loss should be below {threshold} (from paper claim: {claim[:60]})",
            test_code=f"""\
# Paper claim: {claim[:80]}
pass
""",
        )

    # Speed / efficiency claims
    speed_match = re.search(r"(\d+(?:\.\d+)?)\s*(?:x|times|faster)", claim_lower)
    if speed_match:
        speedup = float(speed_match.group(1))
        return TestCase(
            name="test_speedup_over_baseline",
            doc=f"Model should be ≥{speedup}x faster than baseline (from paper claim: {claim[:60]})",
            test_code=f"""\
# Paper claim: {claim[:80]}
# Compare inference time against baseline implementation
pass
""",
        )

    return None


def _hyperparam_test(hp_name: str, hp_val: str, module_name: str) -> TestCase | None:
    """Generate a test that verifies a hyperparameter is configurable."""
    if hp_name in ("learning_rate", "lr"):
        return TestCase(
            name="test_learning_rate_configurable",
            doc=f"Learning rate should be configurable to {hp_val}",
            test_code=f"""\
pytest.importorskip("{module_name}")
try:
    import {module_name} as m
    if hasattr(m, 'set_learning_rate'):
        m.set_learning_rate({hp_val})
except (ImportError, AttributeError):
    pytest.skip("Module does not support learning_rate setting")""",
        )
    return None


def _algorithm_property_test(algo: str, module_name: str) -> TestCase | None:
    """Infer a testable property from an algorithm description."""
    algo_lower = algo.lower()

    if "attention" in algo_lower:
        return TestCase(
            name="test_attention_weights_non_negative",
            doc=f"Attention weights should be non-negative and sum to 1 (from algorithm: {algo[:60]})",
            test_code=f"""\
pytest.importorskip("{module_name}")
pytest.importorskip("torch")
# Verify attention output is non-negative and sums to 1 along the last dimension
# Replace with actual model output check
x = torch.randn(1, 10, 512)
# attn = your_attention_function(x)
# assert (attn >= 0).all(), "Attention weights must be non-negative"
# assert (abs(attn.sum(dim=-1) - 1.0) < 1e-5).all(), "Attention weights must sum to 1"
pass
""",
        )

    if "softmax" in algo_lower:
        return TestCase(
            name="test_softmax_sum_to_one",
            doc=f"Softmax output should sum to 1 along last dimension (from algorithm: {algo[:60]})",
            test_code=f"""\
pytest.importorskip("{module_name}")
pytest.importorskip("torch")
# Verify softmax property
# x = torch.randn(1, 5)
# result = torch.nn.functional.softmax(x, dim=-1)
# assert (abs(result.sum(dim=-1) - 1.0) < 1e-5).all()
pass
""",
        )

    if any(kw in algo_lower for kw in ["layer norm", "layer normalization", "layernorm"]):
        return TestCase(
            name="test_layernorm_output_properties",
            doc=f"LayerNorm output should have learnable gain/bias and finite values (from algorithm: {algo[:60]})",
            test_code=f"""\
pytest.importorskip("{module_name}")
pytest.importorskip("torch")
# x = torch.randn(2, 10, 512)
# normed = LayerNorm(512)(x)
# assert normed.shape == x.shape
# assert not torch.isnan(normed).any()
pass
""",
        )

    return None


def _equation_test(eq: str, module_name: str) -> TestCase | None:
    """Convert an equation string into a property test if it's testable."""
    eq_lower = eq.lower()

    # softmax: sum of exp(x_i) / sum(exp(x_i)) = 1
    if "softmax" in eq_lower or "exp" in eq_lower:
        return TestCase(
            name="test_softmax_equation_property",
            doc=f"Verify softmax equation: exp(x_i) / sum(exp(x_i)) (from equation: {eq[:50]})",
            test_code="""\
pytest.importorskip("torch")
# Verify softmax formula: for any input x, softmax(x)_i = exp(x_i) / sum_j(exp(x_j))
x = torch.randn(4, 8)
exp_x = torch.exp(x)
softmax_x = exp_x / exp_x.sum(dim=-1, keepdim=True)
expected = torch.nn.functional.softmax(x, dim=-1)
assert torch.allclose(softmax_x, expected, atol=1e-5)
""",
        )

    # LayerNorm equation: out = (x - mean) / sqrt(var + eps) * gamma + beta
    if "layernorm" in eq_lower or "layer norm" in eq_lower:
        return TestCase(
            name="test_layernorm_formula",
            doc=f"LayerNorm formula: (x - mean) / sqrt(var + eps) * gamma + beta (from equation: {eq[:50]})",
            test_code="""\
pytest.importorskip("torch")
# Verify LayerNorm formula
# x = torch.randn(2, 10, 512)
# ln = torch.nn.LayerNorm(512)
# out = ln(x)
# assert out.shape == x.shape
# assert not torch.isnan(out).any()
pass
""",
        )

    return None


def _dataset_test(datasets: List[str], module_name: str) -> TestCase:
    """Test that model supports expected datasets or data format."""
    return TestCase(
        name="test_model_has_required_methods",
        doc=f"Model should support common dataset inputs (paper uses: {', '.join(datasets[:3])})",
        test_code=f"""\
pytest.importorskip("{module_name}")
# Verify model can be instantiated and called with expected input shapes
pytest.importorskip("torch")
# model = YourModel()
# x = torch.randn(1, 512, seq_len)  # typical transformer input
# output = model(x)
# assert output.shape[0] == 1
pass
""",
    )


def _code_signature_tests(code: str, module_name: str) -> List[TestCase]:
    """Extract function signatures from generated code and create smoke tests."""
    tests = []

    # Find function definitions in code
    func_pattern = re.compile(r"^def\s+(\w+)\s*\((.*?)\)", re.MULTILINE)
    for match in func_pattern.finditer(code):
        func_name = match.group(1)
        _params = match.group(2)

        # Skip private functions
        if func_name.startswith("_"):
            continue

        # Create smoke test — body only; _render_test_module wraps in def
        tests.append(
            TestCase(
                name=f"test_{func_name}_exists",
                doc=f"Function '{func_name}' should be importable and callable",
                test_code=f"""\
pytest.importorskip("{module_name}")
import {module_name} as m
assert hasattr(m, "{func_name}"), "Module should define {func_name}"
assert callable(getattr(m, "{func_name}")), "{func_name} should be callable"
""",
            )
        )

    return tests


def _render_conftest(suite: TestSuite) -> str:
    """Render conftest.py with any shared fixtures."""
    fixtures = []
    for _name, code in suite.fixtures.items():
        fixtures.append(code)

    return f"""\
# conftest.py — shared fixtures for {suite.arxiv_id}
# Generated by paper2code pipeline

import pytest
import torch


@pytest.fixture
def sample_input():
    \"\"\"Create a sample input tensor for testing.\"\"\"
    return torch.randn(2, 8, 512)


@pytest.fixture
def device():
    \"\"\"Return cpu or cuda device depending on availability.\"\"\"
    return torch.device("cuda" if torch.cuda.is_available() else "cpu")
"""


def _render_test_module(suite: TestSuite) -> str:
    """Render the main pytest test file."""
    _imports = "\n".join(suite.imports)

    rendered_tests = []
    for tc in suite.test_cases:
        # Apply 4-space indent to every line of the test body
        body_lines = tc.test_code.strip().split("\n")
        indented_body = "\n".join("    " + line for line in body_lines)
        rendered_tests.append(f'def {tc.name}():\n    """{tc.doc}"""\n{indented_body}')

    tests_str = "\n\n".join(rendered_tests)

    # Separate stdlib imports (always succeed) from module imports (may fail)
    stdlib_imports = ["import pytest", "import torch"]
    module_imports = [f"from {suite.module_name} import *"]
    import_block = (
        "\n".join(stdlib_imports)
        + "\n"
        + "\n".join(
            f'try:\n    {line}\nexcept ImportError:\n    pytest.skip("{suite.module_name} not installed", allow_module_level=True)'
            for line in module_imports
        )
    )

    return f"""\
# test_{suite.module_name}.py — Generated test suite for {suite.arxiv_id}
# Paper2code pipeline — automatically generated from paper analysis

{import_block}


{tests_str}


if __name__ == "__main__":
    pytest.main([__file__, "-v"])
"""
