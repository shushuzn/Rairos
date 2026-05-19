use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

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
