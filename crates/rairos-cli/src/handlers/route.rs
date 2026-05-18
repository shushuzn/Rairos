#![allow(
    clippy::too_many_arguments,
    clippy::needless_borrow,
    clippy::print_literal,
    clippy::unwrap_or_default,
    clippy::unnecessary_sort_by,
    clippy::format_in_format_args,
    clippy::map_identity,
    clippy::unused_enumerate_index,
    clippy::needless_borrows_for_generic_args,
    clippy::unnecessary_to_owned,
    clippy::manual_range_contains
)]

use anyhow::Result;
use crate::handlers::*;

pub fn handle_route(query: &[String], json: bool, exec: bool, all: bool) -> Result<()> {
    if query.is_empty() {
        eprintln!("Usage: rairos route <query>");
        return Ok(());
    }
    let query_text = query.join(" ");

    // ── QueryType taxonomy (mirrors Python llm.routing.semantic_router.QueryType) ──
    #[derive(Debug, Clone, Copy, PartialEq)]
    enum QueryType {
        GapAnalysis,
        HypothesisGeneration,
        Experiment,
        Insight,
        Narrative,
        PaperSearch,
        QuestionAnswer,
        General,
    }

    impl QueryType {
        fn value(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap_analysis",
                QueryType::HypothesisGeneration => "hypothesis_generation",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "paper_search",
                QueryType::QuestionAnswer => "question_answer",
                QueryType::General => "general",
            }
        }
        fn command(&self) -> &'static str {
            match self {
                QueryType::GapAnalysis => "gap",
                QueryType::HypothesisGeneration => "hypothesize",
                QueryType::Experiment => "experiment",
                QueryType::Insight => "insight",
                QueryType::Narrative => "narrative",
                QueryType::PaperSearch => "search",
                QueryType::QuestionAnswer => "ask",
                QueryType::General => "chat",
            }
        }
    }

    // ── Keyword scoring ──
    fn keyword_score(query_lower: &str, qt: QueryType) -> f64 {
        let keywords: &[&str] = match qt {
            QueryType::GapAnalysis => &[
                "gap", "gaps", "空白", "未解决", "missing", "unresolved", "opportunity",
                "差距", "limitation", "limitations", "不足", "untouched", "overlooked",
                "open problem", "open question",
            ],
            QueryType::HypothesisGeneration => &[
                "hypothesis", "假设", "假设生成", "conjecture", "predict", "预测",
                "实验设计", "hypothesize", "if-then",
            ],
            QueryType::Experiment => &[
                "experiment", "实验", "ab test", "evaluate", "评估", "validate", "验证",
                "trial", "跑实验", "实验结果", "benchmark", "benchmarking",
            ],
            QueryType::Insight => &[
                "insight", "insights", "发现", "洞察", "pattern", "patterns",
                "key finding", "takeaway", "synthesis",
            ],
            QueryType::Narrative => &[
                "narrative", "story", "线程", "progress", "phase", "跟踪", "进展",
                "状态", "story arc",
            ],
            QueryType::PaperSearch => &[
                "paper", "papers", "search", "find", "论文", "搜索", "arxiv",
                "找论文", "文献", "publication",
            ],
            QueryType::QuestionAnswer => &[
                "what", "who", "how", "why", "explain", "什么", "如何", "为什么",
                "请问", "回答", "answer", "can you",
            ],
            QueryType::General => &[
                "chat", "talk", "discuss", "对话", "聊聊", "tell me", "about",
                "introduction", "介绍",
            ],
        };
        keywords.iter().filter(|kw| query_lower.contains(*kw)).count() as f64
    }

    let q_lower = query_text.to_lowercase();
    let query_types = [
        QueryType::GapAnalysis,
        QueryType::HypothesisGeneration,
        QueryType::Experiment,
        QueryType::Insight,
        QueryType::Narrative,
        QueryType::PaperSearch,
        QueryType::QuestionAnswer,
        QueryType::General,
    ];

    let mut best_score = 0.0f64;
    let mut best_qt = QueryType::General;

    for qt in &query_types {
        let score = keyword_score(&q_lower, *qt);
        if score > best_score {
            best_score = score;
            best_qt = *qt;
        }
    }

    let confidence = (best_score / 3.0).min(1.0);

    // ── Output ──
    let bar = "█".repeat((confidence * 10.0) as usize)
        + &"░".repeat(10 - (confidence * 10.0).min(10.0) as usize);

    if json {
        let output = serde_json::json!({
            "query_type": best_qt.value(),
            "confidence": confidence,
            "primary_command": best_qt.command(),
            "reasoning": "[keyword routing]",
            "multi_intent": false,
        });
        println!("{}", serde_json::to_string_pretty(&output).unwrap());
    } else {
        println!("🔍 Query: {}", query_text);
        println!("   Type:          {}", best_qt.value());
        println!("   Command:       {}", best_qt.command());
        println!("   Confidence:    {} {:.0}%", bar, confidence * 100.0);
        println!();
    }

    // ── Execution (print what would run) ──
    if exec || all {
        println!("   [Would run] {} \"{}\"", best_qt.command(), query_text);
    }

    Ok(())
}
