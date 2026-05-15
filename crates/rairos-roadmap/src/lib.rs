//! rairos-roadmap — Research roadmap generator.
//!
//! Ported from `llm/roadmap_generator.py` + `cli/cmd/roadmap.py`.
//! Pure rule-based template engine — generates structured research roadmaps
//! with phases, milestones, duration estimates, and timeline visualization.
//!
//! # Example
//! ```ignore
//! let gen = RoadmapGenerator::new();
//! let roadmap = gen.generate("How to improve LLM reasoning?", "q001", None, "");
//! println!("{}", gen.render_markdown(&roadmap));
//! ```

use chrono::Utc;
use serde::{Deserialize, Serialize};

// ── Data models ──────────────────────────────────────────────────────────────

/// A single milestone/task within a research phase.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Milestone {
    pub id: String,
    pub name: String,
    pub description: String,
    pub duration_weeks: f64,
    #[serde(default)]
    pub tasks: Vec<String>,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

/// A phase of the research roadmap, containing milestones.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Phase {
    pub id: String,
    pub name: String,
    pub description: String,
    pub duration_weeks: f64,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    pub order: usize,
}

/// A complete research roadmap with phases, milestones, and timeline.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResearchRoadmap {
    pub question: String,
    pub question_id: String,
    #[serde(default)]
    pub phases: Vec<Phase>,
    pub total_weeks: f64,
    pub created_at: String,
    #[serde(default)]
    pub notes: String,
}

// ── Template definitions ─────────────────────────────────────────────────────

/// A template milestone definition (used for generating default phases).
#[derive(Debug, Clone)]
pub struct MilestoneDef {
    pub name: String,
    pub description: String,
    pub duration_weeks: f64,
}

/// A template phase definition (used for generating default phases).
#[derive(Debug, Clone)]
pub struct PhaseDef {
    pub name: String,
    pub description: String,
    pub duration_weeks: f64,
    pub milestones: Vec<MilestoneDef>,
}

/// Default research phases (matching Python's DEFAULT_PHASES).
/// Covers: literature review → prototype → experiments → paper writing.
pub fn default_phases() -> Vec<PhaseDef> {
    vec![
        PhaseDef {
            name: "问题分析".into(),
            description: "深入理解问题，阅读相关工作，确定技术路线".into(),
            duration_weeks: 2.0,
            milestones: vec![
                MilestoneDef {
                    name: "文献调研".into(),
                    description: "阅读10-20篇相关论文".into(),
                    duration_weeks: 1.0,
                },
                MilestoneDef {
                    name: "技术方案确定".into(),
                    description: "确定初步技术路线".into(),
                    duration_weeks: 1.0,
                },
            ],
        },
        PhaseDef {
            name: "原型开发".into(),
            description: "搭建baseline，实现核心算法".into(),
            duration_weeks: 4.0,
            milestones: vec![
                MilestoneDef {
                    name: "Baseline搭建".into(),
                    description: "实现简单baseline".into(),
                    duration_weeks: 1.0,
                },
                MilestoneDef {
                    name: "核心算法实现".into(),
                    description: "实现核心改进方法".into(),
                    duration_weeks: 2.0,
                },
                MilestoneDef {
                    name: "初步验证".into(),
                    description: "在小规模数据上验证".into(),
                    duration_weeks: 1.0,
                },
            ],
        },
        PhaseDef {
            name: "实验验证".into(),
            description: "大规模实验，对比分析".into(),
            duration_weeks: 4.0,
            milestones: vec![
                MilestoneDef {
                    name: "实验设计".into(),
                    description: "设计实验方案".into(),
                    duration_weeks: 0.5,
                },
                MilestoneDef {
                    name: "对比实验".into(),
                    description: "与现有方法对比".into(),
                    duration_weeks: 2.0,
                },
                MilestoneDef {
                    name: "消融实验".into(),
                    description: "验证各组件贡献".into(),
                    duration_weeks: 1.0,
                },
                MilestoneDef {
                    name: "结果分析".into(),
                    description: "分析实验结果".into(),
                    duration_weeks: 0.5,
                },
            ],
        },
        PhaseDef {
            name: "论文撰写".into(),
            description: "撰写论文，准备投稿".into(),
            duration_weeks: 3.0,
            milestones: vec![
                MilestoneDef {
                    name: "初稿撰写".into(),
                    description: "完成论文初稿".into(),
                    duration_weeks: 2.0,
                },
                MilestoneDef {
                    name: "修改润色".into(),
                    description: "修改完善论文".into(),
                    duration_weeks: 0.5,
                },
                MilestoneDef {
                    name: "投稿准备".into(),
                    description: "准备投稿材料".into(),
                    duration_weeks: 0.5,
                },
            ],
        },
    ]
}

// ── Generator ────────────────────────────────────────────────────────────────

/// Generates structured research roadmaps from questions.
///
/// Pure rule-based engine — no LLM calls. Uses predefined phase templates
/// to create a consistent research plan structure.
pub struct RoadmapGenerator;

impl RoadmapGenerator {
    pub fn new() -> Self {
        Self
    }

    /// Generate a research roadmap from a question and optional custom phases.
    ///
    /// * `question` — Research question text
    /// * `question_id` — Optional question identifier (from QuestionTracker)
    /// * `custom_phases` — Optional custom phase definitions; uses built-in defaults if `None`
    /// * `notes` — Optional additional notes for the roadmap
    pub fn generate(
        &self,
        question: &str,
        question_id: &str,
        custom_phases: Option<&[PhaseDef]>,
        notes: &str,
    ) -> ResearchRoadmap {
        let phases = custom_phases.unwrap_or_else(|| {
            // Leak the default phases for a static reference. Since there's no
            // way to get a &[PhaseDef] from Vec<PhaseDef> without either
            // storing it or leaking, we leak here — the default phases are
            // truly static data and this is a one-time cost.
            Box::leak(default_phases().into_boxed_slice())
        });

        let mut roadmap_phases = Vec::new();
        let mut milestone_counter = 1usize;

        for (i, phase_def) in phases.iter().enumerate() {
            let mut milestones = Vec::new();

            for m_def in &phase_def.milestones {
                let milestone = Milestone {
                    id: format!("m{milestone_counter}"),
                    name: m_def.name.clone(),
                    description: m_def.description.clone(),
                    duration_weeks: m_def.duration_weeks,
                    tasks: Vec::new(),
                    dependencies: Vec::new(),
                };
                milestones.push(milestone);
                milestone_counter += 1;
            }

            let phase = Phase {
                id: format!("phase{}", i + 1),
                name: phase_def.name.clone(),
                description: phase_def.description.clone(),
                duration_weeks: phase_def.duration_weeks,
                milestones,
                order: i,
            };
            roadmap_phases.push(phase);
        }

        let total_weeks: f64 = roadmap_phases.iter().map(|p| p.duration_weeks).sum();
        let created_at = Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

        ResearchRoadmap {
            question: question.to_string(),
            question_id: question_id.to_string(),
            phases: roadmap_phases,
            total_weeks,
            created_at,
            notes: notes.to_string(),
        }
    }

    /// Render roadmap as formatted plain text (terminal-friendly).
    pub fn render_text(&self, roadmap: &ResearchRoadmap) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# 研究路线图: {}", roadmap.question));
        lines.push(String::new());
        lines.push(format!("📅 总预计时长: {:.0} 周", roadmap.total_weeks));
        lines.push(format!("📅 创建时间: {}", &roadmap.created_at[..10]));
        lines.push(String::new());

        if !roadmap.question_id.is_empty() {
            lines.push(format!("🔗 问题ID: {}", roadmap.question_id));
            lines.push(String::new());
        }

        lines.push("=".repeat(60));
        lines.push(String::new());

        for phase in &roadmap.phases {
            lines.push(format!("## 📦 阶段 {}: {}", phase.order + 1, phase.name));
            lines.push(format!("⏱️  预计: {:.0} 周", phase.duration_weeks));
            lines.push(format!("📝 {}", phase.description));
            lines.push(String::new());

            for milestone in &phase.milestones {
                let deps = if milestone.dependencies.is_empty() {
                    String::new()
                } else {
                    format!(" ← [{}]", milestone.dependencies.join(", "))
                };
                lines.push(format!(
                    "  └── 🎯 [{}] {} ({:.0}周){}",
                    milestone.id, milestone.name, milestone.duration_weeks, deps
                ));
                lines.push(format!("      {}", milestone.description));
                for task in &milestone.tasks {
                    lines.push(format!("         - {}", task));
                }
                lines.push(String::new());
            }
        }

        // Timeline summary
        lines.push("=".repeat(60));
        lines.push(String::new());
        lines.push("## 📊 时间线概览".into());

        let mut current_week = 1.0;
        for phase in &roadmap.phases {
            let end_week = current_week + phase.duration_weeks - 1.0;
            lines.push(format!(
                "Week {:.0}-{:.0}: {} ({:.0}周)",
                current_week, end_week, phase.name, phase.duration_weeks
            ));
            current_week = end_week + 1.0;
        }

        if !roadmap.notes.is_empty() {
            lines.push(String::new());
            lines.push("## 📋 备注".into());
            lines.push(roadmap.notes.clone());
        }

        lines.join("\n")
    }

    /// Render roadmap as Markdown (suitable for export to `.md` files).
    pub fn render_markdown(&self, roadmap: &ResearchRoadmap) -> String {
        let mut lines = Vec::new();

        lines.push(format!("# 研究路线图: {}", roadmap.question));
        lines.push(String::new());
        lines.push(format!("**总预计时长**: {:.0} 周", roadmap.total_weeks));
        lines.push(String::new());
        lines.push("---".into());
        lines.push(String::new());

        for phase in &roadmap.phases {
            lines.push(format!("## {}. {}", phase.order + 1, phase.name));
            lines.push(format!(
                "**时长**: {:.0} 周 | *{}*",
                phase.duration_weeks, phase.description
            ));
            lines.push(String::new());

            for milestone in &phase.milestones {
                let deps = if milestone.dependencies.is_empty() {
                    String::new()
                } else {
                    format!(" ← [{}]", milestone.dependencies.join(", "))
                };
                lines.push(format!(
                    "- [ ] **[[{}]]** {} ({:.0}周){}",
                    milestone.id, milestone.name, milestone.duration_weeks, deps
                ));
                lines.push(format!("  - {}", milestone.description));
                for task in &milestone.tasks {
                    lines.push(format!("  - [ ] {}", task));
                }
            }
            lines.push(String::new());
        }

        // Gantt-style timeline
        lines.push("---".into());
        lines.push(String::new());
        lines.push("## 📊 时间线".into());
        lines.push(String::new());
        lines.push("| 阶段 | 周数 | 内容 |".into());
        lines.push("|------|------|------|".into());

        let mut current_week = 1.0;
        for phase in &roadmap.phases {
            let end_week = current_week + phase.duration_weeks - 1.0;
            let milestone_names: Vec<String> =
                phase.milestones.iter().map(|m| m.name.clone()).collect();
            lines.push(format!(
                "| {} | Week {:.0}-{:.0} | {} |",
                phase.name,
                current_week,
                end_week,
                milestone_names.join(", ")
            ));
            current_week = end_week + 1.0;
        }

        lines.join("\n")
    }

    /// Render roadmap as pretty-printed JSON.
    pub fn render_json(&self, roadmap: &ResearchRoadmap) -> String {
        serde_json::to_string_pretty(roadmap)
            .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
    }
}

impl Default for RoadmapGenerator {
    fn default() -> Self {
        Self::new()
    }
}

// ── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_roadmap() -> ResearchRoadmap {
        let gen = RoadmapGenerator::new();
        gen.generate("How to improve LLM reasoning?", "q001", None, "")
    }

    #[test]
    fn test_generate_default_phases() {
        let roadmap = sample_roadmap();
        assert_eq!(roadmap.question, "How to improve LLM reasoning?");
        assert_eq!(roadmap.question_id, "q001");
        assert_eq!(roadmap.phases.len(), 4);
        assert!(roadmap.total_weeks > 0.0);
        assert!(!roadmap.created_at.is_empty());
    }

    #[test]
    fn test_generate_phase_names() {
        let roadmap = sample_roadmap();
        assert_eq!(roadmap.phases[0].name, "问题分析");
        assert_eq!(roadmap.phases[1].name, "原型开发");
        assert_eq!(roadmap.phases[2].name, "实验验证");
        assert_eq!(roadmap.phases[3].name, "论文撰写");
    }

    #[test]
    fn test_generate_milestones() {
        let roadmap = sample_roadmap();
        // Phase 0 (问题分析): 2 milestones
        assert_eq!(roadmap.phases[0].milestones.len(), 2);
        assert_eq!(roadmap.phases[0].milestones[0].id, "m1");
        assert_eq!(roadmap.phases[0].milestones[1].id, "m2");
        // Phase 1 (原型开发): 3 milestones
        assert_eq!(roadmap.phases[1].milestones.len(), 3);
        assert_eq!(roadmap.phases[1].milestones[0].id, "m3");
        // Phase 2 (实验验证): 4 milestones
        assert_eq!(roadmap.phases[2].milestones.len(), 4);
        // Phase 3 (论文撰写): 3 milestones
        assert_eq!(roadmap.phases[3].milestones.len(), 3);
        // Total milestones: 2+3+4+3 = 12
        assert_eq!(roadmap.phases[3].milestones[2].id, "m12");
    }

    #[test]
    fn test_generate_total_weeks() {
        let roadmap = sample_roadmap();
        // 2 + 4 + 4 + 3 = 13
        assert!((roadmap.total_weeks - 13.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generate_custom_phases() {
        let gen = RoadmapGenerator::new();
        let custom = vec![PhaseDef {
            name: "快速验证".into(),
            description: "快速验证想法".into(),
            duration_weeks: 1.0,
            milestones: vec![MilestoneDef {
                name: "PoC".into(),
                description: "概念验证".into(),
                duration_weeks: 1.0,
            }],
        }];
        let roadmap = gen.generate("Test question", "", Some(&custom), "");
        assert_eq!(roadmap.phases.len(), 1);
        assert_eq!(roadmap.phases[0].name, "快速验证");
        assert_eq!(roadmap.phases[0].milestones.len(), 1);
        assert!((roadmap.total_weeks - 1.0).abs() < f64::EPSILON);
    }

    #[test]
    fn test_generate_with_notes() {
        let gen = RoadmapGenerator::new();
        let roadmap = gen.generate("Q?", "", None, "Important: check related work");
        assert_eq!(roadmap.notes, "Important: check related work");
    }

    #[test]
    fn test_render_text_contains_all_elements() {
        let roadmap = sample_roadmap();
        let gen = RoadmapGenerator::new();
        let text = gen.render_text(&roadmap);

        assert!(text.contains("研究路线图"));
        assert!(text.contains("How to improve LLM reasoning?"));
        assert!(text.contains("q001"));
        assert!(text.contains("问题分析"));
        assert!(text.contains("原型开发"));
        assert!(text.contains("实验验证"));
        assert!(text.contains("论文撰写"));
        assert!(text.contains("文献调研"));
        assert!(text.contains("m1"));
        assert!(text.contains("13 周"));
        assert!(text.contains("时间线概览"));
    }

    #[test]
    fn test_render_markdown_contains_all_elements() {
        let roadmap = sample_roadmap();
        let gen = RoadmapGenerator::new();
        let md = gen.render_markdown(&roadmap);

        assert!(md.contains("# 研究路线图"));
        assert!(md.contains("How to improve LLM reasoning?"));
        assert!(md.contains("## 1. 问题分析"));
        assert!(md.contains("## 2. 原型开发"));
        assert!(md.contains("## 3. 实验验证"));
        assert!(md.contains("## 4. 论文撰写"));
        assert!(md.contains("- [ ] **[[m1]]**")); // Markdown task format
        assert!(md.contains("| 阶段 | 周数 | 内容 |")); // Gantt header
        assert!(md.contains("Week 1-2")); // Timeline
        assert!(md.contains("Week 3-6"));
    }

    #[test]
    fn test_render_json_valid() {
        let roadmap = sample_roadmap();
        let gen = RoadmapGenerator::new();
        let json = gen.render_json(&roadmap);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();

        assert_eq!(parsed["question"], "How to improve LLM reasoning?");
        assert_eq!(parsed["question_id"], "q001");
        assert!(parsed["total_weeks"].as_f64().unwrap() > 0.0);
        assert!(parsed["phases"].is_array());
        assert_eq!(parsed["phases"].as_array().unwrap().len(), 4);
        assert_eq!(parsed["phases"][0]["name"], "问题分析");
        assert!(parsed["phases"][0]["milestones"].is_array());
    }

    #[test]
    fn test_render_json_milestone_fields() {
        let roadmap = sample_roadmap();
        let gen = RoadmapGenerator::new();
        let json = gen.render_json(&roadmap);
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        let m = &parsed["phases"][0]["milestones"][0];
        assert_eq!(m["id"], "m1");
        assert_eq!(m["name"], "文献调研");
        assert!(m["tasks"].is_array());
        assert!(m["dependencies"].is_array());
    }

    #[test]
    fn test_render_text_no_question_id() {
        let gen = RoadmapGenerator::new();
        let roadmap = gen.generate("Just a question", "", None, "");
        let text = gen.render_text(&roadmap);
        assert!(!text.contains("问题ID"));
    }

    #[test]
    fn test_render_text_with_notes() {
        let gen = RoadmapGenerator::new();
        let roadmap = gen.generate("Q?", "", None, "Some notes here");
        let text = gen.render_text(&roadmap);
        assert!(text.contains("备注"));
        assert!(text.contains("Some notes here"));
    }

    #[test]
    fn test_render_roundtrip_custom_phases() {
        let gen = RoadmapGenerator::new();
        let custom = vec![PhaseDef {
            name: "Phase A".into(),
            description: "Desc A".into(),
            duration_weeks: 5.0,
            milestones: vec![
                MilestoneDef {
                    name: "M1".into(),
                    description: "Milestone 1".into(),
                    duration_weeks: 2.0,
                },
                MilestoneDef {
                    name: "M2".into(),
                    description: "Milestone 2".into(),
                    duration_weeks: 3.0,
                },
            ],
        }];
        let roadmap = gen.generate("Custom", "c001", Some(&custom), "Custom notes");

        assert_eq!(roadmap.phases.len(), 1);
        assert_eq!(roadmap.phases[0].milestones.len(), 2);

        let json = gen.render_json(&roadmap);
        let parsed: ResearchRoadmap = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.question, "Custom");
        assert_eq!(parsed.phases[0].name, "Phase A");
        assert_eq!(parsed.phases[0].milestones[0].name, "M1");
    }

    #[test]
    fn test_default_iterator_safe() {
        // Ensure calling default_phases() multiple times works independently
        let p1 = default_phases();
        let p2 = default_phases();
        assert_eq!(p1.len(), p2.len());
        assert_eq!(p1[0].name, p2[0].name);
    }

    #[test]
    fn test_fractional_weeks_in_milestones() {
        let roadmap = sample_roadmap();
        // Phase 2 (实验验证) has 0.5-week milestones
        let m0 = &roadmap.phases[2].milestones[0];
        assert!((m0.duration_weeks - 0.5).abs() < f64::EPSILON);
        assert_eq!(m0.name, "实验设计");
    }

    #[test]
    fn test_phase_order() {
        let roadmap = sample_roadmap();
        for (i, phase) in roadmap.phases.iter().enumerate() {
            assert_eq!(phase.order, i);
        }
    }

    #[test]
    fn test_new_roadmap_has_created_at() {
        let gen = RoadmapGenerator::new();
        let roadmap = gen.generate("Test", "", None, "");
        // ISO format date prefix: YYYY-MM-DD
        assert_eq!(roadmap.created_at.len(), 19);
        assert_eq!(&roadmap.created_at[4..5], "-");
        assert_eq!(&roadmap.created_at[7..8], "-");
    }
}
