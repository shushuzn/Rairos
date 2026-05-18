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

use anyhow::{Context, Result};
use std::path::PathBuf;
use crate::EvoSkillAction;

pub fn handle_evoskill(action: &EvoSkillAction) -> Result<()> {
    match action {
        EvoSkillAction::Status => {
            // Check if evoskill CLI is available
            let which = std::process::Command::new("which")
                .arg("evoskill")
                .output();
            let available = match which {
                Ok(out) => out.status.success(),
                Err(_) => false,
            };

            // Also check ~/.claude/skills/evoskill
            let skill_path = dirs::home_dir()
                .map(|p| p.join(".claude").join("skills").join("evoskill"))
                .filter(|p| p.exists());

            if available || skill_path.is_some() {
                println!("✅ EvoSkill is available");
            } else {
                eprintln!("❌ EvoSkill not found");
                eprintln!("   Install: pip install evoskill");
            }
        }
        EvoSkillAction::Init {
            task,
            dataset,
            harness,
            model,
            question_col,
            answer_col,
            category_col,
        } => {
            println!("📦 Initializing EvoSkill project for task: {}", task);

            let work_dir = PathBuf::from(".evoskill");
            std::fs::create_dir_all(&work_dir)
                .context("Failed to create .evoskill directory")?;

            // Write config.toml
            let category_section = match category_col {
                Some(col) => format!("\ncategory_column = \"{}\"", col),
                None => String::new(),
            };
            let config = format!(
                r#"# EvoSkill project configuration for {task}

[harness]
name = "{harness}"
model = "{model}"
data_dirs = []
timeout_seconds = 1200
max_retries = 3

[evolution]
mode = "skill_only"
iterations = 20
frontier_size = 3
concurrency = 4
no_improvement_limit = 5
failure_samples = 3

[dataset]
path = "{dataset}"
question_column = "{question_col}"
ground_truth_column = "{answer_col}"{category_section}
train_ratio = 0.18
val_ratio = 0.12

[scorer]
type = "multi_tolerance"
"#,
                task = task,
                dataset = dataset,
                harness = harness,
                model = model,
                question_col = question_col,
                answer_col = answer_col,
                category_section = category_section,
            );
            std::fs::write(work_dir.join("config.toml"), &config)
                .context("Failed to write config.toml")?;

            // Write task.md
            let task_md = format!("# {}\n\nTask description for EvoSkill benchmark.\n", task);
            std::fs::write(work_dir.join("task.md"), &task_md)
                .context("Failed to write task.md")?;

            println!("  ✅ Config: {}", work_dir.join("config.toml").display());
            println!("  ✅ Task:   {}", work_dir.join("task.md").display());
            println!();
            println!("  Next: Edit .evoskill/task.md, then run: rairos evoskill run");
        }
        EvoSkillAction::Run {
            continue_mode,
            verbose,
        } => {
            println!("🚀 Running EvoSkill self-improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            if *verbose {
                cmd.arg("--verbose");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ Run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        EvoSkillAction::Eval => {
            println!("📊 Evaluating...");
            let status = std::process::Command::new("evoskill")
                .arg("eval")
                .status()
                .context("Failed to run evoskill eval")?;
            if status.success() {
                println!("✅ Evaluation complete");
            } else {
                anyhow::bail!("evoskill eval failed (exit: {})", status);
            }
        }
        EvoSkillAction::Diff { from_iter, to_iter } => {
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("diff");
            if let (Some(f), Some(t)) = (from_iter, to_iter) {
                cmd.arg(f.to_string());
                cmd.arg(t.to_string());
            }
            let output = cmd.output().context("Failed to run evoskill diff")?;
            if output.status.success() {
                print!("{}", String::from_utf8_lossy(&output.stdout));
            } else {
                let stderr = String::from_utf8_lossy(&output.stderr);
                anyhow::bail!("evoskill diff failed: {}", stderr);
            }
        }
        EvoSkillAction::Reset => {
            println!("🔄 Resetting all program branches...");
            let status = std::process::Command::new("evoskill")
                .arg("reset")
                .status()
                .context("Failed to run evoskill reset")?;
            if status.success() {
                println!("✅ Reset complete");
            } else {
                anyhow::bail!("evoskill reset failed (exit: {})", status);
            }
        }
    }
    Ok(())
}
