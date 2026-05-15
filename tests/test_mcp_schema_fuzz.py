"""
MCP Schema Fuzzer — discovers crashes and schema violations in MCP tool handlers.

For each of the 64 MCP tools:
  1. Generate random VALID inputs matching the schema
  2. Generate random INVALID inputs (missing required, wrong types, extreme values)
  3. Call handle_call_tool and collect results
  4. Track: tool_name → [(args, result, is_error)]

Usage:
    pytest tests/test_mcp_schema_fuzz.py -v
    python tests/test_mcp_schema_fuzz.py --report   # print summary
"""

from __future__ import annotations

import os
import random
import sys
import traceback
from collections import defaultdict
from typing import Any, Dict, List

import pytest

# ─── Tools list (loaded once at import time) ───────────────────────────────────
sys.path.insert(0, ".")
from mcp import get_tools as _get_tools

ALL_TOOLS = _get_tools()


# ─── Fuzzer Core ────────────────────────────────────────────────────────────────


def _generate_value(schema: Dict[str, Any], depth: int = 0) -> Any:
    """Generate a random value matching a JSON schema (simple types only)."""
    if depth > 5:
        return None

    schema_type = schema.get("type")
    if schema_type == "string":
        if "enum" in schema:
            return random.choice(schema["enum"])
        return "fuzz_string_" + str(random.randint(1000, 9999))
    if schema_type == "integer":
        mn = schema.get("minimum", -1000)
        mx = schema.get("maximum", 1000)
        return random.randint(mn, mx)
    if schema_type == "number":
        mn = schema.get("minimum", -1000.0)
        mx = schema.get("maximum", 1000.0)
        return random.uniform(mn, mx)
    if schema_type == "boolean":
        return random.choice([True, False])
    if schema_type == "array":
        items = schema.get("items", {})
        count = random.randint(0, 3)
        return [_generate_value(items, depth + 1) for _ in range(count)]
    if schema_type == "object":
        props = schema.get("properties", {})
        return {k: _generate_value(v, depth + 1) for k, v in props.items()}
    return None


def _generate_valid_args(tool: Dict[str, Any]) -> Dict[str, Any]:
    """Generate a valid set of arguments for a tool (all optional fields included)."""
    schema = tool.get("inputSchema", {})
    props = schema.get("properties", {})
    args = {}
    for name, prop_schema in props.items():
        if random.random() < 0.7:  # 70% chance to include each optional field
            args[name] = _generate_value(prop_schema)
    return args


def _generate_invalid_args(tool: Dict[str, Any]) -> List[Dict[str, Any]]:
    """Generate multiple INVALID argument sets (boundary violations)."""
    schema = tool.get("inputSchema", {})
    props = schema.get("properties", {})
    required = schema.get("required", [])
    results: List[Dict[str, Any]] = []

    # 1. Missing ALL required fields
    if required:
        results.append({"__fuzz_mode__": "missing_all_required"})

    # 2. Missing ONE required field at a time
    for field in required:
        results.append({"__fuzz_mode__": f"missing_{field}"})

    # 3. Wrong type for each field
    for name, prop_schema in props.items():
        ptype = prop_schema.get("type", "string")
        if ptype == "string":
            results.append({"__fuzz_mode__": f"wrong_type_{name}", name: 99999})
        elif ptype == "integer":
            results.append({"__fuzz_mode__": f"wrong_type_{name}", name: "not_an_int"})
        elif ptype == "number":
            results.append({"__fuzz_mode__": f"wrong_type_{name}", name: "not_a_number"})
        elif ptype == "boolean":
            results.append({"__fuzz_mode__": f"wrong_type_{name}", name: "not_a_bool"})
        elif ptype == "array":
            results.append({"__fuzz_mode__": f"wrong_type_{name}", name: "not_an_array"})

    # 4. Extreme / boundary values for numbers
    for name, prop_schema in props.items():
        ptype = prop_schema.get("type")
        if ptype == "integer":
            results.append({"__fuzz_mode__": f"extreme_{name}", name: 999999999})
            results.append({"__fuzz_mode__": f"negative_{name}", name: -999999999})
        if ptype == "number":
            results.append({"__fuzz_mode__": f"extreme_{name}", name: 1e308})

    # 5. Empty string for strings
    for name, prop_schema in props.items():
        if prop_schema.get("type") == "string":
            results.append({"__fuzz_mode__": f"empty_string_{name}", name: ""})

    return results[:8]  # cap at 8 invalid cases per tool


def _call_tool(name: str, arguments: Dict) -> Dict[str, Any]:
    """Call handle_call_tool and return result dict with error info."""
    try:
        from rairos_mcp import handle_call_tool

        # Use ThreadPoolExecutor with timeout to prevent hanging on external APIs
        from concurrent.futures import ThreadPoolExecutor, TimeoutError
        with ThreadPoolExecutor(max_workers=1) as pool:
            future = pool.submit(handle_call_tool, name, arguments)
            try:
                result = future.result(timeout=15)
            except TimeoutError:
                future.cancel()
                return {"ok": False, "is_error": False, "result": None,
                        "exception": TimeoutError(f"Tool '{name}' timed out"),
                        "traceback": ""}

        is_error = result.get("is_error") is True or (
            result.get("error") and "error" in str(result.get("error", "")).lower()
        )
        return {"ok": True, "is_error": is_error, "result": result, "exception": None}
    except Exception as e:
        return {
            "ok": False,
            "is_error": False,
            "result": None,
            "exception": e,
            "traceback": traceback.format_exc(),
        }


# ─── Test Fixtures ────────────────────────────────────────────────────────────


@pytest.fixture(scope="module")
def fuzz_results():
    """Run fuzzer once, share across all test cases."""
    random.seed(42)
    results = defaultdict(list)  # tool_name → [(args, call_result)]

    for tool in ALL_TOOLS:
        name = tool["name"]

        # Generate valid inputs (reduced to 3 to avoid timeouts)
        for _ in range(3):
            args = _generate_valid_args(tool)
            if args:
                call_res = _call_tool(name, args)
                results[name].append((args, call_res))

        # Generate invalid inputs (reduced to 3)
        count = 0
        for args in _generate_invalid_args(tool):
            if count >= 3:
                break
            call_res = _call_tool(name, args)
            results[name].append((args, call_res))
            count += 1

    return results


# ─── Tests ─────────────────────────────────────────────────────────────────────


class TestMCPFuzz:
    """Fuzz tests for MCP tool handlers."""

    @pytest.mark.parametrize("tool", ALL_TOOLS, ids=lambda t: t["name"])
    def test_tool_does_not_crash(self, tool, fuzz_results):
        """Every tool must handle all fuzz inputs without raising an exception."""
        name = tool["name"]
        for args, call_res in fuzz_results[name]:
            assert call_res["ok"], (
                f"Tool '{name}' crashed with args {args!r}: "
                f"{call_res['exception']}\n"
                f"{call_res.get('traceback', '')}"
            )

    @pytest.mark.parametrize("tool", ALL_TOOLS, ids=lambda t: t["name"])
    def test_valid_inputs_accepted(self, tool, fuzz_results):
        """Valid inputs should produce a result (not necessarily correct)."""
        name = tool["name"]
        schema = tool.get("inputSchema", {})
        required = schema.get("required", [])

        for args, call_res in fuzz_results[name]:
            # Only check "valid-ish" inputs (include required fields, not fuzz-mode)
            if all(k in args for k in required) and "__fuzz_mode__" not in args:
                assert call_res["ok"], (
                    f"Tool '{name}' rejected valid args {args!r}: {call_res['exception']}"
                )

    def test_crash_summary(self, fuzz_results):
        """Print summary of any crashes or errors found."""
        crashes = []
        errors = []

        for tool_name, entries in fuzz_results.items():
            for args, call_res in entries:
                if not call_res["ok"]:
                    crashes.append((tool_name, args, call_res["exception"]))
                elif call_res.get("is_error"):
                    errors.append((tool_name, args))

        total_calls = sum(len(v) for v in fuzz_results.values())
        print(f"\n{'=' * 60}")
        print(f"Fuzz Results: {len(ALL_TOOLS)} tools, {total_calls} total calls")
        print(f"Crashes (exceptions): {len(crashes)}")
        print(f"Handled errors: {len(errors)}")

        if crashes:
            print("\nCrashes:")
            for tool_name, args, exc in crashes[:10]:
                print(f"  {tool_name}: {args!r} → {exc}")
        if errors:
            print("\nHandled errors (non-crashing):")
            for tool_name, args in errors[:10]:
                print(f"  {tool_name}: {args!r}")

        # This test always passes — it's a report
        assert True, "Summary printed above"

    @pytest.mark.skipif(not os.environ.get("MCP_FUZZ_NETWORK"), reason="Requires network access for MCP tools")
    @pytest.mark.parametrize("tool", ALL_TOOLS, ids=lambda t: t["name"])
    def test_no_system_crash_on_missing_required(self, tool, fuzz_results):
        """Missing required fields should not crash the handler."""
        name = tool["name"]
        schema = tool.get("inputSchema", {})
        required = schema.get("required", [])

        if not required:
            pytest.skip("No required fields")

        for args, call_res in fuzz_results[name]:
            mode = args.get("__fuzz_mode__", "")
            if "missing" in mode:
                assert call_res["ok"], (
                    f"Tool '{name}' crashed when required fields missing: {call_res['exception']}"
                )


if __name__ == "__main__":
    # Run directly: python tests/test_mcp_schema_fuzz.py
    import argparse

    parser = argparse.ArgumentParser(description="MCP Schema Fuzzer")
    parser.add_argument("--report", action="store_true", help="Print full report")
    args = parser.parse_args()

    # Run fuzzer
    random.seed(42)
    results = defaultdict(list)
    for tool in ALL_TOOLS:
        name = tool["name"]
        for _ in range(10):
            a = _generate_valid_args(tool)
            if a:
                results[name].append((a, _call_tool(name, a)))
        for a in _generate_invalid_args(tool):
            results[name].append((a, _call_tool(name, a)))

    # Print report
    crashes = [
        (n, a, c["exception"]) for n, entries in results.items() for a, c in entries if not c["ok"]
    ]
    total = sum(len(v) for v in results.values())
    print(f"\n{'=' * 60}")
    print(f"MCP Schema Fuzzer: {len(ALL_TOOLS)} tools, {total} total calls")
    print(f"Crashes: {len(crashes)}")
    for name, args, exc in crashes[:20]:
        print(f"  {name}: {args!r} → {exc}")
