# Rairos MCP: 19 Python Fallback Tools — Porting Analysis

Generated: 2026-05-14

## Methodology

For each tool I:
1. Read the full Python implementation from `rairos_mcp.py`
2. Identified Python-only imports (modules not yet in Rust)
3. Checked all 153 Rust crates for existing backend implementations
4. Checked existing Rust MCP handlers for overlap
5. Estimated port effort based on similarity to already-ported tools

---

## Tool-by-Tool Analysis

### 1. tag_all (line 227, 15 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `db.database.Database` |
| **Complexity** | 🟢 LOW |
| **What it does** | Single SQL query: `SELECT name FROM tags ORDER BY name` |
| **Rust backend** | ✅ `rairos-core` has `list_tags()` (line 787 in lib.rs) |
| **Rust MCP** | `TagListHandler` exists but reads from JSONL file, not SQLite DB |
| **Port effort** | 1 day. Just call `rairos_core::Database::list_tags()` and format response |
| **Recommendation** | **PORT** — trivial, pure DB query, Rust backend already exists |

### 2. chart_query (line 282, 79 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `kg.manager.KGManager`, `pdf.chart_kg.ChartKGExtractor` |
| **Complexity** | 🔴 HIGH |
| **What it does** | Queries figures/tables from knowledge graph by paper_id + action |
| **Rust backend** | ⚠️ `rairos-viz` exists but is for **chart generation** (benchmarks), not KG chart query. `rairos-kg` exists for general graph but not chart-specific. |
| **Rust MCP** | ❌ None |
| **Port effort** | ~2 weeks. Need to port `ChartKGExtractor` logic and `KGManager` integration |
| **Recommendation** | **KEEP PYTHON** — tightly coupled to `pdf.chart_kg` Python module with no Rust equivalent |

### 3. gene_pool_watcher (line 754, 64 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.gene_pool_watcher.GenePoolWatcher`, `llm.gene_pool_io.get_gene_pool_diversity` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | start/stop/trigger/status for gene pool diversity watcher |
| **Rust backend** | ✅ `rairos-gene-pool-watcher` (1040 LOC) — has `DiversityPressureEvaluator`, `load_watcher_state`, `save_watcher_state`, `diff_subscriptions`, state persistence |
| **Rust MCP** | ❌ Not wired into MCP yet |
| **Port effort** | 3-5 days. Rust backend exists; needs threading/daemon loop + MCP handler wiring |
| **Recommendation** | **PORT** — substantial Rust backend exists, just needs MCP integration |

### 4. research_run (line 363, 61 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `research_loop.core.search_arxiv`, `db.database.Database`, `core.Paper` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Search arXiv → save papers to DB. Simple orchestrator. |
| **Rust backend** | ✅ `rairos-research` has `arxiv_search.rs`. `rairos-core` has Database with `upsert_paper`. |
| **Rust MCP** | ❌ None |
| **Port effort** | 3-5 days. Two sub-calls: arXiv search → DB upsert. Manageable. |
| **Recommendation** | **SPLIT** — arXiv search porting (rairos-parser/rairos-research already have it) + DB save (rairos-core). Keep as thin Rust MCP handler. |

### 5. paper2code_run (line 496, 85 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `research_loop.paper2code_integration.PaperPipeline`, `llm.subscription_monitor.SubscriptionMonitor`, `db.database.Database` |
| **Complexity** | 🔴 HIGH |
| **What it does** | Multi-step pipeline: download → parse → generate → test → benchmark → Gene Pool. Plus continuous mode with threading. |
| **Rust backend** | ❌ None. Calls external `PaperPipeline` (Python-only skill), `SubscriptionMonitor` (Python LLM module). |
| **Rust MCP** | ❌ None |
| **Port effort** | 2-4 weeks. Requires porting PaperPipeline + SubscriptionMonitor + threading |
| **Recommendation** | **KEEP PYTHON** — heavyweight orchestrator calling multiple Python-only modules (paper2code skill, subscription monitor) |

### 6-9. research_agent_start/stop/status/trigger (lines 656-882, 15-30 LOC each)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `research_loop.orchestrator.AutonomousOrchestrator` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Thin wrappers (15-30 LOC each) around `AutonomousOrchestrator` class |
| **Rust backend** | ❌ No `rairos-orchestrator` Rust crate. `rairos-daemon` exists but is unrelated. |
| **Rust MCP** | ❌ None |
| **Port effort** | 5-7 days for all 4. But requires porting the entire `AutonomousOrchestrator` first (~2K+ LOC Python). |
| **Recommendation** | **KEEP PYTHON** — thin wrappers are cheap to maintain. Orchestrator is massive Python-only module. |

### 10. hypothesis_generate (line 885, 57 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.research.hypothesis_generator.HypothesisGenerator` |
| **Complexity** | 🔴 HIGH |
| **What it does** | LLM-based hypothesis generation with experiment design, risk assessment, novelty/feasibility scoring |
| **Rust backend** | ✅ `rairos-research/src/hypothesis_generator.rs` has `generate_hypothesis_llm()`, `design_experiment()` |
| **Rust MCP** | ❌ None |
| **Port effort** | 3-5 days. Rust backend exists with async LLM calls, but Python uses richer data model (risk assessment, experiment design sub-structs). |
| **Recommendation** | **PORT** — Rust backend exists, good candidate for LLM-backed MCP tool |

### 11. hypothesis_list (line 944, 43 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.insight.evolution.EvolutionTracker`, `llm.experiment_tracker.ExperimentTracker` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Aggregates hypothesis events + experiment data into verdict view |
| **Rust backend** | ⚠️ `rairos-evolution` has `EvolutionMemory` but with different API (event-based). `rairos-experiment-tracker` exists. |
| **Rust MCP** | ❌ None |
| **Port effort** | 2-4 days. Needs combining data from two Rust crates |
| **Recommendation** | **SPLIT** — keep Python fallback until evolution memory integration solidifies in Rust |

### 12. experiment_record (line 1005, 43 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.experiment_tracker.ExperimentTracker`, uuid |
| **Complexity** | 🟢 LOW-MEDIUM |
| **What it does** | Simple CRUD: create experiment record and save |
| **Rust backend** | ✅ `rairos-experiment-tracker` has `ExperimentTracker::run()`, `Experiment`, `ExperimentStatus` |
| **Rust MCP** | ❌ None |
| **Port effort** | 1 day. Direct 1:1 mapping to Rust crate API |
| **Recommendation** | **PORT** — trivial, Rust backend has full implementation |

### 13. litreview_list (line 1050, 29 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | None (stdlib `pathlib`) |
| **Complexity** | 🟢 LOW |
| **What it does** | Scan `data/litreviews/` directory for markdown files |
| **Rust backend** | ❌ No dedicated crate. `rairos-litreview` exists but is for generation, not listing. |
| **Rust MCP** | ❌ None |
| **Port effort** | 1 day. Pure filesystem scan, no dependencies |
| **Recommendation** | **PORT** — trivial stdlib-only tool |

### 14. review_simulate (line 1249, 54 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.review_simulator.ReviewSimulator`, `db.database.Database`, `llm.review_simulator.save_review` |
| **Complexity** | 🔴 HIGH |
| **What it does** | LLM-powered adversarial peer review simulation with persona selection |
| **Rust backend** | ✅ `rairos-review-simulator` (1221 LOC) — has full `ReviewSimulator` with async `review()`, `save_review`, `default_personas`, `ReviewPersona`, `SimulatedReview` |
| **Rust MCP** | ❌ None |
| **Port effort** | 3-5 days. Rust backend exists but needs DB integration for paper fetching |
| **Recommendation** | **PORT** — Rust crate is well-developed, just needs wiring + DB lookup |

### 15. review_list (line 1305, 10 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.review_simulator.list_reviews` |
| **Complexity** | 🟢 LOW |
| **What it does** | Single function call to list reviews |
| **Rust backend** | ✅ `rairos-review-simulator` has `list_reviews(limit: usize)` function |
| **Rust MCP** | ❌ None |
| **Port effort** | Few hours. Direct 1-line delegation to existing Rust function |
| **Recommendation** | **PORT** — absolute no-brainer |

### 16. routeplan_list (line 1352, 31 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.route_planner.RoutePlanner` |
| **Complexity** | 🟡 LOW-MEDIUM |
| **What it does** | Lists all research plans from RoutePlanner |
| **Rust backend** | ⚠️ `rairos-llm/src/route_planner.rs` has `ResearchPlan` struct, `PlanStep`, `Progress` but no `list_plans()` function |
| **Rust MCP** | `routeplan_create` already ported (RouteQueryHandler). List/update/revise are not. |
| **Port effort** | 2-3 days. Need to implement listing in route_planner.rs |
| **Recommendation** | **PORT** — natural extension of already-ported `routeplan_create` |

### 17. routeplan_update_step (line 1385, 45 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.route_planner.RoutePlanner`, `llm.route_planner.StepStatus` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Updates step status/result/notes in a research plan |
| **Rust backend** | ⚠️ Plan struct has `get_step()`, `get_ready_steps()`, `get_progress()` but no mutation methods |
| **Rust MCP** | ❌ None |
| **Port effort** | 2-3 days. Need to add `update_step()` to route_planner.rs |
| **Recommendation** | **PORT** — complements already-ported `routeplan_create` |

### 18. routeplan_revise (line 1432, 25 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.route_planner.RoutePlanner` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Revises a plan when dead ends are hit |
| **Rust backend** | ❌ No `revise_plan()` in route_planner.rs |
| **Rust MCP** | ❌ None |
| **Port effort** | 3-5 days. Need to implement revision (creates new plan from old plan's state) |
| **Recommendation** | **PORT** — completes the routeplan tool family |

### 19. replication_compare (line 1459, 41 LOC)
| Attribute | Value |
|-----------|-------|
| **Python imports** | `llm.replication_checker.ReplicationChecker`, `parsers.semantic_scholar.get_paper_by_id` |
| **Complexity** | 🟡 MEDIUM |
| **What it does** | Compare replication difficulty of two papers |
| **Rust backend** | ✅ `rairos-replication-checker` has `ReplicationChecker::check_paper()` → `ReplicationReport` with `difficulty_score`, `to_dict()`, `render_report()` |
| **Rust MCP** | `replication_check` (single paper) already ported. Compare is not. |
| **Port effort** | 2-3 days. Call check_paper twice, compare results. Need Semantic Scholar API or pass paper details as params. |
| **Recommendation** | **PORT** — Rust backend exists, natural extension of `replication_check` |

---

## Summary Table

| # | Tool | LOC | Python-Only Imports | Complexity | Rust Backend Exists? | Port Effort | Recommendation |
|---|------|-----|---------------------|------------|---------------------|-------------|----------------|
| 1 | tag_all | 15 | db.database.Database | 🟢 LOW | ✅ rairos-core::list_tags | 1 day | **PORT** |
| 2 | chart_query | 79 | kg.manager, pdf.chart_kg | 🔴 HIGH | ❌ | 2 weeks | **KEEP PYTHON** |
| 3 | gene_pool_watcher | 64 | llm.gene_pool_watcher | 🟡 MEDIUM | ✅ rairos-gene-pool-watcher | 3-5 days | **PORT** |
| 4 | research_run | 61 | research_loop.core, core.Paper | 🟡 MEDIUM | ⚠️ arxiv_search + core | 3-5 days | **SPLIT** |
| 5 | paper2code_run | 85 | research_loop.paper2code, llm.subscription_monitor | 🔴 HIGH | ❌ | 2-4 weeks | **KEEP PYTHON** |
| 6 | research_agent_start | 21 | research_loop.orchestrator | 🟡 MEDIUM | ❌ | 5-7 days (all 4) | **KEEP PYTHON** |
| 7 | research_agent_stop | 13 | research_loop.orchestrator | 🟢 LOW | ❌ | (included above) | **KEEP PYTHON** |
| 8 | research_agent_status | 14 | research_loop.orchestrator | 🟢 LOW | ❌ | (included above) | **KEEP PYTHON** |
| 9 | research_agent_trigger | 17 | research_loop.orchestrator | 🟢 LOW | ❌ | (included above) | **KEEP PYTHON** |
| 10 | hypothesis_generate | 57 | llm.research.hypothesis_generator | 🔴 HIGH | ✅ rairos-research/hypothesis_generator.rs | 3-5 days | **PORT** |
| 11 | hypothesis_list | 43 | llm.insight.evolution, llm.experiment_tracker | 🟡 MEDIUM | ⚠️ evolution + experiment-tracker | 2-4 days | **SPLIT** |
| 12 | experiment_record | 43 | llm.experiment_tracker | 🟢 LOW-MEDIUM | ✅ rairos-experiment-tracker | 1 day | **PORT** |
| 13 | litreview_list | 29 | None (stdlib pathlib) | 🟢 LOW | ❌ (easy to write) | 1 day | **PORT** |
| 14 | review_simulate | 54 | llm.review_simulator, db.database | 🔴 HIGH | ✅ rairos-review-simulator (1221 LOC) | 3-5 days | **PORT** |
| 15 | review_list | 10 | llm.review_simulator | 🟢 LOW | ✅ rairos-review-simulator::list_reviews | Few hours | **PORT** |
| 16 | routeplan_list | 31 | llm.route_planner | 🟡 LOW-MEDIUM | ⚠️ rairos-llm::route_planner (no list_plans) | 2-3 days | **PORT** |
| 17 | routeplan_update_step | 45 | llm.route_planner | 🟡 MEDIUM | ⚠️ rairos-llm::route_planner (no update_step) | 2-3 days | **PORT** |
| 18 | routeplan_revise | 25 | llm.route_planner | 🟡 MEDIUM | ❌ (needs new impl) | 3-5 days | **PORT** |
| 19 | replication_compare | 41 | llm.replication_checker, parsers.semantic_scholar | 🟡 MEDIUM | ✅ rairos-replication-checker | 2-3 days | **PORT** |

---

## Aggregate Statistics

| Decision | Count | Tools |
|----------|-------|-------|
| **PORT to Rust** | **11** | tag_all, gene_pool_watcher, hypothesis_generate, experiment_record, litreview_list, review_simulate, review_list, routeplan_list, routeplan_update_step, routeplan_revise, replication_compare |
| **SPLIT (partially port)** | **2** | research_run, hypothesis_list |
| **KEEP PYTHON** | **6** | chart_query, paper2code_run, research_agent_start, research_agent_stop, research_agent_status, research_agent_trigger |

### Key Observations

1. **LLM-dependent tools** (hypothesis_generate, review_simulate) have solid Rust backends already because rairos-llm and rairos-review-simulator were early porting targets. These are high-value ports.

2. **Pipeline orchestrators** (research_run, paper2code_run, research_agent_*) are thin wrappers over Python-only engine modules. The wrappers are cheap to maintain but the engines would be expensive to port. **Keep the wrappers in Python.**

3. **CRUD tools** (tag_all, experiment_record, review_list, litreview_list) are the easiest ports — pure data access with Rust backends already available.

4. **Route plan tools** form a natural family with `routeplan_create` already ported. Porting the remaining three completes the set nicely.

5. **chart_query** is the hardest tool to port because it depends on `kg.manager` (Python KG) and `pdf.chart_kg` (PDF chart extraction) — neither has a Rust equivalent. **Defer.**

6. The `rairos-review-simulator` crate (1221 LOC) and `rairos-experiment-tracker` crate (572 LOC) are already mature Rust ports of their Python counterparts — utilizing them in MCP is the natural next step.

### Recommended Porting Order (by ROI)

1. **Tier 1 (trivial, high confidence)**: review_list, tag_all, experiment_record, litreview_list
2. **Tier 2 (Rust backend exists, needs wiring)**: review_simulate, gene_pool_watcher, replication_compare
3. **Tier 3 (partial backend, needs extension)**: hypothesis_generate, routeplan_list, routeplan_update_step, routeplan_revise
4. **Tier 4 (split/partial)**: research_run, hypothesis_list
5. **Defer**: chart_query, paper2code_run, research_agent_*
