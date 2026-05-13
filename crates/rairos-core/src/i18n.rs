//! Rairos I18n — Internationalization for CLI output
//!
//! Ported from core/i18n.py
//!
//! Supports English (en) and Chinese (zh) localization for CLI messages.

use std::collections::HashMap;
use std::env;
use std::sync::LazyLock;

// ============================================================================
// Language Detection
// ============================================================================

static LANG_CODES: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("en", "en");
    m.insert("zh", "zh");
    m.insert("e", "en");
    m.insert("z", "zh");
    m
});

static LANG: LazyLock<String> = LazyLock::new(|| {
    env::var("AIROS_LANG")
        .or_else(|_| env::var("AIROS_DEFAULT_LANG"))
        .unwrap_or_else(|_| "zh".to_string())
        .to_lowercase()
});

// ============================================================================
// Message Maps
// ============================================================================

type MsgMap = HashMap<&'static str, &'static str>;

static MSGS_EN: LazyLock<MsgMap> = LazyLock::new(|| {
    let mut m = MsgMap::new();
    m.insert(
        "research_searching",
        "[research] Searching arXiv for: {query}",
    );
    m.insert(
        "research_no_papers",
        "[research] No papers found for query: {query}",
    );
    m.insert("research_found", "[research] Found {n} papers");
    m.insert(
        "research_done",
        "\n[research] Done: {processed}/{total} processed, {failed} failed, {skipped} skipped",
    );
    m.insert("research_done_reason", "  [{reason}] {count} paper(s)");
    m.insert("research_skip", "  [skip] Already exists: {name}");
    m.insert(
        "research_pdf_downloaded",
        "  [pdf] Downloaded: {name} ({size:.0} KB)",
    );
    m.insert("research_text_extracted", "  [text] Extracted {n} chars");
    m.insert("research_llm_generating", "  [llm] Generating draft...");
    m.insert(
        "research_llm_generated",
        "  [llm] Draft generated ({n} chars)",
    );
    m.insert(
        "research_pdf_failed",
        "PDF download/extract failed for {uid} after retry",
    );
    m.insert(
        "research_llm_failed",
        "LLM draft generation failed for {uid}",
    );
    m.insert(
        "research_no_api_key",
        "  [skip] No API key — metadata-only note",
    );
    m.insert(
        "research_no_text",
        "  [skip] No extracted text — metadata-only note",
    );
    m.insert("research_saved", "  [saved] {name}");
    m.insert(
        "research_saved_novelty",
        "  [saved] {name} [novelty={score}]",
    );
    m.insert("err_pdf_download", "PDF download failed");
    m.insert("err_pdf_no_url", "No directly downloadable PDF link available (common for DOI-only metadata); skipped PDF extraction.");
    m.insert("err_pdf_extract", "PDF extraction failed");
    m.insert(
        "err_ai_draft",
        "AI Draft generation failed — requires manual verification",
    );
    m.insert("err_detail", "Error: {e}");
    m.insert(
        "err_suggestion",
        "Suggestion: check OPENAI_API_KEY / --api-key / --base-url / --model",
    );
    m.insert(
        "ai_draft_enabled",
        "- AI Draft: ENABLED (see P-Note section: 'AI 自动初稿（待核验）')",
    );
    m.insert(
        "research_done_done",
        "Done: {processed}/{total} processed, {failed} failed, {skipped} skipped",
    );
    m
});

static MSGS_ZH: LazyLock<MsgMap> = LazyLock::new(|| {
    let mut m = MsgMap::new();
    m.insert("research_searching", "[research] 正在搜索 arXiv：{query}");
    m.insert("research_no_papers", "[research] 未找到相关论文：{query}");
    m.insert("research_found", "[research] 找到 {n} 篇论文");
    m.insert(
        "research_done",
        "\n[research] 完成：{processed}/{total} 已处理，{failed} 失败，{skipped} 跳过",
    );
    m.insert("research_done_reason", "  [{reason}] {count} 篇");
    m.insert("research_skip", "  [跳过] 已存在：{name}");
    m.insert(
        "research_pdf_downloaded",
        "  [pdf] 已下载：{name} ({size:.0} KB)",
    );
    m.insert("research_text_extracted", "  [text] 已提取 {n} 字符");
    m.insert("research_llm_generating", "  [llm] 正在生成草稿...");
    m.insert("research_llm_generated", "  [llm] 草稿已生成（{n} 字符）");
    m.insert(
        "research_pdf_failed",
        "PDF 下载/解析失败（重试后仍失败）：{uid}",
    );
    m.insert("research_llm_failed", "LLM 草稿生成失败：{uid}");
    m.insert("research_no_api_key", "  [跳过] 无 API Key — 仅保存元数据");
    m.insert("research_no_text", "  [跳过] 无提取文本 — 仅保存元数据");
    m.insert("research_saved", "  [已保存] {name}");
    m.insert(
        "research_saved_novelty",
        "  [已保存] {name} [新颖度={score}]",
    );
    m.insert("err_pdf_download", "PDF 下载失败");
    m.insert(
        "err_pdf_no_url",
        "未提供可直接下载的 PDF 链接（常见于 DOI-only 元数据），已跳过 PDF 抽取。",
    );
    m.insert("err_pdf_extract", "PDF 抽取失败");
    m.insert("err_ai_draft", "AI 草稿生成失败，需人工核验");
    m.insert("err_detail", "错误：{e}");
    m.insert(
        "err_suggestion",
        "建议：检查 OPENAI_API_KEY / --api-key / --base-url / --model",
    );
    m.insert(
        "ai_draft_enabled",
        "- AI 草稿：已启用（见 P-Note 章节：'AI 自动初稿（待核验）'）",
    );
    m.insert(
        "research_done_done",
        "完成：{processed}/{total} 已处理，{failed} 失败，{skipped} 跳过",
    );
    m
});

fn get_msgs<'a>() -> &'a MsgMap {
    match LANG_CODES.get(LANG.as_str()) {
        Some(&"en") => &MSGS_EN,
        _ => &MSGS_ZH,
    }
}

// ============================================================================
// Public API
// ============================================================================

pub fn get_lang() -> String {
    LANG_CODES
        .get(LANG.as_str())
        .map(|&s| s.to_string())
        .unwrap_or_else(|| "zh".to_string())
}

pub fn set_lang(lang: &str) {
    let lang_lower = lang.to_lowercase();
    let resolved = match lang_lower.as_str() {
        "e" => "en",
        "z" => "zh",
        s if LANG_CODES.contains_key(s) => s,
        _ => "zh",
    };
    env::set_var("AIROS_LANG", resolved);
}

pub fn t(key: &str) -> String {
    get_msgs()
        .get(key)
        .map(|&s| s.to_string())
        .unwrap_or_else(|| key.to_string())
}

pub fn t_fmt(key: &str, replacements: &[(&str, &str)]) -> String {
    let mut msg = t(key);
    for (k, v) in replacements {
        msg = msg.replace(&format!("{{{}}}", k), v);
    }
    msg
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_lang() {
        let lang = get_lang();
        assert!(lang == "en" || lang == "zh");
    }

    #[test]
    fn test_t_key_exists() {
        assert_eq!(
            t("research_searching").find("Searching arXiv").is_some()
                || t("research_searching").find("搜索 arXiv").is_some(),
            true
        );
    }

    #[test]
    fn test_t_unknown_key() {
        assert_eq!(t("unknown_key_xyz"), "unknown_key_xyz");
    }

    #[test]
    fn test_t_fmt() {
        let msg = t_fmt("research_searching", &[("query", "machine learning")]);
        assert!(msg.contains("machine learning"));
    }
}
