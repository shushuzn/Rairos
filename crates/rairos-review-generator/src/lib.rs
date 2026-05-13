//! rairos-review-generator — Literature Review Generator

#![allow(clippy::too_many_arguments)]
//!
//! Ported from `llm/review_generator.py` (329 LOC, pure stdlib).
//!
//! Generates comprehensive reviews with:
//! - Research stream classification
//! - Controversy detection between streams
//! - Evolution timeline construction
//! - Open problem identification

// ============================================================================
// Data Structures
// ============================================================================

/// A research stream/school in the literature.
#[derive(Debug, Clone)]
pub struct ResearchStream {
    pub name: String,
    pub papers: Vec<String>,
    pub methods: Vec<String>,
    pub key_contributions: Vec<String>,
}

impl ResearchStream {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            papers: Vec::new(),
            methods: Vec::new(),
            key_contributions: Vec::new(),
        }
    }
}

/// A controversy between research streams.
#[derive(Debug, Clone)]
pub struct Controversy {
    pub topic: String,
    pub stream_a: String,
    pub stream_b: String,
    pub position_a: String,
    pub position_b: String,
    pub papers: Vec<String>,
}

impl Controversy {
    pub fn new(
        topic: &str,
        stream_a: &str,
        stream_b: &str,
        position_a: &str,
        position_b: &str,
    ) -> Self {
        Self {
            topic: topic.to_string(),
            stream_a: stream_a.to_string(),
            stream_b: stream_b.to_string(),
            position_a: position_a.to_string(),
            position_b: position_b.to_string(),
            papers: Vec::new(),
        }
    }
}

/// A section of the review.
#[derive(Debug, Clone)]
pub struct ReviewSection {
    pub title: String,
    pub content: String,
    pub subsections: Vec<ReviewSection>,
}

impl ReviewSection {
    pub fn new(title: &str, content: &str) -> Self {
        Self {
            title: title.to_string(),
            content: content.to_string(),
            subsections: Vec::new(),
        }
    }
}

/// A paper reference for review generation.
#[derive(Debug, Clone, Default)]
pub struct PaperRef {
    pub uid: String,
    pub title: String,
    pub abstract_text: String,
    pub year: Option<i32>,
}

impl PaperRef {
    pub fn new(uid: &str, title: &str, abstract_text: &str, year: Option<i32>) -> Self {
        Self {
            uid: uid.to_string(),
            title: title.to_string(),
            abstract_text: abstract_text.to_string(),
            year,
        }
    }
}

/// Generated literature review.
#[derive(Debug, Clone)]
pub struct LiteratureReview {
    pub topic: String,
    pub streams: Vec<ResearchStream>,
    pub controversies: Vec<Controversy>,
    pub timeline: Vec<(i32, String)>,
    pub open_problems: Vec<String>,
    pub sections: Vec<ReviewSection>,
}

impl LiteratureReview {
    pub fn new(topic: &str) -> Self {
        Self {
            topic: topic.to_string(),
            streams: Vec::new(),
            controversies: Vec::new(),
            timeline: Vec::new(),
            open_problems: Vec::new(),
            sections: Vec::new(),
        }
    }
}

// ============================================================================
// ReviewGenerator
// ============================================================================

/// Generate structured literature reviews.
pub struct ReviewGenerator {
    papers: Vec<PaperRef>,
}

impl ReviewGenerator {
    pub fn new() -> Self {
        Self {
            papers: Vec::new(),
        }
    }

    /// Add papers to the generator's context.
    pub fn add_papers(&mut self, papers: Vec<PaperRef>) {
        self.papers.extend(papers);
    }

    /// Generate a literature review for the topic.
    pub fn generate(
        &self,
        topic: &str,
        max_papers: usize,
        depth: &str,
        sections_filter: Option<Vec<&str>>,
    ) -> LiteratureReview {
        // 1. Collect papers (slice of self.papers, limited to max_papers)
        let papers_slice = &self.papers[..max_papers.min(self.papers.len())];

        // 2. Classify into streams
        let streams = self.classify_streams(papers_slice);

        // 3. Detect controversies
        let controversies = self.detect_controversies(&streams);

        // 4. Build timeline
        let timeline = self.build_timeline(papers_slice);

        // 5. Identify open problems
        let open_problems = self.identify_gaps(papers_slice, &streams);

        // 6. Generate structured sections
        let review_sections = self.generate_sections(
            topic,
            &streams,
            &controversies,
            &timeline,
            &open_problems,
            depth,
            sections_filter,
        );

        LiteratureReview {
            topic: topic.to_string(),
            streams,
            controversies,
            timeline,
            open_problems,
            sections: review_sections,
        }
    }

    fn classify_streams(&self, papers: &[PaperRef]) -> Vec<ResearchStream> {
        let mut streams: std::collections::HashMap<&str, ResearchStream> =
            std::collections::HashMap::new();

        for paper in papers {
            let text = format!("{} {}", paper.title, paper.abstract_text).to_lowercase();

            let stream_name = if text.contains("retrieval")
                || text.contains("retriever")
                || text.contains("search")
                || text.contains("index")
            {
                "检索增强型"
            } else if text.contains("generation")
                || text.contains("generator")
                || text.contains("decoder")
                || text.contains("llm")
                || text.contains("gpt")
            {
                "生成优化型"
            } else if text.contains("hybrid")
                || text.contains("fusion")
                || text.contains("combine")
                || text.contains("ensemble")
            {
                "混合方法"
            } else if text.contains("fine-tun")
                || text.contains("tuning")
                || text.contains("adaptation")
                || text.contains("transfer")
            {
                "适配优化型"
            } else {
                "其他方法"
            };

            streams
                .entry(stream_name)
                .or_insert_with(|| ResearchStream::new(stream_name))
                .papers
                .push(paper.uid.clone());
        }

        let mut result: Vec<ResearchStream> = streams.into_values().collect();
        result.sort_by(|a, b| a.name.cmp(&b.name));
        result
    }

    fn detect_controversies(&self, streams: &[ResearchStream]) -> Vec<Controversy> {
        let mut controversies = Vec::new();
        let stream_names: Vec<&str> = streams.iter().map(|s| s.name.as_str()).collect();

        // Efficiency vs quality trade-off
        if stream_names.contains(&"检索增强型") && stream_names.contains(&"生成优化型") {
            controversies.push(Controversy::new(
                "效率 vs 质量",
                "检索增强型",
                "生成优化型",
                "检索提供外部知识，减少生成参数",
                "端到端训练，知识内化",
            ));
        }

        // Hybrid vs specialized
        if stream_names.contains(&"混合方法") && streams.len() > 1 {
            controversies.push(Controversy::new(
                "通用性 vs 专用性",
                "混合方法",
                "专用方法",
                "融合多种技术，追求通用性",
                "针对特定场景优化，追求性能",
            ));
        }

        controversies
    }

    fn build_timeline(&self, papers: &[PaperRef]) -> Vec<(i32, String)> {
        let mut timeline: Vec<(i32, String)> = papers
            .iter()
            .filter_map(|p| {
                let year = p.year.unwrap_or(2020);
                if p.title.is_empty() {
                    None
                } else {
                    let title = if p.title.len() > 50 {
                        p.title[..50].to_string()
                    } else {
                        p.title.clone()
                    };
                    Some((year, title))
                }
            })
            .collect();

        timeline.sort_by_key(|k| k.0);
        timeline.truncate(20);
        timeline
    }

    fn identify_gaps(&self, papers: &[PaperRef], streams: &[ResearchStream]) -> Vec<String> {
        let mut gaps = Vec::new();
        let stream_names: Vec<&str> = streams.iter().map(|s| s.name.as_str()).collect();

        // Check for underexplored combinations
        if stream_names.contains(&"检索增强型") && !stream_names.contains(&"生成优化型") {
            gaps.push("检索增强与生成优化的结合尚未充分探索".to_string());
        }

        // Generic gaps based on paper count
        if papers.len() < 10 {
            gaps.push("该领域论文数量较少，研究深度有限".to_string());
        }
        if streams.len() < 2 {
            gaps.push("领域方法单一，缺乏方法多样性".to_string());
        }

        // Common open problems
        gaps.extend(vec![
            "长文档场景下的检索效率问题".to_string(),
            "检索结果与生成质量的一致性保证".to_string(),
            "跨领域知识迁移的有效性评估".to_string(),
        ]);

        gaps.truncate(5);
        gaps
    }

    fn generate_sections(
        &self,
        topic: &str,
        streams: &[ResearchStream],
        controversies: &[Controversy],
        timeline: &[(i32, String)],
        open_problems: &[String],
        depth: &str,
        _sections_filter: Option<Vec<&str>>,
    ) -> Vec<ReviewSection> {
        let mut sections = Vec::new();

        // Overview
        sections.push(ReviewSection::new(
            "概述",
            &format!("本综述覆盖 {} 领域的关键研究，涉及 {} 个主要流派。", topic, streams.len()),
        ));

        // Method Streams
        if !streams.is_empty() {
            let stream_parts: Vec<String> = streams
                .iter()
                .map(|s| {
                    let methods_str = if s.methods.is_empty() {
                        "待识别".to_string()
                    } else {
                        s.methods.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                    };
                    format!(
                        "### {}\n- 论文数: {}\n- 代表方法: {}",
                        s.name,
                        s.papers.len(),
                        methods_str
                    )
                })
                .collect();
            sections.push(ReviewSection::new("方法流派", &stream_parts.join("\n")));
        }

        // Controversies
        if !controversies.is_empty() {
            let controversy_parts: Vec<String> = controversies
                .iter()
                .map(|c| {
                    format!(
                        "### {}\n- {}观点: {}\n- {}观点: {}",
                        c.topic, c.stream_a, c.position_a, c.stream_b, c.position_b
                    )
                })
                .collect();
            sections.push(ReviewSection::new("核心争论", &controversy_parts.join("\n")));
        }

        // Timeline (short mode skips this)
        if !timeline.is_empty() && depth == "full" {
            let timeline_parts: Vec<String> = timeline
                .iter()
                .take(10)
                .map(|(year, event)| format!("- {}: {}", year, event))
                .collect();
            sections.push(ReviewSection::new("演化脉络", &timeline_parts.join("\n")));
        }

        // Open Problems
        if !open_problems.is_empty() {
            let problems_parts: Vec<String> = open_problems
                .iter()
                .enumerate()
                .map(|(i, p)| format!("- {}. {}", i + 1, p))
                .collect();
            sections.push(ReviewSection::new("待解决问题", &problems_parts.join("\n")));
        }

        sections
    }

    /// Render review as Markdown.
    pub fn render_markdown(&self, review: &LiteratureReview) -> String {
        let mut lines = vec![format!("# {} 文献综述", review.topic), String::new()];

        for section in &review.sections {
            lines.push(format!("## {}", section.title));
            lines.push(section.content.clone());
            lines.push(String::new());
        }

        lines.join("\n")
    }

    /// Render review as JSON string.
    pub fn render_json(&self, review: &LiteratureReview) -> String {
        // Simple JSON serialization without external dependencies
        let mut json = String::from("{\n");
        json.push_str(&format!("  \"topic\": \"{}\",\n", escape_json(&review.topic)));
        json.push_str("  \"streams\": [\n");
        for (i, s) in review.streams.iter().enumerate() {
            let comma = if i + 1 < review.streams.len() { "," } else { "" };
            json.push_str(&format!(
                "    {{\"name\": \"{}\", \"paper_count\": {}}}{}\n",
                escape_json(&s.name),
                s.papers.len(),
                comma
            ));
        }
        json.push_str("  ],\n");
        json.push_str("  \"controversies\": [\n");
        for (i, c) in review.controversies.iter().enumerate() {
            let comma = if i + 1 < review.controversies.len() { "," } else { "" };
            json.push_str(&format!(
                "    {{\"topic\": \"{}\", \"sides\": [\"{}\", \"{}\"]}}{}\n",
                escape_json(&c.topic),
                escape_json(&c.stream_a),
                escape_json(&c.stream_b),
                comma
            ));
        }
        json.push_str("  ],\n");
        json.push_str("  \"open_problems\": [\n");
        for (i, p) in review.open_problems.iter().enumerate() {
            let comma = if i + 1 < review.open_problems.len() { "," } else { "" };
            json.push_str(&format!("    \"{}\"{}\n", escape_json(p), comma));
        }
        json.push_str("  ]\n");
        json.push('}');
        json
    }
}

impl Default for ReviewGenerator {
    fn default() -> Self {
        Self::new()
    }
}

/// Escape a string for JSON.
fn escape_json(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => result.push_str("\\\""),
            '\\' => result.push_str("\\\\"),
            '\n' => result.push_str("\\n"),
            '\r' => result.push_str("\\r"),
            '\t' => result.push_str("\\t"),
            c if c.is_control() => {
                result.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => result.push(c),
        }
    }
    result
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    fn make_paper(uid: &str, title: &str, abstract_text: &str, year: i32) -> PaperRef {
        PaperRef::new(uid, title, abstract_text, Some(year))
    }

    #[test]
    fn test_research_stream_new() {
        let stream = ResearchStream::new("Test Stream");
        assert_eq!(stream.name, "Test Stream");
        assert!(stream.papers.is_empty());
        assert!(stream.methods.is_empty());
    }

    #[test]
    fn test_controversy_new() {
        let c = Controversy::new("Topic", "A", "B", "Pos A", "Pos B");
        assert_eq!(c.topic, "Topic");
        assert_eq!(c.stream_a, "A");
        assert_eq!(c.stream_b, "B");
    }

    #[test]
    fn test_review_section_new() {
        let s = ReviewSection::new("Title", "Content");
        assert_eq!(s.title, "Title");
        assert_eq!(s.content, "Content");
        assert!(s.subsections.is_empty());
    }

    #[test]
    fn test_literature_review_new() {
        let r = LiteratureReview::new("Test Topic");
        assert_eq!(r.topic, "Test Topic");
        assert!(r.streams.is_empty());
        assert!(r.controversies.is_empty());
    }

    #[test]
    fn test_paper_ref_new() {
        let p = PaperRef::new("uid1", "Title", "Abstract", Some(2023));
        assert_eq!(p.uid, "uid1");
        assert_eq!(p.title, "Title");
        assert_eq!(p.year, Some(2023));
    }

    #[test]
    fn test_review_generator_empty() {
        let gen = ReviewGenerator::new();
        let review = gen.generate("Test Topic", 50, "full", None);
        assert_eq!(review.topic, "Test Topic");
        assert!(review.streams.is_empty());
    }

    #[test]
    fn test_review_generator_with_papers() {
        let mut gen = ReviewGenerator::new();
        gen.add_papers(vec![
            make_paper(
                "p1",
                "Retrieval Augmented Generation",
                "A method for retrieval augmented generation",
                2023,
            ),
            make_paper(
                "p2",
                "Fine-tuning LLMs",
                "Fine-tuning large language models",
                2022,
            ),
        ]);
        let review = gen.generate("RAG", 50, "full", None);
        assert_eq!(review.topic, "RAG");
        assert!(!review.streams.is_empty());
    }

    #[test]
    fn test_classify_streams_retrieval() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper(
            "p1",
            "Retrieval System",
            "A retrieval system for search",
            2023,
        )];
        let streams = gen.classify_streams(&papers);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "检索增强型");
        assert_eq!(streams[0].papers.len(), 1);
    }

    #[test]
    fn test_classify_streams_generation() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper(
            "p1",
            "LLM Generation",
            "Generation using large language models",
            2023,
        )];
        let streams = gen.classify_streams(&papers);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "生成优化型");
    }

    #[test]
    fn test_classify_streams_hybrid() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper(
            "p1",
            "Hybrid Approach",
            "A hybrid fusion method combining techniques",
            2023,
        )];
        let streams = gen.classify_streams(&papers);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "混合方法");
    }

    #[test]
    fn test_classify_streams_adaptation() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper(
            "p1",
            "Fine-tuning Method",
            "Fine-tuning and adaptation of models",
            2023,
        )];
        let streams = gen.classify_streams(&papers);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "适配优化型");
    }

    #[test]
    fn test_classify_streams_other() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper(
            "p1",
            "Some Paper",
            "Something about something",
            2023,
        )];
        let streams = gen.classify_streams(&papers);
        assert_eq!(streams.len(), 1);
        assert_eq!(streams[0].name, "其他方法");
    }

    #[test]
    fn test_detect_controversies_both_streams() {
        let gen = ReviewGenerator::new();
        let streams = vec![
            ResearchStream::new("检索增强型"),
            ResearchStream::new("生成优化型"),
        ];
        let controversies = gen.detect_controversies(&streams);
        assert_eq!(controversies.len(), 1);
        assert_eq!(controversies[0].topic, "效率 vs 质量");
    }

    #[test]
    fn test_detect_controversies_with_hybrid() {
        let gen = ReviewGenerator::new();
        let streams = vec![
            ResearchStream::new("混合方法"),
            ResearchStream::new("其他方法"),
        ];
        let controversies = gen.detect_controversies(&streams);
        assert_eq!(controversies.len(), 1);
        assert_eq!(controversies[0].topic, "通用性 vs 专用性");
    }

    #[test]
    fn test_build_timeline() {
        let gen = ReviewGenerator::new();
        let papers = vec![
            make_paper("p1", "Later Paper", "Abstract", 2024),
            make_paper("p2", "Earlier Paper", "Abstract", 2022),
        ];
        let timeline = gen.build_timeline(&papers);
        assert_eq!(timeline.len(), 2);
        assert_eq!(timeline[0].0, 2022);
        assert_eq!(timeline[1].0, 2024);
    }

    #[test]
    fn test_build_timeline_truncation() {
        let gen = ReviewGenerator::new();
        let papers: Vec<PaperRef> = (0..25)
            .map(|i| make_paper(&format!("p{}", i), &format!("Paper {}", i), "Abstract", 2020 + i as i32))
            .collect();
        let timeline = gen.build_timeline(&papers);
        assert_eq!(timeline.len(), 20); // truncated to 20
    }

    #[test]
    fn test_identify_gaps_few_papers() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper("p1", "Paper", "Abstract", 2023)];
        let streams = vec![ResearchStream::new("其他方法")];
        let gaps = gen.identify_gaps(&papers, &streams);
        assert!(gaps.iter().any(|g| g.contains("论文数量较少")));
    }

    #[test]
    fn test_identify_gaps_few_streams() {
        let gen = ReviewGenerator::new();
        let papers = vec![
            make_paper("p1", "Paper1", "Abstract", 2023),
            make_paper("p2", "Paper2", "Abstract", 2023),
        ];
        let streams = vec![ResearchStream::new("其他方法")];
        let gaps = gen.identify_gaps(&papers, &streams);
        assert!(gaps.iter().any(|g| g.contains("方法单一")));
    }

    #[test]
    fn test_identify_gaps_combination() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper("p1", "Retrieval Paper", "Abstract", 2023)];
        let streams = vec![ResearchStream::new("检索增强型")];
        let gaps = gen.identify_gaps(&papers, &streams);
        assert!(gaps
            .iter()
            .any(|g| g.contains("检索增强与生成优化的结合")));
    }

    #[test]
    fn test_generate_sections() {
        let gen = ReviewGenerator::new();
        let streams = vec![ResearchStream::new("测试流派")];
        let timeline = vec![(2023, "Test Event".to_string())];
        let sections = gen.generate_sections(
            "Test Topic",
            &streams,
            &[],
            &timeline,
            &[],
            "full",
            None,
        );
        assert_eq!(sections.len(), 3); // overview, streams, timeline
        assert_eq!(sections[0].title, "概述");
    }

    #[test]
    fn test_generate_sections_short_depth() {
        let gen = ReviewGenerator::new();
        let timeline = vec![(2023, "Test Event".to_string())];
        let sections = gen.generate_sections(
            "Test Topic",
            &[],
            &[],
            &timeline,
            &[],
            "short",
            None,
        );
        // short depth should not include timeline section
        assert!(!sections.iter().any(|s| s.title == "演化脉络"));
    }

    #[test]
    fn test_render_markdown() {
        let gen = ReviewGenerator::new();
        let mut review = LiteratureReview::new("Test");
        review
            .sections
            .push(ReviewSection::new("Section 1", "Content 1"));
        let md = gen.render_markdown(&review);
        assert!(md.contains("# Test 文献综述"));
        assert!(md.contains("## Section 1"));
        assert!(md.contains("Content 1"));
    }

    #[test]
    fn test_render_json() {
        let gen = ReviewGenerator::new();
        let mut review = LiteratureReview::new("Test");
        review
            .streams
            .push(ResearchStream::new("测试流派"));
        let json = gen.render_json(&review);
        assert!(json.contains("\"topic\": \"Test\""));
        assert!(json.contains("\"streams\""));
        assert!(json.contains("测试流派"));
    }

    #[test]
    fn test_render_json_with_controversies() {
        let gen = ReviewGenerator::new();
        let mut review = LiteratureReview::new("Test");
        review.controversies.push(Controversy::new(
            "争论点",
            "A",
            "B",
            "Position A",
            "Position B",
        ));
        let json = gen.render_json(&review);
        assert!(json.contains("争论点"));
        assert!(json.contains("A"));
        assert!(json.contains("B"));
    }

    #[test]
    fn test_render_json_with_open_problems() {
        let gen = ReviewGenerator::new();
        let mut review = LiteratureReview::new("Test");
        review
            .open_problems
            .push("问题1".to_string());
        let json = gen.render_json(&review);
        assert!(json.contains("open_problems"));
        assert!(json.contains("问题1"));
    }

    #[test]
    fn test_escape_json() {
        assert_eq!(escape_json("hello"), "hello");
        assert_eq!(escape_json("hello\"world"), "hello\\\"world");
        assert_eq!(escape_json("line\nbreak"), "line\\nbreak");
        assert_eq!(escape_json("backslash\\test"), "backslash\\\\test");
    }

    #[test]
    fn test_max_papers_limit() {
        let mut gen = ReviewGenerator::new();
        for i in 0..100 {
            gen.add_papers(vec![make_paper(
                &format!("p{}", i),
                &format!("Paper {}", i),
                "Abstract",
                2023,
            )]);
        }
        let review = gen.generate("Test", 10, "full", None);
        // Should only process up to max_papers (10)
        let total_papers: usize = review.streams.iter().map(|s| s.papers.len()).sum();
        assert!(total_papers <= 10);
    }

    #[test]
    fn test_empty_title_in_timeline() {
        let gen = ReviewGenerator::new();
        let papers = vec![make_paper("p1", "", "Abstract", 2023)];
        let timeline = gen.build_timeline(&papers);
        assert!(timeline.is_empty());
    }
}
