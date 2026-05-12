//! rairos-pool — Gene Pool I/O
//!
//! Unified read/write for capsules.json + gene_pool.jsonl, import/export, and backup.
//!
//! Ported from `llm/gene_pool_io.py`.

use flate2::read::GzDecoder;
use flate2::write::GzEncoder;
use flate2::Compression;
use serde::{Deserialize, Serialize};
use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

const MAX_BACKUPS: usize = 30;

fn gp_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("evolution")
}

fn capsule_path() -> PathBuf {
    gp_dir().join("capsules.json")
}

fn jsonl_path() -> PathBuf {
    gp_dir().join("gene_pool.jsonl")
}

fn backup_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ai_research_os")
        .join("backups")
}

fn family_of(keywords: &[String]) -> String {
    let kw_set: std::collections::HashSet<&str> = keywords.iter().map(|k| k.as_str()).collect();

    let families: &[(&str, &[&str])] = &[
        (
            "attention",
            &[
                "attention",
                "transformer",
                "multi-head",
                "self-attention",
                "cross-attention",
            ],
        ),
        (
            "reinforcement",
            &[
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
            &[
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
            &[
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
            &["optimizer", "Adam", "SGD", "gradient", "loss", "training"],
        ),
        (
            "graph",
            &["GNN", "graph", "node", "edge", "message passing"],
        ),
        (
            "reasoning",
            &[
                "reasoning",
                "chain-of-thought",
                "logical",
                "inference",
                "planning",
            ],
        ),
        (
            "embodied",
            &["embodied", "robotics", "navigation", "control", "motor"],
        ),
    ];

    for (fam, fam_kws) in families {
        for fk in *fam_kws {
            if kw_set.contains(fk) {
                return fam.to_string();
            }
        }
    }
    "other".to_string()
}

fn load_capsules_internal() -> Vec<serde_json::Value> {
    let path = jsonl_path();
    if !path.exists() {
        return Vec::new();
    }
    let text = match fs::read_to_string(&path) {
        Ok(t) => t.trim().to_string(),
        Err(_) => return Vec::new(),
    };
    if text.is_empty() {
        return Vec::new();
    }
    text.lines()
        .filter(|l| !l.trim().is_empty())
        .filter_map(|l| serde_json::from_str(l).ok())
        .collect()
}

fn sync_capsules_json(capsules: &[serde_json::Value]) {
    let dir = gp_dir();
    if fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = capsule_path();
    let data = serde_json::json!({ "version": "1.0", "capsules": capsules });
    if let Ok(json) = serde_json::to_string_pretty(&data) {
        let _ = fs::write(&path, json);
    }
}

pub fn load_capsules(
    gap_type: Option<&str>,
    status: Option<&str>,
    source_paper_id: Option<&str>,
) -> Vec<serde_json::Value> {
    if !gp_dir().exists() {
        return Vec::new();
    }

    let capsules = load_capsules_internal();

    sync_capsules_json(&capsules);

    let mut result: Vec<serde_json::Value> = capsules;

    if let Some(gt) = gap_type {
        result.retain(|c| {
            c.get("action_gap_type")
                .and_then(|v| v.as_str())
                .map(|s| s == gt)
                .unwrap_or(false)
        });
    }

    if let Some(st) = status {
        result.retain(|c| {
            c.get("status")
                .and_then(|v| v.as_str())
                .map(|s| s == st)
                .unwrap_or(false)
        });
    }

    if let Some(spid) = source_paper_id {
        result.retain(|c| {
            c.get("archetype")
                .and_then(|v| v.get("source_paper_id"))
                .and_then(|v| v.as_str())
                .map(|s| s == spid)
                .unwrap_or(false)
        });
    }

    result
}

pub fn get_capsule_by_paper(paper_id: &str, gap_type: Option<&str>) -> Option<serde_json::Value> {
    let capsules = load_capsules(gap_type, Some("active"), None);
    let candidates: Vec<_> = capsules
        .into_iter()
        .filter(|c| {
            c.get("archetype")
                .and_then(|v| v.get("source_paper_id"))
                .and_then(|v| v.as_str())
                .map(|s| s == paper_id)
                .unwrap_or(false)
        })
        .collect();

    if candidates.is_empty() {
        return None;
    }

    candidates.into_iter().max_by(|a, b| {
        let a_ts = a.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        let b_ts = b.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
        a_ts.cmp(b_ts)
    })
}

pub fn paper_exists_in_pool(paper_id: &str, gap_type: Option<&str>) -> bool {
    get_capsule_by_paper(paper_id, gap_type).is_some()
}

pub fn fingerprint_exists_in_pool(fingerprint: &str, gap_type: Option<&str>) -> bool {
    let capsules = load_capsules(gap_type, Some("active"), None);
    capsules.into_iter().any(|c| {
        c.get("archetype")
            .and_then(|v| v.get("algorithm_fingerprint"))
            .and_then(|v| v.as_str())
            .map(|s| s == fingerprint)
            .unwrap_or(false)
    })
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenePoolDiversity {
    pub shannon_index: f64,
    pub shannon_normalized: f64,
    pub capsule_count: usize,
    pub family_counts: Vec<(String, usize)>,
    pub gap_type_counts: Vec<(String, usize)>,
    pub diversity_score: usize,
    pub underrepresented_families: Vec<String>,
    pub overrepresented_families: Vec<String>,
    pub median_family_count: usize,
    pub family_coverage: f64,
}

pub fn get_gene_pool_diversity() -> GenePoolDiversity {
    let capsules = load_capsules(None, Some("active"), None);

    if capsules.is_empty() {
        return GenePoolDiversity {
            shannon_index: 0.0,
            shannon_normalized: 0.0,
            capsule_count: 0,
            family_counts: Vec::new(),
            gap_type_counts: Vec::new(),
            diversity_score: 0,
            underrepresented_families: Vec::new(),
            overrepresented_families: Vec::new(),
            median_family_count: 1,
            family_coverage: 0.0,
        };
    }

    let family_keyword_count: usize = 8;

    let mut family_counts_map: std::collections::HashMap<String, usize> = Default::default();
    let mut gap_type_counts_map: std::collections::HashMap<String, usize> = Default::default();

    for cap in &capsules {
        let keywords: Vec<String> = cap
            .get("trigger_keywords")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_default();
        let fam = family_of(&keywords);
        *family_counts_map.entry(fam).or_insert(0) += 1;

        let gt = cap
            .get("action_gap_type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        *gap_type_counts_map.entry(gt.to_string()).or_insert(0) += 1;
    }

    let total = capsules.len();
    let shannon: f64 = family_counts_map
        .values()
        .map(|&count| {
            let p = count as f64 / total as f64;
            if p > 0.0 {
                -p * p.log2()
            } else {
                0.0
            }
        })
        .sum();

    let max_entropy = (family_counts_map.len() as f64).log2();
    let normalized_shannon = if max_entropy > 0.0 {
        shannon / max_entropy
    } else {
        0.0
    };

    let family_coverage = family_counts_map.len() as f64 / family_keyword_count as f64;
    let diversity_score =
        (normalized_shannon * 0.6 * 100.0) as usize + (family_coverage * 0.4 * 100.0) as usize;

    let mut sorted_counts: Vec<usize> = family_counts_map.values().cloned().collect();
    sorted_counts.sort();
    let median_count = if sorted_counts.is_empty() {
        1
    } else {
        sorted_counts[sorted_counts.len() / 2]
    };

    let underrep: Vec<String> = family_counts_map
        .iter()
        .filter(|&(_, &c)| c < median_count / 10)
        .map(|(k, _)| k.clone())
        .collect();
    let overrep: Vec<String> = family_counts_map
        .iter()
        .filter(|&(_, &c)| c > median_count * 2)
        .map(|(k, _)| k.clone())
        .collect();

    let mut family_counts: Vec<(String, usize)> = family_counts_map.into_iter().collect();
    family_counts.sort_by_key(|b| std::cmp::Reverse(b.1));

    let mut gap_type_counts: Vec<(String, usize)> = gap_type_counts_map.into_iter().collect();
    gap_type_counts.sort_by_key(|b| std::cmp::Reverse(b.1));

    GenePoolDiversity {
        shannon_index: (shannon * 10000.0).round() / 10000.0,
        shannon_normalized: (normalized_shannon * 10000.0).round() / 10000.0,
        capsule_count: total,
        family_counts,
        gap_type_counts,
        diversity_score,
        underrepresented_families: underrep,
        overrepresented_families: overrep,
        median_family_count: median_count,
        family_coverage: (family_coverage * 10000.0).round() / 10000.0,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImportStats {
    pub capsules_imported: usize,
    pub genes_imported: usize,
}

pub fn export_pool() -> serde_json::Value {
    let capsules_path = capsule_path();
    let jsonl = jsonl_path();

    let mut result = serde_json::json!({
        "version": "1.0",
        "exported_at": chrono::Utc::now().to_rfc3339(),
    });

    if capsules_path.exists() {
        if let Ok(text) = fs::read_to_string(&capsules_path) {
            if let Ok(data) = serde_json::from_str::<serde_json::Value>(&text) {
                result["capsules"] = data;
            }
        }
    }

    if jsonl.exists() {
        let text = fs::read_to_string(&jsonl).unwrap_or_default();
        let genes: Vec<serde_json::Value> = text
            .lines()
            .filter(|l| !l.trim().is_empty())
            .filter_map(|l| serde_json::from_str(l).ok())
            .collect();
        result["genes"] = serde_json::json!(genes);
    }

    result
}

pub fn import_pool(data: serde_json::Value, merge: bool) -> ImportStats {
    let capsules_path = capsule_path();
    let jsonl = jsonl_path();
    let _ = fs::create_dir_all(gp_dir());

    let mut capsules_imported = 0;
    let mut genes_imported = 0;

    if let Some(capsules) = data.get("capsules") {
        let new_capsules: Vec<serde_json::Value> = if let Some(arr) = capsules.as_array() {
            arr.clone()
        } else if let Some(obj) = capsules.as_object() {
            obj.get("capsules")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let existing_caps: Vec<serde_json::Value> = if capsules_path.exists() {
            let text = fs::read_to_string(&capsules_path).unwrap_or_default();
            serde_json::from_str::<serde_json::Value>(&text)
                .ok()
                .and_then(|v| v.get("capsules").cloned())
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default()
        } else {
            Vec::new()
        };

        let existing_ids: std::collections::HashSet<_> = existing_caps
            .iter()
            .filter_map(|c| c.get("capsule_id").and_then(|v| v.as_str()))
            .collect();

        let (final_capsules, imported): (Vec<_>, usize) = if merge {
            let to_add: Vec<_> = new_capsules
                .into_iter()
                .filter(|c| {
                    c.get("capsule_id")
                        .and_then(|v| v.as_str())
                        .map(|id| !existing_ids.contains(id))
                        .unwrap_or(false)
                })
                .collect();
            (
                existing_caps.iter().chain(to_add.iter()).cloned().collect(),
                to_add.len(),
            )
        } else {
            let count = new_capsules.len();
            (new_capsules, count)
        };

        capsules_imported = imported;
        let merged = serde_json::json!({
            "version": "1.0",
            "capsules": final_capsules,
        });
        if let Ok(json) = serde_json::to_string_pretty(&merged) {
            let _ = fs::write(&capsules_path, json);
        }
    }

    if let Some(genes) = data.get("genes") {
        if let Some(arr) = genes.as_array() {
            let existing_gene_ids: std::collections::HashSet<String> = if merge && jsonl.exists() {
                let text = fs::read_to_string(&jsonl).unwrap_or_default();
                text.lines()
                    .filter(|l| !l.trim().is_empty())
                    .filter_map(|l| {
                        serde_json::from_str::<serde_json::Value>(l)
                            .ok()
                            .and_then(|v| {
                                v.get("gene_id").and_then(|v| v.as_str()).map(String::from)
                            })
                    })
                    .collect()
            } else {
                std::collections::HashSet::new()
            };

            let new_genes: Vec<_> = arr
                .iter()
                .filter(|g| {
                    g.get("gene_id")
                        .and_then(|v| v.as_str())
                        .map(|id| !existing_gene_ids.contains(id))
                        .unwrap_or(false)
                })
                .cloned()
                .collect();

            genes_imported = new_genes.len();

            if !new_genes.is_empty() {
                let mut f = OpenOptions::new()
                    .create(true)
                    .append(merge)
                    .open(&jsonl)
                    .ok();
                if let Some(ref mut file) = f {
                    for g in &new_genes {
                        if let Ok(json) = serde_json::to_string(g) {
                            let _ = writeln!(file, "{}", json);
                        }
                    }
                }
            }
        }
    }

    ImportStats {
        capsules_imported,
        genes_imported,
    }
}

fn list_backups() -> Vec<String> {
    let dir = backup_dir();
    if !dir.exists() {
        return Vec::new();
    }
    let mut names: Vec<String> = std::fs::read_dir(&dir)
        .ok()
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let path = e.path();
            if path.extension().map(|s| s == "gz").unwrap_or(false) {
                path.file_stem()
                    .and_then(|s| s.to_str())
                    .map(|s| s.replace("gene_pool_", ""))
            } else {
                None
            }
        })
        .collect();
    names.sort_by(|a, b| b.cmp(a));
    names
}

pub fn create_backup() -> Result<String, String> {
    let dir = backup_dir();
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;

    let stamp = chrono::Utc::now().format("%Y%m%d").to_string();
    let backup_name = format!("gene_pool_{}.tar.gz", stamp);
    let backup_path = dir.join(&backup_name);

    let tmp_dir = tempfile::TempDir::new().map_err(|e| e.to_string())?;
    let tmp_tar = tmp_dir.path().join("backup.tar.gz");

    {
        let file = File::create(&tmp_tar).map_err(|e| e.to_string())?;
        let enc = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(enc);

        for fname in &["gene_pool.jsonl", "capsules.json"] {
            let src = gp_dir().join(fname);
            if src.exists() {
                tar.append_path_with_name(&src, *fname)
                    .map_err(|e| e.to_string())?;
            }
        }
    }

    fs::copy(&tmp_tar, &backup_path).map_err(|e| e.to_string())?;

    prune_backups();

    Ok(stamp)
}

fn prune_backups() {
    let backups = list_backups();
    let dir = backup_dir();
    for old in backups.iter().skip(MAX_BACKUPS) {
        let path = dir.join(format!("gene_pool_{}.tar.gz", old));
        let _ = fs::remove_file(&path);
    }
}

pub fn restore_backup(stamp: &str) -> bool {
    let backup_file = backup_dir().join(format!("gene_pool_{}.tar.gz", stamp));
    if !backup_file.exists() {
        return false;
    }

    let tmp_dir = match tempfile::TempDir::new() {
        Ok(d) => d,
        Err(_) => return false,
    };

    {
        let file = match File::open(&backup_file) {
            Ok(f) => f,
            Err(_) => return false,
        };
        let dec = GzDecoder::new(file);
        let mut archive = tar::Archive::new(dec);
        if archive.unpack(tmp_dir.path()).is_err() {
            return false;
        }
    }

    for fname in &["gene_pool.jsonl", "capsules.json"] {
        let src = tmp_dir.path().join(fname);
        if src.exists() {
            let dst = gp_dir().join(fname);
            let _ = fs::create_dir_all(gp_dir());
            let _ = fs::copy(&src, &dst);
        }
    }

    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupInfo {
    pub available: usize,
    pub stamps: Vec<String>,
    pub total_size_mb: f64,
    pub max_backups: usize,
}

pub fn get_backup_info() -> BackupInfo {
    let backups = list_backups();
    let dir = backup_dir();
    let total_size: u64 = backups
        .iter()
        .filter_map(|b| {
            let path = dir.join(format!("gene_pool_{}.tar.gz", b));
            path.metadata().ok().map(|m| m.len())
        })
        .sum();

    BackupInfo {
        available: backups.len(),
        stamps: backups.into_iter().take(10).collect(),
        total_size_mb: (total_size as f64) / 1024.0 / 1024.0,
        max_backups: MAX_BACKUPS,
    }
}

pub fn render_backup_html(info: Option<&BackupInfo>) -> String {
    let default_info = get_backup_info();
    let info = info.unwrap_or(&default_info);

    let mut lines = Vec::new();
    lines.push("<div class=\"backup-panel\">".to_string());
    lines.push("<h3>Gene Pool Backup</h3>".to_string());
    lines.push(format!(
        "<p style='font-size:13px;color:#A89E8C;margin-bottom:16px'>\
         <b>{}</b> backups &middot; \
         {:.2}MB total &middot; \
         max {} versions retained</p>",
        info.available, info.total_size_mb, info.max_backups
    ));

    lines.push("<div style='margin-bottom:16px'>".to_string());
    lines.push(
        "<button onclick='triggerBackup()' style='background:#6B8FB5;color:white;border:none;\
                border-radius:4px;padding:8px 16px;cursor:pointer;font-size:13px'>\
                Take Backup Now</button>"
            .to_string(),
    );
    lines.push("</div>".to_string());

    if !info.stamps.is_empty() {
        lines
            .push("<table style='width:100%;border-collapse:collapse;font-size:13px'>".to_string());
        lines.push("<tr style='border-bottom:1px solid #e0dbd4'><th style='text-align:left;padding:6px 8px'>Date</th>\
                   <th style='text-align:right;padding:6px 8px'>Action</th></tr>".to_string());

        for stamp in &info.stamps {
            let (yr, mo, day) = (&stamp[..4], &stamp[4..6], &stamp[6..8]);
            lines.push(format!(
                "<tr style='border-bottom:1px solid #f0ebe5'>\
                 <td style='padding:6px 8px'>{}-{}-{}</td>\
                 <td style='text-align:right;padding:6px 8px'>\
                 <button onclick=\"restoreBackup(\\\"{}\\\")\" \
                 style='font-size:11px;padding:2px 8px;cursor:pointer;\
                 background:transparent;border:1px solid #ccc;border-radius:3px'>\
                 Restore</button></td></tr>",
                yr, mo, day, stamp
            ));
        }
        lines.push("</table>".to_string());
    } else {
        lines.push(
            "<p style='color:#A89E8C;font-size:13px'>\
                    No backups yet. Click 'Take Backup Now' to create your first snapshot.</p>"
                .to_string(),
        );
    }

    lines.push("<script>\
               function triggerBackup(){fetch('/gene-pool/backup/create',{method:'POST'})\
               .then(r=>r.json()).then(d=>{alert('Backup created: '+d.stamp);location.reload();});}\
               function restoreBackup(stamp){\
               if(!confirm('Restore backup from '+stamp+'? Current Gene Pool will be overwritten.'))return;\
               fetch('/gene-pool/backup/restore/'+stamp,{method:'POST'})\
               .then(r=>r.json()).then(d=>{alert(d.message);location.reload();});}\
               </script>".to_string());
    lines.push("<style>.backup-panel{font-family:Georgia,serif}</style>".to_string());
    lines.push("</div>".to_string());

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_family_of_attention() {
        let kws = vec!["transformer".to_string(), "attention".to_string()];
        assert_eq!(family_of(&kws), "attention");
    }

    #[test]
    fn test_family_of_rl() {
        let kws = vec!["rl".to_string(), "policy".to_string()];
        assert_eq!(family_of(&kws), "reinforcement");
    }

    #[test]
    fn test_family_of_other() {
        let kws = vec!["unknownkeyword".to_string()];
        assert_eq!(family_of(&kws), "other");
    }

    #[test]
    fn test_load_capsules_missing_dir() {
        let result = load_capsules(None, None, None);
        assert!(result.is_empty());
    }

    #[test]
    fn test_paper_exists_in_pool_missing() {
        assert!(!paper_exists_in_pool("nonexistent", None));
    }

    #[test]
    fn test_fingerprint_exists_in_pool_missing() {
        assert!(!fingerprint_exists_in_pool("nonexistent", None));
    }

    #[test]
    fn test_get_gene_pool_diversity_empty() {
        let div = get_gene_pool_diversity();
        assert_eq!(div.capsule_count, 0);
        assert_eq!(div.diversity_score, 0);
    }

    #[test]
    fn test_list_backups_empty() {
        let backups = list_backups();
        assert!(backups.is_empty());
    }

    #[test]
    fn test_export_pool_returns_valid_json() {
        let result = export_pool();
        assert_eq!(result.get("version").and_then(|v| v.as_str()), Some("1.0"));
    }

    #[test]
    fn test_import_pool_stats() {
        let data = serde_json::json!({
            "capsules": [],
            "genes": []
        });
        let stats = import_pool(data, true);
        assert_eq!(stats.capsules_imported, 0);
        assert_eq!(stats.genes_imported, 0);
    }

    #[test]
    fn test_backup_info_empty() {
        let info = get_backup_info();
        assert_eq!(info.available, 0);
        assert_eq!(info.max_backups, MAX_BACKUPS);
    }

    #[test]
    fn test_render_backup_html() {
        let info = get_backup_info();
        let html = render_backup_html(Some(&info));
        assert!(html.contains("backup-panel"));
        assert!(html.contains("Take Backup Now"));
    }
}
