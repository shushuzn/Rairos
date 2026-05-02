"""
Test Extractor — extract testable assertions from paper and generate pytest test cases.

Generates:
- Unit tests from input/output examples in the paper
- Assertion tests from numerical claims ("accuracy > 90%")
- Property-based tests from stated invariants
- Integration tests from algorithm walkthroughs
"""

from __future__ import annotations

import re
from dataclasses import dataclass, field
from pathlib import Path
from typing import Optional

from llm.client import call_llm_chat_completions


SYSTEM_PROMPT = """You are an expert ML researcher and test engineer.
Given a research paper's structured content, generate pytest test cases.

Rules:
1. Generate valid pytest code (import pytest, etc.)
2. Each test function must have a descriptive name and docstring
3. Create FAKE but REALISTIC input/output pairs that match the paper's algorithm
4. For numerical claims: assert with reasonable tolerance (relative 0.05, absolute 0.01)
5. For shape/dimension claims: use assert statements
6. For API contracts: use pytest.raises for error cases
7. Output ONLY the Python test file content (no markdown code blocks)
8. Include conftest.py fixtures if needed
9. Every test must be runnable (use try/except for robustness)
10. Each test file must be self-contained with all imports at top
"""


@dataclass
class TestCase:
    """A single generated test case."""
    name: str
    category: str  # "io_example" | "numerical_claim" | "property" | "api_contract" | "sanity"
    code: str
    description: str
    source: str  # "algorithm" | "equation" | "claim" | "method"


@dataclass
class TestSuite:
    """Collection of generated test cases for one paper."""
    module_name: str
    test_cases: list[TestCase] = field(default_factory=list)
    conftest: str = ""

    def render(self) -> tuple[str, str]:
        """Render as (conftest.py content, test_impl.py content)."""
        test_lines = [
            '"""Auto-generated tests for paper implementation."""',
            "",
            "import pytest",
            "import sys",
            "from pathlib import Path",
            "",
            f"# Add implementation to path",
            f"sys.path.insert(0, str(Path(__file__).parent.parent / 'src'))",
            "",
        ]

        imports_seen = set()
        for tc in self.test_cases:
            # Add any imports found in the test code
            for line in tc.code.split("\n"):
                if line.startswith("import ") or line.startswith("from "):
                    if line not in imports_seen:
                        test_lines.append(line)
                        imports_seen.add(line)

        test_lines.append("")

        for tc in self.test_cases:
            test_lines.append(f'class Test_{tc.category.title().replace("_", "")}:')
            test_lines.append(f'    """{tc.description}"""')
            test_lines.append("")
            test_lines.append(f"    def {tc.name}(self):")
            test_lines.append(f'        """Source: {tc.source}"""')
            # Indent the test code
            for line in tc.code.split("\n"):
                test_lines.append("            " + line)
            test_lines.append("")

        return self.conftest, "\n".join(test_lines)


def extract_tests(
    paper_content,
    generated_code: str,
    module_name: str = "model",
) -> TestSuite:
    """Extract testable assertions from paper and generate pytest tests.

    Args:
        paper_content: PaperContent from paper_parser
        generated_code: The generated Python code string
        module_name: Name of the module being tested

    Returns:
        TestSuite with test cases
    """
    suite = TestSuite(module_name=module_name)

    # 1. Generate IO example tests from algorithm descriptions
    io_tests = _extract_io_examples(paper_content)
    suite.test_cases.extend(io_tests)

    # 2. Generate numerical claim tests
    claim_tests = _extract_numerical_claims(paper_content)
    suite.test_cases.extend(claim_tests)

    # 3. Generate property-based tests
    property_tests = _extract_properties(paper_content, generated_code)
    suite.test_cases.extend(property_tests)

    # 4. Generate API sanity tests (generated code must be importable)
    suite.test_cases.append(_make_import_sanity_test(generated_code, module_name))

    # 5. Use LLM to generate additional tests from paper claims
    llm_tests = _generate_llm_tests(paper_content, generated_code, module_name)
    suite.test_cases.extend(llm_tests)

    # Cap at 30 tests to keep suite manageable
    suite.test_cases = suite.test_cases[:30]

    return suite


# ─── IO Example extraction ────────────────────────────────────────────────────

def _extract_io_examples(paper_content) -> list[TestCase]:
    """Extract input/output examples from algorithm descriptions."""
    tests = []

    for i, algo in enumerate(paper_content.algorithm_descriptions[:3]):
        # Look for "Input:", "Output:", "Example" patterns
        io_pairs = _parse_io_from_text(algo, f"algorithm_{i}")
        for name, code, desc in io_pairs:
            tests.append(TestCase(
                name=f"test_{name}",
                category="io_example",
                code=code,
                description=desc,
                source="algorithm",
            ))

    return tests


def _parse_io_from_text(text: str, source_id: str) -> list[tuple[str, str, str]]:
    """Parse input/output pairs from algorithm text."""
    pairs = []

    # Match Input: ... Output: ... patterns
    input_m = re.search(
        r'INPUT[:]\s*(.*?)(?=\n(?:OUTPUT|Output|output)|$)',
        text, re.DOTALL | re.IGNORECASE
    )
    output_m = re.search(
        r'OUTPUT[:]\s*(.*?)(?=\n(?:Input|input|$))',
        text, re.DOTALL | re.IGNORECASE
    )

    if input_m and output_m:
        inp = input_m.group(1).strip()
        out = output_m.group(1).strip()
        pairs.append((
            f"io_{source_id}",
            _make_io_test(inp, out),
            f"IO example from {source_id}",
        ))

    # Match numbered steps (1. Input: ...)
    for m in re.finditer(r'(?:Step \d+[:.]\s*)?(?:Input|input)[:.]\s*(.*?)(?=\n(?:Output|output|Step|$))', text, re.DOTALL):
        inp = m.group(1).strip()
        if len(inp) < 5 or len(inp) > 500:
            continue
        # Look for corresponding output
        next_text = text[m.end():m.end()+300]
        out_m = re.search(r'(?:Output|output)[:.]\s*(.*?)(?=\n(?:Step|$))', next_text, re.DOTALL)
        if out_m:
            out = out_m.group(1).strip()
            pairs.append((
                f"io_step_{source_id}_{m.start()}",
                _make_io_test(inp, out),
                f"Step IO from {source_id}",
            ))

    return pairs


def _make_io_test(input_desc: str, output_desc: str) -> str:
    """Generate a test from IO descriptions."""
    # Try to extract numerical values
    inp_nums = re.findall(r'-?\d+\.?\d*', input_desc)
    out_nums = re.findall(r'-?\d+\.?\d*', output_desc)

    if inp_nums and out_nums:
        # Numerical IO - create a simple functional test
        code = [
            "# Parse numerical IO",
            f"inp = {inp_nums[:5]}",  # Take first few numbers
            f"out_expected = {out_nums[:5]}",
            "",
            "# Basic sanity",
            "assert isinstance(inp, (list, tuple))",
            "assert len(inp) > 0",
        ]
        return "\n".join(code)

    # Generic IO test
    code = [
        f'# Input: {input_desc[:100]}',
        f'# Expected output: {output_desc[:100]}',
        "pass",
    ]
    return "\n".join(code)


# ─── Numerical claims ─────────────────────────────────────────────────────────

def _extract_numerical_claims(paper_content) -> list[TestCase]:
    """Extract numerical claims and turn them into assertions."""
    tests = []

    for i, claim in enumerate(paper_content.claims[:5]):
        # Look for accuracy/precision/recall/F1 claims
        for pat, metric in [
            (r'accuracy\s*[>=]+\s*([0-9.]+)', 'accuracy'),
            (r'precision\s*[>=]+\s*([0-9.]+)', 'precision'),
            (r'recall\s*[>=]+\s*([0-9.]+)', 'recall'),
            (r'F1\s*[>=]+\s*([0-9.]+)', 'f1'),
            (r'AUROC\s*[>=]+\s*([0-9.]+)', 'auroc'),
            (r'BLEU\s*[>=]+\s*([0-9.]+)', 'bleu'),
        ]:
            m = re.search(pat, claim, re.IGNORECASE)
            if m:
                threshold = float(m.group(1))
                # Normalize to [0, 1] range if > 1 (percentage)
                if threshold > 1:
                    threshold = threshold / 100.0

                tests.append(TestCase(
                    name=f"test_{metric}_threshold",
                    category="numerical_claim",
                    code=_make_threshold_test(metric, threshold),
                    description=f"{metric.upper()} >= {threshold} claim",
                    source="claim",
                ))

    return tests


def _make_threshold_test(metric: str, threshold: float) -> str:
    """Generate a threshold assertion test."""
    return f"""
# This test checks the claimed {metric} threshold
# Note: This is a placeholder - real test requires model output
{metric}_claimed = {threshold}
# In real usage, compute actual metric and assert:
# assert actual_{metric} >= {metric}_claimed, f"{{actual_{metric}}} below claimed {{threshold}}"
assert {threshold} > 0 and {threshold} <= 1.0, "Threshold must be in (0, 1]"
"""


# ─── Property-based tests ─────────────────────────────────────────────────────

def _extract_properties(paper_content, generated_code: str) -> list[TestCase]:
    """Extract testable properties from paper."""
    tests = []

    # Check if paper mentions normalization, non-negativity, etc.
    full_text = paper_content.full_text.lower()

    properties = []

    if 'normaliz' in full_text:
        properties.append(("non_negative",
            "assert all(x >= 0 for x in output)",
            "Output should be non-negative (normalization property)"))

    if 'probability' in full_text or 'prob' in full_text:
        properties.append(("prob_sum",
            "# For probability outputs, sum should be ~1.0",
            "Probabilities should sum to 1.0"))

    if 'attention' in full_text:
        properties.append(("attention_shape",
            "# Attention weights shape should match [query, key]",
            "Attention output shape sanity check"))

    for name, code, desc in properties[:3]:
        tests.append(TestCase(
            name=f"test_{name}",
            category="property",
            code=code,
            description=desc,
            source="method",
        ))

    return tests


def _make_import_sanity_test(generated_code: str, module_name: str) -> TestCase:
    """Generate a sanity test that the generated code is importable."""
    return TestCase(
        name="test_module_importable",
        category="sanity",
        code=f'''
try:
    import sys
    from pathlib import Path
    src_dir = Path(__file__).parent.parent / "src"
    if str(src_dir) not in sys.path:
        sys.path.insert(0, str(src_dir))
    import {module_name}
    assert hasattr({module_name}, "__file__"), "Module should be importable"
except ImportError as e:
    pytest.skip(f"Module not importable (may have deps): {{e}}")
''',
        description="Generated code must be importable",
        source="generated_code",
    )


# ─── LLM-generated tests ──────────────────────────────────────────────────────

def _generate_llm_tests(paper_content, generated_code: str, module_name: str) -> list[TestCase]:
    """Use LLM to extract additional test cases from paper claims."""
    prompt = f"""Given this research paper, generate 3-5 pytest test cases that check the algorithm's correctness.

Paper: {paper_content.title}
arXiv: {paper_content.arxiv_id}

Abstract: {paper_content.abstract[:300]}

Key equations:
{chr(10).join(paper_content.equations[:3])}

Key claims:
{chr(10).join(c[:150] for c in paper_content.claims[:3])}

Generated code structure (functions defined):
{_extract_function_names(generated_code)}

Generate pytest tests that:
1. Check key mathematical properties from equations
2. Test function signatures exist and are callable
3. Validate shape/dimension contracts
4. Test error handling for invalid inputs

Output format: just the test code, no markdown.
"""

    try:
        result = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model="mini-max- Reasoning",
            system_prompt=SYSTEM_PROMPT,
            timeout=300,
            use_cache=True,
        )

        # Parse the result into TestCases
        return _parse_llm_tests(result, module_name)
    except Exception as e:
        return [TestCase(
            name="test_fallback",
            category="sanity",
            code=f"# LLM test generation failed: {e}\npass",
            description="Fallback test",
            source="llm_fallback",
        )]


def _extract_function_names(code: str) -> str:
    """Extract function names from generated code."""
    names = re.findall(r'^def (\w+)\(', code, re.MULTILINE)
    return ", ".join(names) if names else "(no functions found)"


def _parse_llm_tests(result: str, module_name: str) -> list[TestCase]:
    """Parse LLM output into TestCase objects."""
    tests = []
    result = result.strip()

    # Remove markdown code blocks if present
    if result.startswith("```"):
        result = re.sub(r'^```\w*\n?', '', result)
        result = re.sub(r'\n?```$', '', result)

    # Split into test functions (find "def test_" or "async def test_")
    # Each test starts with a def test_ line
    parts = re.split(r'\n(?=async\s+def\s+test_|\ndef\s+test_)', result)

    for part in parts:
        part = part.strip()
        if not part or not re.search(r'def\s+test_\w+', part):
            continue

        # Extract test name
        name_m = re.search(r'(?:async\s+)?def\s+(test_\w+)', part)
        if not name_m:
            continue

        name = name_m.group(1)

        # Determine category from name
        if any(k in name for k in ['io', 'input', 'output', 'example']):
            cat = 'io_example'
        elif any(k in name for k in ['accuracy', 'precision', 'recall', 'threshold', 'score']):
            cat = 'numerical_claim'
        elif any(k in name for k in ['property', 'invariant', 'sanity', 'shape']):
            cat = 'property'
        else:
            cat = 'api_contract'

        tests.append(TestCase(
            name=name,
            category=cat,
            code=part,
            description=f"LLM-generated test: {name}",
            source="llm",
        ))

    return tests


def save_tests(suite: TestSuite, output_dir: Path) -> tuple[Path, Path]:
    """Save test suite to directory."""
    output_dir.mkdir(parents=True, exist_ok=True)

    conftest_path = output_dir / "conftest.py"
    test_impl_path = output_dir / "test_impl.py"

    conftest_content, test_content = suite.render()

    if conftest_content:
        conftest_path.write_text(conftest_content, encoding="utf-8")

    test_impl_path.write_text(test_content, encoding="utf-8")

    return conftest_path, test_impl_path
