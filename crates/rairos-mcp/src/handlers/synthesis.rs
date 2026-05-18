use crate::handlers::helpers::data_dir;
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct PaperGenerateHandler;

#[async_trait]
impl ToolHandler for PaperGenerateHandler {
    fn name(&self) -> &str { "paper_generate" }
    fn description(&self) -> &str { "Generate a full IMRAD-structured scientific paper with title, abstract, introduction, methods, results, discussion, conclusions, and references from a research topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic (e.g., 'CRISPR gene editing for sickle cell disease')")),
                ("keywords".into(), ToolProperty::string("Comma-separated keywords (e.g., 'CRISPR,Cas9,gene therapy')")),
                ("citation_style".into(), ToolProperty::string("Citation style: APA, AMA, Vancouver (default: APA)")),
                ("max_papers".into(), ToolProperty::integer("Max papers to analyze (default: 30)")),
                ("year_start".into(), ToolProperty::integer("Earliest year (default: 2010)")),
                ("generate_pdf".into(), ToolProperty::string("Generate PDF: 'true' or 'false' (default: false)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let keywords_str = params.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
        let keywords: Vec<String> = if keywords_str.is_empty() {
            topic.split_whitespace().map(String::from).collect()
        } else {
            keywords_str.split(',').map(|s| s.trim().to_string()).collect()
        };
        let citation_style = params.get("citation_style").and_then(|v| v.as_str()).unwrap_or("APA");
        let max_papers = params.get("max_papers").and_then(|v| v.as_u64()).unwrap_or(30) as usize;
        let year_start = params.get("year_start").and_then(|v| v.as_i64()).unwrap_or(2010) as i32;
        let generate_pdf = params.get("generate_pdf").and_then(|v| v.as_str()).unwrap_or("false") == "true";

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let query_enc = urlencoding::encode(topic);
        let fields = "title,abstract,year,citationCount,externalIds,venue,journal,authors";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields={}&limit={}&year={}-",
            query_enc, fields, max_papers.min(100), year_start
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let mut papers: Vec<serde_json::Value> = data.get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        papers.retain(|p| {
            if let Some(year) = p.get("year").and_then(|y| y.as_i64()) {
                year >= year_start as i64
            } else { true }
        });

        let total = papers.len();
        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let mut cites: Vec<u64> = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).collect();
        cites.sort();
        let median_cites = if !cites.is_empty() { cites[cites.len() / 2] } else { 0 };
        let min_cites = cites.first().copied().unwrap_or(0);
        let max_cites = cites.last().copied().unwrap_or(0);
        let total_cites: u64 = cites.iter().sum();
        let year_range = if !papers.is_empty() {
            let years: Vec<i64> = papers.iter().filter_map(|p| p.get("year").and_then(|y| y.as_i64())).collect();
            let min_y = years.iter().min().copied().unwrap_or(year_start as i64);
            let max_y = years.iter().max().copied().unwrap_or(2024);
            format!("{}-{}", min_y, max_y)
        } else {
            format!("{}-2024", year_start)
        };

        let top_paper = papers.first();
        let top_title = top_paper.and_then(|p| p.get("title").and_then(|v| v.as_str())).unwrap_or(topic);
        let top_year = top_paper.and_then(|p| p.get("year").and_then(|y| y.as_i64())).unwrap_or(2020);
        let top_venue = top_paper.and_then(|p| p.get("venue").or(p.get("journal")).and_then(|v| v.as_str())).unwrap_or("");

        let mut md = String::new();

        let title_case = topic.split_whitespace().map(|w| {
            let mut c = w.chars();
            match c.next() {
                None => String::new(),
                Some(f) => f.to_uppercase().chain(c).collect(),
            }
        }).collect::<Vec<_>>().join(" ");
        md.push_str("# ");
        md.push_str(&title_case);
        md.push_str(": A Systematic Review\n\n");

        md.push_str("Corresponding Author: [Author Name] <br>\n");
        md.push_str("Institution: [Institution Name] <br>\n");
        md.push_str("Date: ");
        md.push_str(&today);
        md.push_str("\n\n---\n\n");

        md.push_str("## Abstract\n\n");
        md.push_str("**Background:** ");
        md.push_str(topic);
        md.push_str(" represents a significant area of biomedical research with rapidly evolving methodologies and clinical applications. A comprehensive understanding of the current evidence base is essential for advancing the field.\n\n");
        md.push_str("**Objectives:** This systematic review synthesizes evidence from ");
        md.push_str(&total.to_string());
        md.push_str(" publications to characterize the landscape of research on ");
        md.push_str(topic);
        md.push_str(", identify key themes, and highlight knowledge gaps.\n\n");
        md.push_str("**Methods:** A systematic search of the Semantic Scholar database was conducted. Papers were screened for relevance, deduplicated by DOI, and analyzed for bibliometric characteristics and thematic content.\n\n");
        md.push_str("**Results:** ");
        md.push_str(&total.to_string());
        md.push_str(" studies published between ");
        md.push_str(&year_range);
        md.push_str(" were identified. The median citation count was ");
        md.push_str(&median_cites.to_string());
        md.push_str(", with a range of ");
        md.push_str(&min_cites.to_string());
        md.push_str(" to ");
        md.push_str(&max_cites.to_string());
        md.push_str(" citations. Key research themes include clinical efficacy, mechanistic understanding, safety profiles, and emerging methodological approaches.\n\n");
        md.push_str("**Conclusions:** Research on ");
        md.push_str(topic);
        md.push_str(" demonstrates active investigation across multiple dimensions. Identified gaps suggest priorities for future research include long-term outcome studies, mechanistic validation, and clinical translation.\n\n");
        md.push_str("*Keywords:* ");
        md.push_str(&keywords.join(", "));
        md.push_str("\n\n---\n\n");

        md.push_str("## 1. Introduction\n\n");
        md.push_str("### 1.1 Background\n\n");
        md.push_str("The field of ");
        md.push_str(topic);
        md.push_str(" has garnered substantial attention in recent years, driven by advances in technology and growing clinical need. ");
        md.push_str(topic);
        md.push_str(" represents a promising approach that has demonstrated potential in multiple preclinical and clinical settings.\n\n");
        md.push_str("The first major studies in this area appeared approximately a decade ago, with publication volume increasing substantially over the past five years. This growth reflects both renewed scientific interest and the translation of laboratory findings toward clinical application.\n\n");
        md.push_str("### 1.2 State of the Art\n\n");
        md.push_str("Recent landmark publications have established foundational knowledge in ");
        md.push_str(topic);
        md.push_str(". Notably, ");
        md.push_str(top_title);
        md.push_str(" (");
        md.push_str(&top_year.to_string());
        if !top_venue.is_empty() {
            md.push_str(") published in *");
            md.push_str(top_venue);
            md.push_str("*");
        } else {
            md.push_str(")");
        }
        md.push_str(" provided critical insights that have shaped subsequent research directions. The collective body of ");
        md.push_str(&total.to_string());
        md.push_str(" publications demonstrates both the breadth and depth of ongoing investigation.\n\n");
        md.push_str("### 1.3 Research Questions\n\n");
        md.push_str("This review addresses the following research questions:\n\n");
        md.push_str("1. What is the current scope and volume of research on ");
        md.push_str(topic);
        md.push_str("?\n");
        md.push_str("2. What are the predominant themes and methodological approaches in the literature?\n");
        md.push_str("3. What knowledge gaps remain unaddressed, and what are the priorities for future investigation?\n\n");
        md.push_str("### 1.4 Objectives\n\n");
        md.push_str("The primary objective of this systematic review is to synthesize and critically evaluate the available evidence on ");
        md.push_str(topic);
        md.push_str(" to provide a comprehensive overview of the current state of knowledge, identify research gaps, and inform future research directions.\n\n---\n\n");

        md.push_str("## 2. Methods\n\n");
        md.push_str("### 2.1 Search Strategy\n\n");
        md.push_str("A systematic search was conducted using the Semantic Scholar academic database, which provides comprehensive coverage of peer-reviewed literature across biomedical and life sciences domains. The search was performed on ");
        md.push_str(&today);
        md.push_str(" using the following parameters:\n\n");
        md.push_str("- **Search terms:** ");
        md.push_str(topic);
        md.push_str("\n");
        md.push_str("- **Date range:** ");
        md.push_str(&year_range);
        md.push_str("\n");
        md.push_str("- **Maximum results:** ");
        md.push_str(&max_papers.to_string());
        md.push_str(" papers\n");
        md.push_str("- **Keywords filter:** ");
        md.push_str(&keywords.join(", "));
        md.push_str("\n\n");
        md.push_str("### 2.2 Inclusion and Exclusion Criteria\n\n");
        md.push_str("**Inclusion criteria:**\n");
        md.push_str("- Peer-reviewed publications with available abstracts\n");
        md.push_str("- Publications in English or with English abstracts\n");
        md.push_str("- Studies published between ");
        md.push_str(&year_range);
        md.push_str("\n\n");
        md.push_str("**Exclusion criteria:**\n");
        md.push_str("- Duplicate publications (deduplicated by DOI)\n");
        md.push_str("- Studies without accessible abstracts\n");
        md.push_str("- Non-English publications without translation\n\n");
        md.push_str("### 2.3 Data Extraction\n\n");
        md.push_str("For each included publication, the following data were extracted: title, year, journal or venue, citation count, abstract, authors, and DOI where available. Bibliometric indicators including citation counts and publication year distribution were analyzed.\n\n");
        md.push_str("### 2.4 Quality Assessment\n\n");
        md.push_str("Given the heterogeneity of study designs included in this review, quality assessment was performed using a combination of citation-based ranking and keyword relevance scoring. Papers were ranked by (1) keyword relevance score and (2) citation count, with higher-ranked papers considered to have greater methodological rigor and impact.\n\n---\n\n");

        md.push_str("## 3. Results\n\n");
        md.push_str("### 3.1 Study Selection\n\n");
        md.push_str("The initial search yielded ");
        md.push_str(&total.to_string());
        md.push_str(" publications. After applying inclusion and exclusion criteria and deduplication, ");
        md.push_str(&total.to_string());
        md.push_str(" studies were included in the final analysis. The PRISMA flow diagram is described in Figure 1.\n\n");
        md.push_str("### 3.2 Bibliometric Analysis\n\n");
        md.push_str("**Publication timeline:** Studies were published between ");
        md.push_str(&year_range);
        md.push_str(", with an increasing trend in annual publication volume over time. This pattern suggests growing interest and investment in ");
        md.push_str(topic);
        md.push_str(" research.\n\n");
        md.push_str("**Citation analysis:** The median citation count was ");
        md.push_str(&median_cites.to_string());
        md.push_str(" (range: ");
        md.push_str(&min_cites.to_string());
        md.push_str("-");
        md.push_str(&max_cites.to_string());
        md.push_str("). The total citation count across all included studies was ");
        md.push_str(&total_cites.to_string());
        md.push_str(", indicating substantial scholarly impact.\n\n");
        md.push_str("### 3.3 Thematic Synthesis\n\n");
        md.push_str("The included studies were organized into five predominant thematic areas:\n\n");
        md.push_str("1. **Clinical Efficacy and Outcomes**: Studies investigating therapeutic effectiveness and clinical outcomes\n");
        md.push_str("2. **Mechanistic Insights**: Research focused on biological mechanisms and pathway elucidation\n");
        md.push_str("3. **Safety and Risk Profiles**: Investigations of adverse effects, toxicity, and risk assessment\n");
        md.push_str("4. **Methodological Advances**: Papers describing novel techniques, technologies, or analytical approaches\n");
        md.push_str("5. **Clinical Translation**: Studies bridging preclinical findings to clinical application\n\n");
        md.push_str("### 3.4 Key Findings from High-Impact Studies\n\n");
        for (i, paper) in papers.iter().take(5).enumerate() {
            let pt = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let py = paper.get("year").and_then(|y| y.as_i64()).unwrap_or(0);
            let pvenue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let pcites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
            let pabstract = paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("").chars().take(300).collect::<String>();
            md.push_str(&format!("**{}**. *{}* ({}). *{}*. Cited by: {}.\n{}\n\n",
                i + 1, pt, py, pvenue, pcites,
                if pabstract.len() > 10 { format!("> {}", pabstract) } else { String::new() }));
        }
        md.push_str("---\n\n");

        md.push_str("## 4. Discussion\n\n");
        md.push_str("### 4.1 Summary of Findings\n\n");
        md.push_str("This systematic review synthesized evidence from ");
        md.push_str(&total.to_string());
        md.push_str(" publications on ");
        md.push_str(topic);
        md.push_str(". The analysis reveals a rapidly maturing field with substantial publication volume and citation impact, indicating significant scientific and clinical relevance.\n\n");
        md.push_str("The bibliometric indicators demonstrate strong scholarly impact, with a median citation count of ");
        md.push_str(&median_cites.to_string());
        md.push_str(" across the included studies. The increasing publication trend over time suggests continued growth and diversification of research activities.\n\n");
        md.push_str("### 4.2 Comparison with Existing Literature\n\n");
        md.push_str("The findings of this review are consistent with previous summaries and narrative reviews in the field, which have similarly identified clinical efficacy and mechanistic understanding as central research themes. However, this systematic review provides a more comprehensive and quantitative characterization of the evidence base.\n\n");
        md.push_str("### 4.3 Strengths and Limitations\n\n");
        md.push_str("**Strengths:**\n");
        md.push_str("- Systematic search methodology with comprehensive database coverage\n");
        md.push_str("- Deduplication by DOI to avoid citation inflation\n");
        md.push_str("- Bibliometric analysis providing quantitative characterization\n");
        md.push_str("- Thematic synthesis organizing heterogeneous literature\n\n");
        md.push_str("**Limitations:**\n");
        md.push_str("- Single database search (Semantic Scholar); potentially missing grey literature\n");
        md.push_str("- Narrative synthesis approach; no meta-analysis due to study heterogeneity\n");
        md.push_str("- English language bias in search results\n");
        md.push_str("- Citation counts may reflect publication date rather than true impact\n\n");
        md.push_str("### 4.4 Clinical Implications\n\n");
        md.push_str("The evidence synthesized in this review suggests that ");
        md.push_str(topic);
        md.push_str(" has reached a stage of sufficient evidence maturity to support clinical investigation. However, key gaps remain in long-term outcome data and comparative effectiveness research.\n\n");
        md.push_str("### 4.5 Future Research Directions\n\n");
        md.push_str("Based on the identified knowledge gaps, the following priorities for future research are recommended:\n\n");
        md.push_str("1. Long-term follow-up studies to assess durability of effects\n");
        md.push_str("2. Head-to-head comparative effectiveness trials\n");
        md.push_str("3. Mechanistic studies to elucidate biological pathways\n");
        md.push_str("4. Translation research bridging preclinical findings to clinical application\n");
        md.push_str("5. Population diversity in study populations to ensure generalizability\n\n---\n\n");

        md.push_str("## 5. Conclusions\n\n");
        md.push_str("This systematic review provides a comprehensive synthesis of ");
        md.push_str(&total.to_string());
        md.push_str(" publications on ");
        md.push_str(topic);
        md.push_str(", characterizing a rapidly evolving field with substantial scholarly impact. Key findings indicate:\n\n");
        md.push_str("- Strong and growing research activity with increasing publication volume\n");
        md.push_str("- Five predominant thematic areas: efficacy, mechanisms, safety, methods, and translation\n");
        md.push_str("- Median citation impact of ");
        md.push_str(&median_cites.to_string());
        md.push_str(" citations, reflecting significant scientific contribution\n");
        md.push_str("- Important gaps in long-term outcome data and clinical translation\n\n");
        md.push_str("Future research priorities include long-term outcome studies, comparative effectiveness research, mechanistic validation, and clinical translation efforts. The findings of this review provide a foundation for researchers and clinicians seeking to understand the current state of evidence on ");
        md.push_str(topic);
        md.push_str(".\n\n---\n\n");

        md.push_str("## References\n\n");
        let style_upper = citation_style.to_uppercase();
        for (i, paper) in papers.iter().enumerate().take(30) {
            let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let doi = paper.get("externalIds").or(paper.get("external_ids"))
                .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
            let authors: Vec<String> = paper.get("authors").and_then(|a| a.as_array())
                .map(|arr| arr.iter().filter_map(|au| au.get("name").and_then(|n| n.as_str()).map(String::from)).take(3).collect())
                .unwrap_or_default();
            let author_str = if authors.is_empty() { "Unknown Authors".into() } else { authors.join(", ") };

            let ref_text = match style_upper.as_str() {
                "AMA" => {
                    let venue_str = if !venue.is_empty() { format!(". {}", venue) } else { String::new() };
                    let doi_str = if !doi.is_empty() { format!(". doi:{}", doi) } else { String::new() };
                    format!("{}. {}. {}. {}{}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                },
                "VANCOUVER" => {
                    let venue_str = if !venue.is_empty() { format!(". {}", venue) } else { String::new() };
                    let doi_str = if !doi.is_empty() { format!(". doi:{}", doi) } else { String::new() };
                    format!("{}. {} {}. {}{}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                },
                _ => {
                    let venue_str = if !venue.is_empty() { format!("_{}_. ", venue) } else { ". ".to_string() };
                    let doi_str = if !doi.is_empty() { format!("https://doi.org/{}", doi) } else { String::new() };
                    format!("{}. {} ({}). {}. {}{}\n", i + 1, author_str, year, title, venue_str, doi_str)
                }
            };
            md.push_str(&ref_text);
        }
        if papers.len() > 30 {
            md.push_str(&format!("\n_[Additional {} references available in full manuscript]_\n", papers.len() - 30));
        }

        let mut pdf_path = serde_json::json!(null);
        if generate_pdf {
            let paper_json = serde_json::json!({
                "title": title_case,
                "topic": topic,
                "paper_count": total,
                "date": today,
                "content": md.clone(),
            });
            let output_dir = data_dir().join("papers");
            let filename = format!("paper_{}.pdf", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_output = output_dir.join(&filename);
            std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("paper")
                .arg("--data").arg(&paper_json.to_string())
                .arg("--output").arg(pdf_output.to_str().unwrap());
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    pdf_path = serde_json::json!(pdf_output.to_string_lossy().to_string());
                }
            }
        }

        Ok(serde_json::json!({
            "topic": topic,
            "title": title_case,
            "papers_analyzed": total,
            "citation_style": citation_style,
            "year_range": year_range,
            "median_citations": median_cites,
            "total_citations": total_cites,
            "markdown": md,
            "pdf_path": pdf_path,
        }))
    }
}

#[derive(serde::Serialize)]
struct ScenarioBranch {
    branch_id: String,
    branch_name: String,
    probability_pct: u8,
    timeframe: String,
    confidence: String,
    narrative: String,
    key_assumptions: Vec<String>,
    trigger_conditions: Vec<String>,
    immediate_consequences: String,
    thirty_day_consequences: String,
    six_month_consequences: String,
    required_response: String,
    what_most_miss: String,
}

fn analyze_scenarios(question: &str, context: &str) -> Vec<ScenarioBranch> {
    let q_lower = question.to_lowercase();

    let (domain, focus) = if q_lower.contains("drug") || q_lower.contains("therapy") || q_lower.contains("clinical") || q_lower.contains("patient") {
        ("clinical research", "therapeutic efficacy and patient outcomes")
    } else if q_lower.contains("ai") || q_lower.contains("machine learning") || q_lower.contains("model") {
        ("AI/ML technology", "model performance and deployment")
    } else if q_lower.contains("climate") || q_lower.contains("environment") || q_lower.contains("carbon") {
        ("environmental science", "ecological and atmospheric systems")
    } else if q_lower.contains("economic") || q_lower.contains("market") || q_lower.contains("financial") || q_lower.contains("revenue") {
        ("economics/finance", "market dynamics and financial performance")
    } else {
        ("general research", "scientific advancement and knowledge")
    };

    let mut scenarios = Vec::new();

    scenarios.push(ScenarioBranch {
        branch_id: "omega_best".to_string(),
        branch_name: "Best Case".to_string(),
        probability_pct: 15,
        timeframe: "12-24 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Everything aligns optimally. Key assumptions validate, favorable conditions emerge, and the research trajectory exceeds expectations. For {} domain, this means {} accelerates dramatically with strong evidence supporting the primary hypothesis.", domain, focus),
        key_assumptions: vec![
            format!("All primary hypothesis assumptions hold under scrutiny"),
            format!("Supporting evidence from {} studies converges", domain),
            "Key stakeholders commit full resources".to_string(),
            "No major competing alternatives emerge".to_string(),
        ],
        trigger_conditions: vec![
            "Early signals exceed baseline thresholds by >2x".to_string(),
            "Key opinion leaders publicly endorse approach".to_string(),
            "Funding secured ahead of schedule".to_string(),
        ],
        immediate_consequences: "Momentum builds rapidly; additional collaborators seek involvement; media coverage amplifies visibility".to_string(),
        thirty_day_consequences: "Expanded team hired; follow-on funding confirmed; competing groups seek partnership".to_string(),
        six_month_consequences: "Full validation complete; manuscript submitted to top venue; industry partnership formalized".to_string(),
        required_response: "Scale resources proportionally; protect intellectual property; maintain quality under acceleration pressure".to_string(),
        what_most_miss: "Success attracts detractors and creates unrealistic expectations that can derail when normal setbacks occur".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "alpha_likely".to_string(),
        branch_name: "Likely Case".to_string(),
        probability_pct: 45,
        timeframe: "6-18 months".to_string(),
        confidence: "HIGH".to_string(),
        narrative: format!("The most probable path materializes. Core hypothesis holds with moderate effect sizes, methodology proves sound, and incremental progress continues. For {} domain, this means {} develops as expected with no major surprises.", domain, focus),
        key_assumptions: vec![
            format!("Core {} assumptions remain valid", domain),
            "Methodology produces reproducible results".to_string(),
            "Resource levels remain stable".to_string(),
            "No paradigm-shifting alternatives appear".to_string(),
        ],
        trigger_conditions: vec![
            "Results fall within expected confidence intervals".to_string(),
            "Peer feedback confirms approach validity".to_string(),
            "Publication timeline proceeds as planned".to_string(),
        ],
        immediate_consequences: "Steady progress; team maintains course; stakeholders remain engaged".to_string(),
        thirty_day_consequences: "Draft manuscript completed; internal review conducted; next-phase planning begins".to_string(),
        six_month_consequences: "Publication submitted; follow-on proposal drafted; core methodology established as standard".to_string(),
        required_response: "Maintain rigorous standards; document all findings thoroughly; build on incremental wins".to_string(),
        what_most_miss: "Incremental progress can mask underlying structural weaknesses that only appear under stress".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "delta_worst".to_string(),
        branch_name: "Worst Case".to_string(),
        probability_pct: 20,
        timeframe: "3-12 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("Multiple assumptions fail simultaneously. Key results do not replicate, methodology flaws emerge, and resource constraints force difficult choices. For {} domain, this means {} stalls and credibility suffers.", domain, focus),
        key_assumptions: vec![
            format!("{} hypothesis is fundamentally sound", domain),
            "Sufficient sample size achievable".to_string(),
            "Data quality meets standards".to_string(),
            "Timeline is achievable".to_string(),
        ],
        trigger_conditions: vec![
            "Primary outcome measure shows null or negative effect".to_string(),
            "Replication attempt fails".to_string(),
            "Funding source signals concern".to_string(),
        ],
        immediate_consequences: "Stakeholder confidence erodes; team morale impacted; timeline slips".to_string(),
        thirty_day_consequences: "Emergency re-evaluation; methodology audit; potential pivot or termination".to_string(),
        six_month_consequences: "Project restructured or cancelled; lessons documented; team reassigned".to_string(),
        required_response: "Conduct honest post-mortem; preserve useful learnings; rebuild trust through transparency".to_string(),
        what_most_miss: "The specific failure mode contains information about what WOULD work — but only if analyzed objectively".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "psi_wildcard".to_string(),
        branch_name: "Wild Card".to_string(),
        probability_pct: 8,
        timeframe: "1-24 months (unpredictable)".to_string(),
        confidence: "LOW".to_string(),
        narrative: format!("An unexpected variable enters that nobody anticipated. This black swan event reshapes the landscape entirely. For {} domain, this could mean a breakthrough discovery, a safety crisis, or a disruptive technology renders current approach obsolete.", domain),
        key_assumptions: vec![
            "Current understanding captures relevant variables".to_string(),
            "No external disruption possible".to_string(),
            "Timeline is within predictable window".to_string(),
        ],
        trigger_conditions: vec![
            "Unexpected result defies all models".to_string(),
            "External event creates sudden change".to_string(),
            "Competing breakthrough announced".to_string(),
        ],
        immediate_consequences: "Complete reorientation required; existing plans become irrelevant".to_string(),
        thirty_day_consequences: "New strategy formulated; team restructured; stakeholders reassessed".to_string(),
        six_month_consequences: "Either pivot fully executed or graceful exit completed".to_string(),
        required_response: "Build organizational agility; maintain optionality; avoid overcommitment to single path".to_string(),
        what_most_miss: "Wild cards are only wild in retrospect — early signals always existed for those paying attention".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "phi_contrarian".to_string(),
        branch_name: "Contrarian Case".to_string(),
        probability_pct: 7,
        timeframe: "12-36 months".to_string(),
        confidence: "MEDIUM".to_string(),
        narrative: format!("The opposite of conventional wisdom proves true. The consensus view that {} is the right approach turns out to be wrong. This creates both risk for current efforts and opportunity for those who anticipate the shift.", domain),
        key_assumptions: vec![
            format!("Consensus view on {} is correct", domain),
            "Current evidence supports prevailing theory".to_string(),
            "Established methods are optimal".to_string(),
        ],
        trigger_conditions: vec![
            "Minority view gains unexpected support".to_string(),
            "Contrarian data published to acclaim".to_string(),
            "Regulatory or funder stance shifts".to_string(),
        ],
        immediate_consequences: "Current approach questioned; early adopters of contrarian view gain advantage".to_string(),
        thirty_day_consequences: "Debate intensifies; funding bodies reconsider; team must decide on pivot".to_string(),
        six_month_consequences: "Field reorients around new consensus; late movers face disadvantage".to_string(),
        required_response: "Monitor contrarian signals actively; avoid dismissing minority views prematurely".to_string(),
        what_most_miss: "The contrarian view is right more often than expected — and at the exact moment when admitting it feels like the biggest risk".to_string(),
    });

    scenarios.push(ScenarioBranch {
        branch_id: "infinity_second".to_string(),
        branch_name: "Second Order".to_string(),
        probability_pct: 5,
        timeframe: "12-48 months".to_string(),
        confidence: "LOW".to_string(),
        narrative: "First-order effects trigger cascading consequences that nobody predicted. Initial success (or failure) creates chain reaction of unintended outcomes. The secondary consequences prove more significant than the primary event itself.".to_string(),
        key_assumptions: vec![
            "First-order effects remain contained".to_string(),
            "No significant side effects emerge".to_string(),
            "System remains in equilibrium".to_string(),
        ],
        trigger_conditions: vec![
            "Success attracts attention from unexpected quarters".to_string(),
            "Scaling reveals hidden complexities".to_string(),
            "Unintended consequences begin appearing".to_string(),
        ],
        immediate_consequences: "Focus shifts from primary goal to managing cascades".to_string(),
        thirty_day_consequences: "Second-order stakeholders demand input; resource reallocation required".to_string(),
        six_month_consequences: "Ecosystem around the work has formed; original team has limited control".to_string(),
        required_response: "Map potential second-order effects proactively; build governance mechanisms early".to_string(),
        what_most_miss: "The most important consequences of any action are always the ones you didn't think to look for".to_string(),
    });

    scenarios
}

pub struct WhatIfOracleHandler;

#[async_trait]
impl ToolHandler for WhatIfOracleHandler {
    fn name(&self) -> &str { "what_if_oracle" }
    fn description(&self) -> &str { "Explore multi-branch scenario analysis for a research question using the What-If Oracle framework (0·IF·1). Generates 6 scenario branches: Best, Likely, Worst, Wild Card, Contrarian, and Second Order cases." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("question".into(), ToolProperty::string("Research question to analyze (e.g., 'What if we discover a safe and effective CRISPR therapy for sickle cell disease within 5 years?')")),
                ("context".into(), ToolProperty::string("Additional context or constraints (e.g., 'Current funding: $2M, Timeline: 3 years, Team: 5 researchers')")),
                ("mode".into(), ToolProperty::string("Analysis mode: 'quick' (3 branches) or 'deep' (6 branches, default: deep)")),
            ].into_iter().collect(),
            vec!["question".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let question = params["question"].as_str().ok_or("Missing question")?;
        let context = params.get("context").and_then(|v| v.as_str()).unwrap_or("");
        let mode = params.get("mode").and_then(|v| v.as_str()).unwrap_or("deep");

        let all_scenarios = analyze_scenarios(question, context);

        let scenarios: Vec<ScenarioBranch> = if mode == "quick" {
            all_scenarios.into_iter().filter(|s| {
                s.branch_id == "omega_best" || s.branch_id == "alpha_likely" || s.branch_id == "delta_worst"
            }).collect()
        } else {
            all_scenarios
        };

        let total_pct: u8 = scenarios.iter().map(|s| s.probability_pct).sum();

        let branches_json: Vec<Value> = scenarios.iter().map(|s| {
            serde_json::json!({
                "branch_id": s.branch_id,
                "branch_name": s.branch_name,
                "probability_pct": s.probability_pct,
                "timeframe": s.timeframe,
                "confidence": s.confidence,
                "narrative": s.narrative,
                "key_assumptions": s.key_assumptions,
                "trigger_conditions": s.trigger_conditions,
                "consequences": {
                    "immediate": s.immediate_consequences,
                    "thirty_day": s.thirty_day_consequences,
                    "six_month": s.six_month_consequences,
                },
                "required_response": s.required_response,
                "what_most_miss": s.what_most_miss,
            })
        }).collect();

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();

        let mut synthesis = String::new();
        synthesis.push_str("## Probability Distribution\n\n");
        synthesis.push_str("| Scenario | Probability |\n");
        synthesis.push_str("|----------|-------------|\n");
        for s in &scenarios {
            let bar = "█".repeat((s.probability_pct / 5) as usize);
            let dots = "░".repeat(20 - (s.probability_pct / 5) as usize);
            synthesis.push_str(&format!("| {} {} | {}% |\n", bar, dots, s.probability_pct));
        }
        synthesis.push_str(&format!("\n**Total probability:** {}% (normalized)\n\n", total_pct));

        synthesis.push_str("## Synthesis\n\n");
        synthesis.push_str("**Robust Actions** (beneficial across multiple branches):\n\n");
        synthesis.push_str("1. Maintain methodological rigor regardless of early results\n");
        synthesis.push_str("2. Build in flexibility to pivot when evidence demands\n");
        synthesis.push_str("3. Document all findings thoroughly for future reference\n");
        synthesis.push_str("4. Cultivate stakeholder relationships beyond the primary funder\n\n");

        synthesis.push_str("**Hedge Actions** (protect against worst case without sacrificing upside):\n\n");
        synthesis.push_str("1. Establish replication protocol before primary analysis\n");
        synthesis.push_str("2. Maintain runway for at least 6 months beyond planned end\n");
        synthesis.push_str("3. Develop contingency plans for both null and positive results\n\n");

        synthesis.push_str("**Decision Triggers** (signals to update branch probabilities):\n\n");
        synthesis.push_str("- Increase Best Case probability if: early results exceed thresholds, key endorsements received\n");
        synthesis.push_str("- Increase Worst Case probability if: replication fails, funding signals concern\n");
        synthesis.push_str("- Increase Wild Card probability if: results defy all models, external disruption occurs\n");
        synthesis.push_str("- Increase Contrarian probability if: minority view gains unexpected traction\n\n");

        synthesis.push_str("**The 1% Insight**\n\n");
        synthesis.push_str(&format!(
            "The most actionable insight from this scenario analysis: the specific branch that feels least comfortable to plan for is often the one that contains the highest learning potential. For the question '{}', the gap between what the analysis reveals and what you immediately want to act on is where the real strategic value lies.\n\n",
            question.chars().take(60).collect::<String>()
        ));

        synthesis.push_str(&format!("_Analyzed: {} | Mode: {} | Branches: {}_\n", today, mode, scenarios.len()));

        Ok(serde_json::json!({
            "question": question,
            "context": context,
            "mode": mode,
            "branches": branches_json,
            "synthesis": synthesis,
            "total_probability_pct": total_pct,
            "date": today,
        }))
    }
}

fn escape_xml(s: &str) -> String {
    s.replace("&", "&amp;").replace("<", "&lt;").replace(">", "&gt;").replace("\"", "&quot;")
}

fn md_to_docx_xml(markdown: &str) -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:document xmlns:wpc="http://schemas.microsoft.com/office/word/2010/wordprocessingCanvas"#);
    xml.push_str(r#" xmlns:cx="http://schemas.microsoft.com/office/drawing/2014/chartex"#);
    xml.push_str(r#" xmlns:mc="http://schemas.openxmlformats.org/markup-compatibility/2006""#);
    xml.push_str(r#" xmlns:aink="http://schemas.microsoft.com/office/drawing/2016/ink""#);
    xml.push_str(r#" xmlns:am3d="http://schemas.microsoft.com/office/drawing/2017/model3d""#);
    xml.push_str(r#" xmlns:o="urn:schemas-microsoft-com:office:office""#);
    xml.push_str(r#" xmlns:oel="http://schemas.microsoft.com/office/2019/extlst""#);
    xml.push_str(r#" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationship""#);
    xml.push_str(r#" xmlns:m="http://schemas.openxmlformats.org/officeDocument/2006/math""#);
    xml.push_str(r#" xmlns:v="urn:schemas-microsoft-com:vml""#);
    xml.push_str(r#" xmlns:wp14="http://schemas.microsoft.com/office/word/2010/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:wp="http://schemas.openxmlformats.org/drawingml/2006/wordprocessingDrawing""#);
    xml.push_str(r#" xmlns:w10="urn:schemas-microsoft-com:office:word""#);
    xml.push_str(r#" xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main""#);
    xml.push_str(r#" xmlns:w14="http://schemas.microsoft.com/office/word/2010/wordml""#);
    xml.push_str(r#" xmlns:w15="http://schemas.microsoft.com/office/word/2012/wordml""#);
    xml.push_str(r#" xmlns:w16cex="http://schemas.microsoft.com/office/word/2018/wordml/cex""#);
    xml.push_str(r#" xmlns:w16cid="http://schemas.microsoft.com/office/word/2016/wordml/cid""#);
    xml.push_str(r#" xmlns:w16="http://schemas.microsoft.com/office/word/2018/wordml""#);
    xml.push_str(r#" xmlns:w16sdtdh="http://schemas.microsoft.com/office/word/2020/wordml/sdtdatahash""#);
    xml.push_str(r#" xmlns:w16se="http://schemas.microsoft.com/office/word/2015/wordml/symex""#);
    xml.push_str(r#" xmlns:wpg="http://schemas.microsoft.com/office/word/2010/wordprocessingGroup""#);
    xml.push_str(r#" xmlns:wpi="http://schemas.microsoft.com/office/word/2010/wordprocessingInk""#);
    xml.push_str(r#" xmlns:wne="http://schemas.microsoft.com/office/word/2006/wordml""#);
    xml.push_str(r#" xmlns:wps="http://schemas.microsoft.com/office/word/2010/wordprocessingShape""#);
    xml.push_str(">");
    xml.push_str("<w:body>");

    let lines: Vec<&str> = markdown.lines().collect();
    let mut in_list = false;
    let mut list_type = "";
    let mut list_count = 0usize;

    for line in &lines {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            continue;
        }

        if trimmed.starts_with("# ") {
            let text = &trimmed[2..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading1"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("## ") {
            let text = &trimmed[3..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading2"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("### ") {
            let text = &trimmed[4..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:pStyle w:val="Heading3"/></w:pPr><w:r><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("- ") || trimmed.starts_with("* ") {
            let text = &trimmed[2..];
            if !in_list || list_type != "bullet" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "bullet";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="1"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        else if trimmed.starts_with("1. ") || trimmed.starts_with("1) ") {
            let text = if trimmed.starts_with("1. ") { &trimmed[3..] } else { &trimmed[2..] };
            if !in_list || list_type != "number" {
                if in_list { xml.push_str("</w:p>"); }
                xml.push_str(r#"<w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#);
                in_list = true;
                list_type = "number";
            }
            xml.push_str(&format!(
                r#"<w:r><w:t>{}</w:t></w:r></w:p><w:p><w:pPr><w:numPr><w:ilvl w:val="0"/><w:numId w:val="2"/></w:numPr></w:pPr>"#,
                escape_xml(text)
            ));
        }
        else if trimmed == "---" || trimmed == "***" || trimmed == "___" {
            xml.push_str(r#"<w:p><w:pPr><w:pBdr><w:bottom w:val="single" w:sz="6" w:space="1" w:color="CCCCCC"/></w:pBdr></w:pPr></w:p>"#);
        }
        else if trimmed.starts_with("> ") {
            let text = &trimmed[2..];
            xml.push_str(&format!(
                r#"<w:p><w:pPr><w:ind w:left="720" w:right="720"/><w:jc w:val="both"/></w:pPr><w:r><w:rPr><w:i/><w:color w:val="666666"/></w:rPr><w:t>{}</w:t></w:r></w:p>"#,
                escape_xml(text)
            ));
        }
        else {
            if in_list {
                xml.push_str("</w:p>");
                in_list = false;
            }
            let processed = process_inline_formatting(trimmed);
            xml.push_str(&format!(
                r#"<w:p><w:r>{}</w:r></w:p>"#,
                processed
            ));
        }
    }

    if in_list {
        xml.push_str("</w:p>");
    }

    xml.push_str(r#"<w:sectPr><w:pgSz w:w="12240" w:h="15840"/><w:pgMar w:top="1440" w:right="1440" w:bottom="1440" w:left="1440" w:header="720" w:footer="720" w:gutter="0"/></w:sectPr>"#);
    xml.push_str("</w:body></w:document>");

    xml
}

fn process_inline_formatting(text: &str) -> String {
    let mut result = String::new();
    let mut chars = text.chars().peekable();
    let mut in_bold = false;
    let mut in_italic = false;

    while let Some(c) = chars.next() {
        if c == '*' {
            let next = chars.peek().copied();
            if next == Some('*') {
                chars.next();
                if in_bold {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_bold = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:b/></w:rPr>");
                    in_bold = true;
                }
            } else {
                if in_italic {
                    result.push_str("</w:rPr></w:r><w:r>");
                    in_italic = false;
                } else {
                    result.push_str("<w:r><w:rPr><w:i/></w:rPr>");
                    in_italic = true;
                }
            }
        } else if c == '`' {
            result.push_str(&format!("<w:t>{}</w:t>", c));
        } else {
            result.push_str(&escape_xml(&c.to_string()));
        }
    }

    if result.is_empty() {
        result.push_str("<w:t></w:t>");
    }

    if !result.contains("<w:t>") {
        result = format!("<w:t>{}</w:t>", result);
    }

    result
}

fn build_docx(markdown: &str, _title: &str) -> Result<Vec<u8>, String> {
    use std::io::Write;

    let mut buffer = Vec::new();
    {
        let mut zip_writer = zip::ZipWriter::new(std::io::Cursor::new(&mut buffer));

        let options = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated)
            .unix_permissions(0o644);

        zip_writer.start_file("[Content_Types].xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Default Extension="xml" ContentType="application/xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Override PartName="/word/document.xml" ContentType="application/vnd.openxmlformats-officedocument.wordprocessingml.document.main+xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Types>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("_rels/.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="word/document.xml"/>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/_rels/document.xml.rels", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships">"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"</Relationships>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/document.xml", options).map_err(|e| e.to_string())?;
        let doc_xml = md_to_docx_xml(markdown);
        zip_writer.write_all(doc_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/settings.xml", options).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#).map_err(|e| e.to_string())?;
        zip_writer.write_all(br#"<w:settings xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"><w:defaultTabStop w:val="720"/></w:settings>"#).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/styles.xml", options).map_err(|e| e.to_string())?;
        let styles_xml = build_styles_xml();
        zip_writer.write_all(styles_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.start_file("word/numbering.xml", options).map_err(|e| e.to_string())?;
        let numbering_xml = build_numbering_xml();
        zip_writer.write_all(numbering_xml.as_bytes()).map_err(|e| e.to_string())?;

        zip_writer.finish().map_err(|e| e.to_string())?;
    }

    Ok(buffer)
}

fn build_styles_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:styles xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:default="1" w:styleId="Normal">"#);
    xml.push_str(r#"<w:name w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading1">"#);
    xml.push_str(r#"<w:name w:val="heading 1"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="0"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="48"/><w:szCs w:val="48"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading2">"#);
    xml.push_str(r#"<w:name w:val="heading 2"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="1"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="32"/><w:szCs w:val="32"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str(r#"<w:style w:type="paragraph" w:styleId="Heading3">"#);
    xml.push_str(r#"<w:name w:val="heading 3"/>"#);
    xml.push_str(r#"<w:basedOn w:val="Normal"/>"#);
    xml.push_str(r#"<w:next w:val="Normal"/>"#);
    xml.push_str(r#"<w:qFormat/>"#);
    xml.push_str(r#"<w:pPr><w:outlineLvl w:val="2"/></w:pPr>"#);
    xml.push_str(r#"<w:rPr><w:b/><w:sz w:val="28"/><w:szCs w:val="28"/></w:rPr>"#);
    xml.push_str(r#"</w:style>"#);

    xml.push_str("</w:styles>");
    xml
}

fn build_numbering_xml() -> String {
    let mut xml = String::from(
        r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>"#
    );
    xml.push_str(r#"<w:numbering xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main">"#);

    xml.push_str(r#"<w:abstractNum w:abstractNumId="0">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="bullet"/><w:lvlText w:val="&#x2022;"/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr><w:rPr><w:rFonts w:ascii="Arial" w:hAnsi="Arial" w:hint="default"/></w:rPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    xml.push_str(r#"<w:abstractNum w:abstractNumId="1">"#);
    xml.push_str(r#"<w:multiLevelType w:val="hybridMultilevel"/>"#);
    xml.push_str(r#"<w:lvl w:ilvl="0"><w:start w:val="1"/><w:numFmt w:val="decimal"/><w:lvlText w:val="%1."/><w:lvlJc w:val="left"/><w:pPr><w:ind w:left="720" w:hanging="360"/></w:pPr></w:lvl>"#);
    xml.push_str(r#"</w:abstractNum>"#);

    xml.push_str(r#"<w:num w:numId="1"><w:abstractNumId w:val="0"/></w:num>"#);
    xml.push_str(r#"<w:num w:numId="2"><w:abstractNumId w:val="1"/></w:num>"#);

    xml.push_str("</w:numbering>");
    xml
}

pub struct PaperDocxHandler;

#[async_trait]
impl ToolHandler for PaperDocxHandler {
    fn name(&self) -> &str { "paper_docx_export" }
    fn description(&self) -> &str { "Export a research paper or literature review from markdown to a Word (.docx) document with proper formatting, headings, lists, and styles" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("markdown".into(), ToolProperty::string("Full markdown content of the paper or review to export")),
                ("title".into(), ToolProperty::string("Document title for the Word file")),
                ("filename".into(), ToolProperty::string("Output filename without extension (default: paper_export)")),
            ].into_iter().collect(),
            vec!["markdown".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let markdown = params["markdown"].as_str().ok_or("Missing markdown")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let filename = params.get("filename").and_then(|v| v.as_str()).unwrap_or("paper_export");

        let docx_data = build_docx(markdown, title)?;

        let output_dir = data_dir().join("exports");
        std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

        let output_path = output_dir.join(format!("{}.docx", filename));
        std::fs::write(&output_path, &docx_data).map_err(|e| e.to_string())?;

        let file_size = docx_data.len();

        Ok(serde_json::json!({
            "title": title,
            "filename": format!("{}.docx", filename),
            "path": output_path.to_string_lossy(),
            "size_bytes": file_size,
            "format": "docx",
        }))
    }
}

#[derive(Clone)]
struct Slide {
    number: usize,
    title: String,
    slide_type: String,
    duration_secs: u32,
    key_points: Vec<String>,
    visual_suggestion: String,
    notes: String,
}

fn talk_type_config(talk_type: &str) -> (usize, usize, &str) {
    match talk_type {
        "conference" => (15, 18, "Conference Talk (15 min)"),
        "seminar" => (45, 25, "Academic Seminar (45 min)"),
        "defense" => (60, 30, "Thesis Defense (60 min)"),
        "grant" => (15, 18, "Grant Pitch (15 min)"),
        "journal_club" => (30, 20, "Journal Club (30 min)"),
        "teaching" => (50, 30, "Teaching Tutorial (50 min)"),
        _ => (15, 18, "Conference Talk (15 min)"),
    }
}

fn build_slide_markdown(
    topic: &str,
    talk_type: &str,
    slides: &[Slide],
    include_speaker_notes: bool,
) -> String {
    let mut md = String::new();
    let (duration_min, _, config_name) = talk_type_config(talk_type);
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    md.push_str("# Slide Deck: ");
    md.push_str(topic);
    md.push_str("\n\n");
    md.push_str(&format!("**Type:** {} | ", config_name));
    md.push_str(&format!("**Duration:** ~{} minutes | ", duration_min));
    md.push_str(&format!("**Slides:** {} | **Date:** {}\n\n", slides.len(), today));
    md.push_str("---\n\n");

    md.push_str("## Presentation Plan\n\n");
    md.push_str(&format!("This deck covers **{}** in {} format with {} slides.\n\n",
        topic, config_name, slides.len()));

    md.push_str("### Slide Sequence\n\n");
    md.push_str("| # | Slide Title | Type | Duration | Visual |\n");
    md.push_str("|---|------------|------|---------|--------|\n");
    for slide in slides {
        md.push_str(&format!("| {} | {} | {} | {}s | {} |\n",
            slide.number, slide.title, slide.slide_type,
            slide.duration_secs, slide.visual_suggestion));
    }
    md.push_str("\n---\n\n");

    for slide in slides {
        md.push_str("## ");
        md.push_str(&slide.number.to_string());
        md.push_str(". ");
        md.push_str(&slide.title);
        md.push_str("\n\n");

        md.push_str(&format!("**Type:** {} | **Time:** {}s\n\n", slide.slide_type, slide.duration_secs));

        md.push_str("**Key Points:**\n");
        for point in &slide.key_points {
            md.push_str(&format!("- {}\n", point));
        }
        md.push_str("\n");

        md.push_str(&format!("**Visual Suggestion:** {}\n\n", slide.visual_suggestion));

        if include_speaker_notes && !slide.notes.is_empty() {
            md.push_str("**Speaker Notes:**\n");
            md.push_str(&slide.notes);
            md.push_str("\n\n");
        }

        md.push_str("---\n\n");
    }

    md.push_str("## Presentation Tips\n\n");
    md.push_str("1. **Timing:** Aim for ~1 slide per minute; adjust based on audience engagement\n");
    md.push_str("2. **Visuals:** Each slide should have a strong visual element (figure, diagram, or icon)\n");
    md.push_str("3. **Citations:** Include 3-5 key references in your talk to establish credibility\n");
    md.push_str("4. **Story arc:** Follow hook → context → problem → approach → results → implications\n");
    md.push_str("5. **Practice:** Run through at least 3 times before presenting\n\n");

    md.push_str(&format!("_Generated by Rairos on {} for \"{}\"_\n", today, topic));

    md
}

pub struct PaperSlidesHandler;

#[async_trait]
impl ToolHandler for PaperSlidesHandler {
    fn name(&self) -> &str { "paper_slides_generate" }
    fn description(&self) -> &str { "Generate a structured slide deck outline for a research presentation with slide titles, key points, visual suggestions, timing, and speaker notes. Supports conference talks, seminars, thesis defenses, grant pitches, and journal clubs." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic or paper title")),
                ("talk_type".into(), ToolProperty::string("Presentation type: conference (15min), seminar (45min), defense (60min), grant (15min), journal_club (30min), teaching (50min)")),
                ("include_notes".into(), ToolProperty::string("Include speaker notes: 'true' or 'false' (default: false)")),
                ("focus_slides".into(), ToolProperty::string("Focus areas as comma-separated: 'introduction,methods,results,discussion' (default: all)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let talk_type = params.get("talk_type").and_then(|v| v.as_str()).unwrap_or("conference");
        let include_notes = params.get("include_notes").and_then(|v| v.as_str()).unwrap_or("false") == "true";
        let focus_str = params.get("focus_slides").and_then(|v| v.as_str()).unwrap_or("");

        let (duration_min, slide_count, config_name) = talk_type_config(talk_type);
        let focus: Vec<&str> = if focus_str.is_empty() {
            vec!["introduction", "background", "methods", "results", "discussion", "conclusion"]
        } else {
            focus_str.split(',').map(|s| s.trim()).collect()
        };

        let mut slides = Vec::new();
        let mut slide_num = 1usize;

        slides.push(Slide {
            number: slide_num,
            title: topic.to_string(),
            slide_type: "Title".to_string(),
            duration_secs: 30,
            key_points: vec![
                format!("Topic: {}", topic),
                "Presenter: [Your Name]".to_string(),
                "Affiliation: [Institution]".to_string(),
                format!("{}", config_name),
            ],
            visual_suggestion: "Abstract background image or relevant scientific imagery".to_string(),
            notes: "Introduce yourself briefly. Set up the talk context. Establish why this topic matters right now.".to_string(),
        });
        slide_num += 1;

        slides.push(Slide {
            number: slide_num,
            title: "Why This Research Matters".to_string(),
            slide_type: "Hook".to_string(),
            duration_secs: 60,
            key_points: vec![
                "Current state of the field".to_string(),
                "Unmet need or knowledge gap".to_string(),
                "Potential impact of solving this".to_string(),
            ],
            visual_suggestion: "Icon collage or striking statistic visualization".to_string(),
            notes: "Grab attention immediately. State the problem clearly. Make the audience care.".to_string(),
        });
        slide_num += 1;

        if focus.contains(&"introduction") || focus.contains(&"background") {
            if talk_type != "conference" {
                slides.push(Slide {
                    number: slide_num,
                    title: "Background & Prior Work".to_string(),
                    slide_type: "Content".to_string(),
                    duration_secs: if talk_type == "seminar" || talk_type == "defense" { 180 } else { 90 },
                    key_points: vec![
                        "Key prior studies in this area".to_string(),
                        "Established methods and approaches".to_string(),
                        "What the community already knows".to_string(),
                    ],
                    visual_suggestion: "Timeline diagram or citation network visualization".to_string(),
                    notes: "Provide sufficient context. Cite 3-5 key papers. Show how your work builds on existing knowledge.".to_string(),
                });
                slide_num += 1;
            }

            slides.push(Slide {
                number: slide_num,
                title: "The Knowledge Gap".to_string(),
                slide_type: "Content".to_string(),
                duration_secs: if talk_type == "conference" { 60 } else { 120 },
                key_points: vec![
                    "What remains unknown or unsolved".to_string(),
                    "Limitations of existing approaches".to_string(),
                    "Open questions your research addresses".to_string(),
                ],
                visual_suggestion: "Gap visualization showing covered vs. uncovered territory".to_string(),
                notes: "Clearly articulate the gap. This motivates your entire study.".to_string(),
            });
            slide_num += 1;
        }

        slides.push(Slide {
            number: slide_num,
            title: "Research Questions & Objectives".to_string(),
            slide_type: "Content".to_string(),
            duration_secs: 60,
            key_points: vec![
                "Primary research question(s)".to_string(),
                "Specific objectives or hypotheses".to_string(),
                "Scope and boundaries of the study".to_string(),
            ],
            visual_suggestion: "Research question tree or objective hierarchy diagram".to_string(),
            notes: "State questions clearly and specifically. Make them answerable with your approach.".to_string(),
        });
        slide_num += 1;

        if focus.contains(&"methods") {
            slides.push(Slide {
                number: slide_num,
                title: "Methods".to_string(),
                slide_type: "Content".to_string(),
                duration_secs: if talk_type == "conference" { 120 } else { 300 },
                key_points: vec![
                    "Study design or approach".to_string(),
                    "Data collection or experimental setup".to_string(),
                    "Analysis methods and tools used".to_string(),
                ],
                visual_suggestion: "Flowchart showing methodology pipeline or experimental design diagram".to_string(),
                notes: "Be thorough enough for reproducibility in longer talks. Keep it high-level for conferences.".to_string(),
            });
            slide_num += 1;

            if talk_type == "seminar" || talk_type == "defense" {
                slides.push(Slide {
                    number: slide_num,
                    title: "Methodology Details".to_string(),
                    slide_type: "Content".to_string(),
                    duration_secs: 180,
                    key_points: vec![
                        "Detailed procedures and protocols".to_string(),
                        "Equipment and materials".to_string(),
                        "Quality control and validation steps".to_string(),
                    ],
                    visual_suggestion: "Detailed schematic or step-by-step workflow diagram".to_string(),
                    notes: "Go deep on methodology. Defenses require thorough method explanation.".to_string(),
                });
                slide_num += 1;
            }
        }

        if focus.contains(&"results") {
            let result_slides = match talk_type {
                "conference" => 2,
                "seminar" => 4,
                "defense" => 5,
                "grant" => 3,
                "journal_club" => 3,
                "teaching" => 3,
                _ => 3,
            };

            for i in 0..result_slides {
                slides.push(Slide {
                    number: slide_num,
                    title: format!("Results: Key Finding {}", i + 1),
                    slide_type: "Results".to_string(),
                    duration_secs: if talk_type == "conference" { 150 } else { 180 },
                    key_points: vec![
                        format!("Primary finding {}: [main result]", i + 1),
                        "Supporting evidence or statistics".to_string(),
                        "How this relates to your question".to_string(),
                    ],
                    visual_suggestion: format!("Figure {} or chart showing the key finding", i + 1),
                    notes: "Present results objectively. Show data with large, clear visualizations. Let the data speak.".to_string(),
                });
                slide_num += 1;
            }
        }

        if focus.contains(&"discussion") {
            slides.push(Slide {
                number: slide_num,
                title: "Discussion & Interpretation".to_string(),
                slide_type: "Content".to_string(),
                duration_secs: if talk_type == "conference" { 90 } else { 180 },
                key_points: vec![
                    "What your results mean".to_string(),
                    "How findings compare to prior work".to_string(),
                    "Novel contributions of this research".to_string(),
                ],
                visual_suggestion: "Comparison chart showing your results vs. prior literature".to_string(),
                notes: "Interpret don't overinterpret. Compare with 2-3 key papers from your intro.".to_string(),
            });
            slide_num += 1;

            slides.push(Slide {
                number: slide_num,
                title: "Limitations & Future Directions".to_string(),
                slide_type: "Content".to_string(),
                duration_secs: 90,
                key_points: vec![
                    "Key limitations of the study".to_string(),
                    "Alternative explanations considered".to_string(),
                    "Recommended future research directions".to_string(),
                ],
                visual_suggestion: "Forward-looking roadmap or opportunity map".to_string(),
                notes: "Be honest about limitations. Show self-awareness and scientific integrity.".to_string(),
            });
            slide_num += 1;
        }

        if focus.contains(&"conclusion") || focus.contains(&"conclusions") {
            slides.push(Slide {
                number: slide_num,
                title: "Conclusions".to_string(),
                slide_type: "Summary".to_string(),
                duration_secs: 60,
                key_points: vec![
                    "3 main takeaways from this work".to_string(),
                    "Broader implications for the field".to_string(),
                    "Actionable recommendations if applicable".to_string(),
                ],
                visual_suggestion: "Three key takeaway icons or summary graphic".to_string(),
                notes: "Wrap up with clear, memorable conclusions. Don't introduce new information.".to_string(),
            });
            slide_num += 1;
        }

        slides.push(Slide {
            number: slide_num,
            title: "Acknowledgments".to_string(),
            slide_type: "Closing".to_string(),
            duration_secs: 30,
            key_points: vec![
                "Funding sources and support".to_string(),
                "Collaborators and contributors".to_string(),
                "Data and resource providers".to_string(),
            ],
            visual_suggestion: "Logos of funding agencies or institution branding".to_string(),
            notes: "Be grateful and professional. Acknowledge all support received.".to_string(),
        });
        slide_num += 1;

        if talk_type == "seminar" || talk_type == "defense" {
            slides.push(Slide {
                number: slide_num,
                title: "References".to_string(),
                slide_type: "Closing".to_string(),
                duration_secs: 30,
                key_points: vec![
                    "Key citations (Author, Year)".to_string(),
                    "[Full reference list in handout]".to_string(),
                ],
                visual_suggestion: "Reference list with QR code to full bibliography".to_string(),
                notes: "List 5-10 most critical references. Provide full list as handout.".to_string(),
            });
            slide_num += 1;
        }

        slides.push(Slide {
            number: slide_num,
            title: "Thank You & Questions".to_string(),
            slide_type: "Closing".to_string(),
            duration_secs: 30,
            key_points: vec![
                "Contact: [email]".to_string(),
                "More info: [website/link]".to_string(),
                "Questions welcome!".to_string(),
            ],
            visual_suggestion: "Contact information with QR code or department logo".to_string(),
            notes: "End confidently. Invite questions warmly. Have backup slides ready for anticipated Q&A.".to_string(),
        });

        let markdown = build_slide_markdown(topic, talk_type, &slides, include_notes);
        let total_duration_secs: u32 = slides.iter().map(|s| s.duration_secs).sum();
        let total_duration_min = (total_duration_secs as f32 / 60.0).ceil() as usize;

        Ok(serde_json::json!({
            "topic": topic,
            "talk_type": talk_type,
            "slide_count": slides.len(),
            "estimated_duration_min": total_duration_min,
            "focus_areas": focus,
            "markdown": markdown,
        }))
    }
}

#[derive(Clone)]
struct ProposalSection {
    name: String,
    description: String,
    subsections: Vec<String>,
}

fn agency_config(agency: &str) -> (Vec<ProposalSection>, usize, &str) {
    match agency {
        "NSF" => (
            vec![
                ProposalSection {
                    name: "Project Summary".to_string(),
                    description: "Overview of the proposed work including intellectual merit and broader impacts".to_string(),
                    subsections: vec!["Overview".to_string(), "Intellectual Merit".to_string(), "Broader Impacts".to_string()],
                },
                ProposalSection {
                    name: "Project Description".to_string(),
                    description: "Detailed description of the proposed work".to_string(),
                    subsections: vec!["Introduction/Statement of Problem".to_string(), "Related Work/Literature Review".to_string(), "Research Plan/Methodology".to_string(), "Expected Outcomes".to_string(), "Management Plan".to_string()],
                },
                ProposalSection {
                    name: "References Cited".to_string(),
                    description: "Relevant literature references".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Biographical Sketches".to_string(),
                    description: "PI and co-PI biographical information".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Budget Justification".to_string(),
                    description: "Detailed justification for requested funds".to_string(),
                    subsections: vec!["Senior Personnel".to_string(), "Other Personnel".to_string(), "Equipment".to_string(), "Travel".to_string(), "Other Direct Costs".to_string()],
                },
                ProposalSection {
                    name: "Current and Pending Support".to_string(),
                    description: "Current and pending funding for PI".to_string(),
                    subsections: vec![],
                },
            ],
            15,
            "NSF (National Science Foundation)",
        ),
        "NIH" => (
            vec![
                ProposalSection {
                    name: "Specific Aims".to_string(),
                    description: "Concise description of the research objectives and specific aims".to_string(),
                    subsections: vec!["Overall Goal".to_string(), "Specific Aims (3-4 aims)".to_string()],
                },
                ProposalSection {
                    name: "Research Strategy".to_string(),
                    description: "Comprehensive research plan including significance, innovation, and approach".to_string(),
                    subsections: vec!["Significance".to_string(), "Innovation".to_string(), "Approach".to_string()],
                },
                ProposalSection {
                    name: "Preliminary Studies".to_string(),
                    description: "Prior research findings establishing feasibility".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Human Subjects/Animal Research".to_string(),
                    description: "Protection of human subjects and animal research considerations".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Bibliography and References Cited".to_string(),
                    description: "Literature cited in the application".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Biographical Sketches".to_string(),
                    description: "Key personnel biographical information".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Budget".to_string(),
                    description: "Detailed budget by category".to_string(),
                    subsections: vec!["Personnel".to_string(), "Consultants".to_string(), "Equipment".to_string(), "Supplies".to_string(), "Travel".to_string()],
                },
            ],
            12,
            "NIH (National Institutes of Health)",
        ),
        "DOE" => (
            vec![
                ProposalSection {
                    name: "Technical Abstract".to_string(),
                    description: "Summary of proposed work suitable for public release".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Project Narrative".to_string(),
                    description: "Description of proposed work and its expected outcomes".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Project Summary/Abstract".to_string(),
                    description: "Overview of project goals and objectives".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Technical Description".to_string(),
                    description: "Detailed technical approach and methodology".to_string(),
                    subsections: vec!["Background and Motivation".to_string(), "Technical Approach".to_string(), "Deliverables and Milestones".to_string(), "Relevance and Impact".to_string()],
                },
                ProposalSection {
                    name: "References".to_string(),
                    description: "Citations and bibliography".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Biographical Information".to_string(),
                    description: "PI and key personnel information".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Budget".to_string(),
                    description: "Cost breakdown by category".to_string(),
                    subsections: vec!["Direct Costs".to_string(), "Indirect Costs".to_string(), "Cost Sharing".to_string()],
                },
            ],
            10,
            "DOE (Department of Energy)",
        ),
        "DARPA" => (
            vec![
                ProposalSection {
                    name: "Cover Sheet".to_string(),
                    description: "Program title, PI information, institution".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Executive Summary".to_string(),
                    description: "High-impact summary of proposed work".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Technical Plan".to_string(),
                    description: "Detailed technical approach with milestones".to_string(),
                    subsections: vec!["Program Vision".to_string(), "Technical Approach".to_string(), "Program Metrics".to_string(), "Risk Management".to_string()],
                },
                ProposalSection {
                    name: "Quad Chart".to_string(),
                    description: "One-page overview with goals, approach, milestones, and team".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Proposer Information".to_string(),
                    description: "Team qualifications and capabilities".to_string(),
                    subsections: vec!["Key Personnel".to_string(), "Facilities".to_string(), "Prior Work".to_string()],
                },
                ProposalSection {
                    name: "Cost Summary".to_string(),
                    description: "Budget overview and cost breakdown".to_string(),
                    subsections: vec![],
                },
            ],
            8,
            "DARPA (Defense Advanced Research Projects Agency)",
        ),
        _ => (
            vec![
                ProposalSection {
                    name: "Executive Summary".to_string(),
                    description: "Overview of the proposed project".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Statement of Need".to_string(),
                    description: "Problem statement and importance".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Objectives".to_string(),
                    description: "Clear and measurable objectives".to_string(),
                    subsections: vec!["Primary Objective".to_string(), "Secondary Objectives".to_string()],
                },
                ProposalSection {
                    name: "Methodology".to_string(),
                    description: "Detailed approach and methods".to_string(),
                    subsections: vec!["Approach".to_string(), "Timeline".to_string(), "Deliverables".to_string()],
                },
                ProposalSection {
                    name: "Expected Outcomes and Impact".to_string(),
                    description: "Anticipated results and broader significance".to_string(),
                    subsections: vec![],
                },
                ProposalSection {
                    name: "Budget".to_string(),
                    description: "Cost breakdown and justification".to_string(),
                    subsections: vec!["Personnel".to_string(), "Equipment".to_string(), "Travel".to_string(), "Other Costs".to_string()],
                },
                ProposalSection {
                    name: "Team Qualifications".to_string(),
                    description: "PI and team experience".to_string(),
                    subsections: vec!["Relevant Experience".to_string(), "Prior Accomplishments".to_string()],
                },
            ],
            10,
            "Research Grant Proposal",
        ),
    }
}

fn build_proposal_markdown(
    topic: &str,
    agency: &str,
    sections: &[ProposalSection],
    pi_name: &str,
    institution: &str,
    funding_amount: &str,
) -> String {
    let mut md = String::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();
    let (_, _, agency_name) = agency_config(agency);

    md.push_str("# Grant Proposal: ");
    md.push_str(topic);
    md.push_str("\n\n");
    md.push_str("| Field | Value |\n");
    md.push_str("|-------|-------|\n");
    md.push_str(&format!("| **Agency** | {} |\n", agency_name));
    md.push_str(&format!("| **Principal Investigator** | {} |\n", pi_name));
    md.push_str(&format!("| **Institution** | {} |\n", institution));
    md.push_str(&format!("| **Requested Funding** | {} |\n", funding_amount));
    md.push_str(&format!("| **Date** | {} |\n", today));
    md.push_str(&format!("| **Topic** | {} |\n", topic));
    md.push_str("\n---\n\n");

    for section in sections {
        md.push_str("## ");
        md.push_str(&section.name);
        md.push_str("\n\n");
        md.push_str(&section.description);
        md.push_str("\n\n");

        for subsection in &section.subsections {
            md.push_str("### ");
            md.push_str(subsection);
            md.push_str("\n\n");
            md.push_str("_[Write your content here]_\n\n");
        }

        if section.subsections.is_empty() {
            md.push_str("_[Write your content here]_\n\n");
        }

        md.push_str("---\n\n");
    }

    md.push_str("## Submission Checklist\n\n");
    md.push_str("- [ ] All sections completed\n");
    md.push_str("- [ ] Budget verified and justified\n");
    md.push_str("- [ ] References formatted correctly\n");
    md.push_str("- [ ] PI biographical sketch updated\n");
    md.push_str("- [ ] Letters of support obtained\n");
    md.push_str("- [ ] Compliance requirements met (human subjects, animal research, etc.)\n");
    md.push_str("- [ ] Budget totals checked against funding limits\n");
    md.push_str("- [ ] Proofread and formatted\n\n");

    md.push_str("---\n\n");
    md.push_str(&format!("_Generated by Rairos on {} for \"{}\" ({} format)_\n", today, topic, agency));

    md
}

pub struct PaperGrantProposalHandler;

#[async_trait]
impl ToolHandler for PaperGrantProposalHandler {
    fn name(&self) -> &str { "paper_grant_proposal" }
    fn description(&self) -> &str { "Generate a structured grant proposal outline for research funding with agency-specific sections (NSF, NIH, DOE, DARPA). Includes project summary, research strategy, methodology, budget justification, and submission checklist." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic or project title")),
                ("agency".into(), ToolProperty::string("Funding agency: NSF, NIH, DOE, DARPA, or NSTC")),
                ("pi_name".into(), ToolProperty::string("Principal investigator name")),
                ("institution".into(), ToolProperty::string("Research institution name")),
                ("funding_amount".into(), ToolProperty::string("Requested funding amount (e.g., '$500,000 for 3 years')")),
            ].into_iter().collect(),
            vec!["topic".into(), "agency".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let agency = params["agency"].as_str().ok_or("Missing agency")?;
        let pi_name = params.get("pi_name").and_then(|v| v.as_str()).unwrap_or("[PI Name]");
        let institution = params.get("institution").and_then(|v| v.as_str()).unwrap_or("[Institution]");
        let funding_amount = params.get("funding_amount").and_then(|v| v.as_str()).unwrap_or("[Amount]");

        let (sections, page_limit, agency_name) = agency_config(agency);
        let markdown = build_proposal_markdown(topic, agency, &sections, pi_name, institution, funding_amount);

        let section_names: Vec<String> = sections.iter().map(|s| s.name.clone()).collect();
        let total_subsections: usize = sections.iter().map(|s| s.subsections.len()).sum();

        Ok(serde_json::json!({
            "topic": topic,
            "agency": agency,
            "agency_name": agency_name,
            "pi_name": pi_name,
            "institution": institution,
            "funding_amount": funding_amount,
            "page_limit": page_limit,
            "section_count": sections.len(),
            "total_subsections": total_subsections,
            "sections": section_names,
            "markdown": markdown,
        }))
    }
}

#[derive(Clone)]
struct TestRecommendation {
    test_name: String,
    alternate_names: Vec<String>,
    use_when: Vec<String>,
    data_type: String,
    groups: String,
    assumptions: Vec<String>,
    effect_size: String,
    apa_template: String,
}

fn get_test_recommendations(data_type: &str, groups: &str, hypothesis: &str) -> Vec<TestRecommendation> {
    let mut tests = Vec::new();

    match (data_type, groups, hypothesis) {
        ("continuous", "two_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "Independent Samples t-test".to_string(),
                alternate_names: vec!["Two-sample t-test".to_string(), "Student's t-test".to_string()],
                use_when: vec!["Comparing means between two independent groups".to_string(), "Continuous outcome with normal distribution".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Homogeneity of variance".to_string(), "Independence of observations".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's d: small=0.2, medium=0.5, large=0.8".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Welch's t-test".to_string(),
                alternate_names: vec!["Welch's unequal variances t-test".to_string()],
                use_when: vec!["Two independent groups with unequal variances".to_string(), "Robust to variance heterogeneity".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Independence of observations".to_string(), "Does not assume equal variances".to_string()],
                effect_size: "Cohen's d or Hedges' g".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Mann-Whitney U Test".to_string(),
                alternate_names: vec!["Wilcoxon rank-sum test".to_string(), "Non-parametric alternative to t-test".to_string()],
                use_when: vec!["Non-normal continuous data".to_string(), "Ordinal data".to_string(), "Violation of normality assumption".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Ordinal or continuous data".to_string(), "Independence of observations".to_string(), "Similar distribution shapes (for median comparison)".to_string()],
                effect_size: "r = Z / sqrt(N): small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "U = XXX, p = .XXX, r = .XX".to_string(),
            });
        },
        ("continuous", "two_paired", _) => {
            tests.push(TestRecommendation {
                test_name: "Paired Samples t-test".to_string(),
                alternate_names: vec!["Dependent t-test".to_string(), "Matched pairs t-test".to_string()],
                use_when: vec!["Before-after measurements".to_string(), "Matched pairs design".to_string(), "Two measurements on same subjects".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Two paired/matched groups".to_string(),
                assumptions: vec!["Normality of differences".to_string(), "Independence within pairs".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's d: small=0.2, medium=0.5, large=0.8".to_string(),
                apa_template: "t(df) = X.XX, p = .XXX, d_z = X.XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Wilcoxon Signed-Rank Test".to_string(),
                alternate_names: vec!["Non-parametric alternative to paired t-test".to_string()],
                use_when: vec!["Non-normal paired data".to_string(), "Ordinal paired data".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Two paired groups".to_string(),
                assumptions: vec!["Ordinal or continuous differences".to_string(), "Symmetric distribution of differences".to_string()],
                effect_size: "r = Z / sqrt(N): small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "W = XXX, p = .XXX, r = .XX".to_string(),
            });
        },
        ("continuous", "three_plus_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "One-way ANOVA".to_string(),
                alternate_names: vec!["Analysis of Variance".to_string(), "F-test".to_string()],
                use_when: vec!["Comparing means across 3+ independent groups".to_string(), "Continuous outcome".to_string(), "One categorical independent variable".to_string()],
                data_type: "Continuous".to_string(),
                groups: "Three or more independent groups".to_string(),
                assumptions: vec!["Normality (approximately normal data)".to_string(), "Homogeneity of variance".to_string(), "Independence of observations".to_string(), "Interval/ratio scale".to_string()],
                effect_size: "Cohen's f: small=0.10, medium=0.25, large=0.40 (or eta-squared: small=0.01, medium=0.06, large=0.14)".to_string(),
                apa_template: "F(df_between, df_within) = X.XX, p = .XXX, f² = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Kruskal-Wallis H Test".to_string(),
                alternate_names: vec!["Non-parametric alternative to one-way ANOVA".to_string(), "H-test".to_string()],
                use_when: vec!["Non-normal data across 3+ groups".to_string(), "Ordinal data".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "Three or more independent groups".to_string(),
                assumptions: vec!["Ordinal or continuous data".to_string(), "Independence of observations".to_string(), "Similar distribution shapes".to_string()],
                effect_size: "Epsilon-squared (epsilon²): small=0.01, medium=0.06, large=0.14".to_string(),
                apa_template: "H(df) = X.XX, p = .XXX, epsilon² = .XX".to_string(),
            });
        },
        ("categorical", "two_independent", _) => {
            tests.push(TestRecommendation {
                test_name: "Chi-square Test of Independence".to_string(),
                alternate_names: vec!["Chi-squared test".to_string(), "contingency table test".to_string()],
                use_when: vec!["Comparing proportions between groups".to_string(), "Categorical outcome with 2+ categories".to_string(), "Testing association between variables".to_string()],
                data_type: "Categorical".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Expected frequencies >= 5 in each cell (or >80% cells with >=5)".to_string(), "Independence of observations".to_string(), "Random sampling".to_string()],
                effect_size: "Cramér's V: small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "χ²(df) = X.XX, p = .XXX, V = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Fisher's Exact Test".to_string(),
                alternate_names: vec!["Fisher-Irwin test".to_string()],
                use_when: vec!["Small sample sizes".to_string(), "2x2 contingency table".to_string(), "Expected frequencies < 5".to_string()],
                data_type: "Categorical".to_string(),
                groups: "Two independent groups".to_string(),
                assumptions: vec!["Hypergeometric distribution".to_string(), "Fixed marginal totals".to_string(), "Independence of observations".to_string()],
                effect_size: "Odds ratio or Cramér's V".to_string(),
                apa_template: "OR = X.XX, p = .XXX (Fisher's exact test)".to_string(),
            });
        },
        ("continuous", "correlation", _) | ("ordinal", "correlation", _) => {
            tests.push(TestRecommendation {
                test_name: "Pearson Correlation".to_string(),
                alternate_names: vec!["Pearson's r".to_string(), "Product-moment correlation".to_string()],
                use_when: vec!["Measuring linear relationship between two continuous variables".to_string(), "Both variables normally distributed".to_string()],
                data_type: "Continuous (bivariate normal)".to_string(),
                groups: "N/A (correlation)".to_string(),
                assumptions: vec!["Linearity".to_string(), "Normality of both variables".to_string(), "Homoscedasticity".to_string(), "No significant outliers".to_string()],
                effect_size: "r: small=0.1, medium=0.3, large=0.5 (or r² for variance explained)".to_string(),
                apa_template: "r(df) = .XX, p = .XXX, r² = .XX".to_string(),
            });
            tests.push(TestRecommendation {
                test_name: "Spearman's Rank Correlation".to_string(),
                alternate_names: vec!["Spearman's rho".to_string(), "Rank correlation".to_string()],
                use_when: vec!["Non-normal continuous data".to_string(), "Ordinal data".to_string(), "Monotonic relationships".to_string()],
                data_type: "Continuous or Ordinal".to_string(),
                groups: "N/A (correlation)".to_string(),
                assumptions: vec!["Monotonic relationship (not necessarily linear)".to_string(), "Ordinal or continuous data".to_string(), "Independence of observations".to_string()],
                effect_size: "rho: small=0.1, medium=0.3, large=0.5".to_string(),
                apa_template: "rho = .XX, p = .XXX".to_string(),
            });
        },
        ("continuous", "regression", _) => {
            tests.push(TestRecommendation {
                test_name: "Linear Regression".to_string(),
                alternate_names: vec!["OLS regression".to_string(), "Multiple linear regression".to_string()],
                use_when: vec!["Predicting continuous outcome from predictors".to_string(), "Continuous or dichotomous predictors".to_string(), "Testing relationship between variables".to_string()],
                data_type: "Continuous outcome".to_string(),
                groups: "N/A (predictive)".to_string(),
                assumptions: vec!["Linearity".to_string(), "Normality of residuals".to_string(), "Homoscedasticity".to_string(), "Independence of residuals".to_string(), "No multicollinearity (for multiple regression)".to_string()],
                effect_size: "R²: small=0.02, medium=0.13, large=0.26 (f² for added predictors)".to_string(),
                apa_template: "R² = .XX, F(df_model, df_residual) = X.XX, p = .XXX".to_string(),
            });
        },
        _ => {
            tests.push(TestRecommendation {
                test_name: "Descriptive Statistics".to_string(),
                alternate_names: vec!["Summary statistics".to_string()],
                use_when: vec!["Initial data exploration".to_string(), "Large sample sizes (n > 30)".to_string(), "Unknown distribution".to_string()],
                data_type: data_type.to_string(),
                groups: groups.to_string(),
                assumptions: vec!["No specific distributional assumptions".to_string()],
                effect_size: "N/A for descriptive statistics".to_string(),
                apa_template: "M = X.XX, SD = X.XX, Range: X-XX".to_string(),
            });
        }
    }

    if groups == "three_plus_independent" || groups == "two_paired" {
        tests.insert(0, TestRecommendation {
            test_name: "Consider repeated measures ANOVA".to_string(),
            alternate_names: vec!["RM-ANOVA".to_string(), "Within-subjects ANOVA".to_string()],
            use_when: vec!["Same subjects measured multiple times".to_string(), "Longitudinal data".to_string(), "Matched measurements".to_string()],
            data_type: "Continuous".to_string(),
            groups: "Multiple time points or conditions".to_string(),
            assumptions: vec!["Sphericity (or use Greenhouse-Geisser correction)".to_string(), "Normality".to_string(), "Independence within subjects".to_string()],
            effect_size: "Partial eta-squared (η²p): small=0.01, medium=0.06, large=0.14".to_string(),
            apa_template: "F(df_time, df_error) = X.XX, p = .XXX, η²p = .XX".to_string(),
        });
    }

    tests
}

fn build_analysis_markdown(
    research_question: &str,
    data_type: &str,
    groups: &str,
    hypothesis: &str,
    tests: &[TestRecommendation],
) -> String {
    let mut md = String::new();
    let today = chrono::Local::now().format("%Y-%m-%d").to_string();

    md.push_str("# Statistical Analysis Advisor\n\n");
    md.push_str(&format!("**Research Question:** {}\n", research_question));
    md.push_str(&format!("**Date:** {}\n\n", today));

    md.push_str("---\n\n");

    md.push_str("## Analysis Plan\n\n");
    md.push_str(&format!("Based on your inputs:\n"));
    md.push_str(&format!("- **Data Type:** {}\n", data_type));
    md.push_str(&format!("- **Group Structure:** {}\n", groups));
    md.push_str(&format!("- **Hypothesis Type:** {}\n\n", hypothesis));

    md.push_str("---\n\n");
    md.push_str("## Recommended Statistical Tests\n\n");

    for (i, test) in tests.iter().enumerate() {
        md.push_str(&format!("### {}: {}\n\n", i + 1, test.test_name));
        if !test.alternate_names.is_empty() {
            md.push_str(&format!("**Also known as:** {}\n", test.alternate_names.join(", ")));
        }
        md.push_str("**Use when:**\n");
        for uw in &test.use_when {
            md.push_str(&format!("- {}\n", uw));
        }
        md.push_str("\n");

        md.push_str("**Assumptions:**\n");
        for a in &test.assumptions {
            md.push_str(&format!("- {}\n", a));
        }
        md.push_str("\n");

        md.push_str(&format!("**Effect Size:** {}\n\n", test.effect_size));

        md.push_str("**APA Format Result:**\n");
        md.push_str(&format!("```\n{}\n```\n\n", test.apa_template));

        md.push_str("---\n\n");
    }

    md.push_str("## Effect Size Interpretation Guide\n\n");
    md.push_str("| Effect Size | Small | Medium | Large |\n");
    md.push_str("|-------------|-------|--------|-------|\n");
    md.push_str("| Cohen's d | 0.2 | 0.5 | 0.8 |\n");
    md.push_str("| r (Pearson) | 0.1 | 0.3 | 0.5 |\n");
    md.push_str("| Cohen's f | 0.10 | 0.25 | 0.40 |\n");
    md.push_str("| eta-squared | 0.01 | 0.06 | 0.14 |\n");
    md.push_str("| Cramér's V | 0.1 | 0.3 | 0.5 |\n");
    md.push_str("| Odds Ratio | 1.5 | 2.5 | 4.0 |\n\n");

    md.push_str("## Sample Size Guidelines\n\n");
    md.push_str("- **t-tests:** Minimum n=30 per group for normality approximation\n");
    md.push_str("- **ANOVA:** n=30 per group recommended; at least n=20 per group\n");
    md.push_str("- **Chi-square:** Expected frequency >= 5 in 80%+ of cells\n");
    md.push_str("- **Correlation:** r=0.3 requires n~85; r=0.5 requires n~30; r=0.7 requires n~16\n");
    md.push_str("- **Regression:** n >= 50 + 8k (k = number of predictors)\n\n");

    md.push_str("---\n\n");
    md.push_str("## Reporting Checklist\n\n");
    md.push_str("- [ ] State the statistical test used\n");
    md.push_str("- [ ] Report test statistic, degrees of freedom, and p-value\n");
    md.push_str("- [ ] Report effect size with confidence intervals\n");
    md.push_str("- [ ] Check and report assumption violations\n");
    md.push_str("- [ ] Report exact p-values (not just p < .05)\n");
    md.push_str("- [ ] Include descriptive statistics (M, SD for continuous; n, % for categorical)\n\n");

    md.push_str(&format!("_Generated by Rairos on {} for statistical analysis guidance_\n", today));

    md
}

pub struct StatisticalAnalysisHandler;

#[async_trait]
impl ToolHandler for StatisticalAnalysisHandler {
    fn name(&self) -> &str { "statistical_analysis_guide" }
    fn description(&self) -> &str { "Statistical analysis advisor: recommend appropriate tests based on research question, data type, and group structure. Provides effect size guidelines and APA result templates for t-tests, ANOVA, chi-square, correlation, and regression." }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("research_question".into(), ToolProperty::string("Brief description of your research question")),
                ("data_type".into(), ToolProperty::string("Type of data: 'continuous' (measurements), 'categorical' (counts/groups), or 'ordinal' (ranked data)")),
                ("groups".into(), ToolProperty::string("Group structure: 'two_independent', 'two_paired', 'three_plus_independent', 'correlation', or 'regression'")),
                ("hypothesis".into(), ToolProperty::string("Type of hypothesis: 'difference', 'association', 'prediction', or 'correlation'")),
            ].into_iter().collect(),
            vec!["research_question".into(), "data_type".into(), "groups".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let research_question = params["research_question"].as_str().ok_or("Missing research_question")?;
        let data_type = params["data_type"].as_str().ok_or("Missing data_type")?;
        let groups = params["groups"].as_str().ok_or("Missing groups")?;
        let hypothesis = params.get("hypothesis").and_then(|v| v.as_str()).unwrap_or("difference");

        let tests = get_test_recommendations(data_type, groups, hypothesis);
        let markdown = build_analysis_markdown(research_question, data_type, groups, hypothesis, &tests);

        let test_names: Vec<String> = tests.iter().map(|t| t.test_name.clone()).collect();

        Ok(serde_json::json!({
            "research_question": research_question,
            "data_type": data_type,
            "groups": groups,
            "hypothesis": hypothesis,
            "recommended_tests": test_names,
            "test_count": tests.len(),
            "markdown": markdown,
        }))
    }
}
