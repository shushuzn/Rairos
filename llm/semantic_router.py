"""Re-export from llm.routing.semantic_router for backward compatibility."""
import warnings
warnings.warn(
    f"Import from llm.semantic_router is deprecated, use llm.research.semantic_router instead",
    DeprecationWarning,
    stacklevel=2,
)


from llm.routing.semantic_router import (
    QueryType,
    Route,
    _route_by_keyword,
    _QUERY_TYPE_TO_COMMAND,
    SemanticRouter,
)

__all__ = [
    "QueryType",
    "Route",
    "_route_by_keyword",
    "_QUERY_TYPE_TO_COMMAND",
    "SemanticRouter",
]
