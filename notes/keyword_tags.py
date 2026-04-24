"""Keyword tag inference and predefined keyword tag patterns."""
import re
from functools import lru_cache
from typing import List, Tuple


from core import Paper


# Pre-compile regex patterns for better performance
KEYWORD_TAGS = [
    # Core AI concepts
    (re.compile(r"\bagent(s)?\b|tool\s*use|function\s*calling|autonomous\s*system", re.I), "Agent"),
    (re.compile(r"\brag\b|retrieval-augmented|retrieval augmented|knowledge\s*retrieval", re.I), "RAG"),
    (re.compile(r"\bmoe\b|mixture of experts", re.I), "MoE"),
    (re.compile(r"\brlhf\b|preference optimization|dpo\b|alignment", re.I), "Alignment"),
    (re.compile(r"\bevaluation\b|benchmark|performance\s*metric", re.I), "Evaluation"),
    (re.compile(r"\bcompiler\b|kernel|cuda|inference|hardware|accelerator", re.I), "Infrastructure"),
    (re.compile(r"\bmultimodal\b|vision|audio|text\s*image|cross\s*modal", re.I), "Multimodal"),
    (re.compile(r"\bcompression\b|quantization|distillation|model\s*reduction", re.I), "Optimization"),
    (re.compile(r"\blong context\b|context length|extended\s*context", re.I), "LongContext"),
    (re.compile(r"\bsafety\b|jailbreak|red teaming|adversarial\s*attack", re.I), "Safety"),

    # Additional AI research areas
    (re.compile(r"\bllm\b|large\s*language\s*model|transformer", re.I), "LLM"),
    (re.compile(r"\bgpt\b|generative\s*pre-trained", re.I), "GPT"),
    (re.compile(r"\bcnn\b|convolutional\s*neural\s*network", re.I), "CNN"),
    (re.compile(r"\brnn\b|recurrent\s*neural\s*network", re.I), "RNN"),
    (re.compile(r"\bgans\b|generative\s*adversarial\s*network", re.I), "GAN"),
    (re.compile(r"\bvae\b|variational\s*autoencoder", re.I), "VAE"),
    (re.compile(r"\breinforcement\s*learning|rl\b", re.I), "RL"),
    (re.compile(r"\bsupervised\s*learning", re.I), "SupervisedLearning"),
    (re.compile(r"\bunsupervised\s*learning", re.I), "UnsupervisedLearning"),
    (re.compile(r"\bsemi-supervised\s*learning", re.I), "SemiSupervisedLearning"),
    (re.compile(r"\bself-supervised\s*learning", re.I), "SelfSupervisedLearning"),
    (re.compile(r"\btransfer\s*learning", re.I), "TransferLearning"),
    (re.compile(r"\bfew-shot\s*learning|few\s*shot", re.I), "FewShotLearning"),
    (re.compile(r"\bzero-shot\s*learning|zero\s*shot", re.I), "ZeroShotLearning"),
    (re.compile(r"\bprompt\s*engineering", re.I), "PromptEngineering"),
    (re.compile(r"\btokenization\b|token\s*embedding", re.I), "Tokenization"),
    (re.compile(r"\bembedding\b|vector\s*representation", re.I), "Embedding"),
    (re.compile(r"\bknowledge\s*graph|kg\b", re.I), "KnowledgeGraph"),
    (re.compile(r"\breasoning\b|logical\s*inference", re.I), "Reasoning"),
    (re.compile(r"\bsummarization\b|summary", re.I), "Summarization"),
    (re.compile(r"\btranslation\b|machine\s*translation", re.I), "Translation"),
    (re.compile(r"\bquestion\s*answering|qa\b", re.I), "QA"),
    (re.compile(r"\bdocument\s*understanding", re.I), "DocumentUnderstanding"),
    (re.compile(r"\bcoding\b|code\s*generation", re.I), "Coding"),
    (re.compile(r"\bmedical\s*ai|healthcare\s*ai", re.I), "MedicalAI"),
    (re.compile(r"\bfinance\s*ai|financial\s*ai", re.I), "FinanceAI"),
    (re.compile(r"\beducation\s*ai|educational\s*ai", re.I), "EducationAI"),
    (re.compile(r"\benvironmental\s*ai|climate\s*ai", re.I), "EnvironmentalAI"),
]


@lru_cache(maxsize=1024)
def _get_keywords_signature(text: str) -> Tuple[str, ...]:
    """Get a tuple of matching tags for a given text (cached)."""
    matches = []
    for pattern, tag in KEYWORD_TAGS:
        if pattern.search(text):
            matches.append(tag)
    return tuple(matches)


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags

    # Combine title and abstract for better tag inference
    text = f"{paper.title}\n{paper.abstract}".lower()

    # Use cached function for better performance
    sorted_tags = list(_get_keywords_signature(text))

    # Remove redundant tags
    final_tags = []
    for tag in sorted_tags:
        # Check if this tag is already covered by a more specific tag
        is_redundant = False
        for existing_tag in final_tags:
            # Simple redundancy check: if existing tag is a substring of current tag or vice versa
            if tag.lower() in existing_tag.lower() or existing_tag.lower() in tag.lower():
                # Keep the more specific (longer) tag
                if len(tag) < len(existing_tag):
                    is_redundant = True
                    break
        if not is_redundant:
            final_tags.append(tag)

    from config import MAX_TAGS
    # Limit to maximum tags to avoid over-tagging
    final_tags = final_tags[:MAX_TAGS]

    return final_tags if final_tags else ["Unsorted"]


def get_all_tags() -> List[str]:
    """Get list of all available tags."""
    return [tag for _, tag in KEYWORD_TAGS]


def get_tags_count() -> int:
    """Get count of all predefined tags."""
    return len(KEYWORD_TAGS)
