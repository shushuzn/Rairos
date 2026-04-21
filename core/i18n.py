"""Internationalization support for CLI output."""
import os
from typing import Callable

LANG = os.environ.get("AI_RESEARCH_LANG", "zh").lower()
_in_lang: str = LANG

# en strings
_MSGS_EN = {
    # research loop
    "research_searching": "[research] Searching arXiv for: {query}",
    "research_no_papers": "[research] No papers found for query: {query}",
    "research_found": "[research] Found {n} papers",
    "research_done": "\n[research] Done: {processed}/{total} processed, {failed} failed, {skipped} skipped",
    "research_done_reason": "  [{reason}] {count} paper(s)",
    "research_skip": "  [skip] Already exists: {name}",
    "research_pdf_downloaded": "  [pdf] Downloaded: {name} ({size:.0f} KB)",
    "research_text_extracted": "  [text] Extracted {n} chars",
    "research_llm_generating": "  [llm] Generating draft...",
    "research_llm_generated": "  [llm] Draft generated ({n} chars)",
    "research_pdf_failed": "PDF download/extract failed for {uid} after retry",
    "research_llm_failed": "LLM draft generation failed for {uid}",
    "research_no_api_key": "  [skip] No API key — metadata-only note",
    "research_no_text": "  [skip] No extracted text — metadata-only note",
    "research_saved": "  [saved] {name}",
    "research_saved_novelty": "  [saved] {name} [novelty={score}]",
    # parse errors
    "err_pdf_download": "PDF download failed",
    "err_pdf_no_url": "No directly downloadable PDF link available (common for DOI-only metadata); skipped PDF extraction.",
    "err_pdf_extract": "PDF extraction failed",
    "err_ai_draft": "AI Draft generation failed — requires manual verification",
    "err_detail": "Error: {e}",
    "err_suggestion": "Suggestion: check OPENAI_API_KEY / --api-key / --base-url / --model",
    # misc
    "ai_draft_enabled": "- AI Draft: ENABLED (see P-Note section: 'AI 自动初稿（待核验）')",
    "research_done_done": "Done: {processed}/{total} processed, {failed} failed, {skipped} skipped",
}

# zh strings
_MSGS_ZH = {
    "research_searching": "[research] 正在搜索 arXiv：{query}",
    "research_no_papers": "[research] 未找到相关论文：{query}",
    "research_found": "[research] 找到 {n} 篇论文",
    "research_done": "\n[research] 完成：{processed}/{total} 已处理，{failed} 失败，{skipped} 跳过",
    "research_done_reason": "  [{reason}] {count} 篇",
    "research_skip": "  [跳过] 已存在：{name}",
    "research_pdf_downloaded": "  [pdf] 已下载：{name} ({size:.0f} KB)",
    "research_text_extracted": "  [text] 已提取 {n} 字符",
    "research_llm_generating": "  [llm] 正在生成草稿...",
    "research_llm_generated": "  [llm] 草稿已生成（{n} 字符）",
    "research_pdf_failed": "PDF 下载/解析失败（重试后仍失败）：{uid}",
    "research_llm_failed": "LLM 草稿生成失败：{uid}",
    "research_no_api_key": "  [跳过] 无 API Key — 仅保存元数据",
    "research_no_text": "  [跳过] 无提取文本 — 仅保存元数据",
    "research_saved": "  [已保存] {name}",
    "research_saved_novelty": "  [已保存] {name} [新颖度={score}]",
    "err_pdf_download": "PDF 下载失败",
    "err_pdf_no_url": "未提供可直接下载的 PDF 链接（常见于 DOI-only 元数据），已跳过 PDF 抽取。",
    "err_pdf_extract": "PDF 抽取失败",
    "err_ai_draft": "AI 草稿生成失败，需人工核验",
    "err_detail": "错误：{e}",
    "err_suggestion": "建议：检查 OPENAI_API_KEY / --api-key / --base-url / --model",
    "ai_draft_enabled": "- AI 草稿：已启用（见 P-Note 章节：'AI 自动初稿（待核验）'）",
    "research_done_done": "完成：{processed}/{total} 已处理，{failed} 失败，{skipped} 跳过",
}

_MSGS: dict[str, dict[str, str]] = {"en": _MSGS_EN, "zh": _MSGS_ZH}
_LANG_CODES = {"en", "zh", "e", "z"}


def set_lang(lang: str) -> None:
    """Set active language (en/zh)."""
    global _in_lang
    lang = lang.lower()
    if lang in ("e",):
        _in_lang = "en"
    elif lang in ("z",):
        _in_lang = "zh"
    else:
        _in_lang = lang if lang in _LANG_CODES else "zh"


def get_lang() -> str:
    """Return current language code."""
    return _in_lang


def _(key: str, **kwargs) -> str:
    """Translate key with format substitutions."""
    msg = _MSGS.get(_in_lang, _MSGS_ZH).get(key, key)
    return msg.format(**kwargs) if kwargs else msg
