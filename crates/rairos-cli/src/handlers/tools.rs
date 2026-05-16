//! Handlers for utility/ diagnostic commands:
//!   - diagnostics (ruff/pyright)
//!   - workspace snapshot
//!   - sysinfo

use anyhow::Result;

// ── Diagnostics (rairos-lsp-diagnostics) ─────────────────────────────────

/// Run LSP diagnostics (ruff/pyright) on a file or directory.
pub fn handle_diagnostics(ruff: bool, pyright: bool, path: &str) -> Result<()> {
    let run_ruff = ruff || !pyright;
    let run_pyright = !ruff || pyright;

    if run_ruff {
        let diags = crate::lsp_diagnostics::check_ruff(path);
        let output = crate::lsp_diagnostics::format_diagnostics(&diags, "Ruff");
        if !output.is_empty() {
            println!("{}", output);
        } else {
            println!("Ruff: no issues found");
        }
    }

    if run_pyright {
        let diags = crate::lsp_diagnostics::check_pyright(path);
        let output = crate::lsp_diagnostics::format_diagnostics(&diags, "Pyright");
        if !output.is_empty() {
            println!("{}", output);
        } else {
            println!("Pyright: no issues found");
        }
    }

    Ok(())
}

// ── Workspace Snapshot (rairos-workspace-snapshot) ────────────────────────

/// Create a workspace snapshot of the given path.
pub fn handle_workspace_snapshot(path: &str) -> Result<()> {
    use std::collections::HashMap;
    use std::path::PathBuf;

    let snap = crate::workspace_snapshot::WorkspaceSnapshot::new(None);

    // Collect all files in the given path
    let target = PathBuf::from(path);
    let mut paths = Vec::new();

    if target.is_dir() {
        for entry in walkdir(&target, 3) {
            paths.push(entry);
        }
    } else if target.exists() {
        paths.push(target);
    }

    if paths.is_empty() {
        println!("No files found at path: {}", path);
        return Ok(());
    }

    let session_id = format!("cli-{}", chrono::Utc::now().format("%Y%m%d-%H%M%S"));
    let step = 1u32;
    let metadata = HashMap::new();

    let result_path = snap.capture(&session_id, step, &paths, metadata);
    println!("Snapshot saved at: {}", result_path.display());
    println!("Files captured: {}", paths.len());

    Ok(())
}

/// Simple directory walker with max depth limit.
fn walkdir(path: &std::path::Path, max_depth: usize) -> Vec<std::path::PathBuf> {
    let mut results = Vec::new();
    let mut stack = vec![(path.to_path_buf(), 0usize)];

    while let Some((dir, depth)) = stack.pop() {
        if depth > max_depth {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                let Ok(ft) = entry.file_type() else { continue };
                if ft.is_dir() {
                    stack.push((entry.path(), depth + 1));
                } else if ft.is_file() {
                    results.push(entry.path());
                }
            }
        }
    }
    results
}

// ── Sysinfo (direct sysinfo crate) ───────────────────────────────────────

/// Show system information (CPU, memory, disk).
pub fn handle_sysinfo() -> Result<()> {
    use sysinfo::{Disks, System};

    let mut sys = System::new_all();

    // CPU
    sys.refresh_cpu_specifics(sysinfo::CpuRefreshKind::everything());
    let cpu_count = sys.cpus().len();
    let cpu_usage = sys.global_cpu_usage();
    println!("=== System Information ===");
    println!("CPU cores: {}", cpu_count);
    println!("CPU usage:  {:.1}%", cpu_usage);

    // Memory
    sys.refresh_memory();
    let total_mem = sys.total_memory();
    let used_mem = sys.used_memory();
    println!(
        "Memory:    {:.1} GB used / {:.1} GB total ({:.0}%)",
        used_mem as f64 / 1024.0 / 1024.0 / 1024.0,
        total_mem as f64 / 1024.0 / 1024.0 / 1024.0,
        if total_mem > 0 {
            (used_mem as f64 / total_mem as f64) * 100.0
        } else {
            0.0
        }
    );
    println!(
        "Swap:      {:.1} GB used / {:.1} GB total",
        sys.used_swap() as f64 / 1024.0 / 1024.0 / 1024.0,
        sys.total_swap() as f64 / 1024.0 / 1024.0 / 1024.0
    );

    // Disk
    let disks = Disks::new_with_refreshed_list();
    println!("\nDisks:");
    for disk in &disks {
        let total = disk.total_space();
        let available = disk.available_space();
        let used = total.saturating_sub(available);
        let percent = if total > 0 {
            (used as f64 / total as f64) * 100.0
        } else {
            0.0
        };
        println!(
            "  {}  {:.1} GB / {:.1} GB ({:.0}%)",
            disk.mount_point().display(),
            used as f64 / 1024.0 / 1024.0 / 1024.0,
            total as f64 / 1024.0 / 1024.0 / 1024.0,
            percent,
        );
    }

    // System info
    println!(
        "\nOS: {} {}",
        System::long_os_version().unwrap_or_default(),
        System::kernel_version().unwrap_or_default(),
    );
    println!(
        "Hostname: {}",
        System::host_name().unwrap_or_default()
    );

    Ok(())
}
