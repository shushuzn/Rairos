"""
Search Optimizer.

Improves search relevance and performance.
"""
from typing import List, Dict, Any


class SearchOptimizer:
    """
    Optimize search queries and results.
    
    Features:
    - Query expansion
    - Relevance scoring
    - Result ranking
    - Search suggestions
    """

    def __init__(self):
        self.search_history: List[str] = []

    def optimize_query(self, query: str) -> str:
        """Optimize a search query."""
        # Add to history
        self.search_history.append(query)

        # Basic optimization
        query = query.strip().lower()

        return query

    def expand_query(self, query: str) -> List[str]:
        """Expand query with synonyms."""
        expansions = [query]

        # Common expansions
        synonyms = {
            "ml": ["machine learning", "ml"],
            "ai": ["artificial intelligence", "ai"],
            "nn": ["neural network", "deep learning"],
            "nlp": ["natural language processing"],
            "cv": ["computer vision"],
        }

        words = query.lower().split()
        for word in words:
            if word in synonyms:
                expansions.extend(synonyms[word])

        return list(set(expansions))

    def rank_results(
        self,
        results: List[Dict[str, Any]],
        query: str
    ) -> List[Dict[str, Any]]:
        """Rank search results by relevance."""
        scored = []

        query_words = set(query.lower().split())

        for result in results:
            score = 0

            # Score title matches
            title = result.get("title", "").lower()
            if any(word in title for word in query_words):
                score += 10

            # Score abstract matches
            abstract = result.get("abstract", "").lower()
            if any(word in abstract for word in query_words):
                score += 5

            scored.append((score, result))

        scored.sort(key=lambda x: x[0], reverse=True)
        return [r[1] for r in scored]

    def get_suggestions(self, partial: str) -> List[str]:
        """Get search suggestions based on partial query."""
        suggestions = []
        partial = partial.lower()

        for query in self.search_history:
            if partial in query.lower() and query not in suggestions:
                suggestions.append(query)

        return suggestions[:5]


# Global search optimizer
_search_optimizer = None


def get_search_optimizer() -> SearchOptimizer:
    """Get the global search optimizer."""
    global _search_optimizer
    if _search_optimizer is None:
        _search_optimizer = SearchOptimizer()
    return _search_optimizer
