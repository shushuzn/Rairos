use rairos_codegraph::{CodeGraph, Indexer};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_indexer_new() {
    let indexer = Indexer::new();
    drop(indexer);
}

#[test]
fn test_index_simple_rust_file() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let graph = CodeGraph::open(&db_path).unwrap();
    
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    let source = r#"pub fn test_function(x: i32) -> i32 { x + 1 }
pub struct TestStruct { pub value: i32 }
pub enum TestEnum { Variant1, Variant2(i32) }"#;
    
    fs::write(src_dir.join("lib.rs"), source).unwrap();
    
    let mut indexer = Indexer::new();
    let stats = indexer.index_project(temp_dir.path(), &graph).unwrap();
    
    assert!(stats.functions >= 1, "Expected at least 1 function, got {}", stats.functions);
    assert!(stats.structs >= 1, "Expected at least 1 struct, got {}", stats.structs);
    assert!(stats.enums >= 1, "Expected at least 1 enum, got {}", stats.enums);
    
    let graph_stats = graph.stats().unwrap();
    assert!(graph_stats.nodes >= 3, "Expected at least 3 nodes, got {}", graph_stats.nodes);
}

#[test]
fn test_indexer_default() {
    let indexer = Indexer::default();
    drop(indexer);
}

#[test]
fn test_index_multiple_files() {
    let temp_dir = TempDir::new().unwrap();
    let db_path = temp_dir.path().join("test.db");
    let graph = CodeGraph::open(&db_path).unwrap();
    
    let src_dir = temp_dir.path().join("src");
    fs::create_dir(&src_dir).unwrap();
    
    fs::write(src_dir.join("a.rs"), "pub fn func_a() {}").unwrap();
    fs::write(src_dir.join("b.rs"), "pub fn func_b() {}").unwrap();
    
    let mut indexer = Indexer::new();
    let stats = indexer.index_project(temp_dir.path(), &graph).unwrap();
    
    assert!(stats.functions >= 2, "Expected at least 2 functions, got {}", stats.functions);
    
    let files = graph.files().unwrap();
    assert!(files.iter().any(|f| f.contains("a.rs")), "Missing a.rs in {:?}", files);
    assert!(files.iter().any(|f| f.contains("b.rs")), "Missing b.rs in {:?}", files);
}
