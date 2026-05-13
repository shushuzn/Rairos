"""Re-export from llm.tool.mcp_jin10 for backward compatibility."""
import warnings
warnings.warn(
    "Import from llm.mcp_jin10 is deprecated, use llm.tool.mcp_jin10 instead",
    DeprecationWarning,
    stacklevel=2,
)


from llm.tool.mcp_jin10 import Jin10Client, MCPError

__all__ = ["Jin10Client", "MCPError"]
