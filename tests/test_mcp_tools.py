"""Tests for mcp/tools_defs.py and cli/__main__.py."""

from mcp import get_tools


class TestMcpTools:
    def test_get_tools(self):
        tools = get_tools()
        assert isinstance(tools, list)
        assert len(tools) > 0

    def test_tools_have_required_fields(self):
        """Each tool must have name, description, inputSchema."""
        tools = get_tools()
        for tool in tools:
            assert "name" in tool
            assert "description" in tool
            assert "inputSchema" in tool
            assert isinstance(tool["name"], str)
            assert isinstance(tool["description"], str)
            assert isinstance(tool["inputSchema"], dict)

    def test_tools_have_valid_input_schema(self):
        """Each tool's inputSchema must be a valid JSON Schema object."""
        tools = get_tools()
        for tool in tools:
            schema = tool["inputSchema"]
            assert schema.get("type") == "object", f"Tool {tool['name']}: type must be 'object'"
            assert "properties" in schema
            assert isinstance(schema["properties"], dict)

    def test_required_fields_present(self):
        """Tools marked required must actually exist in properties."""
        tools = get_tools()
        for tool in tools:
            schema = tool["inputSchema"]
            required = schema.get("required", [])
            for field in required:
                assert field in schema["properties"], (
                    f"Tool {tool['name']}: required field '{field}' missing from properties"
                )

    def test_enum_fields_have_valid_values(self):
        """Enum constraints must have non-empty enum values."""
        tools = get_tools()
        for tool in tools:
            schema = tool["inputSchema"]
            for field_name, field_schema in schema.get("properties", {}).items():
                if "enum" in field_schema:
                    assert len(field_schema["enum"]) > 0, (
                        f"Tool {tool['name']}, field {field_name}: enum is empty"
                    )

    def test_array_items_have_type(self):
        """Array fields must declare item type."""
        tools = get_tools()
        for tool in tools:
            schema = tool["inputSchema"]
            for field_name, field_schema in schema.get("properties", {}).items():
                if field_schema.get("type") == "array":
                    assert "items" in field_schema, (
                        f"Tool {tool['name']}, field {field_name}: array type requires 'items'"
                    )

    def test_no_duplicate_tool_names(self):
        """Tool names must be unique."""
        tools = get_tools()
        names = [t["name"] for t in tools]
        assert len(names) == len(set(names)), "Duplicate tool names found"

    def test_tools_minimal_count(self):
        """Smoke test: we expect a reasonable number of tools."""
        tools = get_tools()
        assert len(tools) >= 30, f"Expected >=30 tools, got {len(tools)}"
