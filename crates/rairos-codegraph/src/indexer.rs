//! Code indexer using tree-sitter

use crate::graph::{CodeGraph, Node};
use std::path::Path;
use walkdir::WalkDir;
use tree_sitter::Parser;
use tree_sitter_rust::LANGUAGE;

pub struct Indexer {
    parser: Parser,
}

impl Indexer {
    pub fn new() -> Self {
        let parser = Parser::new();
        Self { parser }
    }

    /// Index a Rust project (writes directly to graph)
    pub fn index_project(&mut self, root: &Path, graph: &CodeGraph) -> std::io::Result<IndexStats> {
        let nodes = self.collect_nodes(root)?;
        let mut stats = IndexStats::default();
        
        graph.clear().ok();
        
        for node in nodes {
            match node.kind.as_str() {
                "function" => stats.functions += 1,
                "struct" => stats.structs += 1,
                "enum" => stats.enums += 1,
                "impl" => stats.impls += 1,
                _ => {}
            }
            graph.add_node(&node).ok();
        }
        
        Ok(stats)
    }

    /// Collect all nodes from a Rust project (returns Vec for testability)
    pub fn collect_nodes(&mut self, root: &Path) -> std::io::Result<Vec<Node>> {
        let mut nodes = Vec::new();
        
        let lang: tree_sitter::Language = LANGUAGE.into();
        self.parser.set_language(&lang).ok();
        
        for entry in WalkDir::new(root)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            let path = entry.path();
            if path.is_file() {
                if let Some(ext) = path.extension() {
                    if ext == "rs" {
                        if let Ok(mut file) = std::fs::File::open(path) {
                            let mut source = String::new();
                            if std::io::Read::read_to_string(&mut file, &mut source).is_ok() {
                                self.collect_from_source(&source, path, &mut nodes)?;
                            }
                        }
                    }
                }
            }
        }
        
        Ok(nodes)
    }

    /// Collect nodes from source code
    fn collect_from_source(&mut self, source: &str, path: &Path, nodes: &mut Vec<Node>) -> std::io::Result<()> {
        let file_str = path.to_string_lossy().to_string();
        
        let tree = match self.parser.parse(source, None) {
            Some(t) => t,
            None => return Ok(()),
        };
        
        self.extract_nodes(source, &tree.root_node(), &file_str, nodes);
        
        Ok(())
    }

    fn extract_nodes(&self, source: &str, node: &tree_sitter::Node, file: &str, nodes: &mut Vec<Node>) {
        match node.kind() {
            "function_item" => {
                nodes.push(self.make_node(source, node, file, "function"));
            }
            "struct_item" => {
                nodes.push(self.make_node(source, node, file, "struct"));
            }
            "enum_item" => {
                nodes.push(self.make_node(source, node, file, "enum"));
            }
            "impl_item" => {
                let start = node.start_position();
                let end = node.end_position();
                nodes.push(Node {
                    id: 0,
                    name: format!("impl@{}:{}", file, start.row + 1),
                    kind: "impl".to_string(),
                    file: file.to_string(),
                    line: start.row as u32 + 1,
                    col: start.column as u32,
                    end_line: end.row as u32 + 1,
                    end_col: end.column as u32,
                    docstring: None,
                });
            }
            _ => {}
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.extract_nodes(source, &child, file, nodes);
            }
        }
    }

    fn make_node(&self, source: &str, node: &tree_sitter::Node, file: &str, kind: &str) -> Node {
        let name = self.extract_name(source, node);
        let start = node.start_position();
        let end = node.end_position();
        
        Node {
            id: 0,
            name,
            kind: kind.to_string(),
            file: file.to_string(),
            line: start.row as u32 + 1,
            col: start.column as u32,
            end_line: end.row as u32 + 1,
            end_col: end.column as u32,
            docstring: self.extract_docstring(source, node),
        }
    }

    fn find_functions(
        &self,
        source: &str,
        node: &tree_sitter::Node,
        file: &str,
        graph: &CodeGraph,
        node_map: &mut std::collections::HashMap<usize, i64>,
        counter: &mut usize,
    ) -> std::io::Result<()> {
        if node.kind() == "function_item" {
            let n = self.make_node(source, node, file, "function");
            
            if let Ok(id) = graph.add_node(&n) {
                node_map.insert(node.id(), id);
                *counter += 1;
            }
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_functions(source, &child, file, graph, node_map, counter)?;
            }
        }
        
        Ok(())
    }

    fn find_structs(
        &self,
        source: &str,
        node: &tree_sitter::Node,
        file: &str,
        graph: &CodeGraph,
        node_map: &mut std::collections::HashMap<usize, i64>,
        counter: &mut usize,
    ) -> std::io::Result<()> {
        if node.kind() == "struct_item" {
            let n = self.make_node(source, node, file, "struct");
            
            if let Ok(id) = graph.add_node(&n) {
                node_map.insert(node.id(), id);
                *counter += 1;
            }
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_structs(source, &child, file, graph, node_map, counter)?;
            }
        }
        
        Ok(())
    }

    fn find_enums(
        &self,
        source: &str,
        node: &tree_sitter::Node,
        file: &str,
        graph: &CodeGraph,
        node_map: &mut std::collections::HashMap<usize, i64>,
        counter: &mut usize,
    ) -> std::io::Result<()> {
        if node.kind() == "enum_item" {
            let name = self.extract_name(source, node);
            let start = node.start_position();
            let end = node.end_position();
            
            let n = Node {
                id: 0,
                name,
                kind: "enum".to_string(),
                file: file.to_string(),
                line: start.row as u32 + 1,
                col: start.column as u32,
                end_line: end.row as u32 + 1,
                end_col: end.column as u32,
                docstring: self.extract_docstring(source, node),
            };
            
            if let Ok(id) = graph.add_node(&n) {
                node_map.insert(node.id(), id);
                *counter += 1;
            }
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_enums(source, &child, file, graph, node_map, counter)?;
            }
        }
        
        Ok(())
    }

    fn find_impls(
        &self,
        source: &str,
        node: &tree_sitter::Node,
        file: &str,
        graph: &CodeGraph,
        node_map: &mut std::collections::HashMap<usize, i64>,
        counter: &mut usize,
    ) -> std::io::Result<()> {
        if node.kind() == "impl_item" {
            let start = node.start_position();
            let name = format!("impl@{}:{}", file, start.row + 1);
            let end = node.end_position();
            
            let n = Node {
                id: 0,
                name,
                kind: "impl".to_string(),
                file: file.to_string(),
                line: start.row as u32 + 1,
                col: start.column as u32,
                end_line: end.row as u32 + 1,
                end_col: end.column as u32,
                docstring: None,
            };
            
            if let Ok(id) = graph.add_node(&n) {
                node_map.insert(node.id(), id);
                *counter += 1;
            }
        }
        
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                self.find_impls(source, &child, file, graph, node_map, counter)?;
            }
        }
        
        Ok(())
    }

    fn extract_name(&self, source: &str, node: &tree_sitter::Node) -> String {
        let bytes = source.as_bytes();
        let (start, end) = (node.byte_range().start, node.byte_range().end);
        
        let mut name_start = start;
        for i in start..end.min(start + 200) {
            if bytes[i].is_ascii_alphabetic() || bytes[i] == b'_' {
                name_start = i;
                break;
            }
        }
        
        let mut name_end = name_start;
        for i in name_start..end.min(start + 300) {
            if bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' {
                name_end = i + 1;
            } else {
                break;
            }
        }
        
        String::from_utf8_lossy(&bytes[name_start..name_end]).to_string()
    }

    fn extract_docstring(&self, source: &str, node: &tree_sitter::Node) -> Option<String> {
        let start = node.start_position();
        if start.row == 0 {
            return None;
        }
        
        let lines: Vec<&str> = source.lines().collect();
        let comment_line = start.row.saturating_sub(1);
        
        if comment_line < lines.len() {
            let prev_line = lines[comment_line].trim();
            if prev_line.starts_with("///") || prev_line.starts_with("//!") {
                return Some(prev_line.trim_start_matches(&['/', '!'][..]).trim().to_string());
            }
        }
        
        None
    }
}

impl Default for Indexer {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Default)]
pub struct IndexStats {
    pub functions: usize,
    pub structs: usize,
    pub enums: usize,
    pub impls: usize,
}
