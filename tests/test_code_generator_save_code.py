"""
Regression tests for research_loop.code_generator.save_code

Covers:
- Plain-text preamble stripping (MiniMax model scenario)
- Markdown ```python``` block extraction (existing behavior)
- Backtick `` `` marker stripping (existing behavior)
"""

import ast


class TestSaveCodePlainTextPreamble:
    """Regression: MiniMax model outputs plain-text description before Python code."""

    def test_strips_plain_text_preamble_before_import_statement(self, tmp_path):
        """MiniMax-style output: description text, then 'import torch'."""
        code = """The following is a refactored and well-documented version of the LoRA implementation.

import torch
import torch.nn as nn


class LoRALinear(nn.Module):
    def __init__(self, in_features, out_features, rank=4, alpha=1.0):
        super().__init__()
        self.rank = rank
        self.alpha = alpha
        self.scaling = alpha / rank

        self.lora_A = nn.Parameter(torch.randn(in_features, rank))
        self.lora_B = nn.Parameter(torch.randn(rank, out_features))
        self.reset_parameters()

    def reset_parameters(self):
        nn.init.zeros_(self.lora_A)
        nn.init.zeros_(self.lora_B)

    def forward(self, x):
        return (self.lora_A @ self.lora_B @ x.T).T * self.scaling
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="lora_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        first_line = content.split("\n")[0].lstrip()
        assert not first_line.startswith("The following"), f"Preamble not stripped: {first_line}"
        assert "import torch" in content

    def test_strips_plain_text_preamble_before_class_definition(self, tmp_path):
        """Variant: plain text description before a class or def at module level."""
        code = """This paper proposes a novel attention mechanism.

class MultiHeadAttention(nn.Module):
    def __init__(self, d_model, num_heads):
        super().__init__()
        assert d_model % num_heads == 0
        self.d_model = d_model
        self.num_heads = num_heads
        self.depth = d_model // num_heads

    def split_heads(self, x):
        batch_size = x.size(0)
        x = x.view(batch_size, -1, self.num_heads, self.depth)
        return x.permute(0, 2, 1, 3)
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="attention_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        first_line = content.split("\n")[0].lstrip()
        assert not first_line.startswith("This paper"), f"Preamble not stripped: {first_line}"
        assert "class MultiHeadAttention" in content

    def test_strips_plain_text_preamble_before_async_or_decorator(self, tmp_path):
        """Variant: plain text before an async def or decorated function."""
        code = """We refactor the original implementation with better patterns.

async def compute_hidden_states(model, input_ids, attention_mask=None):
    \"\"\"Async wrapper for model inference.\"\"\"
    return await model(input_ids, attention_mask=attention_mask)
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="async_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        first_line = content.split("\n")[0].lstrip()
        assert not first_line.startswith("We refactor"), f"Preamble not stripped: {first_line}"

    def test_strips_plain_text_preamble_with_docstring(self, tmp_path):
        """Preamble followed by a module-level docstring before code."""
        code = """Here is the refactored implementation.

\"\"\"Module-level docstring describing the algorithm.\"\"\"

def forward(x, weight, bias=None):
    return F.linear(x, weight, bias)
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="docstring_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        assert "def forward" in content


class TestSaveCodeExistingBehaviorPreserved:
    """Ensure existing marker-based extraction is NOT broken by the heuristic fix."""

    def test_extracts_python_code_block_with_language_marker(self, tmp_path):
        """Standard ```python``` block — existing behavior."""
        code = """Here is the implementation:

```python
import torch
import torch.nn.functional as F


class LoRAPytorch:
    def __init__(self, rank=4):
        self.rank = rank
```
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="block_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        assert "import torch" in content
        assert "class LoRAPytorch" in content

    def test_extracts_with_chinese_marker(self, tmp_path):
        """Existing Chinese想去 marker."""
        code = """ 代码如下：
 ```python
import torch
```
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="chinese_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        assert "import torch" in content


class TestSaveCodeEdgeCases:
    """Edge cases for the preamble stripping heuristic."""

    def test_real_minimax_output_parses(self, tmp_path):
        """Full realistic MiniMax output that previously caused unterminated string literal."""
        code = """The following is a refactored and well-documented version of the code.

\"\"\"LoRA: Low-Rank Adaptation of Large Language Models
Paper: arXiv:2106.09685
\"\"\"

import torch
import torch.nn as nn
import math


class LoRALinear(nn.Module):
    \"\"\"LoRA linear layer implementation.\"\"\"

    def __init__(self, in_features: int, out_features: int, rank: int = 8, alpha: float = 1.0):
        super().__init__()
        self.rank = rank
        self.alpha = alpha
        if rank > 0:
            self.lora_A = nn.Parameter(torch.randn(in_features, rank))
            self.lora_B = nn.Parameter(torch.randn(rank, out_features))
            self.scaling = alpha / rank
        else:
            self.lora_A = None
            self.lora_B = None
            self.scaling = 1.0

    def reset_parameters(self):
        if self.lora_A is not None:
            nn.init.zeros_(self.lora_A)
            nn.init.zeros_(self.lora_B)

    def forward(self, x: torch.Tensor) -> torch.Tensor:
        if self.rank == 0:
            return x
        return x + (x @ self.lora_A @ self.lora_B) * self.scaling
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="lora_regression")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        first_line = content.split("\n")[0].lstrip()
        assert first_line.startswith('"""'), f"Expected docstring start, got: {first_line}"
        assert "class LoRALinear" in content

    def test_type_checking_block_body_preserved(self, tmp_path):
        """TYPE_CHECKING block: preamble stripped, 'from typing import Any' preserved."""
        code = """This describes the implementation.

if TYPE_CHECKING:
    from typing import Any
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="type_check_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        # 'from typing import Any' (lstripped) matches ^from, so it becomes first
        # code line. Everything before it (including 'if TYPE_CHECKING:') is
        # stripped. The result is valid Python — a top-level import statement.
        assert "from typing import Any" in content

    def test_comment_only_lines_preserved(self, tmp_path):
        """Lines starting with # that are NOT valid Python module-level starts should not cut."""
        code = """# This is a comment header
# Another comment line

import os
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="comment_test")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        assert "import os" in content

    def test_single_line_description_stripped(self, tmp_path):
        """One-line preamble + blank line + import is stripped."""
        code = """Here is the implementation.

import os
"""
        from research_loop.code_generator import save_code

        result = save_code(code, tmp_path, module_name="single_preamble")
        content = result.read_text(encoding="utf-8")

        ast.parse(content)
        assert "import os" in content
        assert "Here is" not in content
