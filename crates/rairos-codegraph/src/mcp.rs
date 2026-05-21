//! MCP protocol server for CodeGraph

use crate::graph::CodeGraph;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::runtime::Handle;

/// MCP Tool definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: ToolInputSchema,
}

/// JSON Schema for tool input
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolInputSchema {
    #[serde(rename = "type")]
    pub schema_type: String,
    pub properties: HashMap<String, ToolProperty>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolProperty {
    pub description: String,
    #[serde(rename = "type")]
    pub prop_type: String,
}

/// MCP Server
pub struct McpServer {
    graph: CodeGraph,
}

impl McpServer {
    pub fn new(graph: CodeGraph) -> Self {
        Self { graph }
    }

    /// Handle a tools/list request
    pub fn list_tools(&self) -> Vec<Tool> {
        vec![
            Tool {
                name: "codegraph_search".to_string(),
                description: "Search for symbols by name across the codebase".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("query".to_string(), ToolProperty {
                            description: "Search query".to_string(),
                            prop_type: "string".to_string(),
                        });
                        p.insert("limit".to_string(), ToolProperty {
                            description: "Max results (default 10)".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["query".to_string()],
                },
            },
            Tool {
                name: "codegraph_context".to_string(),
                description: "Build relevant code context for a task".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("task".to_string(), ToolProperty {
                            description: "Task description".to_string(),
                            prop_type: "string".to_string(),
                        });
                        p.insert("max_nodes".to_string(), ToolProperty {
                            description: "Max nodes to return (default 20)".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["task".to_string()],
                },
            },
            Tool {
                name: "codegraph_callers".to_string(),
                description: "Find what calls a function".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("node_id".to_string(), ToolProperty {
                            description: "Node ID".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p.insert("depth".to_string(), ToolProperty {
                            description: "Call depth (default 3)".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["node_id".to_string()],
                },
            },
            Tool {
                name: "codegraph_callees".to_string(),
                description: "Find what a function calls".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("node_id".to_string(), ToolProperty {
                            description: "Node ID".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p.insert("depth".to_string(), ToolProperty {
                            description: "Call depth (default 3)".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["node_id".to_string()],
                },
            },
            Tool {
                name: "codegraph_impact".to_string(),
                description: "Analyze what code is affected by changing a symbol".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("node_id".to_string(), ToolProperty {
                            description: "Node ID".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p.insert("depth".to_string(), ToolProperty {
                            description: "Impact depth (default 2)".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["node_id".to_string()],
                },
            },
            Tool {
                name: "codegraph_node".to_string(),
                description: "Get details about a specific symbol".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: {
                        let mut p = HashMap::new();
                        p.insert("node_id".to_string(), ToolProperty {
                            description: "Node ID".to_string(),
                            prop_type: "number".to_string(),
                        });
                        p
                    },
                    required: vec!["node_id".to_string()],
                },
            },
            Tool {
                name: "codegraph_files".to_string(),
                description: "Get indexed file structure".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
            Tool {
                name: "codegraph_status".to_string(),
                description: "Check index health and statistics".to_string(),
                input_schema: ToolInputSchema {
                    schema_type: "object".to_string(),
                    properties: HashMap::new(),
                    required: vec![],
                },
            },
        ]
    }

    /// Handle a tools/call request
    pub fn call_tool(&self, name: &str, args: &serde_json::Value) -> serde_json::Value {
        match name {
            "codegraph_search" => self.tool_search(args),
            "codegraph_context" => self.tool_context(args),
            "codegraph_callers" => self.tool_callers(args),
            "codegraph_callees" => self.tool_callees(args),
            "codegraph_impact" => self.tool_impact(args),
            "codegraph_node" => self.tool_node(args),
            "codegraph_files" => self.tool_files(args),
            "codegraph_status" => self.tool_status(args),
            _ => serde_json::json!({
                "error": format!("Unknown tool: {}", name)
            }),
        }
    }

    fn tool_search(&self, args: &serde_json::Value) -> serde_json::Value {
        let query = args["query"].as_str().unwrap_or("");
        let limit = args["limit"].as_u64().unwrap_or(10) as usize;
        
        match Handle::current().block_on(self.graph.search(query, limit)) {
            Ok(results) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results).unwrap_or_default()
                }]
            }),
            Err(e) => serde_json::json!({
                "error": format!("Search failed: {}", e)
            }),
        }
    }

    fn tool_context(&self, args: &serde_json::Value) -> serde_json::Value {
        let task = args["task"].as_str().unwrap_or("");
        let max_nodes = args["max_nodes"].as_u64().unwrap_or(20) as usize;
        
        // Simple context building: search for keywords in the task
        let keywords: Vec<&str> = task.split_whitespace()
            .filter(|w| w.len() > 3)
            .take(5)
            .collect();
        
        let kw_count = keywords.len().max(1);
        let mut all_results = Vec::new();
        for kw in keywords {
            if let Ok(mut results) = Handle::current().block_on(self.graph.search(kw, max_nodes / kw_count)) {
                all_results.append(&mut results);
            }
        }
        
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&all_results).unwrap_or_default()
            }]
        })
    }

    fn tool_callers(&self, args: &serde_json::Value) -> serde_json::Value {
        let node_id = args["node_id"].as_i64().unwrap_or(0);
        let depth = args["depth"].as_u64().unwrap_or(3) as usize;
        
        match Handle::current().block_on(self.graph.get_callers(node_id, depth)) {
            Ok(results) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results).unwrap_or_default()
                }]
            }),
            Err(e) => serde_json::json!({
                "error": format!("Callers query failed: {}", e)
            }),
        }
    }

    fn tool_callees(&self, args: &serde_json::Value) -> serde_json::Value {
        let node_id = args["node_id"].as_i64().unwrap_or(0);
        let depth = args["depth"].as_u64().unwrap_or(3) as usize;
        
        match Handle::current().block_on(self.graph.get_callees(node_id, depth)) {
            Ok(results) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&results).unwrap_or_default()
                }]
            }),
            Err(e) => serde_json::json!({
                "error": format!("Callees query failed: {}", e)
            }),
        }
    }

    fn tool_impact(&self, args: &serde_json::Value) -> serde_json::Value {
        let node_id = args["node_id"].as_i64().unwrap_or(0);
        let depth = args["depth"].as_u64().unwrap_or(2) as usize;
        
        // Impact = callers + callees
        let callers = Handle::current().block_on(self.graph.get_callers(node_id, depth)).unwrap_or_default();
        let callees = Handle::current().block_on(self.graph.get_callees(node_id, depth)).unwrap_or_default();
        
        serde_json::json!({
            "content": [{
                "type": "text",
                "text": serde_json::to_string_pretty(&{
                    serde_json::json!({
                        "callers": callers,
                        "callees": callees
                    })
                }).unwrap_or_default()
            }]
        })
    }

    fn tool_node(&self, args: &serde_json::Value) -> serde_json::Value {
        let node_id = args["node_id"].as_i64().unwrap_or(0);
        
        match Handle::current().block_on(self.graph.get_node(node_id)) {
            Ok(Some(node)) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&node).unwrap_or_default()
                }]
            }),
            Ok(None) => serde_json::json!({
                "error": format!("Node {} not found", node_id)
            }),
            Err(e) => serde_json::json!({
                "error": format!("Node query failed: {}", e)
            }),
        }
    }

    fn tool_files(&self, args: &serde_json::Value) -> serde_json::Value {
        let _ = args;
        
        match Handle::current().block_on(self.graph.files()) {
            Ok(files) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&files).unwrap_or_default()
                }]
            }),
            Err(e) => serde_json::json!({
                "error": format!("Files query failed: {}", e)
            }),
        }
    }

    fn tool_status(&self, args: &serde_json::Value) -> serde_json::Value {
        let _ = args;
        
        match Handle::current().block_on(self.graph.stats()) {
            Ok(stats) => serde_json::json!({
                "content": [{
                    "type": "text",
                    "text": serde_json::to_string_pretty(&stats).unwrap_or_default()
                }]
            }),
            Err(e) => serde_json::json!({
                "error": format!("Status query failed: {}", e)
            }),
        }
    }
}
