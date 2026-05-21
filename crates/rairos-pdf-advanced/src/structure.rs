//! Structured document representation and analysis.

use serde::{Deserialize, Serialize};

/// A structured document with parsed content
#[derive(Debug, Clone)]
pub struct StructuredDocument {
    /// Document metadata
    pub metadata: DocumentMetadata,
    /// Extracted sections
    pub sections: Vec<Section>,
    /// Extracted tables
    pub tables: Vec<ParsedTable>,
    /// Extracted figures
    pub figures: Vec<ParsedFigure>,
    /// References/citations
    pub references: Vec<Reference>,
    /// Full text content
    pub full_text: String,
}

/// Document metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DocumentMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    pub abstract_text: Option<String>,
    pub keywords: Vec<String>,
    pub doi: Option<String>,
    pub arxiv_id: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub publisher: Option<String>,
}

impl Default for DocumentMetadata {
    fn default() -> Self {
        Self {
            title: None,
            authors: vec![],
            abstract_text: None,
            keywords: vec![],
            doi: None,
            arxiv_id: None,
            year: None,
            journal: None,
            publisher: None,
        }
    }
}

/// A section of the document
#[derive(Debug, Clone)]
pub struct Section {
    /// Section number (e.g., "1", "2.3", "A")
    pub number: Option<String>,
    /// Section heading/title
    pub heading: String,
    /// Section level (1 for top-level, 2 for subsection, etc.)
    pub level: usize,
    /// Content paragraphs
    pub paragraphs: Vec<Paragraph>,
    /// Subsections
    pub subsections: Vec<Section>,
    /// Page number where section starts
    pub page_start: Option<usize>,
    /// Page number where section ends
    pub page_end: Option<usize>,
}

impl Section {
    /// Create a new section with heading
    pub fn new(heading: impl Into<String>) -> Self {
        Self {
            number: None,
            heading: heading.into(),
            level: 1,
            paragraphs: vec![],
            subsections: vec![],
            page_start: None,
            page_end: None,
        }
    }

    /// Check if this is an introduction section
    pub fn is_introduction(&self) -> bool {
        let h = self.heading.to_lowercase();
        h.contains("introduction") || h.contains("background") || h.contains("overview")
    }

    /// Check if this is a conclusion section
    pub fn is_conclusion(&self) -> bool {
        let h = self.heading.to_lowercase();
        h.contains("conclusion") || h.contains("summary") || h.contains("discussion")
    }

    /// Get total character count of section
    pub fn char_count(&self) -> usize {
        let mut count = self.heading.len();
        for p in &self.paragraphs {
            count += p.text.len();
        }
        for s in &self.subsections {
            count += s.char_count();
        }
        count
    }
}

/// A paragraph within a section
#[derive(Debug, Clone)]
pub struct Paragraph {
    /// Paragraph text
    pub text: String,
    /// Sentence count
    pub sentence_count: usize,
    /// Contains equation
    pub has_equation: bool,
    /// Contains citation reference
    pub has_citation: bool,
    /// Referenced paper IDs
    pub citations: Vec<String>,
}

impl Paragraph {
    /// Create a new paragraph
    pub fn new(text: impl Into<String>) -> Self {
        let text = text.into();
        let has_equation = text.contains("$$") || text.contains("$E=") || text.contains("\\begin{");
        let sentence_count = text.split(|c: char| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .count();

        Self {
            text,
            sentence_count,
            has_equation,
            has_citation: false,
            citations: vec![],
        }
    }

    /// Check if paragraph is short (less than 3 sentences)
    pub fn is_short(&self) -> bool {
        self.sentence_count < 3
    }

    /// Check if paragraph is a list item
    pub fn is_list_item(&self) -> bool {
        self.text.trim_start().starts_with('-')
            || self.text.trim_start().starts_with('•')
            || self.text.trim_start().starts_with('*')
            || self.text.trim_start().starts_with("1.")
            || self.text.trim_start().starts_with("2.")
    }
}

/// A parsed table
#[derive(Debug, Clone)]
pub struct ParsedTable {
    /// Table label (e.g., "Table 1")
    pub label: String,
    /// Table caption
    pub caption: String,
    /// Column headers
    pub headers: Vec<String>,
    /// Table rows (excluding header row)
    pub rows: Vec<Vec<String>>,
    /// Page number
    pub page: Option<usize>,
    /// Is a results table (has numbers/metrics)
    pub is_results: bool,
}

impl ParsedTable {
    /// Check if table appears to be a results table
    pub fn detect_results_table(&self) -> bool {
        // Check if headers or data contain common metric keywords
        let metric_keywords = ["accuracy", "precision", "recall", "f1", "score", "loss", "auc", "bleu", "rouge"];

        let all_text: String = self.headers.iter()
            .chain(self.rows.iter().flat_map(|r| r.iter()))
            .map(|s| s.to_lowercase())
            .collect();

        metric_keywords.iter().any(|kw| all_text.contains(kw))
    }

    /// Extract model names from table (for results tables)
    pub fn extract_model_names(&self) -> Vec<String> {
        if !self.is_results || self.rows.is_empty() {
            return vec![];
        }

        // First column often contains model names
        self.rows.iter()
            .filter_map(|row| row.first())
            .filter(|name| !name.is_empty() && !name.chars().all(|c| c.is_numeric() || c == '.'))
            .cloned()
            .collect()
    }
}

/// A parsed figure
#[derive(Debug, Clone)]
pub struct ParsedFigure {
    /// Figure label (e.g., "Figure 1")
    pub label: String,
    /// Figure caption
    pub caption: String,
    /// Subfigure labels (e.g., ["(a)", "(b)"])
    pub subfigures: Vec<String>,
    /// Page number
    pub page: Option<usize>,
}

impl ParsedFigure {
    /// Get the figure number
    pub fn figure_number(&self) -> Option<usize> {
        self.label
            .split_whitespace()
            .last()
            .and_then(|s| s.parse().ok())
    }
}

/// A reference/citation
#[derive(Debug, Clone)]
pub struct Reference {
    /// Raw citation text
    pub raw_text: String,
    /// Normalized title
    pub title: Option<String>,
    /// Authors
    pub authors: Vec<String>,
    /// Publication year
    pub year: Option<i32>,
    /// Journal/conference name
    pub venue: Option<String>,
    /// DOI
    pub doi: Option<String>,
    /// arXiv ID
    pub arxiv_id: Option<String>,
}

impl Reference {
    /// Check if this is an arXiv reference
    pub fn is_arxiv(&self) -> bool {
        self.arxiv_id.is_some()
            || self.raw_text.to_lowercase().contains("arxiv")
            || self.raw_text.contains("abs/") || self.raw_text.contains("arXiv:")
    }

    /// Get short citation (Author, Year)
    pub fn short_citation(&self) -> String {
        let author = self.authors.first()
            .map(|a| a.split_whitespace().last().unwrap_or(a))
            .unwrap_or("Unknown");
        let year = self.year.map(|y| y.to_string()).unwrap_or("n.d.".to_string());
        format!("{}, {}", author, year)
    }
}

/// Document structure analyzer
pub struct DocumentAnalyzer;

impl DocumentAnalyzer {
    /// Analyze a document and extract structure
    pub fn analyze(full_text: &str) -> StructuredDocument {
        let sections = Self::extract_sections(full_text);
        let tables = Self::extract_tables(full_text);
        let figures = Self::extract_figures(full_text);

        StructuredDocument {
            metadata: DocumentMetadata::default(),
            sections,
            tables,
            figures,
            references: vec![],
            full_text: full_text.to_string(),
        }
    }

    /// Extract sections from text
    fn extract_sections(text: &str) -> Vec<Section> {
        let mut sections = Vec::new();
        let lines: Vec<&str> = text.lines().collect();

        let mut current_heading = String::new();
        let mut current_content = Vec::new();
        let mut level = 1;

        for line in lines {
            let trimmed = line.trim();

            // Detect heading patterns
            if Self::is_heading(trimmed) {
                // Save previous section
                if !current_heading.is_empty() {
                    sections.push(Section {
                        number: None,
                        heading: current_heading.clone(),
                        level,
                        paragraphs: current_content.drain(..)
                            .map(|t| Paragraph::new(t))
                            .collect(),
                        subsections: vec![],
                        page_start: None,
                        page_end: None,
                    });
                }

                current_heading = Self::clean_heading(trimmed);
                level = Self::detect_heading_level(trimmed);
            } else if !trimmed.is_empty() {
                current_content.push(trimmed.to_string());
            }
        }

        // Don't forget last section
        if !current_heading.is_empty() {
            sections.push(Section {
                number: None,
                heading: current_heading,
                level,
                paragraphs: current_content.drain(..)
                    .map(|t| Paragraph::new(t))
                    .collect(),
                subsections: vec![],
                page_start: None,
                page_end: None,
            });
        }

        sections
    }

    fn is_heading(line: &str) -> bool {
        let trimmed = line.trim();
        // Numbered heading: "1. Introduction", "2.3 Methods"
        if trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
            return trimmed.len() < 100 && (trimmed.contains('.') || trimmed.contains(':'));
        }
        // ALL CAPS heading (common in papers)
        if trimmed.len() < 80 && trimmed == trimmed.to_uppercase() && trimmed.len() > 3 {
            return true;
        }
        // Short line ending with colon (section headers)
        if trimmed.len() < 60 && trimmed.ends_with(':') && trimmed.chars().all(|c| c.is_alphabetic() || c.is_whitespace() || c == ':') {
            return true;
        }
        false
    }

    fn clean_heading(heading: &str) -> String {
        heading.trim()
            .trim_start_matches(|c: char| c.is_ascii_digit() || c == '.' || c == ' ')
            .trim()
            .to_string()
    }

    fn detect_heading_level(heading: &str) -> usize {
        let trimmed = heading.trim();
        if let Some(dot_pos) = trimmed.find('.') {
            if trimmed[..dot_pos].parse::<usize>().is_ok() {
                return trimmed[..dot_pos].parse::<usize>().unwrap_or(1) + 1;
            }
        }
        1
    }

    /// Extract tables from text
    fn extract_tables(text: &str) -> Vec<ParsedTable> {
        let mut tables = Vec::new();

        // Simple table detection - look for table captions
        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Table ") || trimmed.starts_with("table ") {
                if let Some(caption) = Self::extract_table_caption(trimmed) {
                    tables.push(ParsedTable {
                        label: trimmed.to_string(),
                        caption,
                        headers: vec![],
                        rows: vec![],
                        page: None,
                        is_results: false,
                    });
                }
            }
        }

        tables
    }

    fn extract_table_caption(line: &str) -> Option<String> {
        // Extract caption after label (e.g., "Table 1: Performance comparison")
        if let Some(colon_pos) = line.find(':') {
            Some(line[colon_pos + 1..].trim().to_string())
        } else {
            Some(line.replace("Table ", "").replace("table ", "").to_string())
        }
    }

    /// Extract figures from text
    fn extract_figures(text: &str) -> Vec<ParsedFigure> {
        let mut figures = Vec::new();

        for line in text.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("Figure ") || trimmed.starts_with("Figure ") {
                if let Some(caption) = Self::extract_figure_caption(trimmed) {
                    figures.push(ParsedFigure {
                        label: trimmed.to_string(),
                        caption,
                        subfigures: vec![],
                        page: None,
                    });
                }
            }
        }

        figures
    }

    fn extract_figure_caption(line: &str) -> Option<String> {
        if let Some(dot_pos) = line.find('.') {
            Some(line[dot_pos + 1..].trim().to_string())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heading_detection() {
        assert!(DocumentAnalyzer::is_heading("1. Introduction"));
        assert!(DocumentAnalyzer::is_heading("2.3 Methods"));
        assert!(DocumentAnalyzer::is_heading("BACKGROUND"));
        assert!(DocumentAnalyzer::is_heading("Experimental Setup:"));
        assert!(!DocumentAnalyzer::is_heading("This is a regular paragraph."));
    }

    #[test]
    fn test_is_heading_numbered() {
        // Numbered headings like "1. Introduction", "2.3 Methods"
        assert!(DocumentAnalyzer::is_heading("1. Introduction"));
        assert!(DocumentAnalyzer::is_heading("2.3 Methods"));
        assert!(DocumentAnalyzer::is_heading("10. Conclusion"));
        assert!(DocumentAnalyzer::is_heading("1.2.1 Nested Section"));
        // Too long numbered heading should not match
        assert!(!DocumentAnalyzer::is_heading("1. This is a very long heading that exceeds the typical length limit for a heading in a document and should not be detected as such."));
    }

    #[test]
    fn test_is_heading_all_caps() {
        // ALL CAPS headings
        assert!(DocumentAnalyzer::is_heading("BACKGROUND"));
        assert!(DocumentAnalyzer::is_heading("METHODS"));
        assert!(DocumentAnalyzer::is_heading("RESULTS AND DISCUSSION"));
        // Must be > 3 chars
        assert!(!DocumentAnalyzer::is_heading("AB"));
        // Mixed case should not match
        assert!(!DocumentAnalyzer::is_heading("Background"));
    }

    #[test]
    fn test_is_heading_colon_ending() {
        // Short line ending with colon
        assert!(DocumentAnalyzer::is_heading("Introduction:"));
        assert!(DocumentAnalyzer::is_heading("Experimental Setup:"));
        assert!(DocumentAnalyzer::is_heading("Conclusion:"));
        // Too long should not match
        assert!(!DocumentAnalyzer::is_heading("This is a very long heading that ends with a colon: but it should not be detected as a heading because it is too long."));
        // Must end with colon
        assert!(!DocumentAnalyzer::is_heading("Introduction"));
    }

    #[test]
    fn test_paragraph_sentence_count() {
        let p = Paragraph::new("This is sentence one. This is sentence two. This is sentence three.");
        assert_eq!(p.sentence_count, 3);
    }

    #[test]
    fn test_paragraph_sentence_count_various_punctuation() {
        // Question marks
        let p1 = Paragraph::new("Is this a question? Yes it is.");
        assert_eq!(p1.sentence_count, 2);

        // Exclamation marks
        let p2 = Paragraph::new("Hello! Welcome to the show.");
        assert_eq!(p2.sentence_count, 2);

        // Mixed punctuation
        let p3 = Paragraph::new("What is this? It is amazing! This is a test.");
        assert_eq!(p3.sentence_count, 3);
    }

    #[test]
    fn test_paragraph_sentence_count_empty_segments() {
        // Multiple periods should not inflate count
        let p = Paragraph::new("One... Two... Three.");
        assert_eq!(p.sentence_count, 3);

        // Trailing punctuation
        let p2 = Paragraph::new("Hello world.");
        assert_eq!(p2.sentence_count, 1);
    }

    #[test]
    fn test_paragraph_has_equation() {
        let p1 = Paragraph::new("The energy is $$E = mc^2$$.");
        assert!(p1.has_equation);

        let p2 = Paragraph::new("Using $E=mc^2$ formula.");
        assert!(p2.has_equation);

        let p3 = Paragraph::new("Using \\begin{equation} formula.");
        assert!(p3.has_equation);

        let p4 = Paragraph::new("This is a normal paragraph.");
        assert!(!p4.has_equation);
    }

    #[test]
    fn test_paragraph_is_short() {
        let p1 = Paragraph::new("Short.");
        assert!(p1.is_short());

        // 2 sentences is still short (< 3)
        let p2 = Paragraph::new("This is sentence one. This is sentence two.");
        assert!(p2.is_short());

        // 3 sentences is not short
        let p3 = Paragraph::new("Sentence one. Sentence two. Sentence three.");
        assert!(!p3.is_short());
    }

    #[test]
    fn test_paragraph_is_list_item() {
        let p1 = Paragraph::new("- This is a list item");
        assert!(p1.is_list_item());

        let p2 = Paragraph::new("• Bullet point");
        assert!(p2.is_list_item());

        let p3 = Paragraph::new("* Asterisk item");
        assert!(p3.is_list_item());

        let p4 = Paragraph::new("1. First numbered item");
        assert!(p4.is_list_item());

        let p5 = Paragraph::new("This is not a list item");
        assert!(!p5.is_list_item());
    }

    #[test]
    fn test_section_is_introduction() {
        let s1 = Section::new("Introduction");
        assert!(s1.is_introduction());

        let s2 = Section::new("1. Introduction");
        assert!(s2.is_introduction());

        let s3 = Section::new("BACKGROUND AND MOTIVATION");
        assert!(s3.is_introduction());

        let s4 = Section::new("Overview of Methods");
        assert!(s4.is_introduction());

        let s5 = Section::new("Results");
        assert!(!s5.is_introduction());
    }

    #[test]
    fn test_section_is_conclusion() {
        let s1 = Section::new("Conclusion");
        assert!(s1.is_conclusion());

        let s2 = Section::new("Summary");
        assert!(s2.is_conclusion());

        let s3 = Section::new("Discussion and Conclusions");
        assert!(s3.is_conclusion());

        let s4 = Section::new("Introduction");
        assert!(!s4.is_conclusion());
    }

    #[test]
    fn test_section_char_count() {
        let mut section = Section::new("Title");
        section.paragraphs.push(Paragraph::new("Hello world."));
        section.paragraphs.push(Paragraph::new("Another paragraph."));

        // "Title" = 5, "Hello world." = 12, "Another paragraph." = 18
        // Total = 35
        assert_eq!(section.char_count(), 35);
    }

    #[test]
    fn test_paragraph_new() {
        let p = Paragraph::new("This is a test paragraph. It contains multiple sentences.");
        assert_eq!(p.sentence_count, 2);
        assert!(!p.has_equation);
        assert!(!p.has_citation);
        assert!(p.citations.is_empty());
    }
}
