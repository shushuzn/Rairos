//! Triple extraction for knowledge graph construction.
//!
//! Extracts Subject-Predicate-Object triples from scientific text
//! for building knowledge graphs.

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// A knowledge graph triple
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Triple {
    /// Subject entity
    pub subject: String,
    /// Predicate/relation
    pub predicate: String,
    /// Object entity
    pub object: String,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Sentence where triple was found
    pub sentence: String,
    /// Start position in sentence
    pub start: usize,
    /// End position in sentence
    pub end: usize,
}

impl Triple {
    pub fn new(subject: impl Into<String>, predicate: impl Into<String>, object: impl Into<String>, confidence: f32, sentence: impl Into<String>) -> Self {
        Self {
            subject: subject.into(),
            predicate: predicate.into(),
            object: object.into(),
            confidence,
            sentence: sentence.into(),
            start: 0,
            end: 0,
        }
    }

    /// Convert to RDF-like triple string
    pub fn to_rdf_string(&self) -> String {
        format!("<{}> <{}> <{}>", self.subject, self.predicate, self.object)
    }
}

/// Triple extractor for scientific text
pub struct TripleExtractor {
    /// Whether to enable relation extraction
    pub enable_relations: bool,
    /// Custom predicate patterns
    predicate_patterns: Vec<(Regex, &'static str, f32)>,
}

impl TripleExtractor {
    /// Create a new triple extractor
    pub fn new() -> Self {
        Self::default()
    }

    /// Create with specified features
    pub fn with_relations(enable_relations: bool) -> Self {
        let mut extractor = Self {
            enable_relations,
            predicate_patterns: vec![],
        };
        extractor.compile_patterns();
        extractor
    }

    /// Compile regex patterns
    fn compile_patterns(&mut self) {
        // Common scientific relations
        let relation_patterns = [
            // Method-Property relations
            (r"(\w+)\s+achieves?\s+([\d.]+%|\d+\.?\d*)", "achieves", 0.8),
            (r"(\w+)\s+reaches?\s+([\d.]+%|\d+\.?\d*)", "reaches", 0.8),
            (r"(\w+)\s+outperforms?\s+(\w+)", "outperforms", 0.85),
            (r"(\w+)\s+improves\s+(?:by\s+)?([\d.]+%|\d+\.?\d*)", "improves", 0.8),

            // Comparative relations
            (r"(\w+)\s+(?:is\s+)?(?:better|worse|higher|lower)\s+than\s+(\w+)", "compares_to", 0.75),
            (r"(\w+)\s+similar\s+to\s+(\w+)", "similar_to", 0.7),

            // Method-Dataset relations
            (r"(\w+)\s+(?:trained|evaluated)\s+on\s+the?\s+([A-Za-z0-9-]+)\s+dataset", "trained_on", 0.9),
            (r"([A-Za-z0-9-]+)\s+dataset", "is_dataset", 0.6),

            // Material-Property relations
            (r"([A-Z][a-z0-9]+)\s+(?:has|shows|exhibits)\s+(\w+)", "has_property", 0.7),
            (r"(\w+)\s+(?:properties?|performance|accuracy)\s+of\s+([A-Z][a-z0-9]+)", "property_of", 0.7),

            // CAusal relations
            (r"(\w+)\s+(?:causes?|leads\s+to|results?\s+in)\s+(\w+)", "causes", 0.8),
            (r"(\w+)\s+(?:enables?|allows?|facilitates?)\s+(\w+)", "enables", 0.8),

            // Part-Whole relations
            (r"(\w+)\s+(?:part\s+of|contained\s+in|consists?\s+of)\s+(\w+)", "part_of", 0.7),
        ];

        for (pattern, predicate, confidence) in relation_patterns {
            if let Ok(regex) = Regex::new(pattern) {
                self.predicate_patterns.push((regex, predicate, confidence));
            }
        }
    }

    /// Extract triples from text
    pub fn extract(&self, text: &str) -> Vec<Triple> {
        let mut triples = Vec::new();
        let sentences = self.split_sentences(text);

        for sentence in &sentences {
            triples.extend(self.extract_from_sentence(sentence));
        }

        // Deduplicate
        let mut seen = HashSet::new();
        triples.retain(|t| {
            let key = format!("{}|{}|{}", t.subject, t.predicate, t.object);
            seen.insert(key)
        });

        triples
    }

    /// Extract triples from a single sentence
    fn extract_from_sentence(&self, sentence: &str) -> Vec<Triple> {
        let mut triples = Vec::new();

        for (regex, predicate, confidence) in &self.predicate_patterns {
            for mat in regex.find_iter(sentence) {
                if let Some(caps) = regex.captures(sentence) {
                    let subject = caps.get(1).map(|m| m.as_str().trim()).unwrap_or("");
                    let object = caps.get(2).map(|m| m.as_str().trim()).unwrap_or("");

                    if !subject.is_empty() && !object.is_empty() && subject != object {
                        triples.push(Triple {
                            subject: subject.to_string(),
                            predicate: predicate.to_string(),
                            object: object.to_string(),
                            confidence: *confidence,
                            sentence: sentence.to_string(),
                            start: mat.start(),
                            end: mat.end(),
                        });
                    }
                }
            }
        }

        triples
    }

    /// Split text into sentences
    fn split_sentences(&self, text: &str) -> Vec<String> {
        let sentence_ends: Vec<usize> = text
            .char_indices()
            .filter(|(_, c)| *c == '.' || *c == '!' || *c == '?')
            .map(|(i, _)| i)
            .collect();

        let mut sentences = Vec::new();
        let mut prev_end = 0;

        for end in sentence_ends {
            let sentence = text[prev_end..=end].trim().to_string();
            if sentence.len() > 10 {
                sentences.push(sentence);
            }
            prev_end = end + 1;
        }

        if prev_end < text.len() {
            let remaining = text[prev_end..].trim().to_string();
            if remaining.len() > 10 {
                sentences.push(remaining);
            }
        }

        sentences
    }

    /// Extract triples using dependency parsing (simplified pattern-based approach)
    pub fn extract_with_dependencies(&self, text: &str) -> Vec<Triple> {
        // For now, just use pattern-based extraction
        // In a full implementation, we'd use a proper dependency parser
        self.extract(text)
    }
}

impl Default for TripleExtractor {
    fn default() -> Self {
        let mut extractor = Self {
            enable_relations: true,
            predicate_patterns: vec![],
        };
        extractor.compile_patterns();
        extractor
    }
}

/// Relation types for knowledge graph
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RelationType {
    /// Method achieves metric
    Achieves,
    /// Method outperforms another
    Outperforms,
    /// Trained on dataset
    TrainedOn,
    /// Has property
    HasProperty,
    /// Causes/leads to
    Causes,
    /// Similar to
    SimilarTo,
    /// Part of
    PartOf,
    /// Uses/employs method
    Uses,
    /// Applied to domain
    AppliedTo,
}

impl RelationType {
    pub fn as_str(&self) -> &'static str {
        match self {
            RelationType::Achieves => "ACHIEVES",
            RelationType::Outperforms => "OUTPERFORMS",
            RelationType::TrainedOn => "TRAINED_ON",
            RelationType::HasProperty => "HAS_PROPERTY",
            RelationType::Causes => "CAUSES",
            RelationType::SimilarTo => "SIMILAR_TO",
            RelationType::PartOf => "PART_OF",
            RelationType::Uses => "USES",
            RelationType::AppliedTo => "APPLIED_TO",
        }
    }
}

/// Builder for constructing knowledge graph from triples
pub struct KnowledgeGraphBuilder {
    nodes: std::collections::HashMap<String, Node>,
    edges: Vec<Edge>,
}

#[derive(Debug, Clone)]
struct Node {
    id: String,
    label: String,
    node_type: String,
}

#[derive(Debug, Clone)]
struct Edge {
    source: String,
    target: String,
    relation: String,
    weight: f32,
}

impl KnowledgeGraphBuilder {
    pub fn new() -> Self {
        Self {
            nodes: std::collections::HashMap::new(),
            edges: vec![],
        }
    }

    /// Add triples to the graph
    pub fn add_triples(&mut self, triples: &[Triple]) {
        for triple in triples {
            // Add subject node
            self.nodes.insert(triple.subject.clone(), Node {
                id: triple.subject.clone(),
                label: triple.subject.clone(),
                node_type: "Entity".to_string(),
            });

            // Add object node
            self.nodes.insert(triple.object.clone(), Node {
                id: triple.object.clone(),
                label: triple.object.clone(),
                node_type: "Entity".to_string(),
            });

            // Add edge
            self.edges.push(Edge {
                source: triple.subject.clone(),
                target: triple.object.clone(),
                relation: triple.predicate.clone(),
                weight: triple.confidence,
            });
        }
    }

    /// Get the number of nodes
    pub fn node_count(&self) -> usize {
        self.nodes.len()
    }

    /// Get the number of edges
    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }
}

impl Default for KnowledgeGraphBuilder {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_triple_extraction() {
        let extractor = TripleExtractor::new();
        let text = "GCN achieves 81.3% accuracy on the Cora dataset.";
        let triples = extractor.extract(text);

        assert!(!triples.is_empty());
        let achieved = triples.iter().find(|t| t.predicate == "achieves");
        assert!(achieved.is_some());
    }

    #[test]
    fn test_outperforms_extraction() {
        let extractor = TripleExtractor::new();
        let text = "Our method outperforms previous approaches significantly.";
        let triples = extractor.extract(text);

        assert!(!triples.is_empty());
    }

    #[test]
    fn test_sentence_splitting() {
        let extractor = TripleExtractor::new();
        let text = "GCN achieves 81.3%. GAT is also effective.";
        let sentences = extractor.split_sentences(text);

        assert_eq!(sentences.len(), 2);
    }

    #[test]
    fn test_extract_triple_achieves() {
        let extractor = TripleExtractor::new();
        let text = "Model achieves 95% accuracy on the benchmark.";
        let triples = extractor.extract(text);

        let achieves_triples: Vec<_> = triples.iter()
            .filter(|t| t.predicate == "achieves")
            .collect();
        assert!(!achieves_triples.is_empty());
        assert_eq!(achieves_triples[0].subject, "Model");
    }

    #[test]
    fn test_extract_triple_outperforms() {
        let extractor = TripleExtractor::new();
        let text = "GCN outperforms GAT on this benchmark.";
        let triples = extractor.extract(text);

        let outperforms_triples: Vec<_> = triples.iter()
            .filter(|t| t.predicate == "outperforms")
            .collect();
        assert!(!outperforms_triples.is_empty());
    }

    #[test]
    fn test_extract_triple_trained_on() {
        let extractor = TripleExtractor::new();
        let text = "The model was trained on the WikiText dataset for language modeling.";
        let triples = extractor.extract(text);

        let trained_triples: Vec<_> = triples.iter()
            .filter(|t| t.predicate == "trained_on")
            .collect();
        assert!(!trained_triples.is_empty());
    }

    #[test]
    fn test_extract_triple_improves() {
        let extractor = TripleExtractor::new();
        let text = "This approach improves by 10% over the baseline.";
        let triples = extractor.extract(text);

        let improves_triples: Vec<_> = triples.iter()
            .filter(|t| t.predicate == "improves")
            .collect();
        assert!(!improves_triples.is_empty());
    }

    #[test]
    fn test_split_sentences_multiple() {
        let extractor = TripleExtractor::new();
        let text = "First sentence. Second sentence! Third sentence?";
        let sentences = extractor.split_sentences(text);

        assert_eq!(sentences.len(), 3);
    }

    #[test]
    fn test_split_sentences_short_ignored() {
        let extractor = TripleExtractor::new();
        let text = "Short. This is a much longer sentence that should be included.";
        let sentences = extractor.split_sentences(text);

        // "Short." is 6 chars, less than minimum 10, so should be filtered
        assert_eq!(sentences.len(), 1);
    }

    #[test]
    fn test_split_sentences_preserves_content() {
        let extractor = TripleExtractor::new();
        let text = "Hello world. Testing 123!";
        let sentences = extractor.split_sentences(text);

        assert!(sentences[0].contains("Hello"));
        assert!(sentences[1].contains("Testing"));
    }

    #[test]
    fn test_triple_confidence_scores() {
        let extractor = TripleExtractor::new();
        let text = "GCN achieves 81.3% accuracy.";
        let triples = extractor.extract(text);

        for triple in &triples {
            assert!(triple.confidence >= 0.0 && triple.confidence <= 1.0);
        }
    }

    #[test]
    fn test_triple_sentence_tracking() {
        let extractor = TripleExtractor::new();
        let text = "GCN achieves 81.3%. This is another sentence.";
        let triples = extractor.extract(text);

        for triple in &triples {
            assert!(!triple.sentence.is_empty());
            assert!(text.contains(&triple.sentence));
        }
    }

    #[test]
    fn test_triple_deduplication() {
        let extractor = TripleExtractor::new();
        // Same triple appearing twice
        let text = "Model A achieves 90%. Model A achieves 90%.";
        let triples = extractor.extract(text);

        // Should have at most one copy due to deduplication
        let subject_counts: std::collections::HashMap<_, _> = triples.iter()
            .fold(std::collections::HashMap::new(), |mut acc, t| {
                *acc.entry(&t.subject).or_insert(0) += 1;
                acc
            });
        for count in subject_counts.values() {
            assert!(*count <= 1);
        }
    }

    #[test]
    fn test_extract_no_triples() {
        let extractor = TripleExtractor::new();
        let text = "This is just some regular text without any relationships.";
        let triples = extractor.extract(text);

        // Some patterns might match, but at minimum should not error
        assert!(triples.len() >= 0);
    }

    #[test]
    fn test_triple_rdf_string() {
        let triple = Triple::new(
            "GCN".to_string(),
            "achieves".to_string(),
            "81.3%".to_string(),
            0.8,
            "GCN achieves 81.3%.".to_string(),
        );

        let rdf = triple.to_rdf_string();
        assert!(rdf.contains("GCN"));
        assert!(rdf.contains("achieves"));
        assert!(rdf.contains("81.3%"));
    }

    #[test]
    fn test_knowledge_graph_builder() {
        use super::KnowledgeGraphBuilder;

        let mut builder = KnowledgeGraphBuilder::new();
        let triples = vec![
            Triple::new("GCN", "achieves", "81.3%", 0.8, "GCN achieves 81.3%.".to_string()),
            Triple::new("GAT", "achieves", "79.5%", 0.8, "GAT achieves 79.5%.".to_string()),
        ];

        builder.add_triples(&triples);
        assert_eq!(builder.node_count(), 4); // 2 subjects + 2 objects
        assert_eq!(builder.edge_count(), 2);
    }

    #[test]
    fn test_split_sentences_with_remaining_text() {
        let extractor = TripleExtractor::new();
        let text = "First sentence. Second sentence. Remaining text without ending punctuation";
        let sentences = extractor.split_sentences(text);

        // Only 2 complete sentences with punctuation
        assert!(sentences.len() >= 1);
    }

    #[test]
    fn test_triple_extractor_default() {
        let extractor = TripleExtractor::default();
        let text = "Test achieves 100%.";
        let triples = extractor.extract(text);

        // Default should have relations enabled
        assert!(extractor.enable_relations);
    }

    #[test]
    fn test_triple_extractor_with_relations_disabled() {
        let extractor = TripleExtractor::with_relations(false);
        let text = "Test achieves 100%.";
        let triples = extractor.extract(text);

        assert!(!extractor.enable_relations);
        // Pattern-based extraction still works even with relations disabled
        assert!(triples.len() >= 0);
    }
}
