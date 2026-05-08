"""


Code Generator — LLM generate code skeleton from parsed paper content.





Generates:


- Implementation skeleton matching the paper's algorithm


- docstrings with paper citation


- Basic function signatures matching the described API


"""

from __future__ import annotations

import re


from pathlib import Path


from typing import Optional


from llm.client import call_llm_chat_completions


# System prompt for code generation from paper


CODE_GEN_SYSTEM = """You are an expert ML/AI researcher and Python programmer.

Given a research paper's structured content, generate a clean, runnable Python implementation.

CRITICAL RULES:
1. EVERY class body must contain at least one statement (pass or a real implementation).
2. EVERY function body must contain at least one statement (pass or a real implementation).
3. NEVER leave a class or function with only a comment and no body.
4. Use pass only when truly no implementation is possible; never use it as a placeholder.
5. Output a SINGLE valid Python file content (no markdown code blocks).
6. Every function must have a docstring citing the paper.
7. Include a main() function with a usage example.
8. Use type hints on all function signatures.
9. Include assertions for key preconditions from the paper.
10. Generate realistic placeholder implementations for LLM-related parts.
"""


def generate_code(
    paper_content,
    framework: str = "pytorch",
    model_name: Optional[str] = None,
) -> str:
    """Generate code skeleton from parsed paper content.





    Args:


        paper_content: PaperContent from paper_parser


        framework: "pytorch" | "jax" | "numpy"


        model_name: Override LLM model name





    Returns:


        Python code as string


    """

    model = model_name or _get_default_model()

    prompt = _build_prompt(paper_content, framework)

    result = call_llm_chat_completions(
        messages=[{"role": "user", "content": prompt}],
        model=model,
        system_prompt=CODE_GEN_SYSTEM,
        timeout=300,
        use_cache=True,
    )

    # Clean markdown code blocks if LLM wrapped in them

    result = result.strip()

    # Strip thinking/reasoning blocks that some models emit before code
    result = re.sub(r"^去想[\s\S]*?```python\n", "", result).strip()
    result = re.sub(r"^<think>[\s\S]*?```python\n", "", result).strip()

    if result.startswith("```python"):
        result = result[7:]

    if result.startswith("```"):
        result = result[3:]

    if result.endswith("```"):
        result = result[:-3]

    result = result.strip()

    return result


def _get_default_model() -> str:
    """Get default model from config or environment.



    When MiniMax API is detected (via MINIMAX_CN_BASE_URL or hermes config),

    use MiniMax model; otherwise fall back to gpt-4o-mini.

    """

    import os

    from pathlib import Path as P

    # Check if hermes config has MiniMax

    hermes_env = P.home() / ".hermes" / ".env"

    minimax_detected = False

    if hermes_env.exists():
        for line in hermes_env.read_text(encoding="utf-8").splitlines():
            line = line.strip()

            if "=" in line and not line.startswith("#"):
                k, _ = line.split("=", 1)

                if k in ("MINIMAX_CN_API_KEY", "MINIMAX_CN_BASE_URL"):
                    minimax_detected = True

                    break

    if minimax_detected:
        # Set env vars so call_llm_chat_completions uses the right API
        if not os.getenv("MINIMAX_BASE_URL"):
            os.environ["MINIMAX_BASE_URL"] = os.getenv(
                "MINIMAX_CN_BASE_URL", "https://api.minimaxi.com/v1"
            )
        if not os.getenv("MINIMAX_API_KEY"):
            # Read from hermes .env
            hermes_env_path = P.home() / ".hermes" / ".env"
            if hermes_env_path.exists():
                for line in hermes_env_path.read_text(encoding="utf-8").splitlines():
                    line = line.strip()
                    if line.startswith("MINIMAX_CN_API_KEY="):
                        _, val = line.split("=", 1)
                        os.environ["MINIMAX_API_KEY"] = val.strip()
                        break
        return os.getenv("AIROS_DEFAULT_MODEL_MINIMAX", "minimax-m2.7-highspeed")

    try:
        from config import DEFAULT_LLM_MODEL_RESEARCH

        return DEFAULT_LLM_MODEL_RESEARCH

    except Exception:
        pass

    return os.getenv("DEFAULT_LLM_MODEL", "gpt-4o-mini")


def _build_prompt(paper_content, framework: str) -> str:
    """Build the user prompt from paper content."""

    parts = []

    parts.append(f"# Paper: {paper_content.title}")

    parts.append(f"arXiv ID: {paper_content.arxiv_id}")

    authors = paper_content.authors[:3]

    parts.append(
        f"Authors: {', '.join(authors)}{' et al.' if len(paper_content.authors) > 3 else ''}"
    )

    parts.append(f"\n## Abstract\n{paper_content.abstract[:500]}")

    # Use provenance-tagged sources if available (from _enrich_from_pdf), fall back to flat lists
    algorithm_sources = getattr(paper_content, "algorithm_sources", []) or []
    equation_sources = getattr(paper_content, "equation_sources", []) or []
    claim_sources = getattr(paper_content, "claim_sources", []) or []

    if algorithm_sources:
        parts.append("\n## Algorithms\n")
        for src in algorithm_sources[:3]:
            loc_str = (
                f"§{src.location.section} p{src.location.page}"
                if src.location.section != "unknown"
                else f"p{src.location.page}"
            )
            parts.append(f"[{src.tag()}] {loc_str}\n{src.description[:500]}")
    elif paper_content.algorithm_descriptions:
        parts.append("\n## Algorithms\n")
        for i, algo in enumerate(paper_content.algorithm_descriptions[:3], 1):
            parts.append(f"### Algorithm {i}\n{algo[:500]}")

    if equation_sources:
        parts.append("\n## Key Equations\n")
        for src in equation_sources[:5]:
            loc_str = (
                f"§{src.location.section} p{src.location.page}"
                if src.location.section != "unknown"
                else f"p{src.location.page}"
            )
            parts.append(f"[{src.tag()}] {loc_str}  $$ {src.equation} $$")
    elif paper_content.equations:
        parts.append("\n## Key Equations\n")
        for eq in paper_content.equations[:5]:
            parts.append(f"$${eq}$$")

    if claim_sources:
        parts.append("\n## Key Claims\n")
        for src in claim_sources[:5]:
            loc_str = (
                f"§{src.location.section} p{src.location.page}"
                if src.location.section != "unknown"
                else f"p{src.location.page}"
            )
            parts.append(f"[{src.tag()}] {loc_str}\n- {src.claim[:200]}")
    elif paper_content.claims:
        parts.append("\n## Key Claims\n")
        for claim in paper_content.claims[:5]:
            parts.append(f"- {claim[:200]}")

    if paper_content.hyperparameters:
        parts.append("\n## Hyperparameters\n")

        for k, v in paper_content.hyperparameters.items():
            parts.append(f"- {k}: {v}")

    if paper_content.datasets:
        parts.append(f"\n## Datasets\n{', '.join(paper_content.datasets)}")

    if paper_content.methods:
        parts.append("\n## Methods\n")

        for m in paper_content.methods[:5]:
            parts.append(f"- {m[:150]}")

    parts.append(f"\n## Framework: {framework.upper()}")

    # Provenance tracing instruction — injected into prompt so LLM embeds source tags in code comments
    parts.append(
        "\n## Source Tracing Instructions\n"
        "For each section of code that implements a specific equation, algorithm description, "
        "or claim, add an inline comment on the first line with its source tag.\n"
        "Format: # source: @<type>[<index>] — <human-readable description>\n"
        "Examples:\n"
        "  # source: @eq[0] — Attention equation from §3.2 p4\n"
        "  # source: @algo[1] — Transformer encoder from §2.1\n"
        "  # source: @eq[0], @eq[2] — Combined attention and feed-forward\n"
        "If a code section implements multiple sources, list all tags separated by commas.\n"
        "Tag each distinct functional block (class, main function, key sub-routine) that maps to a paper source."
    )

    parts.append(
        f"\nGenerate a clean Python implementation skeleton in {framework.upper()}. "
        f"Match the algorithm exactly as described. "
        f"Output ONLY the Python code (no markdown)."
    )

    return "\n\n".join(parts)


def _strip_prose_secondary(code: str) -> str:
    """Strip prose lines using alpha-ratio + code-marker heuristic.

    Applied after primary marker-based stripping for models (e.g. MiniMax)
    that output plain-text descriptions without markdown fences.
    """
    lines = code.splitlines(keepends=True)
    result = []
    for s in lines:
        stripped = s.lstrip()
        total = len(stripped.rstrip("\r\n"))
        if total == 0:
            result.append(s)
            continue
        alpha = sum(c.isalpha() for c in stripped)
        ratio = alpha / total if total > 0 else 0
        markers = set('(){}=[]<@#"')
        has_marker = any(c in markers for c in stripped)
        is_import = stripped.startswith("import ") or stripped.startswith("from ")
        is_py_kw = re.match(
            r"^(class |def |async |@|if |elif |for |while |with |try:|except:|finally:|raise |return |yield |pass |break |continue |assert |import |from |#|$)",
            stripped,
        )
        if total > 10 and ratio > 0.75 and not has_marker and not is_import and not is_py_kw:
            continue  # drop prose line
        result.append(s)
    return "".join(result)


def save_code(code: str, output_dir: Path, module_name: str = "model") -> Path:
    """Save generated code to a file, stripping markdown code-block wrappers."""
    import re

    # Strip triple-backtick markdown wrappers: ```python ... ``` or ``` ... ```
    code = re.sub(r"^\s*```(?:python)?\s*\n", "", code, flags=re.MULTILINE)
    code = re.sub(r"\n\s*```\s*\n", "\n", code)

    # Strip thinking/reasoning blocks: anchor to code-block opener to avoid
    # intermediate delimiters (e.g. 刀 inside the thinking text) stopping the
    # non-greedy match early
    code = re.sub(r"去想[\s\S]*?```python\n", "", code)
    code = re.sub(r"<think>[\s\S]*?```python\n", "", code)

    # Heuristic strip for models (e.g. MiniMax) that output plain-text
    # description paragraphs BEFORE the actual Python code without any
    # markdown fences.  Detect the first line that starts with a Python
    # construct (import, class, def, async, @, #, ''', """, if __name__, pass)
    # and discard everything before it.
    lines = code.splitlines(keepends=True)
    first_code_line = None
    for i, line in enumerate(lines):
        stripped = line.lstrip()
        if re.match(
            r"^(import |from |class |def |async |#|@|"
            r'"""|\'\'\'|if __name__ == |elif |pass |for |while |with |try:|except:|finally:|raise |return |'
            r"@dataclass)",
            stripped,
        ):
            first_code_line = i
            break

    if first_code_line is not None and first_code_line > 0:
        # Only strip if we removed meaningful prose (more than 1 line of text).
        # Using > 1 (not > 3) so that even "Description text\n\nimport torch" gets cleaned.
        if first_code_line > 0:
            code = "".join(lines[first_code_line:])
        else:
            code = "".join(lines)

    # Strip free text appended after valid Python entry point — LLM sometimes
    # writes a description after `if __name__ == "__main__": main()`
    code = re.sub(
        r'\nif __name__ == "__main__":\s*main\(\)\s*[\w\W]*$', "", code, flags=re.MULTILINE
    )
    code = _strip_prose_secondary(code)

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / f"{module_name}.py"
    out_path.write_text(code.strip(), encoding="utf-8")
    return out_path
