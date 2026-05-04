"""Cross-source paper search: arXiv + Semantic Scholar."""

import asyncio
import logging
from concurrent.futures import ThreadPoolExecutor
from typing import List, Tuple

from core import Paper

logger = logging.getLogger(__name__)

_executor = ThreadPoolExecutor(max_workers=4)


def search_papers_multi(
    query: str,
    max_per_source: int = 5,
    sources: Tuple[str, ...] = ("arxiv", "semantic_scholar"),
) -> List[Paper]:
    """
    Search multiple sources concurrently and merge results.

    Args:
        query: Search query
        max_per_source: Max papers per source (default 5)
        sources: Which sources to query (default: arxiv, semantic_scholar)

    Returns:
        Combined list of Paper objects (no dedup, sorted by source order)
    """
    loop = asyncio.new_event_loop()
    try:
        return loop.run_until_complete(_search_multi_async(query, max_per_source, sources))
    finally:
        loop.close()


async def _search_multi_async(
    query: str,
    max_per_source: int,
    sources: Tuple[str, ...],
) -> List[Paper]:
    """Async wrapper for concurrent multi-source search."""
    loop = asyncio.get_event_loop()
    tasks = []

    if "arxiv" in sources:
        tasks.append(loop.run_in_executor(_executor, _search_arxiv, query, max_per_source))
    if "semantic_scholar" in sources:
        tasks.append(loop.run_in_executor(_executor, _search_semantic, query, max_per_source))

    if not tasks:
        return []

    results = await asyncio.gather(*tasks, return_exceptions=True)
    papers: List[Paper] = []
    for r in results:
        if isinstance(r, Exception):
            logger.warning(f"Search source failed: {r}")
        elif isinstance(r, list):
            papers.extend(r)
    return papers


def _search_arxiv(query: str, max_results: int) -> List[Paper]:
    """Sync wrapper for arXiv search."""
    from parsers.arxiv_search import search_arxiv

    try:
        return search_arxiv(query, max_results=max_results)
    except Exception as e:
        logger.warning(f"arXiv search failed: {e}")
        return []


def _search_semantic(query: str, max_results: int) -> List[Paper]:
    """Sync wrapper for Semantic Scholar search."""
    from parsers.semantic_scholar import search_semantic_scholar

    try:
        s2_papers = search_semantic_scholar(query, max_results=max_results)
        return [p.to_paper() for p in s2_papers]
    except Exception as e:
        logger.warning(f"Semantic Scholar search failed: {e}")
        return []
