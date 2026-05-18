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

use rairos_core::{Database, ParseStatus};

pub fn handle_queue(
    db: &Database,
    add: Option<&str>,
    list: bool,
    pending: bool,
    dequeue: bool,
    cancel: Option<i64>,
    clear: bool,
    format: &str,
) -> Result<()> {
    if list {
        let jobs = db.get_queue_jobs(None, 100)?;
        if format == "json" {
            println!("{}", serde_json::to_string_pretty(&jobs)?);
        } else {
            for j in &jobs {
                println!(
                    "[{}] {} ({}) priority={} status={}",
                    j.id, j.paper_id, j.job_type, j.priority, j.status
                );
            }
        }
        if jobs.is_empty() {
            println!("Queue empty");
        }
    } else if pending {
        let all_papers = db.list_papers(Some(ParseStatus::Pending), 200, 0)?;
        if all_papers.is_empty() {
            println!("No pending papers");
        } else {
            println!("{} paper(s) awaiting processing:", all_papers.len());
            for p in &all_papers {
                println!(
                    "  {} [{}]",
                    p.id,
                    p.arxiv_id.as_deref().unwrap_or("no-arxiv")
                );
            }
        }
    } else if dequeue {
        match db.dequeue_job()? {
            Some(job) => println!("Dequeued: {} (id={})", job.paper_id, job.id),
            None => println!("Queue empty"),
        }
    } else if let Some(paper_id) = add {
        db.enqueue_job(paper_id, "parse", 5)?;
        println!("Added {} to queue", paper_id);
    } else if let Some(job_id) = cancel {
        if db.cancel_job(job_id)? {
            println!("Cancelled job {}", job_id);
        } else {
            println!("No such job {}", job_id);
        }
    } else if clear {
        let n = db.clear_pending_papers()?;
        println!("Cleared {} pending paper(s)", n);
    } else {
        println!("Use --list, --dequeue, --add UID, --cancel JOB_ID, or --clear");
    }
    Ok(())
}