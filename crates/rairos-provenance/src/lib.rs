//! Provenance — track paper location for AI-extracted content.
//!
//! Tracks the absolute position of content (claims, equations, algorithms)
//! within the concatenated paper text for citation and verification.
//!
//! Python original: `research_loop/provenance.py` (54 lines)

use serde::{Deserialize, Serialize};

// ─── PaperLocation ────────────────────────────────────────────────────────────

/// Absolute position of a content item within the concatenated paper text.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PaperLocation {
    /// e.g. "3.2", "Abstract", "Algorithm"
    pub section: String,
    /// 1-based page number (0 = unknown)
    pub page: u32,
    /// character offset in concatenated full-text
    pub char_start: u32,
    /// inclusive end offset
    pub char_end: u32,
}

impl PaperLocation {
    pub fn new(section: &str, page: u32, char_start: u32, char_end: u32) -> Self {
        Self {
            section: section.to_string(),
            page,
            char_start,
            char_end,
        }
    }

    /// Short reference string, e.g. "§3.2p5@1023"
    pub fn short_ref(&self) -> String {
        format!("§{}p{}@{}", self.section, self.page, self.char_start)
    }
}

// ─── EquationSource ───────────────────────────────────────────────────────────

/// An equation with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EquationSource {
    pub index: u32,
    /// raw LaTeX string
    pub equation: String,
    pub location: PaperLocation,
}

impl EquationSource {
    pub fn new(index: u32, equation: &str, location: PaperLocation) -> Self {
        Self {
            index,
            equation: equation.to_string(),
            location,
        }
    }

    /// Citation tag, e.g. "@eq[3]"
    pub fn tag(&self) -> String {
        format!("@eq[{}]", self.index)
    }
}

// ─── ClaimSource ──────────────────────────────────────────────────────────────

/// A claim with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClaimSource {
    pub index: u32,
    pub claim: String,
    pub location: PaperLocation,
}

impl ClaimSource {
    pub fn new(index: u32, claim: &str, location: PaperLocation) -> Self {
        Self {
            index,
            claim: claim.to_string(),
            location,
        }
    }

    /// Citation tag, e.g. "@claim[7]"
    pub fn tag(&self) -> String {
        format!("@claim[{}]", self.index)
    }
}

// ─── AlgorithmSource ─────────────────────────────────────────────────────────

/// An algorithm description with its provenance.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlgorithmSource {
    pub index: u32,
    pub description: String,
    pub location: PaperLocation,
}

impl AlgorithmSource {
    pub fn new(index: u32, description: &str, location: PaperLocation) -> Self {
        Self {
            index,
            description: description.to_string(),
            location,
        }
    }

    /// Citation tag, e.g. "@algo[1]"
    pub fn tag(&self) -> String {
        format!("@algo[{}]", self.index)
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paper_location_short_ref() {
        let loc = PaperLocation::new("3.2", 5, 1023, 1150);
        assert_eq!(loc.short_ref(), "§3.2p5@1023");
    }

    #[test]
    fn test_equation_source_tag() {
        let loc = PaperLocation::new("2.1", 1, 0, 50);
        let eq = EquationSource::new(3, "E = mc^2", loc);
        assert_eq!(eq.tag(), "@eq[3]");
    }

    #[test]
    fn test_claim_source_tag() {
        let loc = PaperLocation::new("Abstract", 1, 0, 200);
        let claim = ClaimSource::new(7, "The model achieves 95% accuracy", loc);
        assert_eq!(claim.tag(), "@claim[7]");
    }

    #[test]
    fn test_algorithm_source_tag() {
        let loc = PaperLocation::new("Algorithm 1", 3, 500, 1200);
        let algo = AlgorithmSource::new(1, "Backpropagation steps", loc);
        assert_eq!(algo.tag(), "@algo[1]");
    }

    #[test]
    fn test_serde_round_trip() {
        let loc = PaperLocation::new("1.0", 2, 100, 200);
        let json = serde_json::to_string(&loc).unwrap();
        let restored: PaperLocation = serde_json::from_str(&json).unwrap();
        assert_eq!(restored.section, "1.0");
        assert_eq!(restored.page, 2);
    }
}
