//! rairos-provenance — Provenance tracking for paper content extraction.
//!
//! Ported from `research_loop/provenance.py` (54 LOC, pure stdlib).

use serde::{Deserialize, Serialize};

/// Absolute position of a content item within the concatenated paper text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperLocation {
    pub section: String,  // e.g. "3.2", "Abstract", "Algorithm"
    pub page: u32,        // 1-based page number (0 = unknown)
    pub char_start: u32,  // character offset in concatenated full-text
    pub char_end: u32,    // inclusive end offset
}

impl PaperLocation {
    pub fn short_ref(&self) -> String {
        format!("§{}p{}@{}", self.section, self.page, self.char_start)
    }
}

/// An equation with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationSource {
    pub index: u32,
    pub equation: String,       // raw LaTeX string
    pub location: PaperLocation,
}

impl EquationSource {
    pub fn tag(&self) -> String {
        format!("@eq[{}]", self.index)
    }
}

/// A claim with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSource {
    pub index: u32,
    pub claim: String,
    pub location: PaperLocation,
}

impl ClaimSource {
    pub fn tag(&self) -> String {
        format!("@claim[{}]", self.index)
    }
}

/// An algorithm description with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmSource {
    pub index: u32,
    pub description: String,
    pub location: PaperLocation,
}

impl AlgorithmSource {
    pub fn tag(&self) -> String {
        format!("@algo[{}]", self.index)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_location_short_ref() {
        let loc = PaperLocation {
            section: "3.2".to_string(),
            page: 5,
            char_start: 1234,
            char_end: 4567,
        };
        assert_eq!(loc.short_ref(), "§3.2p5@1234");
    }

    #[test]
    fn test_equation_source_tag() {
        let loc = PaperLocation {
            section: "Abstract".to_string(),
            page: 1,
            char_start: 0,
            char_end: 100,
        };
        let eq = EquationSource {
            index: 2,
            equation: "E = mc^2".to_string(),
            location: loc,
        };
        assert_eq!(eq.tag(), "@eq[2]");
        assert_eq!(eq.equation, "E = mc^2");
    }

    #[test]
    fn test_claim_source_tag() {
        let loc = PaperLocation {
            section: "2.1".to_string(),
            page: 3,
            char_start: 500,
            char_end: 800,
        };
        let claim = ClaimSource {
            index: 7,
            claim: "Transformers scale sub-quadratically".to_string(),
            location: loc,
        };
        assert_eq!(claim.tag(), "@claim[7]");
    }

    #[test]
    fn test_algorithm_source_tag() {
        let loc = PaperLocation {
            section: "Algorithm".to_string(),
            page: 4,
            char_start: 900,
            char_end: 2000,
        };
        let algo = AlgorithmSource {
            index: 1,
            description: "Backpropagation".to_string(),
            location: loc,
        };
        assert_eq!(algo.tag(), "@algo[1]");
    }

    #[test]
    fn test_paper_location_page_zero() {
        let loc = PaperLocation {
            section: "Appendix".to_string(),
            page: 0,
            char_start: 0,
            char_end: 50,
        };
        assert_eq!(loc.short_ref(), "§Appendixp0@0");
    }
}
