"""Patch: add GapClusterer class to gap_analyzer.py"""
import re

with open('llm/research/gap_analyzer.py', 'r', encoding='utf-8') as f:
    content = f.read()

clusterer_code = '''


# ─── Gap Semantic Clustering ─────────────────────────────────────────────────────


class GapClusterer:
    """Clusters gaps by semantic similarity to discover hot research areas.

    Uses agglomerative hierarchical clustering with average linkage.
    Distance = 1 - semantic_similarity (title 60% + gap_type 40%).

    Clustering helps:
    - Deduplication: similar gaps get merged into one cluster
    - Hotspot discovery: clusters with many recent gaps = active research area
    - Trend detection: which gap types are growing/shrinking over time
    """

    SIMILARITY_THRESHOLD = 0.60   # gaps above this similarity → same cluster
    MIN_CLUSTER_SIZE = 1          # single-element clusters are valid (singleton)

    def __init__(self, confidence_scorer: "ConfidenceScorer"):
        self.scorer = confidence_scorer

    # ── Core clustering ─────────────────────────────────────────────────────────

    def _title_similarity(self, t1: str, t2: str) -> float:
        """Word-overlap Jaccard similarity between two titles (0-1)."""
        if not t1 or not t2:
            return 0.0
        words1 = set(t1.lower().split())
        words2 = set(t2.lower().split())
        if not words1 or not words2:
            return 0.0
        intersection = len(words1 & words2)
        union = len(words1 | words2)
        return intersection / union if union > 0 else 0.0

    def _gap_similarity(self, a: "ResearchGapV2", b: "ResearchGapV2") -> float:
        """Combined similarity: title (60%) + gap_type (40%)."""
        title_sim = self._title_similarity(a.title or "", b.title or "")
        type_sim = 1.0 if a.gap_type == b.gap_type else 0.0
        return title_sim * 0.6 + type_sim * 0.4

    def _distance(self, a: "ResearchGapV2", b: "ResearchGapV2") -> float:
        """Distance = 1 - similarity (for clustering)."""
        return 1.0 - self._gap_similarity(a, b)

    def cluster_gaps(self, gaps: List["ResearchGapV2"]) -> List[List["ResearchGapV2"]]:
        """Agglomerative hierarchical clustering of gaps by semantic similarity.

        Returns list of clusters, each cluster is a list of ResearchGapV2.
        Singleton clusters (no similar gaps) are included.

        Algorithm: agglomerative with average linkage + threshold cutoff.
        O(n²) pairwise distances, then O(n³) naive clustering.
        For n<500 this is fine; for large n, switch to DBSCAN.
        """
        if len(gaps) <= 1:
            return [list(gaps)]

        # Compute pairwise distance matrix
        n = len(gaps)
        dist: Dict[tuple, float] = {}
        for i in range(n):
            for j in range(i + 1, n):
                dist[(i, j)] = self._distance(gaps[i], gaps[j])

        # Union-Find with threshold cutoff
        parent = list(range(n))
        rank = [0] * n

        def find(x: int) -> int:
            while parent[x] != x:
                parent[x] = parent[parent[x]]  # path compression
                x = parent[x]
            return x

        def union(x: int, y: int) -> None:
            rx, ry = find(x), find(y)
            if rx == ry:
                return
            if rank[rx] < rank[ry]:
                parent[rx] = ry
            elif rank[rx] > rank[ry]:
                parent[ry] = rx
            else:
                parent[ry] = rx
                rank[rx] += 1

        # Merge pairs whose distance is below threshold
        threshold = 1.0 - self.SIMILARITY_THRESHOLD
        edges = [(d, i, j) for (i, j), d in dist.items() if d < threshold]
        edges.sort()  # greedy: merge closest pairs first

        for _, i, j in edges:
            union(i, j)

        # Collect clusters
        clusters_map: Dict[int, List[int]] = defaultdict(list)
        for i in range(n):
            clusters_map[find(i)].append(i)

        clusters: List[List["ResearchGapV2"]] = []
        for indices in clusters_map.values():
            clusters.append([gaps[i] for i in indices])

        # Sort clusters: largest first
        clusters.sort(key=len, reverse=True)
        return clusters

    # ── Cluster-level analytics ────────────────────────────────────────────────

    def cluster_stats(self, clusters: List[List["ResearchGapV2"]]) -> List[Dict]:
        """Compute per-cluster statistics for hotspot analysis.

        Returns list of dicts with keys:
          - cluster_id, size, gap_types (set), avg_novelty, avg_confidence,
          - titles (sample), is_hot, keywords (top words across titles)
        """
        stats = []
        for cid, cluster in enumerate(clusters):
            all_titles = [g.title for g in cluster if g.title]
            all_types = {g.gap_type for g in cluster}

            # Top keywords across all titles in cluster
            word_counts: Dict[str, int] = defaultdict(int)
            for title in all_titles:
                for word in title.lower().split():
                    if len(word) > 3:  # skip short words
                        word_counts[word] += 1
            top_keywords = sorted(word_counts.items(), key=lambda x: -x[1])[:5]
            top_keywords = [w for w, _ in top_keywords]

            stats.append({
                "cluster_id": cid,
                "size": len(cluster),
                "gap_types": all_types,
                "avg_novelty": sum(g.novelty_score for g in cluster) / len(cluster),
                "avg_confidence": sum(g.confidence for g in cluster) / len(cluster)
                    if all(hasattr(g, 'confidence') for g in cluster) else 0.0,
                "titles_sample": all_titles[:3],
                "top_keywords": top_keywords,
                "is_hot": len(cluster) >= 3,  # 3+ gaps = hot cluster
            })
        return stats

    def detect_trends(self, clusters: List[List["ResearchGapV2"]]) -> Dict[str, Any]:
        """Analyze gap type distribution trends across clusters.

        Returns:
          - rising_types: gap types with avg cluster size > overall avg
          - declining_types: gap types with avg cluster size < overall avg
          - emerging_types: gap types only appearing in recent clusters
        """
        all_types: Dict[str, List[int]] = defaultdict(list)  # type → list of cluster sizes

        for cluster in clusters:
            by_type: Dict[str, int] = defaultdict(int)
            for g in cluster:
                by_type[g.gap_type] += 1
            for t, cnt in by_type.items():
                all_types[t].append(cnt)

        if not all_types:
            return {"rising": [], "declining": [], "emerging": []}

        avg_size = sum(len(c) for c in clusters) / len(clusters)

        rising = []
        declining = []
        emerging = []

        for gtype, sizes in all_types.items():
            avg = sum(sizes) / len(sizes)
            if avg > avg_size * 1.3:
                rising.append(gtype)
            elif avg < avg_size * 0.7:
                declining.append(gtype)
            if len(sizes) == 1 and sizes[0] <= 2:
                emerging.append(gtype)

        return {
            "rising": rising,
            "declining": declining,
            "emerging": emerging,
            "avg_cluster_size": avg_size,
        }

    def deduplicate_clusters(self, clusters: List[List["ResearchGapV2"]]) -> List["ResearchGapV2"]:
        """Return one representative gap per cluster (the one with highest novelty).

        Use this to get a de-duplicated list of gaps for downstream processing.
        """
        representatives = []
        for cluster in clusters:
            # Pick gap with highest novelty_score as representative
            rep = max(cluster, key=lambda g: getattr(g, 'novelty_score', 0.0))
            representatives.append(rep)
        return representatives

'''

if 'class GapClusterer' in content:
    print('GapClusterer already exists')
else:
    content += clusterer_code
    with open('llm/research/gap_analyzer.py', 'w', encoding='utf-8', newline='\n') as f:
        f.write(content)
    print(f'Added GapClusterer, file size: {len(content)}')
