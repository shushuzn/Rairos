use rairos_core::constants::GP_DIR_NAME;
use std::collections::HashMap;
use std::io::Write;
use std::path::PathBuf;

const CAPSULE_FILE: &str = "capsules.json";
const JSONL_FILE: &str = "gene_pool.jsonl";

fn get_gp_dir() -> PathBuf {
    std::env::var("RAIROS_HOME")
        .ok()
        .map(PathBuf::from)
        .or_else(|| dirs::home_dir().map(|p| p.join(GP_DIR_NAME)))
        .unwrap_or_else(|| PathBuf::from(GP_DIR_NAME))
}

fn get_capsule_path() -> PathBuf {
    get_gp_dir().join(CAPSULE_FILE)
}

fn get_jsonl_path() -> PathBuf {
    get_gp_dir().join(JSONL_FILE)
}

pub fn load_capsules(
    gap_type: Option<&str>,
    status: Option<&str>,
    source_paper_id: Option<&str>,
) -> Vec<HashMap<String, serde_json::Value>> {
    let gp_dir = get_gp_dir();
    if !gp_dir.exists() {
        return vec![];
    }

    let jsonl_path = gp_dir.join(JSONL_FILE);
    let text = match std::fs::read_to_string(&jsonl_path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return vec![],
    };

    let mut capsules: Vec<HashMap<String, serde_json::Value>> = text
        .lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect();

    if let Some(gt) = gap_type {
        capsules.retain(|c| {
            c.get("action_gap_type")
                .and_then(|v| v.as_str())
                .map(|v| v == gt)
                .unwrap_or(false)
        });
    }

    if let Some(s) = status {
        capsules.retain(|c| {
            c.get("status")
                .and_then(|v| v.as_str())
                .map(|v| v == s)
                .unwrap_or(false)
        });
    }

    if let Some(spid) = source_paper_id {
        capsules.retain(|c| {
            c.get("archetype")
                .and_then(|v| v.get("source_paper_id"))
                .and_then(|v| v.as_str())
                .map(|v| v == spid)
                .unwrap_or(false)
        });
    }

    let _ = sync_capsules_json(&capsules);
    capsules
}

fn sync_capsules_json(
    capsules: &[HashMap<String, serde_json::Value>],
) -> Result<(), std::io::Error> {
    let gp_dir = get_gp_dir();
    std::fs::create_dir_all(&gp_dir)?;
    let path = gp_dir.join(CAPSULE_FILE);
    let data = serde_json::json!({"version": "1.0", "capsules": capsules});
    let text = serde_json::to_string_pretty(&data)?;
    std::fs::write(path, text)?;
    Ok(())
}

pub fn get_capsule_by_paper(
    paper_id: &str,
    gap_type: Option<&str>,
) -> Option<HashMap<String, serde_json::Value>> {
    let capsules = load_capsules(gap_type, Some("active"), None);
    let candidates: Vec<_> = capsules
        .into_iter()
        .filter(|c| {
            c.get("archetype")
                .and_then(|v| v.get("source_paper_id"))
                .and_then(|v| v.as_str())
                == Some(paper_id)
        })
        .collect();

    candidates.into_iter().max_by(|a, b| {
        let a_time = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        let b_time = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        a_time.cmp(b_time)
    })
}

pub fn paper_exists_in_pool(paper_id: &str, gap_type: Option<&str>) -> bool {
    get_capsule_by_paper(paper_id, gap_type).is_some()
}

pub fn fingerprint_exists_in_pool(fingerprint: &str, gap_type: Option<&str>) -> bool {
    let capsules = load_capsules(gap_type, Some("active"), None);
    capsules.iter().any(|c| {
        c.get("archetype")
            .and_then(|v| v.get("algorithm_fingerprint"))
            .and_then(|v| v.as_str())
            == Some(fingerprint)
    })
}

pub fn get_gene_pool_diversity() -> HashMap<String, serde_json::Value> {
    let capsules = load_capsules(None, Some("active"), None);
    if capsules.is_empty() {
        return serde_json::json!({
            "shannon_index": 0.0,
            "capsule_count": 0,
            "family_counts": {},
            "gap_type_counts": {},
            "diversity_score": 0,
            "underrepresented_families": [],
            "overrepresented_families": [],
        })
        .as_object()
        .unwrap()
        .clone()
        .into_iter()
        .collect();
    }

    let family_keywords: HashMap<&str, Vec<&str>> = HashMap::from([
        (
            "attention",
            vec![
                "attention",
                "transformer",
                "multi-head",
                "self-attention",
                "cross-attention",
            ],
        ),
        (
            "reinforcement",
            vec![
                "rl",
                "reinforcement",
                "policy",
                "reward",
                "agent",
                "DQN",
                "PPO",
                "A3C",
            ],
        ),
        (
            "language_model",
            vec![
                "LM",
                "language model",
                "decoder",
                "autoregressive",
                "LLM",
                "GPT",
                "BERT",
            ],
        ),
        (
            "vision",
            vec![
                "CNN",
                "convolution",
                "resnet",
                "image",
                "vision",
                "ViT",
                "classification",
            ],
        ),
        (
            "optimization",
            vec!["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
        ),
        (
            "graph",
            vec!["GNN", "graph", "node", "edge", "message passing"],
        ),
        (
            "reasoning",
            vec![
                "reasoning",
                "chain-of-thought",
                "logical",
                "inference",
                "planning",
            ],
        ),
        (
            "embodied",
            vec!["embodied", "robotics", "navigation", "control", "motor"],
        ),
    ]);

    fn family_of(
        keywords: &[serde_json::Value],
        family_keywords: &HashMap<&str, Vec<&str>>,
    ) -> String {
        let kw_set: std::collections::HashSet<String> = keywords
            .iter()
            .filter_map(|k| k.as_str())
            .map(|k| k.to_lowercase())
            .collect();

        for (fam, fam_kws) in family_keywords {
            if fam_kws.iter().any(|fk| kw_set.contains(&fk.to_lowercase())) {
                return fam.to_string();
            }
        }
        "other".to_string()
    }

    let mut family_counts: HashMap<String, i32> = HashMap::new();
    let mut gap_type_counts: HashMap<String, i32> = HashMap::new();

    for cap in &capsules {
        let keywords: Vec<serde_json::Value> = cap
            .get("trigger_keywords")
            .and_then(|v| serde_json::from_value(v.clone()).ok())
            .unwrap_or_default();
        let fam = family_of(&keywords, &family_keywords);
        *family_counts.entry(fam).or_insert(0) += 1;

        let gt = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        *gap_type_counts.entry(gt.to_string()).or_insert(0) += 1;
    }

    let total = capsules.len() as f64;
    let mut shannon = 0.0;
    for count in family_counts.values() {
        let p = *count as f64 / total;
        if p > 0.0 {
            shannon -= p * p.ln();
        }
    }
    let num_families = family_counts.len().max(1) as f64;
    let max_entropy = num_families.ln();
    let normalized_shannon = if max_entropy > 0.0 {
        shannon / max_entropy
    } else {
        0.0
    };

    let family_coverage = family_counts.len() as f64 / family_keywords.len() as f64;
    let diversity_score = (normalized_shannon * 0.6 * 100.0 + family_coverage * 0.4 * 100.0) as i32;

    let mut sorted_counts: Vec<_> = family_counts.values().collect();
    sorted_counts.sort();
    let median_count = if family_counts.is_empty() {
        1
    } else {
        *sorted_counts[sorted_counts.len() / 2]
    };

    let underrep: Vec<String> = family_counts
        .iter()
        .filter(|(_, &c)| c < median_count / 10)
        .map(|(f, _)| f.clone())
        .collect();
    let overrep: Vec<String> = family_counts
        .iter()
        .filter(|(_, &c)| c > median_count * 2)
        .map(|(f, _)| f.clone())
        .collect();

    let mut result = HashMap::new();
    result.insert(
        "shannon_index".to_string(),
        serde_json::json!(round(shannon, 4)),
    );
    result.insert(
        "shannon_normalized".to_string(),
        serde_json::json!(round(normalized_shannon, 4)),
    );
    result.insert("capsule_count".to_string(), serde_json::json!(total as i32));
    result.insert(
        "family_counts".to_string(),
        serde_json::json!(family_counts),
    );
    result.insert(
        "gap_type_counts".to_string(),
        serde_json::json!(gap_type_counts),
    );
    result.insert(
        "diversity_score".to_string(),
        serde_json::json!(diversity_score),
    );
    result.insert(
        "underrepresented_families".to_string(),
        serde_json::json!(underrep),
    );
    result.insert(
        "overrepresented_families".to_string(),
        serde_json::json!(overrep),
    );
    result.insert(
        "median_family_count".to_string(),
        serde_json::json!(median_count),
    );
    result.insert(
        "family_coverage".to_string(),
        serde_json::json!(round(family_coverage, 4)),
    );

    result
}

pub fn export_pool() -> HashMap<String, serde_json::Value> {
    let capsules_path = get_capsule_path();
    let jsonl_path = get_jsonl_path();

    let mut result = HashMap::new();
    result.insert("version".to_string(), serde_json::json!("1.0"));
    result.insert(
        "exported_at".to_string(),
        serde_json::json!(chrono::Utc::now().to_rfc3339()),
    );

    if capsules_path.exists() {
        if let Ok(text) = std::fs::read_to_string(&capsules_path) {
            if let Ok(data) = serde_json::from_str(&text) {
                result.insert("capsules".to_string(), data);
            }
        }
    }

    if jsonl_path.exists() {
        if let Ok(text) = std::fs::read_to_string(&jsonl_path) {
            let genes: Vec<serde_json::Value> = text
                .lines()
                .filter(|l| !l.trim().is_empty())
                .filter_map(|l| serde_json::from_str(l).ok())
                .collect();
            result.insert("genes".to_string(), serde_json::json!(genes));
        }
    }

    result
}

pub fn import_pool(data: &HashMap<String, serde_json::Value>, merge: bool) -> HashMap<String, i32> {
    let capsules_path = get_capsule_path();
    let jsonl_path = get_jsonl_path();
    let gp_dir = get_gp_dir();

    std::fs::create_dir_all(&gp_dir).ok();

    let mut stats: HashMap<String, i32> = HashMap::new();
    stats.insert("capsules_imported".to_string(), 0);
    stats.insert("genes_imported".to_string(), 0);

    if let Some(capsules) = data.get("capsules") {
        let capsules_arr: Vec<serde_json::Value> =
            serde_json::from_value(capsules.clone()).unwrap_or_default();

        let mut existing_ids: std::collections::HashSet<String> = std::collections::HashSet::new();
        if capsules_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&capsules_path) {
                if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&text) {
                    if let Some(caps) = existing.get("capsules") {
                        if let Some(arr) = caps.as_array() {
                            for cap in arr {
                                if let Some(id) = cap.get("capsule_id").and_then(|v| v.as_str()) {
                                    existing_ids.insert(id.to_string());
                                }
                            }
                        }
                    }
                }
            }
        }

        let new_capsules: Vec<_> = if merge {
            capsules_arr
                .iter()
                .filter(|c| {
                    c.get("capsule_id")
                        .and_then(|v| v.as_str())
                        .map(|id| !existing_ids.contains(id))
                        .unwrap_or(false)
                })
                .cloned()
                .collect()
        } else {
            capsules_arr
        };

        if !new_capsules.is_empty() || !merge {
            let existing_capsules: Vec<_> = if capsules_path.exists() && merge {
                if let Ok(text) = std::fs::read_to_string(&capsules_path) {
                    if let Ok(existing) = serde_json::from_str::<serde_json::Value>(&text) {
                        existing
                            .get("capsules")
                            .and_then(|v| serde_json::from_value(v.clone()).ok())
                            .unwrap_or_default()
                    } else {
                        vec![]
                    }
                } else {
                    vec![]
                }
            } else {
                vec![]
            };

            let merged = serde_json::json!({
                "version": "1.0",
                "capsules": if merge { existing_capsules.into_iter().chain(new_capsules.clone()).collect::<Vec<_>>() } else { new_capsules.clone() }
            });

            if let Ok(text) = serde_json::to_string_pretty(&merged) {
                std::fs::write(&capsules_path, text).ok();
            }
            stats.insert("capsules_imported".to_string(), new_capsules.len() as i32);
        }
    }

    if let Some(genes) = data.get("genes") {
        let genes_arr: Vec<serde_json::Value> =
            serde_json::from_value(genes.clone()).unwrap_or_default();

        let mut existing_gene_ids: std::collections::HashSet<String> =
            std::collections::HashSet::new();
        if merge && jsonl_path.exists() {
            if let Ok(text) = std::fs::read_to_string(&jsonl_path) {
                for line in text.lines() {
                    if let Ok(gene) = serde_json::from_str::<serde_json::Value>(line) {
                        if let Some(id) = gene.get("gene_id").and_then(|v| v.as_str()) {
                            existing_gene_ids.insert(id.to_string());
                        }
                    }
                }
            }
        }

        let new_genes: Vec<_> = genes_arr
            .iter()
            .filter(|g| {
                g.get("gene_id")
                    .and_then(|v| v.as_str())
                    .map(|id| !existing_gene_ids.contains(id))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        if !new_genes.is_empty() {
            let mut open_result = if merge {
                std::fs::OpenOptions::new()
                    .append(true)
                    .create(true)
                    .open(&jsonl_path)
            } else {
                std::fs::OpenOptions::new()
                    .write(true)
                    .create(true)
                    .truncate(true)
                    .open(&jsonl_path)
            };

            if let Ok(ref mut f) = open_result {
                for g in &new_genes {
                    if let Ok(line) = serde_json::to_string(g) {
                        writeln!(f, "{}", line).ok();
                    }
                }
            }
            stats.insert("genes_imported".to_string(), new_genes.len() as i32);
        }
    }

    stats
}

fn round(v: f64, decimals: usize) -> f64 {
    let mul = 10_f64.powi(decimals as i32);
    (v * mul).round() / mul
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_round() {
        assert_eq!(round(1.23456, 2), 1.23);
        assert_eq!(round(1.235, 2), 1.24);
    }

    #[test]
    fn test_load_capsules_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("RAIROS_HOME", tmp.path());
        let capsules = load_capsules(None, None, None);
        assert!(capsules.is_empty());
        std::env::remove_var("RAIROS_HOME");
    }

    #[test]
    fn test_fingerprint_exists() {
        let result = fingerprint_exists_in_pool("test-fingerprint", None);
        assert!(!result);
    }

    #[test]
    fn test_get_gene_pool_diversity_empty() {
        let tmp = tempfile::tempdir().unwrap();
        std::env::set_var("RAIROS_HOME", tmp.path());
        let result = get_gene_pool_diversity();
        assert_eq!(
            result
                .get("capsule_count")
                .and_then(|v| v.as_i64())
                .unwrap_or(0),
            0
        );
        std::env::remove_var("RAIROS_HOME");
    }

    #[test]
    fn test_export_pool_empty() {
        let result = export_pool();
        assert_eq!(result.get("version").and_then(|v| v.as_str()), Some("1.0"));
    }

    #[test]
    fn test_import_pool_empty() {
        let data: HashMap<String, serde_json::Value> = HashMap::new();
        let stats = import_pool(&data, true);
        assert_eq!(stats.get("capsules_imported").copied(), Some(0));
        assert_eq!(stats.get("genes_imported").copied(), Some(0));
    }
}
