use rairos_codegraph::{CodeGraph, Node};
use tempfile::TempDir;

fn create_test_graph() -> (CodeGraph, TempDir) {
    let dir = TempDir::new().unwrap();
    let db_path = dir.path().join("test.db");
    let graph = CodeGraph::open(&db_path).unwrap();
    (graph, dir)
}

#[test]
fn test_open_creates_schema() {
    let (graph, _dir) = create_test_graph();
    let stats = graph.stats().unwrap();
    assert_eq!(stats.nodes, 0);
    assert_eq!(stats.edges, 0);
    assert_eq!(stats.files, 0);
}

#[test]
fn test_add_node() {
    let (graph, _dir) = create_test_graph();
    
    let node = Node {
        id: 0,
        name: "test_function".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 10,
        col: 0,
        end_line: 15,
        end_col: 1,
        docstring: Some("Test doc".to_string()),
    };
    
    let node_id = graph.add_node(&node).unwrap();
    assert!(node_id > 0);
    
    let retrieved = graph.get_node(node_id).unwrap();
    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.name, "test_function");
    assert_eq!(retrieved.kind, "function");
}

#[test]
fn test_add_edge() {
    let (graph, _dir) = create_test_graph();
    
    let node1 = Node {
        id: 0,
        name: "caller".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };
    
    let node2 = Node {
        id: 0,
        name: "callee".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 10,
        col: 0,
        end_line: 15,
        end_col: 1,
        docstring: None,
    };
    
    let id1 = graph.add_node(&node1).unwrap();
    let id2 = graph.add_node(&node2).unwrap();
    
    let edge_id = graph.add_edge(id1, id2, "calls").unwrap();
    assert!(edge_id > 0);
}

#[test]
fn test_clear() {
    let (graph, _dir) = create_test_graph();
    
    let node = Node {
        id: 0,
        name: "test".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };
    
    graph.add_node(&node).unwrap();
    let stats_before = graph.stats().unwrap();
    assert_eq!(stats_before.nodes, 1);
    
    graph.clear().unwrap();
    let stats_after = graph.stats().unwrap();
    assert_eq!(stats_after.nodes, 0);
    assert_eq!(stats_after.edges, 0);
}

#[test]
fn test_files() {
    let (graph, _dir) = create_test_graph();
    
    let node1 = Node {
        id: 0,
        name: "func1".to_string(),
        kind: "function".to_string(),
        file: "a.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };
    
    let node2 = Node {
        id: 0,
        name: "func2".to_string(),
        kind: "function".to_string(),
        file: "b.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };
    
    graph.add_node(&node1).unwrap();
    graph.add_node(&node2).unwrap();
    
    let files = graph.files().unwrap();
    assert_eq!(files.len(), 2);
    assert!(files.contains(&"a.rs".to_string()));
    assert!(files.contains(&"b.rs".to_string()));
}

#[test]
fn test_get_nonexistent_node() {
    let (graph, _dir) = create_test_graph();
    let result = graph.get_node(9999).unwrap();
    assert!(result.is_none());
}

#[test]
fn test_stats() {
    let (graph, _dir) = create_test_graph();
    
    let node = Node {
        id: 0,
        name: "test".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };
    
    graph.add_node(&node).unwrap();
    graph.add_node(&node).unwrap();
    
    let stats = graph.stats().unwrap();
    assert_eq!(stats.nodes, 2);
    assert_eq!(stats.edges, 0);
    assert_eq!(stats.files, 1);
}

#[test]
fn test_codegraph_backend_trait() {
    use rairos_codegraph::{CodeGraphBackend, Node};

    let (graph, _dir) = create_test_graph();

    let node = Node {
        id: 0,
        name: "backend_test".to_string(),
        kind: "function".to_string(),
        file: "test.rs".to_string(),
        line: 1,
        col: 0,
        end_line: 5,
        end_col: 1,
        docstring: None,
    };

    let id = graph.add_node(&node).unwrap();
    assert!(id > 0);

    let stats = graph.stats().unwrap();
    assert_eq!(stats.nodes, 1);

    let retrieved = graph.get_node(id).unwrap();
    assert!(retrieved.is_some());
    assert_eq!(retrieved.unwrap().name, "backend_test");
}
