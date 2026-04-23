"""Keyword tag inference and predefined keyword tag patterns."""
import re
from typing import List

from core import Paper


KEYWORD_TAGS = [
    # Core AI concepts
    (r"\bagent(s)?\b|tool\s*use|function\s*calling|autonomous\s*system", "Agent"),
    (r"\brag\b|retrieval-augmented|retrieval augmented|knowledge\s*retrieval", "RAG"),
    (r"\bmoe\b|mixture of experts", "MoE"),
    (r"\brlhf\b|preference optimization|dpo\b|alignment", "Alignment"),
    (r"\bevaluation\b|benchmark|performance\s*metric", "Evaluation"),
    (r"\bcompiler\b|kernel|cuda|inference|hardware|accelerator", "Infrastructure"),
    (r"\bmultimodal\b|vision|audio|text\s*image|cross\s*modal", "Multimodal"),
    (r"\bcompression\b|quantization|distillation|model\s*reduction", "Optimization"),
    (r"\blong context\b|context length|extended\s*context", "LongContext"),
    (r"\bsafety\b|jailbreak|red teaming|adversarial\s*attack", "Safety"),
    
    # Additional AI research areas
    (r"\bllm\b|large\s*language\s*model|transformer", "LLM"),
    (r"\bgpt\b|generative\s*pre-trained", "GPT"),
    (r"\bcnn\b|convolutional\s*neural\s*network", "CNN"),
    (r"\brnn\b|recurrent\s*neural\s*network", "RNN"),
    (r"\bgans\b|generative\s*adversarial\s*network", "GAN"),
    (r"\bvae\b|variational\s*autoencoder", "VAE"),
    (r"\breinforcement\s*learning|rl\b", "RL"),
    (r"\bsupervised\s*learning", "SupervisedLearning"),
    (r"\bunsupervised\s*learning", "UnsupervisedLearning"),
    (r"\bsemi-supervised\s*learning", "SemiSupervisedLearning"),
    (r"\bself-supervised\s*learning", "SelfSupervisedLearning"),
    (r"\btransfer\s*learning", "TransferLearning"),
    (r"\bfew-shot\s*learning|few\s*shot", "FewShotLearning"),
    (r"\bzero-shot\s*learning|zero\s*shot", "ZeroShotLearning"),
    (r"\bprompt\s*engineering", "PromptEngineering"),
    (r"\btokenization\b|token\s*embedding", "Tokenization"),
    (r"\bembedding\b|vector\s*representation", "Embedding"),
    (r"\bknowledge\s*graph|kg\b", "KnowledgeGraph"),
    (r"\breasoning\b|logical\s*inference", "Reasoning"),
    (r"\bsummarization\b|summary", "Summarization"),
    (r"\btranslation\b|machine\s*translation", "Translation"),
    (r"\bquestion\s*answering|qa\b", "QA"),
    (r"\bdocument\s*understanding", "DocumentUnderstanding"),
    (r"\bcoding\b|code\s*generation", "Coding"),
    (r"\bmedical\s*ai|healthcare\s*ai", "MedicalAI"),
    (r"\bfinance\s*ai|financial\s*ai", "FinanceAI"),
    (r"\beducation\s*ai|educational\s*ai", "EducationAI"),
    (r"\benvironmental\s*ai|climate\s*ai", "EnvironmentalAI"),
]


def infer_tags_if_empty(tags: List[str], paper: Paper) -> List[str]:
    if tags:
        return tags
    
    # Combine title and abstract for better tag inference
    text = f"{paper.title}\n{paper.abstract}".lower()
    
    # Dictionary to store tags with their match positions (for priority)
    tag_matches = {}
    
    # First pass: find all matching tags
    for pattern, tag in KEYWORD_TAGS:
        match = re.search(pattern, text, flags=re.I)
        if match:
            # Store the position of the match (earlier matches have higher priority)
            tag_matches[tag] = match.start()
    
    # Sort tags by match position (earlier matches first)
    sorted_tags = sorted(tag_matches.keys(), key=lambda x: tag_matches[x])
    
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
