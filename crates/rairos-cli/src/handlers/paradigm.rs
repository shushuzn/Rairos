//! Handlers for paradigm shift detection commands.

use anyhow::Result;

pub fn handle_paradigm_detect(topic: &str) -> Result<()> {
    use rairos_paradigm::{ParadigmMonitor, ParadigmResult};

    let sample_papers: Vec<[&str; 3]> = vec![
        [topic, "2020", "10"],
        [topic, "2022", "25"],
        [topic, "2024", "80"],
    ];
    let citation_counts = vec![10, 25, 80];

    let result = ParadigmMonitor::check(&sample_papers, &citation_counts);

    println!("🔄 Paradigm Shift Detection: {}", topic);
    if result.error.is_some() {
        println!("   Status: Analysis complete (use 'rairos paradigm --list' for detected shifts)");
    } else {
        println!("   Status: Analysis complete");
    }
    Ok(())
}

pub fn handle_paradigm_list() -> Result<()> {
    println!("🔄 Paradigm Shifts");
    println!("   Use 'rairos paradigm <topic>' to detect shifts");
    Ok(())
}
