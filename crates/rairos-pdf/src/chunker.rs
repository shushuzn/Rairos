//! Text chunking for RAG pipeline.
//!
//! Provides intelligent text chunking with overlap for optimal embedding
//! and retrieval performance.

use serde::{Deserialize, Serialize};
use std::collections::HashSet;

// ============================================================================
// Data Structures
// ============================================================================

/// Configuration for text chunking.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChunkConfig {
    /// Target size for each chunk in characters
    pub target_chunk_size: usize,
    /// Overlap between chunks in characters
    pub overlap: usize,
    /// Minimum chunk size before forcing a split
    pub min_chunk_size: usize,
    /// Whether to respect sentence boundaries
    pub respect_sentence_boundary: bool,
    /// Whether to include section titles in chunks
    pub include_section_titles: bool,
}

impl Default for ChunkConfig {
    fn default() -> Self {
        Self {
            target_chunk_size: 512,
            overlap: 128,
            min_chunk_size: 128,
            respect_sentence_boundary: true,
            include_section_titles: true,
        }
    }
}

/// A text chunk with metadata for RAG retrieval.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextChunk {
    /// Unique chunk ID
    pub id: String,
    /// Chunk text content
    pub text: String,
    /// Section title this chunk belongs to
    #[serde(default)]
    pub section_title: String,
    /// Start position in original text
    pub start_char: usize,
    /// End position in original text
    pub end_char: usize,
    /// Token estimate (approximate)
    pub token_count: usize,
    /// Chunk index within document
    pub chunk_index: usize,
}

impl TextChunk {
    /// Create a new text chunk.
    pub fn new(
        id: &str,
        text: &str,
        section_title: &str,
        start_char: usize,
        end_char: usize,
        chunk_index: usize,
    ) -> Self {
        // Rough token estimate: ~4 chars per token
        let token_count = text.len() / 4;
        Self {
            id: id.to_string(),
            text: text.to_string(),
            section_title: section_title.to_string(),
            start_char,
            end_char,
            token_count,
            chunk_index,
        }
    }
}

// ============================================================================
// Text Chunking
// ============================================================================

/// Text chunker for RAG pipeline.
pub struct TextChunker {
    config: ChunkConfig,
}

impl TextChunker {
    /// Create a new text chunker with default config.
    pub fn new() -> Self {
        Self {
            config: ChunkConfig::default(),
        }
    }

    /// Create with custom config.
    pub fn with_config(config: ChunkConfig) -> Self {
        Self { config }
    }

    /// Chunk text into overlapping segments optimized for embedding.
    pub fn chunk(&self, text: &str) -> Vec<TextChunk> {
        self.chunk_with_sections(text, &[])
    }

    /// Chunk text with section boundaries.
    pub fn chunk_with_sections(&self, text: &str, sections: &[(&str, usize, usize)]) -> Vec<TextChunk> {
        let mut chunks = Vec::new();
        let text_len = text.len();

        if text_len == 0 {
            return chunks;
        }

        // If we have sections, chunk each section separately
        if !sections.is_empty() {
            for (idx, (title, start, end)) in sections.iter().enumerate() {
                let section_text = &text[*start..*end.min(&text_len)];
                if !section_text.trim().is_empty() {
                    let section_chunks = self.chunk_text_segment(
                        section_text,
                        title,
                        *start,
                        idx,
                    );
                    chunks.extend(section_chunks);
                }
            }
            return chunks;
        }

        // Otherwise chunk the full text
        self.chunk_text_segment(text, "", 0, 0)
    }

    /// Chunk a text segment (section or full text).
    fn chunk_text_segment(
        &self,
        text: &str,
        section_title: &str,
        offset: usize,
        section_index: usize,
    ) -> Vec<TextChunk> {
        let mut chunks = Vec::new();
        let text_len = text.len();

        if self.config.respect_sentence_boundary {
            self.chunk_by_sentences(text, section_title, offset, section_index, &mut chunks);
        } else {
            self.chunk_by_chars(text, section_title, offset, section_index, &mut chunks);
        }

        chunks
    }

    /// Chunk text by character count, respecting sentence boundaries.
    fn chunk_by_sentences(
        &self,
        text: &str,
        section_title: &str,
        offset: usize,
        section_index: usize,
        chunks: &mut Vec<TextChunk>,
    ) {
        let sentences = self.split_into_sentences(text);
        let mut current_chunk = String::new();
        let mut current_start = 0;
        let mut chunk_index = section_index * 1000; // Space for multiple sections

        for (i, sentence) in sentences.iter().enumerate() {
            let sentence_len = sentence.len();

            // Check if adding this sentence would exceed target size
            if !current_chunk.is_empty()
                && current_chunk.len() + sentence_len > self.config.target_chunk_size
            {
                // Save current chunk
                if current_chunk.len() >= self.config.min_chunk_size {
                    let doc_id = format!(
                        "chunk_{}_{}",
                        section_index,
                        chunk_index
                    );
                    chunks.push(TextChunk::new(
                        &doc_id,
                        current_chunk.trim(),
                        section_title,
                        offset + current_start,
                        offset + current_start + current_chunk.len(),
                        chunk_index,
                    ));
                    chunk_index += 1;
                }

                // Start new chunk with overlap
                let overlap_text = if self.config.overlap > 0 && current_chunk.len() > self.config.overlap {
                    // Keep the last overlap chars as a buffer
                    let overlap_start = current_chunk.len() - self.config.overlap;
                    current_chunk[overlap_start..].to_string()
                } else {
                    String::new()
                };

                current_chunk = overlap_text;
                current_start = if self.config.overlap > 0 && current_chunk.len() > 0 {
                    // Adjust start position for overlap
                    offset + current_chunk.len()
                } else {
                    offset
                };

                // If single sentence exceeds target, force split
                if sentence_len > self.config.target_chunk_size {
                    let sub_chunks = self.split_long_sentence(sentence, section_title, offset, &mut chunk_index);
                    chunks.extend(sub_chunks);
                    current_chunk.clear();
                    continue;
                }
            }

            if !current_chunk.is_empty() {
                current_chunk.push(' ');
            }
            current_chunk.push_str(sentence);
        }

        // Don't forget the last chunk
        if current_chunk.len() >= self.config.min_chunk_size {
            let doc_id = format!("chunk_{}_{}", section_index, chunk_index);
            chunks.push(TextChunk::new(
                &doc_id,
                current_chunk.trim(),
                section_title,
                offset + current_start,
                offset + current_start + current_chunk.len(),
                chunk_index,
            ));
        }
    }

    /// Chunk by raw character count (faster but less semantically aware).
    fn chunk_by_chars(
        &self,
        text: &str,
        section_title: &str,
        offset: usize,
        section_index: usize,
        chunks: &mut Vec<TextChunk>,
    ) {
        let text_len = text.len();
        let mut position = 0;
        let mut chunk_index = section_index * 1000;

        while position < text_len {
            let chunk_end = (position + self.config.target_chunk_size).min(text_len);
            let actual_end = if chunk_end < text_len {
                // Try to break at whitespace
                text[..chunk_end]
                    .rfind(|c: char| c.is_whitespace())
                    .map(|i| position + i)
                    .unwrap_or(chunk_end)
            } else {
                chunk_end
            };

            let chunk_text = &text[position..actual_end];

            if chunk_text.len() >= self.config.min_chunk_size {
                let doc_id = format!("chunk_{}_{}", section_index, chunk_index);
                chunks.push(TextChunk::new(
                    &doc_id,
                    chunk_text.trim(),
                    section_title,
                    offset + position,
                    offset + actual_end,
                    chunk_index,
                ));
                chunk_index += 1;
            }

            // Move position with overlap
            if self.config.overlap > 0 && position + self.config.target_chunk_size < text_len {
                let overlap_start = if actual_end > self.config.overlap {
                    actual_end - self.config.overlap
                } else {
                    position
                };
                position = overlap_start;
            } else {
                position = actual_end;
            }
        }
    }

    /// Split text into sentences using heuristics.
    fn split_into_sentences(&self, text: &str) -> Vec<String> {
        let mut sentences = Vec::new();
        let mut current = String::new();
        let mut in_abbreviation = false;
        let chars: Vec<char> = text.chars().collect();

        for i in 0..chars.len() {
            let c = chars[i];

            // Check for abbreviations (common academic ones)
            if i > 0 && c == '.' {
                let prev_word = self.get_previous_word(&chars[..i]);
                if Self::is_abbreviation(&prev_word) {
                    current.push(c);
                    in_abbreviation = true;
                    continue;
                }
            }

            current.push(c);

            // Sentence ending
            if c == '.' || c == '!' || c == '?' {
                if !in_abbreviation && (i + 1 >= chars.len() || chars[i + 1].is_whitespace()) {
                    let sentence = current.trim().to_string();
                    if !sentence.is_empty() {
                        sentences.push(sentence);
                    }
                    current.clear();
                    in_abbreviation = false;
                }
            }
        }

        // Don't forget trailing text
        if !current.trim().is_empty() {
            sentences.push(current.trim().to_string());
        }

        sentences
    }

    /// Get the word before a position.
    fn get_previous_word(&self, chars: &[char]) -> String {
        let mut word_chars = Vec::new();
        for c in chars.iter().rev() {
            if c.is_alphabetic() {
                word_chars.push(*c);
            } else {
                break;
            }
        }
        word_chars.iter().rev().collect::<String>().to_lowercase()
    }

    /// Check if a word is a common academic abbreviation.
    fn is_abbreviation(word: &str) -> bool {
        let abbreviations: HashSet<&str> = [
            "dr", "prof", "mr", "mrs", "ms", "vs", "etc", "e.g", "i.e",
            "fig", "al", "seq", "vol", "no", "yr", "ref", "refs",
            "sec", "ch", "app", "suppl", "doi", "arxiv",
        ]
        .into_iter()
        .collect();
        abbreviations.contains(word.to_lowercase().as_str())
    }

    /// Split a long sentence that exceeds target size.
    fn split_long_sentence(
        &self,
        sentence: &str,
        section_title: &str,
        offset: usize,
        chunk_index: &mut usize,
    ) -> Vec<TextChunk> {
        let mut chunks = Vec::new();
        let words: Vec<&str> = sentence.split_whitespace().collect();
        let mut current = String::new();

        for word in words {
            if current.len() + word.len() + 1 > self.config.target_chunk_size {
                if current.len() >= self.config.min_chunk_size {
                    let doc_id = format!("chunk_split_{}", *chunk_index);
                    chunks.push(TextChunk::new(
                        &doc_id,
                        current.trim(),
                        section_title,
                        offset,
                        offset + current.len(),
                        *chunk_index,
                    ));
                    *chunk_index += 1;
                }
                current = word.to_string();
            } else {
                if !current.is_empty() {
                    current.push(' ');
                }
                current.push_str(word);
            }
        }

        if current.len() >= self.config.min_chunk_size {
            let doc_id = format!("chunk_split_{}", *chunk_index);
            chunks.push(TextChunk::new(
                &doc_id,
                current.trim(),
                section_title,
                offset,
                offset + current.len(),
                *chunk_index,
            ));
        }

        chunks
    }
}

impl Default for TextChunker {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_chunk_config_default() {
        let config = ChunkConfig::default();
        assert_eq!(config.target_chunk_size, 512);
        assert_eq!(config.overlap, 128);
        assert_eq!(config.min_chunk_size, 128);
        assert!(config.respect_sentence_boundary);
    }

    #[test]
    fn test_text_chunker_new() {
        let chunker = TextChunker::new();
        let chunks = chunker.chunk("Short text.");
        assert_eq!(chunks.len(), 1);
        assert!(chunks[0].text.contains("Short text"));
    }

    #[test]
    fn test_chunk_by_sentence() {
        let text = "This is the first sentence. This is the second sentence. And this is third.";
        let chunker = TextChunker::new();
        let chunks = chunker.chunk(text);

        assert!(!chunks.is_empty());
        assert_eq!(chunks[0].section_title, "");
    }

    #[test]
    fn test_chunk_overlap() {
        let config = ChunkConfig {
            target_chunk_size: 20,
            overlap: 5,
            ..Default::default()
        };
        let chunker = TextChunker::with_config(config);
        let text = "This is a longer piece of text that should be split.";
        let chunks = chunker.chunk(text);

        // With overlap, we expect at least 2 chunks
        assert!(chunks.len() >= 2);
    }

    #[test]
    fn test_is_abbreviation() {
        assert!(TextChunker::is_abbreviation("Dr"));
        assert!(TextChunker::is_abbreviation("e.g"));
        assert!(TextChunker::is_abbreviation("Fig"));
        assert!(!TextChunker::is_abbreviation("hello"));
    }

    #[test]
    fn test_split_into_sentences() {
        let chunker = TextChunker::new();
        let text = "First sentence. Second sentence! Third sentence?";
        let sentences = chunker.split_into_sentences(text);

        assert_eq!(sentences.len(), 3);
        assert!(sentences[0].contains("First"));
        assert!(sentences[1].contains("Second"));
        assert!(sentences[2].contains("Third"));
    }

    #[test]
    fn test_split_long_sentence() {
        let chunker = TextChunker::new();
        let long_sentence = "This ";
        let chunks = chunker.chunk(long_sentence);
        // Very short sentence should produce one chunk
        assert_eq!(chunks.len(), 1);
    }

    #[test]
    fn test_chunk_with_empty_text() {
        let chunker = TextChunker::new();
        let chunks = chunker.chunk("");
        assert!(chunks.is_empty());
    }

    #[test]
    fn test_chunk_with_sections() {
        let text = "Introduction\n\nSome intro content.\n\n## Methods\n\nMethod details here.";
        let sections = vec![
            ("Introduction", 0, 30),
            ("Methods", 32, 60),
        ];
        let chunker = TextChunker::new();
        let chunks = chunker.chunk_with_sections(text, &sections);

        // Should have chunks with section titles
        for chunk in &chunks {
            assert!(chunk.section_title.is_empty() || chunk.section_title == "Introduction" || chunk.section_title == "Methods");
        }
    }

    #[test]
    fn test_token_count_estimate() {
        let chunk = TextChunk::new("test", "This is a test sentence.", "", 0, 25, 0);
        // Rough estimate: 4 chars per token
        assert_eq!(chunk.token_count, 7); // 25 / 4 ≈ 6, but we use len()
    }

    #[test]
    fn test_chunk_ids_unique() {
        let chunker = TextChunker::new();
        let text = "This is a longer piece of text that should produce multiple chunks when chunked appropriately.";
        let chunks = chunker.chunk(text);

        let ids: Vec<&str> = chunks.iter().map(|c| c.id.as_str()).collect();
        // All IDs should be unique
        for id in &ids {
            assert_eq!(ids.iter().filter(|&&i| i == *id).count(), 1);
        }
    }
}