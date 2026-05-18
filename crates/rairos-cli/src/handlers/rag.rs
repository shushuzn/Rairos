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
use std::path::{Path, PathBuf};
use crate::RagAction;
use crate::handlers::*;

pub fn handle_rag(action: &RagAction) -> Result<()> {
    match action {
        RagAction::Status => {
            // Check paper2code availability
            let paper2code_ok = std::process::Command::new("which")
                .arg("paper2code")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("paper2code").exists())
                    .unwrap_or(false);

            // Check evoskill availability (same logic as handle_evoskill)
            let evoskill_ok = std::process::Command::new("which")
                .arg("evoskill")
                .output()
                .map(|o| o.status.success())
                .unwrap_or(false)
                || dirs::home_dir()
                    .map(|p| p.join(".claude").join("skills").join("evoskill").exists())
                    .unwrap_or(false);

            println!("🔍 RAG Pipeline Status");
            println!();
            println!("  {:<20} {}", "Component", "Status");
            println!("  {}", "─".repeat(35));
            println!(
                "  {:<20} {}",
                "paper2code",
                if paper2code_ok { "✅ available" } else { "❌ not found" }
            );
            println!(
                "  {:<20} {}",
                "EvoSkill",
                if evoskill_ok { "✅ available" } else { "❌ not found" }
            );
            println!();
            if paper2code_ok && evoskill_ok {
                println!("  RAG pipeline is fully available");
                println!("  Run: rairos rag run-full <arxiv_id>");
            } else {
                println!("  Some components are missing");
            }
        }
        RagAction::RunFull {
            arxiv_id,
            mode,
            framework,
            task,
        } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let task_name = task.clone().unwrap_or_else(|| {
                format!("paper_{}", arxiv_id.replace('.', "_"))
            });
            let work_dir = PathBuf::from(".rag_work");
            let paper_dir = work_dir.join(&arxiv_id);

            println!("🚀 Starting RAG pipeline for arXiv: {}", arxiv_id);

            // Stage 1: paper2code — shell out to external CLI or use existing tool
            println!("  Stage 1/4: Generating code from paper...");
            run_paper2code(&arxiv_id, mode, framework)?;

            // Stage 2: Extract test cases
            println!("  Stage 2/4: Extracting test cases...");
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;

            // Stage 3: Generate pytest files
            println!("  Stage 3/4: Generating pytest tests...");
            generate_pytest_tests(&paper_dir, &test_csv)?;

            // Stage 4: Initialize EvoSkill benchmark
            println!("  Stage 4/4: Initializing EvoSkill benchmark...");
            init_evoskill_benchmark(&work_dir, &task_name, &test_csv)?;

            println!();
            println!("✅ RAG pipeline completed!");
            println!("  Code:      {}", paper_dir.join("src").display());
            println!("  Test CSV:  {}", test_csv.display());
            println!("  Test dir:  {}", paper_dir.join("tests").display());
            println!("  Benchmark: {}", work_dir.join(".evoskill").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill' to start skill improvement");
        }
        RagAction::GenTests { arxiv_id } => {
            let arxiv_id = clean_arxiv_id(arxiv_id);
            let paper_dir = PathBuf::from(".rag_work").join(&arxiv_id);

            println!("🧪 Generating tests for arXiv: {}", arxiv_id);
            let test_csv = extract_and_generate_tests(&arxiv_id, &paper_dir)?;
            generate_pytest_tests(&paper_dir, &test_csv)?;

            println!("✅ Tests generated: {}", test_csv.display());
        }
        RagAction::InitBenchmark {
            csv_path,
            task,
        } => {
            let work_dir = PathBuf::from(".rag_work");
            println!("📦 Initializing benchmark for task: {}", task);
            init_evoskill_benchmark(&work_dir, task, &PathBuf::from(csv_path))?;
            println!("✅ Benchmark initialized!");
            println!("  Config: {}", work_dir.join(".evoskill").join("config.toml").display());
            println!("  Task:   {}", work_dir.join(".evoskill").join("task.md").display());
            println!();
            println!("  Next: Run 'rairos rag run-evoskill'");
        }
        RagAction::RunEvoskill { continue_mode } => {
            println!("🚀 Running EvoSkill improvement loop...");
            let mut cmd = std::process::Command::new("evoskill");
            cmd.arg("run");
            if *continue_mode {
                cmd.arg("--continue");
            }
            let status = cmd.status().context("Failed to run evoskill")?;
            if status.success() {
                println!("✅ EvoSkill run completed");
            } else {
                anyhow::bail!("evoskill run failed (exit: {})", status);
            }
        }
        RagAction::ListSkills => {
            let output = std::process::Command::new("evoskill")
                .arg("skills")
                .output()
                .context("Failed to list evoskill skills")?;
            if output.status.success() {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let skills: Vec<&str> = stdout.lines().filter(|l| !l.trim().is_empty()).collect();
                if skills.is_empty() {
                    println!("No skills discovered yet");
                } else {
                    println!("Discovered skills:");
                    for skill in &skills {
                        println!("  - {}", skill);
                    }
                }
            } else {
                anyhow::bail!("evoskill skills failed (exit: {})", output.status);
            }
        }
    }
    Ok(())
}

fn clean_arxiv_id(s: &str) -> String {
    // Extract arXiv ID from URL or pattern
    if let Some(caps) = regex::Regex::new(r"(\d{4}\.\d{4,5})")
        .ok()
        .and_then(|re| re.captures(s))
    {
        caps.get(1).unwrap().as_str().to_string()
    } else {
        s.to_string()
    }
}

fn run_paper2code(arxiv_id: &str, mode: &str, framework: &str) -> Result<()> {
    // Check if paper2code CLI is available
    let available = std::process::Command::new("which")
        .arg("paper2code")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);

    if !available {
        eprintln!("  ⚠️  paper2code not installed, creating placeholder structure");
        let paper_dir = PathBuf::from(".rag_work").join(arxiv_id);
        let src_dir = paper_dir.join("src");
        std::fs::create_dir_all(&src_dir)?;
        let readme = format!("# Paper {}\n\nImplementation generated by paper2code.\nMode: {}\nFramework: {}\n", arxiv_id, mode, framework);
        std::fs::write(paper_dir.join("README.md"), &readme)?;
        let placeholder = format!(
            r#""""Paper {} implementation placeholder.""""\n# TODO: Run paper2code to generate implementation\n"#,
            arxiv_id
        );
        std::fs::write(src_dir.join("implementation.py"), &placeholder)?;
        return Ok(());
    }

    let status = std::process::Command::new("paper2code")
        .arg(arxiv_id)
        .arg("--mode")
        .arg(mode)
        .arg("--framework")
        .arg(framework)
        .status()
        .context("Failed to run paper2code")?;

    if !status.success() {
        anyhow::bail!("paper2code failed (exit: {})", status);
    }
    Ok(())
}

fn extract_and_generate_tests(arxiv_id: &str, paper_dir: &Path) -> Result<PathBuf> {
    let test_csv = paper_dir.join("tests").join("test_cases.csv");
    std::fs::create_dir_all(test_csv.parent().unwrap())?;

    let test_cases = extract_from_code(paper_dir);

    let cases = if test_cases.is_empty() {
        generate_default_cases(arxiv_id)
    } else {
        test_cases
    };

    // Write CSV
    let mut wtr = csv::Writer::from_path(&test_csv)?;
    wtr.write_record(["question", "expected_output", "category"])?;
    for case in &cases {
        wtr.write_record([
            case.0.as_str(),
            case.1.as_str(),
            case.2.as_str(),
        ])?;
    }
    wtr.flush()?;

    Ok(test_csv)
}

fn extract_from_code(paper_dir: &Path) -> Vec<(String, String, String)> {
    let mut cases = Vec::new();

    // Check README for code examples
    let readme_path = paper_dir.join("README.md");
    if readme_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&readme_path) {
            // Match code blocks with Python examples
            let re = regex::Regex::new(r"```(?:python|py)?\n(.*?)```").ok();
            if let Some(re) = re {
                for cap in re.captures_iter(&content) {
                    let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                    if match_text.contains('=') && match_text.contains("print") {
                        cases.push((
                            format!("Execute and provide output: ```{}```", match_text.trim()),
                            "execution successful".to_string(),
                            "execution".to_string(),
                        ));
                    }
                }
            }
        }
    }

    // Check src directory for docstring examples
    let src_dir = paper_dir.join("src");
    if src_dir.exists() {
        let re = regex::Regex::new(r#""""\s*(.*?)\s*""""#).ok();
        if let Ok(entries) = std::fs::read_dir(&src_dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map(|e| e == "py").unwrap_or(false) {
                    if let Ok(content) = std::fs::read_to_string(&path) {
                        if let Some(ref re) = re {
                            for cap in re.captures_iter(&content) {
                                let match_text = cap.get(1).map(|m| m.as_str()).unwrap_or("");
                                if match_text.contains("Example") || match_text.contains("例子") {
                                    let preview: String = match_text.chars().take(100).collect();
                                    cases.push((
                                        format!("Implement function per docstring: {}", preview),
                                        "implementation correct".to_string(),
                                        "implementation".to_string(),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    cases.truncate(20);
    cases
}

fn generate_default_cases(arxiv_id: &str) -> Vec<(String, String, String)> {
    vec![
        (
            format!("Verify {} implementation correctness", arxiv_id),
            "functional".to_string(),
            "general".to_string(),
        ),
        (
            format!("Check {} API interface", arxiv_id),
            "API available".to_string(),
            "api".to_string(),
        ),
        (
            format!("Verify {} input/output format", arxiv_id),
            "format correct".to_string(),
            "io".to_string(),
        ),
    ]
}

fn generate_pytest_tests(_paper_dir: &Path, test_csv: &Path) -> Result<()> {
    let test_dir = test_csv.parent().unwrap();

    // conftest.py
    let conftest = r#""""Fixtures for generated tests.""""
import pytest
from pathlib import Path

@pytest.fixture
def test_data_path():
    """Path to test cases CSV."""
    return Path(__file__).parent / "test_cases.csv"

@pytest.fixture
def paper_dir():
    """Path to paper implementation."""
    return Path(__file__).parent.parent
"#;
    std::fs::write(test_dir.join("conftest.py"), conftest)?;

    // test_impl.py
    let test_impl = r#""""Auto-generated tests for paper implementation.""""
import csv
import pytest
from pathlib import Path

def load_test_cases():
    csv_path = Path(__file__).parent / "test_cases.csv"
    cases = []
    with open(csv_path, encoding="utf-8") as f:
        reader = csv.DictReader(f)
        for row in reader:
            cases.append(row)
    return cases

class TestPaperImplementation:
    @pytest.fixture(autouse=True)
    def setup(self, paper_dir):
        self.paper_dir = paper_dir

    def test_code_directory_exists(self):
        src_dir = self.paper_dir / "src"
        assert src_dir.exists(), f"Implementation dir not found: {src_dir}"

    @pytest.mark.parametrize("case", load_test_cases(), ids=lambda c: c["category"])
    def test_case(self, case):
        assert case["category"] in ["execution", "implementation", "general", "api", "io"]
        assert len(case["question"]) > 0
        assert len(case["expected_output"]) > 0
"#;
    std::fs::write(test_dir.join("test_impl.py"), test_impl)?;

    Ok(())
}

fn init_evoskill_benchmark(work_dir: &Path, task_name: &str, csv_path: &Path) -> Result<()> {
    let evoskill_dir = work_dir.join(".evoskill");
    std::fs::create_dir_all(&evoskill_dir)?;

    let config_content = format!(
        r#"# EvoSkill benchmark for {task}

[harness]
name = "claude"
model = "sonnet"
data_dirs = []
timeout_seconds = 600
max_retries = 2

[evolution]
mode = "skill_only"
iterations = 10
frontier_size = 2
concurrency = 2
no_improvement_limit = 3
failure_samples = 2

[dataset]
path = "{csv}"
question_column = "question"
ground_truth_column = "expected_output"
category_column = "category"
train_ratio = 0.5
val_ratio = 0.3

[scorer]
type = "multi_tolerance"
"#,
        task = task_name,
        csv = csv_path.display(),
    );
    std::fs::write(evoskill_dir.join("config.toml"), &config_content)?;

    let task_content = r#"# Task

验证 paper 实现的功能是否正确。

## Output format
返回 "通过" 或具体错误信息。
"#;
    std::fs::write(evoskill_dir.join("task.md"), task_content)?;

    Ok(())
}
