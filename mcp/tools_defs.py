"""MCP tool definitions for Rairos — name, description, inputSchema per tool."""

from typing import Any, Dict, List


def get_tools() -> List[Dict[str, Any]]:
    """Return list of available tools."""
    return [
        {
            "name": "paper_ingest",
            "description": "Import a paper from arXiv ID, DOI, or PDF file into Rairos",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "identifier": {
                        "type": "string",
                        "description": "arXiv ID (e.g. '2601.00155'), DOI, or path to PDF file",
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Topic tags for the paper",
                    },
                },
                "required": ["identifier"],
            },
        },
        {
            "name": "paper_search",
            "description": "Search papers across Rairos local DB and external sources (arXiv, Semantic Scholar, Crossref)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query": {
                        "type": "string",
                        "description": "Search query (title, author, or keyword)",
                    },
                    "source": {
                        "type": "string",
                        "description": "Source to search: 'local', 'arxiv', 'semantic_scholar', 'crossref', or 'all'",
                        "enum": ["local", "arxiv", "semantic_scholar", "crossref", "all"],
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Maximum results (default 10)",
                    },
                },
                "required": ["query"],
            },
        },
        {
            "name": "paper_chat",
            "description": "Ask questions about papers in your library using RAG (Retrieval-Augmented Generation)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Your research question",
                    },
                    "paper_id": {
                        "type": "string",
                        "description": "Optional: scope to a specific paper",
                    },
                    "top_k": {
                        "type": "integer",
                        "description": "Number of chunks to retrieve (default 5)",
                    },
                },
                "required": ["question"],
            },
        },
        {
            "name": "paper_recommend",
            "description": "Get paper recommendations based on your reading history using collaborative filtering",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "strategy": {
                        "type": "string",
                        "description": "Recommendation strategy: 'recent', 'top_tags', 'underread', 'diverse', or 'cold_start'",
                        "enum": ["recent", "top_tags", "underread", "diverse", "cold_start"],
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Number of recommendations (default 5)",
                    },
                },
            },
        },
        {
            "name": "pdf_download",
            "description": "Download a PDF from arXiv",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv paper ID",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "pdf_extract_text",
            "description": "Extract plain text from a PDF file (with OCR and pdfminer fallback)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "pdf_extract_structured",
            "description": "Extract structured content from PDF (text blocks, tables, math)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "kg_query",
            "description": "Query the knowledge graph — stats, papers, tags, or neighbor queries",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "query_type": {
                        "type": "string",
                        "description": "Type of KG query",
                        "enum": ["stats", "papers", "tags", "neighbors"],
                    },
                    "limit": {
                        "type": "integer",
                        "description": "Max results",
                    },
                },
            },
        },
        {
            "name": "kg_paper_subgraph",
            "description": "Get the ego graph (subgraph) around a specific paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID to get subgraph for",
                    },
                    "max_nodes": {
                        "type": "integer",
                        "description": "Max nodes to include (default 20)",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "kg_tag_graph",
            "description": "Get papers and notes for a specific tag",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tag": {
                        "type": "string",
                        "description": "Tag name",
                    },
                },
                "required": ["tag"],
            },
        },
        {
            "name": "kg_full_graph",
            "description": "Export the full knowledge graph (up to N nodes)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "max_nodes": {
                        "type": "integer",
                        "description": "Max nodes (default 100)",
                    },
                },
            },
        },
        {
            "name": "tag_add",
            "description": "Add one or more tags to a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags to add",
                    },
                },
                "required": ["paper_id", "tags"],
            },
        },
        {
            "name": "tag_remove",
            "description": "Remove one or more tags from a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                    "tags": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Tags to remove",
                    },
                },
                "required": ["paper_id", "tags"],
            },
        },
        {
            "name": "tag_list",
            "description": "List all tags for a given paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "tag_all",
            "description": "Get all known tags and their paper counts",
            "inputSchema": {
                "type": "object",
                "properties": {},
            },
        },
        {
            "name": "trends_detect_trending",
            "description": "Detect trending research topics from radar history using OLS regression",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "threshold": {
                        "type": "number",
                        "description": "Slope threshold for 'trending' (default 0.1)",
                    },
                },
            },
        },
        {
            "name": "trends_predict_next",
            "description": "Predict the next heat score for a given tag using Holt's exponential smoothing",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tag": {
                        "type": "string",
                        "description": "Research tag to forecast",
                    },
                },
                "required": ["tag"],
            },
        },
        {
            "name": "trends_top_predictions",
            "description": "Get top-k predicted trending tags ranked by predicted_score * confidence",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "k": {
                        "type": "integer",
                        "description": "Number of predictions (default 5)",
                    },
                },
            },
        },
        {
            "name": "trends_compare_tags",
            "description": "Compare trends trajectories of two tags side by side",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "tag_a": {
                        "type": "string",
                        "description": "First tag",
                    },
                    "tag_b": {
                        "type": "string",
                        "description": "Second tag",
                    },
                },
                "required": ["tag_a", "tag_b"],
            },
        },
        {
            "name": "chart_query",
            "description": "Query figures, tables, and extracted chart data from papers",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                    "query": {
                        "type": "string",
                        "description": "Query for figures/tables (e.g. 'accuracy', 'loss curve')",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "research_run",
            "description": "Run the autonomous research loop — search, download, extract, LLM analyze, save",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic to explore",
                    },
                    "max_papers": {
                        "type": "integer",
                        "description": "Max papers to process (default 3)",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "slides_generate",
            "description": "Generate a slide deck from a paper's content",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "cite_fetch",
            "description": "Fetch citation metadata for a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv paper ID",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "paper_analyze",
            "description": "Perform full structured analysis of a paper (methods, datasets, metrics, claims)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID to analyze",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "paper2code_run",
            "description": "Run the full paper2code pipeline: download paper -> parse -> generate code skeleton -> extract tests -> run benchmark -> encode successful pattern to Gene Pool",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper to implement",
                    },
                    "framework": {
                        "type": "string",
                        "description": "Target framework: 'pytorch', 'jax', or 'numpy'",
                        "enum": ["pytorch", "jax", "numpy"],
                    },
                    "skip_gene_pool": {
                        "type": "boolean",
                        "description": "Skip Gene Pool encoding",
                    },
                    "continuous": {
                        "type": "boolean",
                        "description": "Run in continuous mode (poll arXiv subscriptions)",
                    },
                    "interval_minutes": {
                        "type": "integer",
                        "description": "Poll interval in minutes (default 15)",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "citation_graph",
            "description": "Build a citation graph for a given paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID to build citation graph for",
                    },
                    "depth": {
                        "type": "integer",
                        "description": "Max depth of citation traversal (default 2)",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "gap_detect",
            "description": "Detect research gaps from the paper corpus for a topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic to analyze",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "gap_submit",
            "description": "Submit a gap directly to the Gene Pool as a CapsuleGene",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic",
                    },
                    "gap_type": {
                        "type": "string",
                        "description": "Type of gap",
                    },
                    "title": {
                        "type": "string",
                        "description": "Gap title",
                    },
                    "description": {
                        "type": "string",
                        "description": "Gap description",
                    },
                    "keywords": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Keywords for matching",
                    },
                },
                "required": ["topic", "gap_type", "title"],
            },
        },
        {
            "name": "gap_evolve",
            "description": "Run the Gene Pool evolution cycle (audit-propose-evaluate-apply)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Optional: scope evolution to a specific topic",
                    },
                },
            },
        },
        {
            "name": "gene_pool_watcher",
            "description": "Auto-detect Gene Pool diversity gaps and create ArXiv subscriptions to fill them. Call trigger_now() for an immediate check, or start() to run continuously in background.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action: 'status' (default), 'trigger_now', 'start', or 'stop'",
                    },
                    "interval_minutes": {
                        "type": "integer",
                        "description": "Check interval in minutes (default 60, only for 'start' action)",
                    },
                    "min_diversity_score": {
                        "type": "number",
                        "description": "Trigger gap-filling only when diversity_score falls below this threshold (default 50.0)",
                    },
                },
            },
        },
        {
            "name": "claim_graph",
            "description": "Cross-paper numerical claim tracking and contradiction detection. Find claim conflicts between papers, render as D3.js graph, and import claims from Gene Pool capsules.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "action": {
                        "type": "string",
                        "description": "Action: 'status' (default), 'add_claim', 'add_edge', 'contradictions', 'render', 'export', 'import_capsules'",
                    },
                    "paper_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper (for 'add_claim')",
                    },
                    "claim_type": {
                        "type": "string",
                        "description": "Type: accuracy | speedup | reduction | param_size | memory | other (for 'add_claim', 'add_edge')",
                    },
                    "value": {
                        "type": "number",
                        "description": "Claimed numerical value (for 'add_claim')",
                    },
                    "source_text": {
                        "type": "string",
                        "description": "Original paper text snippet (for 'add_claim', 'add_edge')",
                    },
                    "from_paper": {
                        "type": "string",
                        "description": "Source arXiv ID for improvement claim (for 'add_edge')",
                    },
                    "to_paper": {
                        "type": "string",
                        "description": "Target arXiv ID being compared (for 'add_edge')",
                    },
                    "improvement_ratio": {
                        "type": "number",
                        "description": "Improvement multiplier, e.g. 1.23 means 23% better (for 'add_edge')",
                    },
                },
            },
        },
        {
            "name": "research_agent_start",
            "description": "Start the background autonomous research agent for a topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic to watch",
                    },
                    "interval_minutes": {
                        "type": "integer",
                        "description": "Polling interval in minutes (default 60)",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "research_agent_stop",
            "description": "Stop the background autonomous research agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Topic of the agent to stop",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "research_agent_status",
            "description": "Get the status of all background research agents",
            "inputSchema": {
                "type": "object",
                "properties": {},
            },
        },
        {
            "name": "research_agent_trigger",
            "description": "Manually trigger a research cycle for a specific agent",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Topic of the agent to trigger",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "hypothesis_generate",
            "description": "Generate testable hypotheses from research gaps",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "gap_type": {
                        "type": "string",
                        "description": "Type of gap to hypothesize about",
                    },
                    "topic": {
                        "type": "string",
                        "description": "Research topic",
                    },
                },
                "required": ["gap_type", "topic"],
            },
        },
        {
            "name": "hypothesis_list",
            "description": "List generated hypotheses with verdicts",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "gap_type": {
                        "type": "string",
                        "description": "Optional: filter by gap type",
                    },
                },
            },
        },
        {
            "name": "experiment_record",
            "description": "Record experiment results against a hypothesis",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "hypothesis_id": {
                        "type": "string",
                        "description": "Hypothesis ID",
                    },
                    "verdict": {
                        "type": "string",
                        "description": "Experiment verdict: 'supported', 'partially_supported', 'rejected', or 'inconclusive'",
                        "enum": ["supported", "partially_supported", "rejected", "inconclusive"],
                    },
                    "notes": {
                        "type": "string",
                        "description": "Experiment notes",
                    },
                },
                "required": ["hypothesis_id", "verdict"],
            },
        },
        {
            "name": "litreview_generate",
            "description": "Generate a structured literature review for a topic",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Research topic for the literature review",
                    },
                    "max_papers": {
                        "type": "integer",
                        "description": "Max papers to include (default 10)",
                    },
                },
                "required": ["topic"],
            },
        },
        {
            "name": "litreview_list",
            "description": "List saved literature reviews",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "topic": {
                        "type": "string",
                        "description": "Optional: filter by topic",
                    },
                },
            },
        },
        {
            "name": "research_memory_add_stance",
            "description": "Record a research stance (supported/rejected/deferred) for a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                    "claim": {
                        "type": "string",
                        "description": "The claim being evaluated",
                    },
                    "stance": {
                        "type": "string",
                        "description": "Stance: 'supports', 'rejects', or 'neutral'",
                        "enum": ["supports", "rejects", "neutral"],
                    },
                },
                "required": ["paper_id", "claim", "stance"],
            },
        },
        {
            "name": "research_memory_list_stances",
            "description": "List all recorded research stances",
            "inputSchema": {
                "type": "object",
                "properties": {},
            },
        },
        {
            "name": "research_memory_check_paper",
            "description": "Check if a paper is already in research memory",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "research_memory_anomalies",
            "description": "Detect stance anomalies — papers that contradict the majority view",
            "inputSchema": {
                "type": "object",
                "properties": {},
            },
        },
        {
            "name": "review_simulate",
            "description": "Simulate an adversarial peer review for a paper with multiple reviewer personas",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID to review",
                    },
                    "reviewer_count": {
                        "type": "integer",
                        "description": "Number of simulated reviewers (default 3)",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "review_list",
            "description": "List all simulated peer reviews",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Optional: filter by paper",
                    },
                },
            },
        },
        {
            "name": "routeplan_create",
            "description": "Create a multi-step research route plan",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "goal": {
                        "type": "string",
                        "description": "Overall research goal",
                    },
                    "steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Ordered list of steps to achieve the goal",
                    },
                },
                "required": ["goal", "steps"],
            },
        },
        {
            "name": "routeplan_list",
            "description": "List all research route plans",
            "inputSchema": {
                "type": "object",
                "properties": {},
            },
        },
        {
            "name": "routeplan_update_step",
            "description": "Update the status of a route plan step",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "plan_id": {
                        "type": "string",
                        "description": "Route plan ID",
                    },
                    "step_index": {
                        "type": "integer",
                        "description": "Step index (0-based)",
                    },
                    "status": {
                        "type": "string",
                        "description": "New status: 'pending', 'in_progress', 'done', or 'blocked'",
                        "enum": ["pending", "in_progress", "done", "blocked"],
                    },
                    "notes": {
                        "type": "string",
                        "description": "Optional notes",
                    },
                },
                "required": ["plan_id", "step_index", "status"],
            },
        },
        {
            "name": "routeplan_revise",
            "description": "Revise a route plan with new steps or adjustments",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "plan_id": {
                        "type": "string",
                        "description": "Route plan ID",
                    },
                    "steps": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "Revised ordered list of steps",
                    },
                },
                "required": ["plan_id", "steps"],
            },
        },
        {
            "name": "briefing_generate",
            "description": "Generate a research briefing for a paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "arxiv_id": {
                        "type": "string",
                        "description": "arXiv ID of the paper",
                    },
                },
                "required": ["arxiv_id"],
            },
        },
        {
            "name": "citation_chain_build",
            "description": "Build a citation chain starting from a seed paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "seed_arxiv_id": {
                        "type": "string",
                        "description": "Seed arXiv ID",
                    },
                    "max_depth": {
                        "type": "integer",
                        "description": "Max traversal depth (default 2)",
                    },
                },
                "required": ["seed_arxiv_id"],
            },
        },
        {
            "name": "citation_chain_families",
            "description": "Find citation families (groups of related papers) in a citation chain",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chain_data": {
                        "type": "string",
                        "description": "JSON-serialized chain data from citation_chain_build",
                    },
                },
                "required": ["chain_data"],
            },
        },
        {
            "name": "citation_chain_silent",
            "description": "Detect silent citations — papers that should cite each other but don't",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chain_data": {
                        "type": "string",
                        "description": "JSON-serialized chain data from citation_chain_build",
                    },
                },
                "required": ["chain_data"],
            },
        },
        {
            "name": "citation_chain_render",
            "description": "Render a citation chain as Mermaid diagram, graphviz DOT, or text",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "chain_data": {
                        "type": "string",
                        "description": "JSON-serialized chain data",
                    },
                    "format": {
                        "type": "string",
                        "description": "Output format: 'mermaid', 'graphviz', or 'text'",
                        "enum": ["mermaid", "graphviz", "text"],
                    },
                },
                "required": ["chain_data", "format"],
            },
        },
        {
            "name": "impact_rank",
            "description": "Rank papers by composite impact score (citations, PageRank, author h-index)",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "top_k": {
                        "type": "integer",
                        "description": "Number of top papers (default 20)",
                    },
                },
            },
        },
        {
            "name": "impact_score_paper",
            "description": "Get the impact score for a specific paper",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "impact_leaderboard",
            "description": "Get the impact leaderboard across all papers",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "top_k": {
                        "type": "integer",
                        "description": "Number of top papers (default 10)",
                    },
                },
            },
        },
        {
            "name": "replication_check",
            "description": "Check if a paper's results can be reproduced from available code and data signals",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_id": {
                        "type": "string",
                        "description": "Paper ID to check",
                    },
                },
                "required": ["paper_id"],
            },
        },
        {
            "name": "replication_compare",
            "description": "Compare replication results across multiple papers",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "paper_ids": {
                        "type": "array",
                        "items": {"type": "string"},
                        "description": "List of paper IDs to compare",
                    },
                },
                "required": ["paper_ids"],
            },
        },
        {
            "name": "rairos",
            "description": "Ask the Rairos research assistant a question about your paper library or research topics",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "question": {
                        "type": "string",
                        "description": "Your research question",
                    },
                },
                "required": ["question"],
            },
        },
    ]
