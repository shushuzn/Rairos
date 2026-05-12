"""CLI command: kg."""

from cli.cmd.kg.kg import _build_kg_parser

# Re-export KGManager so cli.cmd.kg.kg can import it without circular issues
from kg.manager import KGManager

__all__ = ["_build_kg_parser", "KGManager"]
