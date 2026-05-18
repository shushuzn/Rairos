//! Handlers for briefing generation commands.

use anyhow::Result;

pub fn handle_briefing_generate(arxiv_id: &str) -> Result<()> {
    use rairos_briefing_generator::BriefingGenerator;

    let generator = BriefingGenerator::new();
    let result = generator.generate(arxiv_id, false, None, None, None, None);

    match result.success {
        true => {
            println!("📋 Briefing generated for: {}", arxiv_id);
            if !result.markdown.is_empty() {
                println!("\n{}", result.markdown);
            }
        }
        false => {
            eprintln!("Error: {}", result.error);
        }
    }
    Ok(())
}

pub fn handle_briefing_list(_limit: usize) -> Result<()> {
    println!("📋 Briefings");
    println!("   Use 'rairos briefing <arxiv_id>' to generate");
    Ok(())
}
