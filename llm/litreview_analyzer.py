"""Literature Review Analyzer: Analyzes papers for trends and insights."""
from __future__ import annotations

from typing import List, Dict, Any, Optional
from collections import defaultdict


class LitReviewAnalyzer:
    """Analyze papers to generate literature review content."""

    def __init__(self, db=None):
        self.db = db

    def analyze_trends(self, papers: List[Dict[str, Any]]) -> Dict[str, Any]:
        """Analyze paper collection for research trends.

        Args:
            papers: List of paper dicts with title, abstract, published, score

        Returns:
            Dict with trend analysis results:
            {
                "method_evolution": [...],
                "temporal_distribution": {...},
                "rising_topics": [...],
            }
        """
        if not papers:
            return {"method_evolution": [], "temporal_distribution": {}, "rising_topics": []}

        # Analyze method evolution
        method_evolution = self._analyze_method_evolution(papers)

        # Temporal distribution
        temporal = self._analyze_temporal_distribution(papers)

        # Rising topics (based on recent papers)
        rising = self._detect_rising_topics(papers)

        return {
            "method_evolution": method_evolution,
            "temporal_distribution": temporal,
            "rising_topics": rising,
        }

    def find_controversies(self, papers: List[Dict[str, Any]]) -> List[str]:
        """Detect potential controversies or contradictions between papers.

        Returns list of controversy descriptions.
        """
        controversies = []

        # Signal phrases that might indicate disagreements
        controversy_signals = [
            ("outperforms", "while others argue"),
            ("contrary to", "however"),
            ("different from", "unlike"),
            ("challenge", "question"),
            ("limitation of", "we show that"),
        ]

        for paper in papers[:20]:
            abstract = paper.get("abstract", "").lower()
            title = paper.get("title", "")

            for signal1, signal2 in controversy_signals:
                if signal1 in abstract and signal2 in abstract:
                    controversies.append(
                        f"_{title[:40]}..._: 可能在方法选择或结论上存在争议"
                    )
                    break

        return controversies[:5]

    def extract_open_problems(self, papers: List[Dict[str, Any]]) -> List[str]:
        """Extract open problems and future directions from papers.

        Returns list of open problem descriptions.
        """
        open_problems = []

        signal_phrases = [
            "remain an open problem",
            "future work",
            "future research",
            "left for future",
            "beyond the scope",
            "opportunity for future",
            "potential future direction",
            "remain challenging",
            "still needs",
            "requires further",
        ]

        for paper in papers[:30]:
            abstract = paper.get("abstract", "")
            if not abstract:
                continue

            title = paper.get("title", "")[:40]
            abstract_lower = abstract.lower()

            for phrase in signal_phrases:
                idx = abstract_lower.find(phrase)
                if idx > 0:
                    # Extract surrounding context
                    start = max(0, idx - 30)
                    end = min(len(abstract), idx + 100)
                    context = abstract[start:end].strip()
                    # Clean whitespace
                    context = " ".join(context.split())

                    if len(context) > 20:
                        open_problems.append(
                            f"_{title}..._: ...{context}..."
                        )
                        break

        return open_problems[:8]

    def group_by_methodology(self, papers: List[Dict[str, Any]]) -> Dict[str, List[Dict]]:
        """Group papers by detected methodology or approach type.

        Returns:
            Dict mapping methodology name to list of papers
        """
        method_keywords = {
            "Transformer/Attention": [
                "transformer", "attention", "self-attention", "bert", "gpt",
                "vit", "vision transformer", "llama", "decoder-only"
            ],
            "CNN/卷积网络": [
                "convolutional", "cnn", "convolution", "resnet", "vgg",
                "efficientnet", "mobilenet", "inception"
            ],
            "图神经网络": [
                "graph", "gnn", "gcn", "gat", "graph neural",
                "message passing", "graph attention"
            ],
            "强化学习": [
                "reinforcement learning", "rl ", "policy gradient",
                "q-learning", "ddpg", "ppo", "actor-critic",
                "reward", "environment interaction"
            ],
            "扩散模型": [
                "diffusion", "ddpm", "score-based", "stable diffusion",
                "ddim", "latent diffusion", "generative model"
            ],
            "检索增强": [
                "retrieval-augmented", "rag", "knowledge retrieval",
                "retrieval", "dense retrieval", "BM25"
            ],
            "多模态": [
                "multimodal", "vision-language", "image-text",
                "vqa", "visual question", "cross-modal",
                "clip", "flamingo"
            ],
            "对比学习": [
                "contrastive learning", "contrastive loss", "simclr",
                "triplet loss", "infoNCE", "momentum contrast"
            ],
            "知识蒸馏": [
                "knowledge distillation", "distillation", "teacher-student",
                "model compression", "pruning", "quantization"
            ],
            "自监督学习": [
                "self-supervised", " pretext task", "masked",
                "BYOL", "SwAV", "momentum encoder"
            ],
        }

        groups = defaultdict(list)
        unclassified = []

        for paper in papers:
            text = (paper.get("title", "") + " " + paper.get("abstract", "")).lower()
            matched = False

            for method, keywords in method_keywords.items():
                if any(kw in text for kw in keywords):
                    groups[method].append(paper)
                    matched = True
                    break

            if not matched:
                unclassified.append(paper)

        if unclassified:
            groups["其他/未分类"] = unclassified

        return dict(groups)

    def _analyze_method_evolution(self, papers: List[Dict[str, Any]]) -> List[str]:
        """Analyze how methods have evolved over time."""
        evolution = []

        # Sort by date
        sorted_papers = sorted(
            papers,
            key=lambda p: p.get("published", ""),
            reverse=True
        )

        # Detect method mentions in recent papers
        recent_methods = set()
        for paper in sorted_papers[:10]:
            text = (paper.get("title", "") + " " + paper.get("abstract", "")).lower()

            if "transformer" in text or "attention" in text:
                recent_methods.add("Transformer架构")
            if "diffusion" in text or "ddpm" in text:
                recent_methods.add("扩散模型")
            if "retrieval" in text or "rag" in text:
                recent_methods.add("检索增强生成")
            if "multimodal" in text or "vision-language" in text:
                recent_methods.add("多模态学习")
            if "graph" in text or "gnn" in text:
                recent_methods.add("图神经网络")

        for method in recent_methods:
            evolution.append(f"近期研究重点: {method}")

        return evolution

    def _analyze_temporal_distribution(self, papers: List[Dict[str, Any]]) -> Dict[str, int]:
        """Get paper count by year/quarter."""
        distribution = defaultdict(int)

        for paper in papers:
            published = paper.get("published", "")
            if published:
                year = published[:4]
                if year.isdigit():
                    distribution[year] += 1

        return dict(sorted(distribution.items(), reverse=True))

    def _detect_rising_topics(self, papers: List[Dict[str, Any]]) -> List[str]:
        """Detect topics that are trending upward based on recent papers."""
        # Sort by date (newest first)
        sorted_papers = sorted(
            papers,
            key=lambda p: p.get("published", ""),
            reverse=True
        )

        # Check recent papers (last 20) for emerging patterns
        recent_text = ""
        for paper in sorted_papers[:20]:
            recent_text += " " + paper.get("title", "").lower()

        rising = []

        if "diffusion" in recent_text or "ddpm" in recent_text:
            rising.append("扩散模型 (Diffusion Models)")
        if "llm" in recent_text or "large language" in recent_text:
            rising.append("大语言模型 (LLMs)")
        if "multimodal" in recent_text or "vision-language" in recent_text:
            rising.append("多模态学习")
        if "retrieval" in recent_text or "rag" in recent_text:
            rising.append("检索增强生成 (RAG)")
        if "instruction" in recent_text and "tuning" in recent_text:
            rising.append("指令微调 (Instruction Tuning)")
        if "chain-of-thought" in recent_text or "cot" in recent_text:
            rising.append("思维链推理 (Chain-of-Thought)")
        if "scaling" in recent_text and "law" in recent_text:
            rising.append("Scaling Laws")

        return rising[:5]

    def update_for_subscription(
        self,
        subscription_id: str,
        new_papers: List[Dict[str, Any]],
    ) -> Optional[str]:
        """Update literature review for a subscription after new papers arrive.

        Args:
            subscription_id: ID of subscription
            new_papers: Newly discovered papers

        Returns:
            File path of updated review, or None if no review exists
        """
        if not self.db:
            return None

        # Get subscription info
        sub = self.db.get_arxiv_subscription(subscription_id)
        if not sub:
            return None

        topic = sub.get("topic", "")

        # Find associated review
        reviews = self.db.list_literature_reviews()
        review = None
        for r in reviews:
            if r.get("subscription_id") == subscription_id:
                review = r
                break

        if not review:
            return None

        file_path = review.get("file_path")
        if not file_path or not os.path.exists(file_path):
            return None

        # Read existing review
        with open(file_path, "r", encoding="utf-8") as f:
            existing_content = f.read()

        # Get all papers for this subscription
        all_papers = self.db.get_subscription_papers(subscription_id, limit=200)

        # Update the review
        from renderers.litreview import update_litreview
        updated_content = update_litreview(existing_content, new_papers, all_papers)

        # Write back
        with open(file_path, "w", encoding="utf-8") as f:
            f.write(updated_content)

        # Update DB
        self.db.update_literature_review(review["id"], paper_count=len(all_papers))

        return file_path


import os  # Needed for file existence check
