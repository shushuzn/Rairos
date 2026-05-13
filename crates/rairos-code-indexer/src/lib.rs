use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::Path;

const MAX_EMBEDDINGS_CACHE: usize = 1000;

const STOPWORDS: &[&str] = &[
    "def", "class", "return", "if", "else", "elif", "for", "while", "try", "except",
    "finally", "with", "as", "import", "from", "pass", "break", "continue", "and", "or",
    "not", "in", "is", "None", "True", "False", "self", "self.", "lambda", "yield",
    "raise", "assert", "del", "global", "nonlocal",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChunk {
    pub file: String,
    pub start_line: usize,
    pub end_line: usize,
    pub chunk_type: String,
    pub name: String,
    pub content: String,
    pub tokens: Vec<String>,
    pub embedding: Option<Vec<f32>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IndexerStats {
    pub files_indexed: usize,
    pub chunks_created: usize,
    pub total_tokens: usize,
    pub cache_usage: usize,
}

pub struct CodeIndexer {
    chunks: Vec<CodeChunk>,
    stopwords: HashSet<String>,
    cache: Vec<Vec<f32>>,
}

impl Default for CodeIndexer {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeIndexer {
    pub fn new() -> Self {
        Self {
            chunks: Vec::new(),
            stopwords: STOPWORDS.iter().map(|s| s.to_string()).collect(),
            cache: Vec::new(),
        }
    }

    pub fn tokenize(&self, code: &str) -> Vec<String> {
        let mut tokens = Vec::new();
        let mut current = String::new();
        for ch in code.chars() {
            if ch.is_alphanumeric() || ch == '_' {
                current.push(ch);
            } else {
                if !current.is_empty() {
                    let lower = current.to_lowercase();
                    if !self.stopwords.contains(&lower) && current.len() >= 2 {
                        tokens.push(current.clone());
                    }
                    current.clear();
                }
            }
        }
        if !current.is_empty() {
            let lower = current.to_lowercase();
            if !self.stopwords.contains(&lower) && current.len() >= 2 {
                tokens.push(current);
            }
        }
        tokens
    }

    pub fn chunk_file(&mut self, file_path: &str, content: &str) -> Vec<CodeChunk> {
        let mut chunks = Vec::new();
        let lines: Vec<&str> = content.lines().collect();
        let path = Path::new(file_path);
        let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
        let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("unknown");

        let mut i = 0;
        while i < lines.len() {
            let trimmed = lines[i].trim();
            let (chunk_type, name, start) = if trimmed.starts_with("def ") {
                let name = trimmed.trim_start_matches("def ").trim();
                let name = name.split('(').next().unwrap_or(name).to_string();
                ("function", name, i)
            } else if trimmed.starts_with("class ") {
                let name = trimmed.trim_start_matches("class ").trim();
                let name = name.split('(').next().unwrap_or(name).to_string();
                let name = name.split(':').next().unwrap_or(&name).to_string();
                ("class", name, i)
            } else if trimmed.starts_with("async def ") {
                let name = trimmed.trim_start_matches("async def ").trim();
                let name = name.split('(').next().unwrap_or(name).to_string();
                ("function", name, i)
            } else {
                i += 1;
                continue;
            };

            let mut j = start + 1;
            let mut depth = 1;
            while j < lines.len() {
                let line = lines[j];
                if line.starts_with("def ")
                    || line.starts_with("class ")
                    || line.starts_with("async def ")
                {
                    let indent = line.chars().take_while(|c| c.is_whitespace()).count();
                    if indent <= 4 {
                        break;
                    }
                }
                if line.trim().starts_with("def ") || line.trim().starts_with("class ") {
                    break;
                }
                j += 1;
            }

            let chunk_content: String = lines[start..=j.min(lines.len() - 1)].join("\n");
            let tokens = self.tokenize(&chunk_content);
            let chunk = CodeChunk {
                file: file_path.to_string(),
                start_line: start + 1,
                end_line: j,
                chunk_type: chunk_type.to_string(),
                name,
                content: chunk_content,
                tokens,
                embedding: None,
            };
            chunks.push(chunk);
            i = j;
        }

        if chunks.is_empty() {
            let tokens = self.tokenize(content);
            chunks.push(CodeChunk {
                file: file_path.to_string(),
                start_line: 1,
                end_line: lines.len(),
                chunk_type: "module".to_string(),
                name: file_stem.to_string(),
                content: content.to_string(),
                tokens,
                embedding: None,
            });
        }

        self.chunks.extend(chunks.clone());
        chunks
    }

    pub fn search(&self, query: &str) -> Vec<&CodeChunk> {
        let query_tokens: HashSet<String> = self
            .tokenize(query)
            .into_iter()
            .map(|t| t.to_lowercase())
            .collect();
        let mut scored: Vec<(f64, &CodeChunk)> = self
            .chunks
            .iter()
            .map(|chunk| {
                let chunk_tokens: HashSet<String> = chunk
                    .tokens
                    .iter()
                    .map(|t| t.to_lowercase())
                    .collect();
                let intersection = query_tokens.intersection(&chunk_tokens).count();
                let score = if !query_tokens.is_empty() {
                    intersection as f64 / query_tokens.len() as f64
                } else {
                    0.0
                };
                (score, chunk)
            })
            .collect();
        scored.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scored.into_iter().take(10).map(|(_, c)| c).collect()
    }

    pub fn stats(&self) -> IndexerStats {
        IndexerStats {
            files_indexed: self
                .chunks
                .iter()
                .map(|c| c.file.clone())
                .collect::<HashSet<_>>()
                .len(),
            chunks_created: self.chunks.len(),
            total_tokens: self.chunks.iter().map(|c| c.tokens.len()).sum(),
            cache_usage: self.cache.len(),
        }
    }

    pub fn clear(&mut self) {
        self.chunks.clear();
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenize() {
        let indexer = CodeIndexer::new();
        let tokens = indexer.tokenize("def hello_world():\n    return 42");
        assert!(tokens.iter().any(|t| t.contains("hello")));
        assert!(!tokens.iter().any(|t| t == "def"));
    }

    #[test]
    fn test_chunk_file() {
        let mut indexer = CodeIndexer::new();
        let code = r#"
def foo():
    pass

class Bar:
    def baz(self):
        return 1
"#;
        let chunks = indexer.chunk_file("test.py", code);
        assert!(chunks.len() >= 2);
        assert_eq!(chunks[0].chunk_type, "function");
        assert_eq!(chunks[0].name, "foo");
    }

    #[test]
    fn test_search() {
        let mut indexer = CodeIndexer::new();
        indexer.chunk_file(
            "a.py",
            "def process_data():\n    return transformed",
        );
        indexer.chunk_file(
            "b.py",
            "class DataProcessor:\n    def run(self): pass",
        );
        let results = indexer.search("process");
        assert!(!results.is_empty());
    }

    #[test]
    fn test_stats() {
        let mut indexer = CodeIndexer::new();
        indexer.chunk_file("test.py", "def f(): pass\nclass C: pass");
        let stats = indexer.stats();
        assert!(stats.chunks_created >= 2);
    }

    #[test]
    fn test_clear() {
        let mut indexer = CodeIndexer::new();
        indexer.chunk_file("t.py", "def f(): pass");
        assert!(indexer.stats().chunks_created > 0);
        indexer.clear();
        assert_eq!(indexer.stats().chunks_created, 0);
    }

    #[test]
    fn test_empty_file() {
        let mut indexer = CodeIndexer::new();
        let chunks = indexer.chunk_file("empty.py", "");
        assert_eq!(chunks.len(), 1);
        assert_eq!(chunks[0].chunk_type, "module");
    }
}
