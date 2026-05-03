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





Rules:


1. Generate complete function signatures with type hints


2. Include docstrings with the paper title and arxiv ID


3. Use the EXACT algorithm described - do not simplify or skip steps


4. Include assertions for key preconditions from the paper


5. Generate realistic placeholder implementations for LLM-related parts


6. Output a SINGLE valid Python file content (no markdown code blocks)


7. Every function must have a docstring citing the paper


8. Include a main() function with a usage example


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
    result = re.sub(r'^去想[\s\S]*?```python\n', '', result).strip()
    result = re.sub(r'^<think>[\s\S]*?```python\n', '', result).strip()



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
            os.environ["MINIMAX_BASE_URL"] = os.getenv("MINIMAX_CN_BASE_URL", "https://api.minimaxi.com/v1")
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


    parts.append(f"Authors: {', '.join(authors)}{' et al.' if len(paper_content.authors) > 3 else ''}")


    parts.append(f"\n## Abstract\n{paper_content.abstract[:500]}")





    if paper_content.algorithm_descriptions:


        parts.append("\n## Algorithms\n")


        for i, algo in enumerate(paper_content.algorithm_descriptions[:3], 1):


            parts.append(f"### Algorithm {i}\n{algo[:500]}")





    if paper_content.equations:


        parts.append("\n## Key Equations\n")


        for eq in paper_content.equations[:5]:


            parts.append(f"$${eq}$$")





    if paper_content.claims:


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


    parts.append(f"\nGenerate a clean Python implementation skeleton in {framework.upper()}. "


                 f"Match the algorithm exactly as described. "


                 f"Output ONLY the Python code (no markdown).")





    return "\n\n".join(parts)








def save_code(code: str, output_dir: Path, module_name: str = "model") -> Path:
    """Save generated code to a file, stripping markdown code-block wrappers."""
    import re

    # Strip triple-backtick markdown wrappers: ```python ... ``` or ``` ... ```
    code = re.sub(r'^```(?:python)?\s*\n', '', code, flags=re.MULTILINE)
    code = re.sub(r'\n```\s*$', '', code)

    output_dir.mkdir(parents=True, exist_ok=True)
    out_path = output_dir / f"{module_name}.py"
    out_path.write_text(code.strip(), encoding="utf-8")
    return out_path


