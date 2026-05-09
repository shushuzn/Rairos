"""Patch: add ConfidenceScorer class to gap_analyzer.py"""
import re

with open('llm/research/gap_analyzer.py', 'r', encoding='utf-8') as f:
    content = f.read()

confidence_scorer = '''

# ─── Insight Confidence Scorer (Bayesian) ────────────────────────────────────


class ConfidenceScorer:
    """Bayesian confidence scoring for gap insights.

    Estimates P(novel | evidence) using:
      - novelty:    semantic distance from known gaps (0-1)
      - support:    ratio of supporting papers vs expected (0-1)
      - bayes_p:    posterior P(H=is_real_gap | evidence)

    Filtering rule:  confidence < 0.3 AND novelty < 0.2  → low-quality gap
    """

    # Maximum plausible papers per gap (for support ratio)
    MAX_SUPPORTING_PAPERS = 5
    # Prior probability a random insight is a real gap (skeptical prior)
    PRIOR_P_GAP = 0.4

    def __init__(self, gap_analyzer_v2: "GapAnalyzerV2"):
        self.gap_analyzer = gap_analyzer_v2

    # ── Semantic similarity ────────────────────────────────────────────────────

    def _title_similarity(self, t1: str, t2: str) -> float:
        """Word-overlap similarity between two titles (0-1)."""
        if not t1 or not t2:
            return 0.0
        words1 = set(t1.lower().split())
        words2 = set(t2.lower().split())
        if not words1 or not words2:
            return 0.0
        intersection = len(words1 & words2)
        union = len(words1 | words2)
        return intersection / union if union > 0 else 0.0

    def _semantic_similarity(self, gap_a: Gap, gap_b: Gap) -> float:
        """Combined similarity: title (60%) + gap_type (40%)."""
        title_sim = self._title_similarity(gap_a.title or "", gap_b.title or "")
        type_sim = 1.0 if gap_a.gap_type == gap_b.gap_type else 0.0
        return title_sim * 0.6 + type_sim * 0.4

    def _avg_similarity_to_known(self, gap: Gap, known_gaps: List["Gap"], top_k: int = 10) -> float:
        """Average similarity to top-k most-similar known gaps."""
        if not known_gaps:
            return 0.0
        similarities = sorted(
            [self._semantic_similarity(gap, kg) for kg in known_gaps],
            reverse=True,
        )
        k = min(top_k, len(similarities))
        return sum(similarities[:k]) / k if k > 0 else 0.0

    # ── Core scores ───────────────────────────────────────────────────────────

    def score_novelty(self, gap: Gap, known_gaps: List["Gap"]) -> float:
        """Novelty score: 1 - avg_similarity to known gaps (0-1)."""
        avg_sim = self._avg_similarity_to_known(gap, known_gaps)
        return max(0.0, min(1.0, 1.0 - avg_sim))

    def score_support(self, gap: Gap) -> float:
        """Support score: ratio of supporting papers / max_expected (0-1)."""
        n_papers = len(gap.supporting_papers)
        return min(1.0, n_papers / self.MAX_SUPPORTING_PAPERS)

    def score_bayesian(
        self,
        novelty: float,
        support: float,
        prior: float = None,
    ) -> float:
        """Posterior P(H=is_real_gap | novelty, support) via Bayes' rule.

        Uses a simplified Bayesian update:
          likelihood_ratio = (novelty * support) / max(novelty, support, 0.01)
          posterior = (prior * likelihood_ratio) / (prior * likelihood_ratio + (1-prior))

        Returns P(H | E) in [0, 1].
        """
        if prior is None:
            prior = self.PRIOR_P_GAP

        # Combined evidence strength
        evidence = novelty * support

        # Likelihood ratio: how much more likely this evidence is under H than ¬H
        p_evidence_given_h = max(0.01, min(0.99, evidence + (1 - evidence) * 0.5))
        p_evidence_given_not_h = max(0.01, min(0.99, 1.0 - evidence * 0.5))

        likelihood_ratio = p_evidence_given_h / p_evidence_given_not_h

        # Bayes' rule: P(H|E) = (P(E|H) * P(H)) / (P(E|H) * P(H) + P(E|¬H) * P(¬H))
        numerator = p_evidence_given_h * prior
        denominator = numerator + p_evidence_given_not_h * (1 - prior)
        posterior = numerator / denominator if denominator > 0 else prior

        return max(0.0, min(1.0, posterior))

    def score_confidence(
        self,
        gap: Gap,
        known_gaps: List["Gap"],
        prior: float = None,
    ) -> Dict[str, float]:
        """Compute all confidence metrics for a gap.

        Returns dict with keys: novelty, support, bayes_p, confidence.
        """
        novelty = self.score_novelty(gap, known_gaps)
        support = self.score_support(gap)
        bayes_p = self.score_bayesian(novelty, support, prior)

        # Final confidence: weighted combination of novelty + support + bayes_p
        confidence = novelty * 0.35 + support * 0.25 + bayes_p * 0.40

        return {
            "novelty": novelty,
            "support": support,
            "bayes_p": bayes_p,
            "confidence": confidence,
        }

    # ── Filtering ─────────────────────────────────────────────────────────────

    def is_low_quality(self, confidence_data: Dict[str, float]) -> bool:
        """Return True if gap should be suppressed as low-quality."""
        return (
            confidence_data["confidence"] < 0.3
            and confidence_data["novelty"] < 0.2
        )

    def filter_gaps(
        self,
        gaps: List[Gap],
        known_gaps: List[Gap],
        prior: float = None,
    ) -> tuple[List[Gap], List[Gap]]:
        """Filter gaps into high-quality and low-quality buckets.

        Returns (accepted_gaps, suppressed_gaps).
        """
        accepted: List[Gap] = []
        suppressed: List[Gap] = []

        for gap in gaps:
            conf = self.score_confidence(gap, known_gaps, prior)
            gap.confidence = conf["confidence"]
            gap.support_score = conf["support"]
            gap.novelty_bayes = conf["bayes_p"]

            if self.is_low_quality(conf):
                suppressed.append(gap)
            else:
                accepted.append(gap)

        return accepted, suppressed

    # ── Reporting ─────────────────────────────────────────────────────────────

    def get_quality_summary(self, gaps: List[Gap]) -> Dict[str, Any]:
        """Return quality statistics for a list of gaps."""
        if not gaps:
            return {"total": 0, "avg_confidence": 0.0, "avg_novelty": 0.0}

        confidences = [g.confidence for g in gaps if hasattr(g, "confidence")]
        novelties = [g.confidence for g in gaps if hasattr(g, "confidence")]

        return {
            "total": len(gaps),
            "avg_confidence": sum(confidences) / len(confidences) if confidences else 0.0,
            "avg_novelty": sum(novelties) / len(novelties) if novelties else 0.0,
            "low_quality_count": sum(1 for g in gaps if hasattr(g, "confidence") and g.confidence < 0.3),
        }

'''

if 'class ConfidenceScorer' not in content:
    # Insert before the last class definition or at end of file
    # Find a good insertion point before the final class
    insert_marker = '# ─── V2 Gap Analyzer'
    idx = content.find(insert_marker)
    if idx == -1:
        # Just append at end
        content = content.rstrip() + confidence_scorer
        print('Appended ConfidenceScorer to end')
    else:
        content = content[:idx] + confidence_scorer + '\n' + content[idx:]
        print('Inserted ConfidenceScorer before V2 Gap Analyzer')
else:
    print('ConfidenceScorer already exists')

# Update Gap class to include new fields
gap_fields = '''    # Confidence scoring fields
    confidence: float = 0.0          # Bayesian P(is_real_gap | evidence)
    support_score: float = 0.0        # Support ratio (0-1)
    novelty_bayes: float = 0.0         # Bayesian posterior novelty component'''

if 'confidence: float' not in content:
    # Add to Gap class after priority field
    import re
    # Find "priority: int = 0" in the Gap dataclass
    pattern = r'(    priority: int = 0\n)'
    if re.search(pattern, content):
        content = re.sub(pattern, r'\1' + gap_fields + '\n', content)
        print('Added confidence fields to Gap class')
    else:
        print('WARNING: priority field not found, fields may not be added')

with open('llm/research/gap_analyzer.py', 'w', encoding='utf-8', newline='\n') as f:
    f.write(content)
print(f'File size: {len(content)}')
