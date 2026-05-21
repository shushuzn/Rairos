use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use crate::llm_handlers::helpers::gene_pool_data_dir;
use async_trait::async_trait;
use rand::{Rng, SeedableRng};
use rairos_gene_pool_watcher::GenePoolWatcher;
use rairos_llm::insight::crossover::CapsuleGene;
use rairos_llm::insight::storage::CapsuleStorage;
use serde_json::Value;

fn compute_impact_score(
    capsule: &CapsuleGene,
    lambda_: f64,
) -> f64 {
    let age_days = chrono::DateTime::parse_from_rfc3339(&capsule.created_at)
        .map(|dt| {
            let now = chrono::Utc::now();
            let dur = now.signed_duration_since(dt.with_timezone(&chrono::Utc));
            dur.num_days() as f64
        })
        .unwrap_or(0.0)
        .max(0.0);

    let recency = (-lambda_ * age_days).exp();
    let quality = capsule.outcome_success_score;
    let feedback_boost = (capsule.feedback_count as f64).ln_1p() * 0.1;
    let credibility = capsule.credibility_score;

    (quality * 0.5 + credibility * 0.3 + feedback_boost * 0.2) * recency
}

pub struct GenePoolDecayHandler;

#[async_trait]
impl ToolHandler for GenePoolDecayHandler {
    fn name(&self) -> &str { "gene_pool_decay" }
    fn description(&self) -> &str { "Time-weighted impact scoring and auto-archive for Gene Pool capsules" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, rank, or archived (default: status)")),
                ("min_impact".into(), ToolProperty::string("Minimum impact threshold (default: 0.1)")),
                ("lambda_".into(), ToolProperty::string("Decay rate lambda (default: 0.01)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let min_impact = params.get("min_impact").and_then(|v| v.as_f64()).unwrap_or(0.1);
        let lambda_ = params.get("lambda_").and_then(|v| v.as_f64()).unwrap_or(0.01);

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .await.map_err(|e| format!("Failed to open gene pool storage: {}", e))?;
        let capsules = storage.load_all_capsules()
            .await.map_err(|e| format!("Failed to load capsules: {}", e))?;

        match action {
            "rank" => {
                let mut scored: Vec<_> = capsules.iter().map(|c| {
                    (c, compute_impact_score(c, lambda_))
                }).collect();
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                let ranked: Vec<Value> = scored.iter().enumerate().map(|(i, (c, s))| {
                    serde_json::json!({
                        "rank": i + 1,
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "title": c.action_gap_title,
                        "topic": c.trigger_topic,
                        "success_score": c.outcome_success_score,
                    })
                }).collect();
                Ok(serde_json::json!({ "ranked": ranked, "total": scored.len() }))
            }
            "archived" => {
                let archived: Vec<Value> = capsules.iter().filter(|c| c.status == "archived").map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "archived_at": c.created_at,
                    })
                }).collect();
                Ok(serde_json::json!({ "archived": archived, "total": archived.len() }))
            }
            _ => {
                let active: Vec<&CapsuleGene> = capsules.iter().filter(|c| c.status == "active").collect();
                let mut scored: Vec<_> = active.iter().map(|c| {
                    let impact = compute_impact_score(c, lambda_);
                    (c, impact)
                }).collect();
                scored.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));

                let top: Vec<Value> = scored.iter().take(10).map(|(c, s)| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "success_score": c.outcome_success_score,
                        "feedback_count": c.feedback_count,
                        "credibility_score": c.credibility_score,
                        "age_days": chrono::DateTime::parse_from_rfc3339(&c.created_at)
                            .map(|dt| chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc)).num_days())
                            .unwrap_or(0),
                    })
                }).collect();
                let bottom: Vec<Value> = scored.iter().rev().take(5).map(|(c, s)| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "impact_score": (s * 1000.0).round() / 1000.0,
                        "age_days": chrono::DateTime::parse_from_rfc3339(&c.created_at)
                            .map(|dt| chrono::Utc::now().signed_duration_since(dt.with_timezone(&chrono::Utc)).num_days())
                            .unwrap_or(0),
                    })
                }).collect();

                let total_scored = scored.len();
                let below_threshold = scored.iter().filter(|(_, s)| *s < min_impact).count();

                Ok(serde_json::json!({
                    "total_scored": total_scored,
                    "below_threshold": below_threshold,
                    "min_impact": min_impact,
                    "top_capsules": top,
                    "bottom_capsules": bottom,
                }))
            }
        }
    }
}

pub struct CrossoverHandler;

#[async_trait]
impl ToolHandler for CrossoverHandler {
    fn name(&self) -> &str { "crossover" }
    fn description(&self) -> &str { "Run CapsuleGene genetic algorithm: select parents, crossover, mutate, encode V3" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: evolve, rank_v3, mutate, or best (default: evolve)")),
                ("offspring_count".into(), ToolProperty::integer("Number of offspring to produce (default: 5)")),
                ("capsule_id".into(), ToolProperty::string("Capsule ID for mutate/lineage actions (optional)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("evolve");
        let offspring_count = params.get("offspring_count").and_then(|v| v.as_u64()).unwrap_or(5) as usize;
        let capsule_id = params.get("capsule_id").and_then(|v| v.as_str());

        let data_dir = gene_pool_data_dir();
        let storage = CapsuleStorage::new(&data_dir)
            .await.map_err(|e| format!("Failed to open gene pool storage: {}", e))?;

        match action {
            "rank_v3" => {
                let capsules = storage.load_all_capsules()
                    .await.map_err(|e| format!("Failed to load capsules: {}", e))?;
                let v3: Vec<serde_json::Value> = capsules.iter()
                    .filter(|c| c.evolved_generation >= 1 && c.status == "active")
                    .map(|c| {
                        let fitness = rairos_llm::insight::crossover::compute_fitness(c);
                        serde_json::json!({
                            "capsule_id": c.capsule_id,
                            "title": c.action_gap_title,
                            "evolved_generation": c.evolved_generation,
                            "fitness": (fitness * 1000.0).round() / 1000.0,
                            "success_score": c.outcome_success_score,
                        })
                    })
                    .collect();
                Ok(serde_json::json!({ "v3_capsules": v3, "total_v3": v3.len() }))
            }
            "mutate" => {
                let cid = capsule_id.ok_or("capsule_id required for mutate action")?;
                let all = storage.load_all_capsules()
                    .await.map_err(|e| format!("Failed to load capsules: {}", e))?;
                let pos = all.iter().position(|c| c.capsule_id == cid)
                    .ok_or_else(|| format!("Capsule {} not found", cid))?;
                let mut capsule = all[pos].clone();
                let mutated_arch = rairos_llm::insight::crossover::mutate_archetype(capsule.archetype.clone());
                capsule.archetype = mutated_arch;
                storage.save_capsules(&[capsule.clone()])
                    .await.map_err(|e| format!("Failed to save mutated capsule: {}", e))?;
                Ok(serde_json::json!({
                    "mutated": {
                        "capsule_id": capsule.capsule_id,
                        "title": capsule.action_gap_title,
                        "status": "mutated"
                    }
                }))
            }
            "best" => {
                let capsules = storage.load_all_capsules()
                    .await.map_err(|e| format!("Failed to load capsules: {}", e))?;
                let mut active: Vec<_> = capsules.iter()
                    .filter(|c| c.status == "active" && c.credibility_badge != "low")
                    .collect();
                active.sort_by(|a, b| {
                    rairos_llm::insight::crossover::compute_fitness(b)
                        .partial_cmp(&rairos_llm::insight::crossover::compute_fitness(a))
                        .unwrap_or(std::cmp::Ordering::Equal)
                });
                let candidates: Vec<Value> = active.iter().take(offspring_count).map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "fitness": (rairos_llm::insight::crossover::compute_fitness(c) * 1000.0).round() / 1000.0,
                        "success_score": c.outcome_success_score,
                        "credibility_badge": c.credibility_badge,
                    })
                }).collect();
                Ok(serde_json::json!({ "candidates": candidates, "total": candidates.len() }))
            }
            _ => {
                let all = storage.load_all_capsules()
                    .await.map_err(|e| format!("Failed to load capsules: {}", e))?;
                let active: Vec<CapsuleGene> = all.into_iter()
                    .filter(|c| c.status == "active")
                    .collect();

                if active.len() < 2 {
                    return Ok(serde_json::json!({
                        "error": "Need at least 2 active capsules for crossover",
                        "active_count": active.len(),
                    }));
                }

                let mut rng = rand::rngs::StdRng::from_entropy();
                let count = offspring_count.min(active.len() / 2);
                let mut offspring = Vec::new();
                let mut parents_used = Vec::new();

                for _ in 0..count {
                    let idx_a = rng.gen_range(0..active.len());
                    let idx_b = rng.gen_range(0..active.len());
                    if idx_a == idx_b { continue; }

                    let parent_a = &active[idx_a];
                    let parent_b = &active[idx_b];

                    let cross_result = rairos_llm::insight::crossover::crossover(parent_a, parent_b);
                    let mutated_arch = rairos_llm::insight::crossover::mutate_archetype(cross_result.archetype);

                    let child = CapsuleGene {
                        capsule_id: uuid::Uuid::new_v4().to_string()[..12].to_string(),
                        created_at: chrono::Utc::now().to_rfc3339(),
                        trigger_topic: format!("{} & {}", parent_a.trigger_topic, parent_b.trigger_topic),
                        trigger_gap_type: parent_a.trigger_gap_type.clone(),
                        trigger_keywords: {
                            let mut kws = parent_a.trigger_keywords.clone();
                            kws.extend(parent_b.trigger_keywords.clone());
                            kws.sort();
                            kws.dedup();
                            kws.truncate(15);
                            kws
                        },
                        action_gap_type: parent_a.action_gap_type.clone(),
                        action_gap_title: format!("Crossover: {} x {}", parent_a.action_gap_title, parent_b.action_gap_title),
                        outcome_success_score: cross_result.parent_fitness_a.max(cross_result.parent_fitness_b).min(1.0),
                        feedback_count: 0,
                        evolved_generation: cross_result.parent_generations,
                        archetype: mutated_arch,
                        status: "active".to_string(),
                        low_score_streak: 0,
                        credibility_score: 0.5,
                        trendslop: false,
                        trendslop_reason: String::new(),
                        credibility_badge: "medium".to_string(),
                        source_arxiv_category: parent_a.source_arxiv_category.clone(),
                    };

                    parents_used.push((parent_a.capsule_id.clone(), parent_b.capsule_id.clone()));
                    offspring.push(child);
                }

                storage.save_capsules(&offspring)
                    .await.map_err(|e| format!("Failed to save offspring: {}", e))?;

                let offspring_json: Vec<Value> = offspring.iter().map(|c| {
                    serde_json::json!({
                        "capsule_id": c.capsule_id,
                        "title": c.action_gap_title,
                        "evolved_generation": c.evolved_generation,
                    })
                }).collect();

                Ok(serde_json::json!({
                    "offspring": offspring_json,
                    "total_new": offspring.len(),
                    "parents_used": parents_used.iter().map(|(a, b)| serde_json::json!({"parent_a": a, "parent_b": b})).collect::<Vec<Value>>(),
                }))
            }
        }
    }
}

pub struct GenePoolWatcherHandler;

#[async_trait]
impl ToolHandler for GenePoolWatcherHandler {
    fn name(&self) -> &str { "gene_pool_watcher" }
    fn description(&self) -> &str { "Manage GenePoolWatcher: check diversity gaps and auto-subscribe" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("action".into(), ToolProperty::string("Action: status, start, stop, or trigger_now")),
                ("interval_minutes".into(), ToolProperty::integer("Check interval in minutes")),
                ("min_diversity_score".into(), ToolProperty::string("Minimum diversity score threshold (0-100)")),
            ].into_iter().collect(),
            vec![],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let action = params.get("action").and_then(|v| v.as_str()).unwrap_or("status");
        let interval = params.get("interval_minutes").and_then(|v| v.as_u64()).unwrap_or(60);
        let min_score = params.get("min_diversity_score").and_then(|v| v.as_f64()).unwrap_or(50.0);

        match action {
            "start" => {
                let mut watcher = GenePoolWatcher::new(interval, min_score, true);
                let result = watcher.watch();
                Ok(serde_json::json!({
                    "status": "started",
                    "message": format!("GenePoolWatcher started. Will check diversity every {}min.", interval),
                    "diversity_score": result.diversity_score,
                    "underrepresented_families": result.underrepresented_families,
                }))
            }
            "trigger_now" => {
                let mut watcher = GenePoolWatcher::new(interval, min_score, true);
                let result = watcher.watch();
                Ok(serde_json::json!({
                    "status": "checked",
                    "diversity_score": result.diversity_score,
                    "total_capsules": result.total_capsules,
                    "underrepresented_families": result.underrepresented_families,
                    "gap_subscriptions_added": result.gap_subscriptions_added,
                    "gap_subscriptions_removed": result.gap_subscriptions_removed,
                    "triggered": result.triggered,
                }))
            }
            _ => {
                let watcher = GenePoolWatcher::new(interval, min_score, false);
                let state = watcher.get_state();
                Ok(serde_json::json!({
                    "status": "ok",
                    "diversity_score": state.diversity_score,
                    "underrepresented_families": state.underrepresented_families,
                    "gap_subscriptions": state.gap_subscriptions.iter().map(|gs| {
                        serde_json::json!({
                            "family": gs.family,
                            "enabled": gs.enabled,
                            "keywords": gs.keywords,
                        })
                    }).collect::<Vec<_>>(),
                }))
            }
        }
    }
}
