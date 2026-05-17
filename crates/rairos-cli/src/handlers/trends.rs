//! Handlers for trend analysis commands.

use anyhow::Result;
use chrono::Datelike;

pub fn handle_trends_analyze(topic: &str, years: Option<i32>) -> Result<()> {
    use rairos_trend_analyzer::TrendAnalyzer;
    
    let analyzer = TrendAnalyzer::new();
    let year_range = years.map(|y| {
        let current_year = chrono::Utc::now().year();
        (current_year - y, current_year)
    });
    
    let result = analyzer.analyze(topic, year_range, 5, &[]);
    
    if result.total_papers == 0 {
        println!("No papers found for topic '{}'.", topic);
        println!("Add papers to your database first using 'rairos add <arxiv_id>'");
        return Ok(());
    }
    
    println!("{}", analyzer.render_result(&result));
    Ok(())
}

pub fn handle_trends_mermaid(topic: &str, years: Option<i32>) -> Result<()> {
    use rairos_trend_analyzer::TrendAnalyzer;
    
    let analyzer = TrendAnalyzer::new();
    let year_range = years.map(|y| {
        let current_year = chrono::Utc::now().year();
        (current_year - y, current_year)
    });
    
    let result = analyzer.analyze(topic, year_range, 5, &[]);
    
    if result.total_papers == 0 {
        println!("No papers found for topic '{}'.", topic);
        return Ok(());
    }
    
    println!("{}", analyzer.render_mermaid_timeline(&result));
    Ok(())
}
