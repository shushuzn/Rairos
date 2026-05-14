# Rust CLI: Complete `parse` + `repl` Handlers

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Complete the last two stub handlers in the Rust CLI — PDF parsing and interactive REPL.

**Architecture:** Add `lopdf` dependency to `rairos-pdf` crate for actual PDF text extraction. Update `rairos-cli` handlers to use real implementations instead of stubs. REPL reuses existing handler functions via stdin dispatch.

**Tech Stack:** Rust, lopdf, clap, reqwest

---

### Task 1: Add lopdf + implement `extract_pdf_text()`

**Files:**
- Modify: `crates/rairos-pdf/Cargo.toml`
- Modify: `crates/rairos-pdf/src/lib.rs`

- [ ] **Step 1: Add lopdf dependency**

Edit `crates/rairos-pdf/Cargo.toml`:
```toml
# add after hex
lopdf = "0.34"
```

- [ ] **Step 2: Implement extract_pdf_text()**

Replace the stub in `crates/rairos-pdf/src/lib.rs`:

```rust
pub fn extract_pdf_text(pdf_path: &Path) -> Result<String> {
    let doc = lopdf::Document::load(pdf_path).map_err(|e| {
        PdfError::ParseFailed(format!("Failed to load PDF: {}", e))
    })?;
    let mut text = String::new();
    for page_num in 1..=doc.page_count() {
        if let Ok(page_text) = doc.extract_text(&[page_num]) {
            if !text.is_empty() {
                text.push('\n');
            }
            text.push_str(&page_text);
        }
    }
    if text.is_empty() {
        return Err(PdfError::ParseFailed("No text extracted from PDF".into()));
    }
    Ok(text)
}
```

Remove the underscore prefix from the parameter.

- [ ] **Step 3: Build to verify compilation**

Run: `cargo build -p rairos-pdf 2>&1`
Expected: Compilation succeeds

- [ ] **Step 4: Commit**

```
git add crates/rairos-pdf/Cargo.toml crates/rairos-pdf/src/lib.rs
git commit -m "feat: implement PDF text extraction via lopdf"
```

---

### Task 2: Implement `handle_parse` in CLI

**Files:**
- Modify: `crates/rairos-cli/src/main.rs`

- [ ] **Step 1: Replace handle_parse stub**

Replace existing `handle_parse` (lines 1176-1189) with:

```rust
fn handle_parse(db: &Database, id: &str) -> Result<()> {
    let paper = if let Ok(p) = db.get_paper(id) {
        p
    } else if let Ok(Some(p)) = db.get_paper_by_arxiv(id) {
        p
    } else {
        anyhow::bail!("Paper not found: {}", id);
    };

    let arxiv_id = paper.arxiv_id.as_deref().unwrap_or(&paper.id);
    println!("Parsing paper: {}", paper.title);
    println!("  arXiv: {}", arxiv_id);

    // Create pdfs directory
    let pdf_dir = dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("pdfs");
    std::fs::create_dir_all(&pdf_dir).context("Failed to create pdfs directory")?;

    let pdf_path = pdf_dir.join(format!("{}.pdf", arxiv_id));

    // Download PDF if not cached
    if !pdf_path.exists() {
        let pdf_url = format!("https://arxiv.org/pdf/{}.pdf", arxiv_id);
        println!("  Downloading from {} ...", pdf_url);
        let rt = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;
        rt.block_on(rairos_pdf::download_pdf(&pdf_url, &pdf_path))
            .context("Failed to download PDF")?;
        println!("  Downloaded to: {}", pdf_path.display());
    } else {
        println!("  Using cached PDF: {}", pdf_path.display());
    }

    // Extract text
    println!("  Extracting text ...");
    let text = rairos_pdf::extract_pdf_text(&pdf_path)
        .context("Failed to extract text from PDF")?;

    println!("\n  Text length: {} characters", text.len());

    // Preview
    let preview: String = text.chars().take(500).collect();
    println!("\n--- Preview (first 500 chars) ---\n{}", preview);

    // Update DB status
    db.update_paper_status(&paper.id, rairos_core::ParseStatus::Done)
        .context("Failed to update paper status")?;
    println!("\n[OK] Parse complete. Status set to 'done'.");
    Ok(())
}
```

- [ ] **Step 2: Build to verify compilation**

Run: `cargo build -p rairos-cli 2>&1`
Expected: Compilation succeeds

- [ ] **Step 3: Commit**

```
git add crates/rairos-cli/src/main.rs
git commit -m "feat: implement rairos parse handler with PDF text extraction"
```

---

### Task 3: Implement `handle_repl` in CLI

**Files:**
- Modify: `crates/rairos-cli/src/main.rs`

- [ ] **Step 1: Replace handle_repl stub**

Replace existing `handle_repl` (lines 2645-2655) with:

```rust
fn handle_repl(query: Option<String>) -> Result<()> {
    let db_path = PathBuf::from("rairos.db");
    if !db_path.exists() {
        eprintln!("Database not found. Run 'rairos init' first.");
        std::process::exit(1);
    }
    let db = Database::open(&db_path).context("Failed to open database")?;

    println!("=== Rairos REPL ===");
    println!("Type 'help' for commands, 'exit' to quit.\n");

    if let Some(q) = query {
        println!("Pre-loading papers matching: {}", q);
        match db.search_papers(&q, 10) {
            Ok(papers) if !papers.is_empty() => {
                println!("Found {} papers:\n", papers.len());
                for (i, p) in papers.iter().enumerate() {
                    let title = if p.title.len() > 60 {
                        format!("{}...", &p.title[..60])
                    } else {
                        p.title.clone()
                    };
                    let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                    println!("  {}. [{}] {} — {}", i + 1, &p.id[..8], title, arxiv);
                }
                println!();
            }
            _ => println!("No papers found for query: {}\n", q),
        }
    }

    loop {
        print!("rairos> ");
        std::io::Write::flush(&mut std::io::stdout()).ok();

        let mut input = String::new();
        if std::io::stdin().read_line(&mut input).is_err() || input.trim().is_empty() {
            continue;
        }
        let input = input.trim();

        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let cmd = parts[0].to_lowercase();
        let arg = parts.get(1).map(|s| s.trim()).unwrap_or("");

        match cmd.as_str() {
            "exit" | "quit" => {
                println!("Goodbye!");
                break;
            }
            "help" => {
                println!("\nCommands:");
                println!("  help                   Show this help");
                println!("  exit / quit            Exit REPL");
                println!("  search <query>         Search papers");
                println!("  show <id>              Show paper details");
                println!("  list [status]          List papers (pending/done/all)");
                println!("  stats                  Show DB statistics");
                println!("  gap <topic>            Detect research gaps");
                println!("  add <arxiv_id>         Import paper from arXiv");
                println!();
            }
            "search" if !arg.is_empty() => {
                match db.search_papers(arg, 20) {
                    Ok(papers) if papers.is_empty() => {
                        println!("No papers found for: {}", arg);
                    }
                    Ok(papers) => {
                        println!("Found {} papers:\n", papers.len());
                        for (i, p) in papers.iter().enumerate() {
                            let title = if p.title.len() > 60 {
                                format!("{}...", &p.title[..60])
                            } else {
                                p.title.clone()
                            };
                            let arxiv = p.arxiv_id.as_deref().unwrap_or("-");
                            println!("  {}. [{}] {} — {}", i + 1, &p.id[..8], title, arxiv);
                        }
                        println!();
                    }
                    Err(e) => println!("Error: {}\n", e),
                }
            }
            "show" if !arg.is_empty() => {
                if let Err(e) = handle_show(&db, arg, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "list" => {
                let status = if arg.is_empty() { None } else { Some(arg.to_string()) };
                if let Err(e) = handle_list(&db, status, None, &[], 20, 0, "published", "desc", "table") {
                    println!("Error: {}\n", e);
                }
            }
            "stats" => {
                if let Err(e) = handle_stats(&db, false, "table") {
                    println!("Error: {}\n", e);
                }
            }
            "gap" if !arg.is_empty() => {
                if let Err(e) = handle_gap(&db, arg, 5, "table", None) {
                    println!("Error: {}\n", e);
                }
            }
            "add" if !arg.is_empty() => {
                if let Err(e) = handle_add(&db, arg) {
                    println!("Error: {}\n", e);
                }
            }
            _ => {
                println!("Unknown command: {}. Type 'help' for available commands.\n", cmd);
            }
        }
    }
    Ok(())
}
```

- [ ] **Step 2: Build to verify compilation**

Run: `cargo build -p rairos-cli 2>&1`
Expected: Compilation succeeds

- [ ] **Step 3: Commit**

```
git add crates/rairos-cli/src/main.rs
git commit -m "feat: implement rairos repl with interactive command loop"
```

---

### Task 4: Run full test suite + final commit

- [ ] **Step 1: Run all Rust tests**

Run: `cargo test 2>&1 | grep -E "FAILED|test result"`
Expected: No FAILED, all test result lines show 0 failed

- [ ] **Step 2: Push**

```
git push
```
