"""Re-export from llm.routing.semantic_router for backward compatibility."""

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
