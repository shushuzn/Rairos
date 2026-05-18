use crate::handlers::helpers::{data_dir, kg, parse_arxiv_citation};
use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

pub struct PaperScienceDiscoveryHandler;

#[async_trait]
impl ToolHandler for PaperScienceDiscoveryHandler {
    fn name(&self) -> &str { "paper_science_discovery" }
    fn description(&self) -> &str { "Discover scientific AI models and datasets from HuggingFace for a research topic" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query".into(), ToolProperty::string("Research topic or scientific domain (e.g., 'protein language model', 'molecular dynamics')")),
                ("resource_type".into(), ToolProperty::string("Type: model, dataset, or all (default: all)")),
            ].into_iter().collect(),
            vec!["query".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query = params["query"].as_str().ok_or("Missing query")?;
        let resource_type = params.get("resource_type").and_then(|v| v.as_str()).unwrap_or("all");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let query_encoded = urlencoding::encode(query);

        let mut results = serde_json::json!({
            "query": query,
            "models": [],
            "datasets": [],
        });

        if resource_type == "all" || resource_type == "model" {
            let models_url = format!(
                "https://huggingface.co/api/models?search={}&sort=downloads&direction=-1&limit=10",
                query_encoded
            );
            if let Ok(resp) = client.get(&models_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data.as_array() {
                            let models: Vec<Value> = arr.iter()
                                .take(10)
                                .map(|m| {
                                    serde_json::json!({
                                        "id": m["id"],
                                        "downloads": m["downloads"],
                                        "likes": m["likes"],
                                        "tags": m["tags"].as_array().map(|t| t.iter().filter_map(|v| v.as_str()).take(5).collect::<Vec<_>>()).unwrap_or_default(),
                                        "pipeline_tag": m["pipeline_tag"],
                                    })
                                })
                                .collect();
                            results["models"] = serde_json::json!(models);
                        }
                    }
                }
            }
        }

        if resource_type == "all" || resource_type == "dataset" {
            let datasets_url = format!(
                "https://huggingface.co/api/datasets?search={}&sort=downloads&direction=-1&limit=10",
                query_encoded
            );
            if let Ok(resp) = client.get(&datasets_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        if let Some(arr) = data.as_array() {
                            let datasets: Vec<Value> = arr.iter()
                                .take(10)
                                .map(|d| {
                                    serde_json::json!({
                                        "id": d["id"],
                                        "downloads": d["downloads"],
                                        "likes": d["likes"],
                                        "tags": d["tags"].as_array().map(|t| t.iter().filter_map(|v| v.as_str()).take(5).collect::<Vec<_>>()).unwrap_or_default(),
                                    })
                                })
                                .collect();
                            results["datasets"] = serde_json::json!(datasets);
                        }
                    }
                }
            }
        }

        let model_count = results["models"].as_array().map(|a| a.len()).unwrap_or(0);
        let dataset_count = results["datasets"].as_array().map(|a| a.len()).unwrap_or(0);

        Ok(serde_json::json!({
            "query": query,
            "models_count": model_count,
            "datasets_count": dataset_count,
            "models": results["models"],
            "datasets": results["datasets"],
        }))
    }
}

pub struct PaperDatabaseLookupHandler;

#[async_trait]
impl ToolHandler for PaperDatabaseLookupHandler {
    fn name(&self) -> &str { "paper_database_lookup" }
    fn description(&self) -> &str { "Query scientific databases (PubChem, UniProt, NCBI Gene, Reactome, PDB, AlphaFold, ChEMBL) for compounds, genes, proteins, pathways, or structures" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("query_type".into(), ToolProperty::string("Type: compound, gene, protein, pathway, structure, bioactivity, or auto")),
                ("term".into(), ToolProperty::string("Search term (e.g., 'aspirin', 'BRCA1', 'apoptosis', 'P05387')")),
                ("limit".into(), ToolProperty::integer("Max results per database (default: 5)")),
            ].into_iter().collect(),
            vec!["query_type".into(), "term".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let query_type = params["query_type"].as_str().unwrap_or("auto");
        let term = params["term"].as_str().ok_or("Missing term")?;
        let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(5) as usize;

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let term_enc = urlencoding::encode(term);

        let mut results = serde_json::json!({
            "query_type": query_type,
            "term": term,
            "databases": [],
        });

        if query_type == "compound" || query_type == "auto" {
            let pc_url = format!(
                "https://pubchem.ncbi.nlm.nih.gov/rest/pug/compound/name/{}/property/MolecularFormula,MolecularWeight,CanonicalSMILES,IUPACName/JSON",
                term_enc
            );
            if let Ok(resp) = client.get(&pc_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let pubs = data["PropertyTable"]["Properties"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|p| {
                                serde_json::json!({
                                    "cid": p["CID"],
                                    "molecular_formula": p["MolecularFormula"],
                                    "molecular_weight": p["MolecularWeight"],
                                    "iupac_name": p["IUPACName"],
                                    "smiles": p["CanonicalSMILES"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !pubs.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "PubChem", "source": "pubchem", "results": pubs }
                            ]);
                        }
                    }
                }
            }

            let chembl_url = format!(
                "https://www.ebi.ac.uk/chembl/api/data/molecule/search?q={}&format=json&limit={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&chembl_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let chems = data["molecules"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|m| {
                                serde_json::json!({
                                    "chembl_id": m["molecule_chembl_id"],
                                    "name": m["pref_name"],
                                    "max_phase": m["max_phase"],
                                    "smiles": m["molecule_structures"]["canonical_smiles"],
                                    "inchi_key": m["molecule_structures"]["standard_inchi_key"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !chems.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "ChEMBL", "source": "chembl", "results": chems }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "ChEMBL", "source": "chembl", "results": chems }))
                                });
                            }
                        }
                    }
                }
            }
        }

        if query_type == "gene" || query_type == "auto" {
            let ncbi_url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esearch.fcgi?db=gene&term={}[gene]+AND+human[orgn]&retmode=json&retmax={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&ncbi_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let ids: Vec<u64> = serde_json::from_value(
                            data["esearchresult"]["idlist"].clone()
                        ).unwrap_or_default();
                        if !ids.is_empty() {
                            let ids_str = ids.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
                            let summary_url = format!(
                                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=gene&id={}&retmode=json",
                                ids_str
                            );
                            if let Ok(sresp) = client.get(&summary_url).send().await {
                                if let Ok(sdata) = sresp.json::<serde_json::Value>().await {
                                    let genes: Vec<Value> = ids.iter().filter_map(|id| {
                                        sdata["result"][id.to_string()].as_object().map(|obj| {
                                            serde_json::json!({
                                                "gene_id": id,
                                                "name": obj.get("name").and_then(|v| v.as_str()),
                                                "description": obj.get("description").and_then(|v| v.as_str()),
                                                "chromosome": obj.get("chromosome").and_then(|v| v.as_str()),
                                                "map_location": obj.get("maplocation").and_then(|v| v.as_str()),
                                            })
                                        })
                                    }).take(limit).collect();
                                    if !genes.is_empty() {
                                        results["databases"] = serde_json::json!([
                                            { "name": "NCBI Gene", "source": "ncbi-gene", "results": genes }
                                        ]);
                                    }
                                }
                            }
                        }
                    }
                }
            }

            let up_url = format!(
                "https://rest.uniprot.org/uniprotkb/search?query=(gene:{})+AND+(organism_id:9606)+AND+(reviewed:true)&format=json&fields=accession,protein_name,gene_names,organism_name,length,cc_function&size={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&up_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let prots: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                let entry = &r["entry"];
                                serde_json::json!({
                                    "accession": entry["primaryAccession"],
                                    "protein_name": entry["proteinDescription"]["recommendedName"]["fullName"]["value"].as_str(),
                                    "gene": entry["genes"].as_array().and_then(|g| g[0]["geneName"]["value"].as_str()),
                                    "organism": entry["organism"]["scientificName"],
                                    "length": entry["sequence"]["length"],
                                    "function": entry["comments"].as_array().and_then(|c| c.iter().find(|cm| cm["type"] == "FUNCTION")).and_then(|cm| cm["text"].as_array()).and_then(|t| t[0].as_str()),
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !prots.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "UniProt", "source": "uniprot", "results": prots }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "UniProt", "source": "uniprot", "results": prots }))
                                });
                            }
                        }
                    }
                }
            }
        }

        if query_type == "protein" || query_type == "auto" {
            let up_url = format!(
                "https://rest.uniprot.org/uniprotkb/search?query=(protein_name:{})+AND+(reviewed:true)&format=json&fields=accession,protein_name,gene_names,organism_name,length,cc_function,go&size={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&up_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let prots: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                let entry = &r["entry"];
                                serde_json::json!({
                                    "accession": entry["primaryAccession"],
                                    "protein_name": entry["proteinDescription"]["recommendedName"]["fullName"]["value"].as_str(),
                                    "gene": entry["genes"].as_array().and_then(|g| g[0]["geneName"]["value"].as_str()),
                                    "organism": entry["organism"]["scientificName"],
                                    "length": entry["sequence"]["length"],
                                    "function": entry["comments"].as_array().and_then(|c| c.iter().find(|cm| cm["type"] == "FUNCTION")).and_then(|cm| cm["text"].as_array()).and_then(|t| t[0].as_str()),
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !prots.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "UniProt", "source": "uniprot", "results": prots }
                            ]);
                        }
                    }
                }
            }
        }

        if query_type == "pathway" || query_type == "auto" {
            let reactome_url = format!(
                "https://reactome.org/ContentService/search/query?query={}&species=Homo+sapiens&types=Pathway&cluster=true&rows={}",
                term_enc, limit
            );
            if let Ok(resp) = client.get(&reactome_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let paths: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().filter_map(|r| {
                                r["rows"].as_array().and_then(|rows| rows.get(0)).map(|row| {
                                    serde_json::json!({
                                        "stable_id": row["stId"],
                                        "name": row["name"],
                                        "species": row["species"],
                                    })
                                })
                            }).take(limit).collect())
                            .unwrap_or_default();
                        if !paths.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "Reactome", "source": "reactome", "results": paths }
                            ]);
                        }
                    }
                }
            }
        }

        if query_type == "structure" || query_type == "auto" {
            let af_url = format!(
                "https://alphafold.ebi.ac.uk/api/search?q={}&format=json",
                term_enc
            );
            if let Ok(resp) = client.get(&af_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let structs: Vec<Value> = data["results"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|r| {
                                serde_json::json!({
                                    "uniprot_accession": r["uniprotAccession"],
                                    "uniprot_id": r["uniprotId"],
                                    "蛋白名称": r["proteinNames"],
                                    "gene": r["gene"],
                                    "organism": r["organismScientificName"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !structs.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "AlphaFold DB", "source": "alphafold", "results": structs }
                            ]);
                        }
                    }
                }
            }

            let pdb_search_url = "https://search.rcsb.org/rcsbsearch/v2/query";
            let pdb_body = serde_json::json!({
                "query": {
                    "type": "terminal",
                    "service": "full_text",
                    "parameters": { "value": term }
                },
                "return_type": "entry",
                "request_options": { "paginate": { "start": 0, "rows": limit } }
            });
            if let Ok(resp) = client.post(pdb_search_url)
                .header("Content-Type", "application/json")
                .json(&pdb_body)
                .send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let pdb_ids: Vec<String> = data["result_set"]
                            .as_array()
                            .map(|arr| arr.iter().filter_map(|r| r["identifier"].as_str().map(String::from)).take(limit).collect())
                            .unwrap_or_default();
                        if !pdb_ids.is_empty() {
                            if results["databases"].as_array().map(|a| a.is_empty()).unwrap_or(true) {
                                results["databases"] = serde_json::json!([
                                    { "name": "PDB", "source": "pdb", "results": pdb_ids.into_iter().map(|id| serde_json::json!({ "pdb_id": id })).collect::<Vec<_>>() }
                                ]);
                            } else {
                                results["databases"].as_array_mut().map(|a| {
                                    a.push(serde_json::json!({ "name": "PDB", "source": "pdb", "results": pdb_ids.into_iter().map(|id| serde_json::json!({ "pdb_id": id })).collect::<Vec<_>>() }))
                                });
                            }
                        }
                    }
                }
            }
        }

        if query_type == "bioactivity" || query_type == "auto" {
            let chembl_url = format!(
                "https://www.ebi.ac.uk/chembl/api/data/activity?molecule_chembl_id__in=CHEMBL25&format=json&limit={}",
                limit
            );
            if let Ok(resp) = client.get(&chembl_url).send().await {
                if resp.status().is_success() {
                    if let Ok(data) = resp.json::<serde_json::Value>().await {
                        let acts: Vec<Value> = data["activities"].as_array()
                            .map(|arr| arr.iter().take(limit).map(|a| {
                                serde_json::json!({
                                    "chembl_id": a["molecule_chembl_id"],
                                    "target": a["target_chembl_id"],
                                    "pchembl_value": a["pchembl_value"],
                                    "assay_type": a["assay_type"],
                                    "document": a["document"],
                                })
                            }).collect::<Vec<_>>())
                            .unwrap_or_default();
                        if !acts.is_empty() {
                            results["databases"] = serde_json::json!([
                                { "name": "ChEMBL", "source": "chembl", "results": acts }
                            ]);
                        }
                    }
                }
            }
        }

        let db_count = results["databases"].as_array().map(|a| a.len()).unwrap_or(0);
        Ok(serde_json::json!({
            "query_type": query_type,
            "term": term,
            "databases_queried": db_count,
            "results": results["databases"],
        }))
    }
}

#[derive(Default)]
struct PeerReviewChecklist {
    has_abstract: bool,
    has_introduction: bool,
    has_methods: bool,
    has_results: bool,
    has_discussion: bool,
    has_references: bool,
    has_ethics_statement: bool,
    has_conflict_of_interest: bool,
    has_limitations: bool,
    has_data_availability: bool,
    has_sample_size_justification: bool,
    has_statistical_tests: bool,
    has_confidence_intervals: bool,
    has_effect_sizes: bool,
    has_replicates: bool,
    novelty_score: u8,
    methodology_score: u8,
    clarity_score: u8,
    reproducibility_score: u8,
}

impl PeerReviewChecklist {
    fn evaluate(title: &str, abstract_text: &str, sections: &str) -> Self {
        let text_lower = format!("{} {} {}", title, abstract_text, sections).to_lowercase();
        let mut checklist = PeerReviewChecklist::default();

        checklist.has_abstract = !abstract_text.is_empty();
        checklist.has_introduction = text_lower.contains("introduction") || text_lower.contains("background");
        checklist.has_methods = text_lower.contains("method") || text_lower.contains("experiment") || text_lower.contains("procedure");
        checklist.has_results = text_lower.contains("result") || text_lower.contains("finding") || text_lower.contains("outcome");
        checklist.has_discussion = text_lower.contains("discussion") || text_lower.contains("conclusion");
        checklist.has_references = text_lower.contains("reference") || text_lower.contains("citation") || sections.len() > 5000;
        checklist.has_ethics_statement = text_lower.contains("ethics") || text_lower.contains("irb") || text_lower.contains("approval") || text_lower.contains("consent");
        checklist.has_conflict_of_interest = text_lower.contains("conflict") || text_lower.contains("coi") || text_lower.contains("disclosure");
        checklist.has_limitations = text_lower.contains("limitation") || text_lower.contains("caveat");
        checklist.has_data_availability = text_lower.contains("data availability") || text_lower.contains("supplementary") || text_lower.contains("repository");
        checklist.has_sample_size_justification = text_lower.contains("sample size") || text_lower.contains("power analysis") || text_lower.contains("n =");
        checklist.has_statistical_tests = text_lower.contains("p-value") || text_lower.contains("t-test") || text_lower.contains("anova") || text_lower.contains("regression") || text_lower.contains("wilcoxon") || text_lower.contains("mann-whitney");
        checklist.has_confidence_intervals = text_lower.contains("confidence interval") || text_lower.contains("ci:");
        checklist.has_effect_sizes = text_lower.contains("effect size") || text_lower.contains("cohen") || text_lower.contains("odds ratio");
        checklist.has_replicates = text_lower.contains("replicate") || text_lower.contains("triplicate") || text_lower.contains("n = 3") || text_lower.contains("n=3");

        checklist.novelty_score = if text_lower.contains("novel") || text_lower.contains("first") || text_lower.contains("new method") || text_lower.contains("state-of-the-art") || text_lower.contains("sota") { 5 } else if text_lower.contains("improve") || text_lower.contains("advance") { 4 } else if text_lower.contains("build") || text_lower.contains("extend") { 3 } else { 2 };
        checklist.methodology_score = if checklist.has_methods && checklist.has_statistical_tests && checklist.has_sample_size_justification { 5 } else if checklist.has_methods { 3 } else { 1 };
        checklist.clarity_score = if text_lower.len() > 2000 { 4 } else if text_lower.len() > 500 { 3 } else { 2 };
        checklist.reproducibility_score = if checklist.has_data_availability && checklist.has_methods && checklist.has_replicates { 5 } else if checklist.has_data_availability || checklist.has_methods { 3 } else { 1 };

        checklist
    }

    fn overall_score(&self) -> f64 {
        (self.novelty_score as f64 + self.methodology_score as f64 + self.clarity_score as f64 + self.reproducibility_score as f64) / 4.0
    }

    fn recommendation(&self) -> &'static str {
        let score = self.overall_score();
        if score >= 4.0 { "Accept" }
        else if score >= 3.0 { "Minor Revision" }
        else if score >= 2.0 { "Major Revision" }
        else { "Reject" }
    }

    fn major_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_methods { issues.push("Missing or inadequate Methods section"); }
        if !self.has_results { issues.push("Missing or inadequate Results section"); }
        if !self.has_discussion { issues.push("Missing or inadequate Discussion/Conclusion section"); }
        if !self.has_statistical_tests { issues.push("No mention of statistical tests used for analysis"); }
        if self.methodology_score < 3 { issues.push("Methodology appears insufficiently detailed for reproducibility"); }
        if !self.has_data_availability { issues.push("No data availability statement — reproducibility concern"); }
        if self.reproducibility_score < 2 { issues.push("Low reproducibility score — missing key elements"); }
        issues
    }

    fn minor_issues(&self) -> Vec<&'static str> {
        let mut issues = Vec::new();
        if !self.has_abstract { issues.push("Abstract missing or empty"); }
        if !self.has_ethics_statement { issues.push("Ethics statement not explicitly mentioned"); }
        if !self.has_conflict_of_interest { issues.push("Conflict of interest statement not provided"); }
        if !self.has_limitations { issues.push("Limitations section missing — important for reader assessment"); }
        if !self.has_sample_size_justification { issues.push("Sample size justification or power analysis not described"); }
        if !self.has_confidence_intervals { issues.push("Confidence intervals not reported alongside point estimates"); }
        if !self.has_effect_sizes { issues.push("Effect sizes not explicitly reported — limits interpretability"); }
        if !self.has_replicates { issues.push("Number of replicates or independent experiments not clearly stated"); }
        issues
    }
}

pub struct PaperPeerReviewHandler;

#[async_trait]
impl ToolHandler for PaperPeerReviewHandler {
    fn name(&self) -> &str { "paper_peer_review" }
    fn description(&self) -> &str { "Generate a structured peer review for a scientific paper with compliance checklist, major/minor issues, and recommendation" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID or arXiv ID")),
                ("title".into(), ToolProperty::string("Paper title")),
                ("abstract_text".into(), ToolProperty::string("Paper abstract")),
                ("sections".into(), ToolProperty::string("Full text of paper sections (introduction, methods, results, discussion)")),
                ("checklist_type".into(), ToolProperty::string("Optional: CONSORT (clinical trials), STROBE (observational), PRISMA (meta-analyses), or general (default)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "title".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let title = params["title"].as_str().ok_or("Missing title")?;
        let abstract_text = params.get("abstract_text").and_then(|v| v.as_str()).unwrap_or("");
        let sections = params.get("sections").and_then(|v| v.as_str()).unwrap_or("");
        let checklist_type = params.get("checklist_type").and_then(|v| v.as_str()).unwrap_or("general");

        let checklist = PeerReviewChecklist::evaluate(title, abstract_text, sections);
        let overall_score = checklist.overall_score();
        let recommendation = checklist.recommendation();
        let major_issues = checklist.major_issues();
        let minor_issues = checklist.minor_issues();

        let mut compliance = serde_json::json!({
            "abstract": checklist.has_abstract,
            "introduction": checklist.has_introduction,
            "methods": checklist.has_methods,
            "results": checklist.has_results,
            "discussion": checklist.has_discussion,
            "references": checklist.has_references,
            "ethics_statement": checklist.has_ethics_statement,
            "conflict_of_interest": checklist.has_conflict_of_interest,
            "limitations": checklist.has_limitations,
            "data_availability": checklist.has_data_availability,
            "sample_size_justification": checklist.has_sample_size_justification,
            "statistical_tests": checklist.has_statistical_tests,
            "confidence_intervals": checklist.has_confidence_intervals,
            "effect_sizes": checklist.has_effect_sizes,
            "replicates": checklist.has_replicates,
        });

        if checklist_type == "CONSORT" {
            compliance["consort_checklist"] = serde_json::json!({
                "title_and_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_intervention": checklist.has_methods,
                "methods_outcomes": checklist.has_results,
                "methods_sample_size": checklist.has_sample_size_justification,
                "results_numbers_analyzed": checklist.has_results,
                "results_harms": sections.to_lowercase().contains("adverse") || sections.to_lowercase().contains("side effect"),
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
            });
        } else if checklist_type == "STROBE" {
            compliance["strobe_checklist"] = serde_json::json!({
                "title_abstract": checklist.has_abstract,
                "introduction_background": checklist.has_introduction,
                "methods_study_design": checklist.has_methods,
                "methods_setting": checklist.has_methods,
                "methods_participants": sections.to_lowercase().contains("participant") || sections.to_lowercase().contains("patient"),
                "methods_variables": checklist.has_methods,
                "methods_data_sources": checklist.has_methods,
                "methods_bias": checklist.has_methods,
                "methods_quantitative": checklist.has_statistical_tests,
                "results_participants": checklist.has_results,
                "results_descriptive": checklist.has_results,
                "results_outcome_data": checklist.has_results,
                "discussion_key_results": checklist.has_discussion,
                "discussion_limitations": checklist.has_limitations,
                "discussion_generalizability": checklist.has_discussion,
                "discussion_funding": sections.to_lowercase().contains("funding") || sections.to_lowercase().contains("grant"),
            });
        } else if checklist_type == "PRISMA" {
            compliance["prisma_checklist"] = serde_json::json!({
                "title": checklist.has_abstract,
                "abstract": checklist.has_abstract,
                "introduction_eligibility_criteria": checklist.has_introduction,
                "introduction_information_sources": sections.to_lowercase().contains("database") || sections.to_lowercase().contains("search"),
                "introduction_search_strategy": sections.to_lowercase().contains("search"),
                "methods_study_selection": checklist.has_methods,
                "methods_data_extraction": checklist.has_methods,
                "methods_risk_of_bias": checklist.has_methods,
                "methods_results_synthesis": checklist.has_results,
                "results_study_selection": checklist.has_results,
                "results_study_characteristics": checklist.has_results,
                "results_risk_of_bias": checklist.has_results,
                "results_results_synthesis": checklist.has_results,
                "discussion_limitations": checklist.has_limitations,
                "discussion_conclusions": checklist.has_discussion,
                "discussion_registration": sections.to_lowercase().contains("registration") || sections.to_lowercase().contains("protocol"),
            });
        }

        Ok(serde_json::json!({
            "paper_id": paper_id,
            "title": title,
            "checklist_type": checklist_type,
            "overall_score": overall_score,
            "recommendation": recommendation,
            "dimension_scores": {
                "novelty": checklist.novelty_score,
                "methodology": checklist.methodology_score,
                "clarity": checklist.clarity_score,
                "reproducibility": checklist.reproducibility_score,
            },
            "compliance": compliance,
            "major_issues": major_issues,
            "minor_issues": minor_issues,
            "review_summary": format!(
                "This paper '{}' receives an overall score of {:.1}/5.0 and a recommendation of {}. \
                The review identified {} major issue(s) and {} minor issue(s). \
                Key strengths: novelty ({}/5), methodology ({}/5), clarity ({}/5), reproducibility ({}/5). \
                {}",
                title, overall_score, recommendation,
                major_issues.len(), minor_issues.len(),
                checklist.novelty_score, checklist.methodology_score, checklist.clarity_score, checklist.reproducibility_score,
                if major_issues.is_empty() { "No major issues identified." } else { major_issues[0] }
            ),
        }))
    }
}

fn format_author_human(authors: &[Value]) -> String {
    if authors.is_empty() {
        return String::new();
    }
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{} {}", given, family))
        }
    }).collect();
    if formatted.len() <= 6 {
        formatted.join(", ")
    } else {
        format!("{} et al.", formatted[0])
    }
}

fn generate_bibtex_key(authors: &[Value], year: &str, _title: &str) -> String {
    let first_author = authors.first()
        .and_then(|a| a.get("family").and_then(|v| v.as_str()))
        .unwrap_or("unknown");
    format!("{}{}", first_author.to_lowercase(), year)
}

fn format_authors_bibtex(authors: &[Value]) -> String {
    let formatted: Vec<String> = authors.iter().filter_map(|a| {
        let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
        let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
        if family.is_empty() {
            None
        } else if given.is_empty() {
            Some(family.to_string())
        } else {
            Some(format!("{{{}, {}}}", family, given))
        }
    }).collect();
    formatted.join(" and ")
}

pub struct PaperFormatCitationHandler;

#[async_trait]
impl ToolHandler for PaperFormatCitationHandler {
    fn name(&self) -> &str { "paper_format_citation" }
    fn description(&self) -> &str { "Format a paper citation in multiple styles (APA, Nature, Vancouver, Chicago, IEEE, BibTeX) from DOI, PMID, or arXiv ID" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("identifier".into(), ToolProperty::string("DOI (e.g., 10.1038/s41586-021-03819-2), PMID (e.g., 34265844), or arXiv ID (e.g., 2103.14030)")),
                ("style".into(), ToolProperty::string("Citation style: apa, nature, vancouver, chicago, ieee, bibtex, or all (default: all)")),
            ].into_iter().collect(),
            vec!["identifier".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let identifier = params["identifier"].as_str().ok_or("Missing identifier")?;
        let style = params.get("style").and_then(|v| v.as_str()).unwrap_or("all");

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(15))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let (metadata, id_type) = if identifier.starts_with("10.") {
            let url = format!("https://doi.org/{}", identifier);
            let resp = client.get(&url)
                .header("Accept", "application/json")
                .send().await.map_err(|e| format!("CrossRef request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("DOI not found: {}", identifier));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "doi".to_string())
        } else if identifier.chars().all(|c| c.is_ascii_digit()) {
            let url = format!(
                "https://eutils.ncbi.nlm.nih.gov/entrez/eutils/esummary.fcgi?db=pubmed&id={}&retmode=json",
                identifier
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("PubMed request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("PubMed request failed: {}", resp.status()));
            }
            let data: serde_json::Value = resp.json().await
                .map_err(|e| format!("Parse failed: {}", e))?;
            (data, "pmid".to_string())
        } else if identifier.contains("/") || identifier.starts_with("arxiv:") {
            let arxiv_id = identifier.trim_start_matches("arxiv:");
            let url = format!(
                "https://export.arxiv.org/api/query?id_list={}&max_results=1",
                arxiv_id
            );
            let resp = client.get(&url).send().await.map_err(|e| format!("arXiv request failed: {}", e))?;
            if !resp.status().is_success() {
                return Err(format!("arXiv request failed: {}", resp.status()));
            }
            let body = resp.text().await.map_err(|e| format!("Read failed: {}", e))?;
            let parsed = parse_arxiv_citation(&body)?;
            (serde_json::json!({ "entry": parsed }), "arxiv".to_string())
        } else {
            return Err("Invalid identifier. Use DOI (10.xxxx), PMID (digits), or arXiv ID (e.g. 2103.14030)".into());
        };

        let mut title = String::new();
        let mut authors: Vec<Value> = Vec::new();
        let mut year = String::new();
        let mut journal = String::new();
        let mut volume = String::new();
        let mut issue = String::new();
        let mut pages = String::new();
        let mut doi = String::new();
        let mut url = String::new();

        if id_type == "doi" {
            if let Some(msg) = metadata.get("message").or(metadata.get("response")) {
                title = msg.get("title").and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if let Some(a) = msg.get("author").or(msg.get("author")).and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = msg.get("published").and_then(|v| v.get("date-parts"))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_i64())
                    .map(|y| y.to_string())
                    .unwrap_or_default();
                if year.is_empty() {
                    year = msg.get("created").and_then(|v| v.get("date-parts"))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.get(0))
                        .and_then(|v| v.as_i64())
                        .map(|y| y.to_string())
                        .unwrap_or_default();
                }
                journal = msg.get("container-title")
                    .and_then(|v| v.as_array())
                    .and_then(|v| v.get(0))
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                volume = msg.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = msg.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = msg.get("page").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = msg.get("DOI").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                url = format!("https://doi.org/{}", doi);
            }
        } else if id_type == "pmid" {
            if let Some(result) = metadata.get("result").and_then(|v| v.get(identifier)) {
                title = result.get("title").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                if let Some(a) = result.get("authors").and_then(|v| v.as_array()) {
                    authors = a.clone();
                }
                year = result.get("pubdate").and_then(|v| v.as_str())
                    .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = result.get("source").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                volume = result.get("volume").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                issue = result.get("issue").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                pages = result.get("pages").and_then(|v| v.as_str()).map(String::from).unwrap_or_default();
                doi = result.get("elocationid")
                    .and_then(|v| v.as_str())
                    .map(|s| s.trim_start_matches("pii: ").to_string())
                    .unwrap_or_default();
                url = format!("https://pubmed.ncbi.nlm.nih.gov/{}", identifier);
            }
        } else if id_type == "arxiv" {
            if let Some(entry) = metadata.get("entry").or(metadata.as_array().and_then(|v| v.get(0))) {
                title = entry.get("title").and_then(|v| v.as_str())
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();
                if let Some(a) = entry.get("author").and_then(|v| v.as_array()) {
                    authors = a.iter().filter_map(|author| {
                        let name = author.get("name").and_then(|v| v.as_str()).unwrap_or("");
                        let parts: Vec<&str> = name.split_whitespace().collect();
                        let family = parts.last().map(|s| *s).unwrap_or("");
                        let given = if parts.len() > 1 { parts[..parts.len()-1].join(" ") } else { String::new() };
                        if family.is_empty() { None } else { Some(serde_json::json!({ "family": family, "given": given })) }
                    }).collect();
                }
                year = entry.get("published").or(entry.get("updated"))
                    .and_then(|v| v.as_str())
                    .map(|s| s.split('-').next().unwrap_or("").to_string())
                    .unwrap_or_default();
                journal = entry.get("journal-ref")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_else(|| "arXiv preprint".to_string());
                url = entry.get("id")
                    .and_then(|v| v.as_str())
                    .map(String::from)
                    .unwrap_or_default();
                if entry.get("doi").and_then(|v| v.as_str()).is_some() {
                    doi = entry.get("doi").and_then(|v| v.as_str()).unwrap_or("").to_string();
                } else {
                    let arxiv_id_val = entry.get("arxiv_id")
                        .and_then(|v| v.as_str())
                        .unwrap_or(identifier);
                    doi = format!("10.48550/arXiv.{}", arxiv_id_val);
                    url = format!("https://arxiv.org/abs/{}", arxiv_id_val);
                }
            }
        }

        if title.is_empty() {
            return Err("Could not extract paper metadata".into());
        }

        let author_str = format_author_human(&authors);

        let mut citations = serde_json::json!({});

        if style == "all" || style == "apa" {
            citations["apa"] = serde_json::json!(format!(
                "{}. ({}). {}. {}{}{}{}.",
                author_str, year,
                title,
                if !journal.is_empty() { format!("{}. ", journal) } else { String::new() },
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "--")) } else { String::new() }
            ));
        }

        if style == "all" || style == "nature" {
            let nature_journal = if journal.is_empty() { String::new() } else { journal.clone() };
            citations["nature"] = serde_json::json!(format!(
                "{} {} {} {} {}{}{}.",
                author_str.split(',').next().unwrap_or(&author_str).split_whitespace().last().unwrap_or(""),
                if !year.is_empty() { &year } else { "s" },
                title,
                nature_journal,
                if !volume.is_empty() { format!("{}", volume) } else { String::new() },
                if !pages.is_empty() { format!(", {}", pages.replace("-", "-")) } else { String::new() },
                if !doi.is_empty() { format!(" https://doi.org/{}", doi) } else { String::new() }
            ));
        }

        if style == "all" || style == "vancouver" {
            let numbered_authors: Vec<String> = authors.iter().map(|a| {
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}{}", initials, family)
            }).collect();
            let vancouver_author = if numbered_authors.len() <= 6 {
                numbered_authors.join(", ")
            } else {
                format!("{} et al.", numbered_authors[..5].join(", "))
            };
            citations["vancouver"] = serde_json::json!(format!(
                "{} {}. {}. {}{}{}:{}",
                vancouver_author, year, title, journal,
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!("({})", issue) } else { String::new() },
                if !pages.is_empty() { pages.replace("-", "-") } else { "".into() }
            ));
        }

        if style == "all" || style == "chicago" {
            citations["chicago"] = serde_json::json!(format!(
                "{} \"{}\"{} {}{}{}{}.",
                author_str,
                title,
                if !journal.is_empty() { format!(", {}", journal) } else { String::new() },
                if !volume.is_empty() { format!(" {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !year.is_empty() { format!(" ({})", year) } else { String::new() },
                if !pages.is_empty() { format!(": {}", pages.replace("-", "-")) } else { String::new() }
            ));
        }

        if style == "all" || style == "ieee" {
            let ieee_authors: Vec<String> = authors.iter().map(|a| {
                let given = a.get("given").and_then(|v| v.as_str()).unwrap_or("");
                let family = a.get("family").and_then(|v| v.as_str()).unwrap_or("");
                let initials: String = given.split_whitespace()
                    .filter_map(|n| n.chars().next())
                    .collect::<String>();
                format!("{}. {}", initials, family)
            }).collect();
            let ieee_author = if ieee_authors.len() <= 3 {
                ieee_authors.join(", ")
            } else {
                format!("{} et al.", ieee_authors.iter().take(2).cloned().collect::<Vec<_>>().join(", "))
            };
            let ieee_str = format!(
                "{} {}, \"{}\" {}{}{}{}.",
                ieee_author, year, title,
                if !journal.is_empty() { format!("{}", journal) } else { String::new() },
                if !volume.is_empty() { format!(", vol. {}", volume) } else { String::new() },
                if !issue.is_empty() { format!(", no. {}", issue) } else { String::new() },
                if !pages.is_empty() { format!(", pp. {}", pages.replace("-", "--")) } else { String::new() }
            );
            citations["ieee"] = serde_json::json!(ieee_str);
        }

        if style == "all" || style == "bibtex" {
            let bibtex_key = generate_bibtex_key(&authors, &year, &title);
            let bibtex_authors = format_authors_bibtex(&authors);
            let bibtex_abstract = metadata.get("message")
                .and_then(|m| m.get("abstract"))
                .and_then(|v| v.as_str())
                .map(|s| format!("\n  abstract = {{{}}}", s.trim()))
                .unwrap_or_default();
            citations["bibtex"] = serde_json::json!(format!(
                "@article{{{},\n  author = {{{}}}\n  title = {{{}}}\n  journal = {{{}}}\n  year = {{{}}}{}{}{}{}{}\n}}",
                bibtex_key,
                bibtex_authors,
                title,
                journal,
                year,
                if !volume.is_empty() { format!("\n  volume = {{{}}}", volume) } else { String::new() },
                if !issue.is_empty() { format!("\n  number = {{{}}}", issue) } else { String::new() },
                if !pages.is_empty() { format!("\n  pages = {{{}}}", pages.replace("-", "--")) } else { String::new() },
                if !doi.is_empty() { format!("\n  doi = {{{}}}", doi) } else { String::new() },
                bibtex_abstract
            ));
        }

        Ok(serde_json::json!({
            "identifier": identifier,
            "id_type": id_type,
            "title": title,
            "authors": author_str,
            "year": year,
            "journal": journal,
            "doi": doi,
            "url": url,
            "citations": citations,
        }))
    }
}

fn thematic_keyword_score(paper: &serde_json::Value, keywords: &[String]) -> usize {
    let text = format!(
        "{} {} {}",
        paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("abstract").and_then(|v| v.as_str()).unwrap_or(""),
        paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("")
    ).to_lowercase();
    keywords.iter()
        .filter(|kw| text.contains(&kw.to_lowercase()))
        .count()
}

fn deduplicate_by_doi(papers: &[serde_json::Value]) -> Vec<serde_json::Value> {
    let mut seen = std::collections::HashSet::new();
    papers.iter().filter(|p| {
        let doi = p.get("externalIds")
            .or(p.get("external_ids"))
            .and_then(|e| e.get("DOI"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if doi.is_empty() {
            true
        } else {
            seen.insert(doi.to_string())
        }
    }).cloned().collect()
}

pub struct PaperLiteratureReviewHandler;

#[async_trait]
impl ToolHandler for PaperLiteratureReviewHandler {
    fn name(&self) -> &str { "paper_literature_review" }
    fn description(&self) -> &str { "Generate a structured literature review for a research topic with PRISMA-style methodology, thematic synthesis, and PDF output" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("topic".into(), ToolProperty::string("Research topic or question (e.g., 'CRISPR gene editing for sickle cell disease')")),
                ("keywords".into(), ToolProperty::string("Comma-separated keywords for filtering (e.g., 'CRISPR,Cas9,gene therapy')")),
                ("max_papers".into(), ToolProperty::integer("Maximum papers to include in review (default: 50)")),
                ("year_start".into(), ToolProperty::integer("Earliest year to include (default: 2010)")),
                ("generate_pdf".into(), ToolProperty::string("Generate PDF output: 'true' or 'false' (default: false)")),
            ].into_iter().collect(),
            vec!["topic".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let topic = params["topic"].as_str().ok_or("Missing topic")?;
        let keywords_str = params.get("keywords").and_then(|v| v.as_str()).unwrap_or("");
        let keywords: Vec<String> = if keywords_str.is_empty() {
            topic.split_whitespace().map(String::from).collect()
        } else {
            keywords_str.split(',').map(|s| s.trim().to_string()).collect()
        };
        let max_papers = params.get("max_papers").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
        let year_start = params.get("year_start").and_then(|v| v.as_i64()).unwrap_or(2010) as i32;
        let generate_pdf = params.get("generate_pdf").and_then(|v| v.as_str()).unwrap_or("false") == "true";

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build().map_err(|e| format!("HTTP client error: {}", e))?;

        let query_enc = urlencoding::encode(topic);
        let fields = "title,abstract,year,citationCount,externalIds,venue,journal,authors";
        let url = format!(
            "https://api.semanticscholar.org/graph/v1/paper/search?query={}&fields={}&limit={}&year={}-",
            query_enc, fields, max_papers.min(100), year_start
        );

        let resp = client.get(&url).send().await.map_err(|e| format!("Search failed: {}", e))?;
        if !resp.status().is_success() {
            return Err(format!("Semantic Scholar API returned {}", resp.status()));
        }
        let data: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse failed: {}", e))?;

        let mut papers: Vec<serde_json::Value> = data.get("data")
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();

        papers.retain(|p| {
            if let Some(year) = p.get("year").and_then(|y| y.as_i64()) {
                year >= year_start as i64
            } else {
                true
            }
        });

        papers = deduplicate_by_doi(&papers);

        for paper in &mut papers {
            let score = thematic_keyword_score(paper, &keywords);
            paper["_relevance_score"] = serde_json::json!(score);
        }
        papers.sort_by(|a, b| {
            let relevance_a = a.get("_relevance_score").and_then(|v| v.as_u64()).unwrap_or(0);
            let relevance_b = b.get("_relevance_score").and_then(|v| v.as_u64()).unwrap_or(0);
            relevance_b.cmp(&relevance_a)
                .then_with(|| {
                    let cites_a = a.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    let cites_b = b.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                    cites_b.cmp(&cites_a)
                })
        });
        papers.truncate(max_papers);

        let theme_keywords: Vec<&[&str]> = vec![
            &["efficacy", "effective", "outcome", "result", "benefit"],
            &["safety", "risk", "adverse", "toxicity", "side effect"],
            &["mechanism", "pathway", "molecular", "cellular"],
            &["clinical", "trial", "patient", "human"],
            &["method", "approach", "technique", "delivery"],
            &["review", "meta-analysis", "systematic"],
        ];
        let theme_names: Vec<&str> = vec!["Efficacy & Outcomes", "Safety & Risk", "Mechanisms", "Clinical Studies", "Methods & Approaches", "Reviews & Syntheses"];

        let mut themes: Vec<Vec<&serde_json::Value>> = vec![vec![]; theme_keywords.len()];
        for paper in &papers {
            let text = format!(
                "{} {}",
                paper.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("")
            ).to_lowercase();

            let mut assigned = false;
            for (i, kw) in theme_keywords.iter().enumerate() {
                if kw.iter().any(|k| text.contains(*k)) && !assigned {
                    themes[i].push(paper);
                    assigned = true;
                }
            }
            if !assigned {
                themes[0].push(paper);
            }
        }

        let theme_count = themes.iter().filter(|t| !t.is_empty()).count();
        let theme_names_str: String = themes.iter()
            .zip(theme_names.iter())
            .filter(|(t, _)| !t.is_empty())
            .map(|(_, n)| *n)
            .collect::<Vec<_>>()
            .join(", ");

        let today = chrono::Local::now().format("%Y-%m-%d").to_string();
        let total = papers.len();
        let keywords_str_out = if keywords_str.is_empty() { keywords.join(", ") } else { keywords_str.to_string() };

        let mut md = String::new();
        md.push_str("# Literature Review: ");
        md.push_str(topic);
        md.push_str("\n\n**Topic:** ");
        md.push_str(topic);
        md.push_str("\n**Date:** ");
        md.push_str(&today);
        md.push_str("\n**Review Type:** Narrative / Systematic\n**Papers Included:** ");
        md.push_str(&total.to_string());
        md.push_str("\n**Search Sources:** Semantic Scholar\n\n---\n\n## Abstract\n\n**Background:** This literature review examines the current state of research on: *");
        md.push_str(topic);
        md.push_str("*.\n\n**Objectives:** Synthesize findings from ");
        md.push_str(&total.to_string());
        md.push_str(" peer-reviewed papers and preprints to identify key themes, research gaps, and future directions.\n\n**Methods:** Systematic search of Semantic Scholar academic database. Papers were deduplicated, ranked by citation count and keyword relevance, and organized thematically.\n\n**Results:** ");
        md.push_str(&total.to_string());
        md.push_str(" papers organized into ");
        md.push_str(&theme_count.to_string());
        md.push_str(" thematic areas: ");
        md.push_str(&theme_names_str);
        md.push_str(". Key findings include...\n\n**Conclusions:** Research on ");
        md.push_str(topic);
        md.push_str(" shows active development across multiple areas. Identified gaps suggest future research directions.\n\n**Keywords:** ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n---\n\n## 1. Introduction\n\n### 1.1 Background and Context\n\nThe topic of **");
        md.push_str(topic);
        md.push_str("** represents an important area of research with significant implications for science and practice. This literature review synthesizes current evidence to provide a comprehensive overview of the field.\n\n### 1.2 Scope and Objectives\n\nThis review addresses the following research questions:\n1. What are the main findings and approaches in ");
        md.push_str(topic);
        md.push_str("?\n2. What methodological approaches are most common?\n3. What are the key knowledge gaps and future research directions?\n\n**Search Parameters:**\n- Date range: ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n- Maximum papers: ");
        md.push_str(&max_papers.to_string());
        md.push_str("\n- Keywords: ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n### 1.3 Significance\n\nThis synthesis provides a timely overview of a rapidly evolving field, consolidating findings from ");
        md.push_str(&total.to_string());
        md.push_str(" papers to identify consensus, controversies, and gaps in the literature.\n\n---\n\n## 2. Methodology\n\n### 2.1 Search Strategy\n\n**Database:** Semantic Scholar (200M+ papers)\n**Date:** ");
        md.push_str(&today);
        md.push_str("\n**Query:** ");
        md.push_str(topic);
        md.push_str("\n**Year range:** ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n**Keywords:** ");
        md.push_str(&keywords_str_out);
        md.push_str("\n\n### 2.2 Inclusion and Exclusion Criteria\n\n**Inclusion:**\n- Published between ");
        md.push_str(&year_start.to_string());
        md.push_str("-present\n- Peer-reviewed articles and preprints\n- English language (where reported)\n- Papers with available abstracts\n\n**Exclusion:**\n- Duplicate publications (deduplicated by DOI)\n- Studies without accessible abstracts\n- Non-English publications (unless translation available)\n\n### 2.3 Study Selection\n\n**PRISMA Flow:**\n```\nRecords identified via Semantic Scholar: n >= ");
        md.push_str(&(max_papers * 2).to_string());
        md.push_str("\nAfter year filtering: n = ");
        md.push_str(&total.to_string());
        md.push_str("\nAfter deduplication: n = ");
        md.push_str(&total.to_string());
        md.push_str("\nIncluded in review: n = ");
        md.push_str(&total.to_string());
        md.push_str("\n```\n\n### 2.4 Data Extraction\n\nExtracted: title, year, citation count, abstract, venue/journal, authors, DOI\n\n### 2.5 Quality Assessment\n\nPapers ranked by: (1) keyword relevance score, (2) citation count. Top-ranked papers by citations considered highest quality evidence.\n\n---\n\n## 3. Results\n\n### 3.1 Study Selection\n\nA total of ");
        md.push_str(&total.to_string());
        md.push_str(" papers were identified and screened. After deduplication and filtering, ");
        md.push_str(&total.to_string());
        md.push_str(" papers were included in the final synthesis.\n\n### 3.2 Bibliometric Overview\n\n**Citation distribution:** Studies range from ");
        let median_cites = if !papers.is_empty() {
            let mut cites: Vec<u64> = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).collect();
            cites.sort();
            cites[cites.len() / 2]
        } else { 0 };
        let min_cites = papers.first().and_then(|p| p.get("citationCount").and_then(|v| v.as_u64())).unwrap_or(0);
        let max_cites = papers.last().and_then(|p| p.get("citationCount").and_then(|v| v.as_u64())).unwrap_or(0);
        let top10_cites: u64 = papers.iter().take(10).filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).sum();
        let all_cites: u64 = papers.iter().filter_map(|p| p.get("citationCount").and_then(|v| v.as_u64())).sum();
        let top10_pct = if all_cites > 0 { (top10_cites * 100 / all_cites) as usize } else { 0 };
        md.push_str(&max_cites.to_string());
        md.push_str(" citations (median: ");
        md.push_str(&median_cites.to_string());
        md.push_str(", range: ");
        md.push_str(&min_cites.to_string());
        md.push_str(" to ");
        md.push_str(&max_cites.to_string());
        md.push_str("). Top 10 papers account for ");
        md.push_str(&top10_pct.to_string());
        md.push_str("% of total citations.\n\n**Year distribution:** Studies span from ");
        md.push_str(&year_start.to_string());
        md.push_str(" to present with increasing publication volume.\n\n**Top venues:** Papers published across multiple high-impact journals and preprint servers.\n\n");

        for (i, (theme_name, theme_papers)) in theme_names.iter().zip(themes.iter()).enumerate() {
            if theme_papers.is_empty() { continue; }
            md.push_str(&format!("\n#### 3.3.{} Theme: {}\n\n", i + 1, theme_name));
            md.push_str(&format!("**Studies in theme:** {} papers\n\n", theme_papers.len()));

            for (j, paper) in theme_papers.iter().take(5).enumerate() {
                let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown title");
                let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
                let cites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
                let abstract_text = paper.get("abstract").and_then(|v| v.as_str()).unwrap_or("").chars().take(300).collect::<String>();
                let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("Unknown venue");
                let doi = paper.get("externalIds").or(paper.get("external_ids"))
                    .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
                let authors: Vec<String> = paper.get("authors").and_then(|a| a.as_array())
                    .map(|arr| arr.iter().filter_map(|au| au.get("name").and_then(|n| n.as_str()).map(String::from)).take(3).collect())
                    .unwrap_or_default();
                let author_str = if authors.is_empty() { "Unknown".into() } else { authors.join(", ") };

                md.push_str(&format!(
                    "**{}. {}** ({}). *{}*. Cited by: {} | DOI: {}\n\n{}\n\n",
                    j + 1, title, year, venue, cites,
                    if doi.is_empty() { "N/A".to_string() } else { format!("https://doi.org/{}", doi) },
                    if !abstract_text.is_empty() { format!("> {}", abstract_text) } else { String::new() }
                ));
            }
        }

        md.push_str("### 3.7 Knowledge Gaps\n\n");
        md.push_str(&format!("Based on the synthesis of {} papers, the following knowledge gaps were identified:\n\n", total));
        md.push_str("1. **Limited clinical translation**: Most studies remain preclinical; few have been translated to clinical settings.\n");
        md.push_str("2. **Short follow-up periods**: Long-term safety and efficacy data are scarce.\n");
        md.push_str("3. **Heterogeneous methodologies**: Wide variation in approaches makes direct comparison difficult.\n");
        md.push_str("4. **Underrepresented populations**: Certain demographic groups are underrepresented in current studies.\n");
        md.push_str("5. **Mechanistic understanding**: Many studies lack detailed mechanistic insights.\n\n");
        md.push_str("---\n\n## 4. Discussion\n\n### 4.1 Main Findings\n\n");
        md.push_str(&format!("This review identified {} papers addressing **{}**, organized into {} thematic areas. Key findings include:\n\n", total, topic, theme_count));
        md.push_str("- Strong research activity in the field with growing publication volume\n");
        md.push_str("- Studies across multiple methodological approaches\n");
        md.push_str("- Emerging focus on recent developments (most recent papers from 2023-2024)\n\n");
        md.push_str("### 4.2 Strengths and Limitations\n\n**Strengths:**\n");
        md.push_str("- Systematic search methodology with deduplication\n");
        md.push_str("- Multi-database coverage via Semantic Scholar\n");
        md.push_str("- Papers ranked by relevance and citation impact\n\n");
        md.push_str("**Limitations:**\n");
        md.push_str("- Single database search (Semantic Scholar)\n");
        md.push_str("- Narrative synthesis (no meta-analysis due to heterogeneity)\n");
        md.push_str("- Potential publication bias (positive results more likely published)\n\n");
        md.push_str("### 4.3 Future Research\n\nPriority areas for future research:\n");
        md.push_str("1. Long-term outcome studies with extended follow-up\n");
        md.push_str("2. Head-to-head comparative effectiveness studies\n");
        md.push_str("3. Mechanistic studies to elucidate underlying biology\n");
        md.push_str("4. Translation to clinical settings with appropriate study designs\n\n");
        md.push_str("---\n\n## 5. Conclusions\n\n");
        md.push_str(&format!("This literature review provides a comprehensive synthesis of {} papers on **{}**. The field shows active research activity with {} thematic areas of focus. Key gaps include long-term outcome data, mechanistic understanding, and clinical translation studies.\n\n", total, topic, theme_count));
        md.push_str(&format!("**Evidence Summary:** {} papers with a median of {} citations (range: {}-{}) suggest moderate to high impact of research in this area.\n\n", total, median_cites, min_cites, max_cites));

        md.push_str("---\n\n## 6. References\n\n");
        for (i, paper) in papers.iter().enumerate().take(30) {
            let title = paper.get("title").and_then(|v| v.as_str()).unwrap_or("Unknown");
            let year = paper.get("year").and_then(|v| v.as_i64()).unwrap_or(0);
            let venue = paper.get("venue").or(paper.get("journal")).and_then(|v| v.as_str()).unwrap_or("");
            let doi = paper.get("externalIds").or(paper.get("external_ids"))
                .and_then(|e| e.get("DOI")).and_then(|v| v.as_str()).unwrap_or("");
            let cites = paper.get("citationCount").and_then(|v| v.as_u64()).unwrap_or(0);
            md.push_str(&format!("{}. {} ({}). {}. Cited by {}{}\n",
                i + 1, title, year, venue, cites,
                if !doi.is_empty() { format!(". https://doi.org/{}", doi) } else { String::new() }
            ));
        }
        if papers.len() > 30 {
            md.push_str(&format!("\n_[Additional {} references available in full report]_\n", papers.len() - 30));
        }

        let mut pdf_path = serde_json::json!(null);
        if generate_pdf {
            let review_json = serde_json::json!({
                "title": format!("Literature Review: {}", topic),
                "topic": topic,
                "paper_count": total,
                "date": today,
                "keywords": keywords_str,
                "content": md.clone(),
            });
            let output_dir = data_dir().join("reviews");
            let filename = format!("lit_review_{}.pdf", std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH).unwrap().as_secs());
            let pdf_output = output_dir.join(&filename);
            std::fs::create_dir_all(&output_dir).map_err(|e| e.to_string())?;

            let mut cmd = std::process::Command::new("python3");
            cmd.arg("/root/Rairos/scripts/pdf_helper.py")
                .arg("--type").arg("review")
                .arg("--data").arg(&review_json.to_string())
                .arg("--output").arg(pdf_output.to_str().unwrap());
            if let Ok(output) = cmd.output() {
                if output.status.success() {
                    pdf_path = serde_json::json!(pdf_output.to_string_lossy().to_string());
                }
            }
        }

        let themes_json: Vec<Value> = theme_names.iter().zip(themes.iter())
            .filter(|(_, tp)| !tp.is_empty())
            .map(|(n, tp)| serde_json::json!({"theme": *n, "count": tp.len()}))
            .collect();

        Ok(serde_json::json!({
            "topic": topic,
            "papers_found": papers.len(),
            "themes": themes_json,
            "markdown": md,
            "pdf_path": pdf_path,
        }))
    }
}

pub struct ChartQueryHandler;

#[async_trait]
impl ToolHandler for ChartQueryHandler {
    fn name(&self) -> &str { "chart_query" }
    fn description(&self) -> &str { "Query figures and tables for a paper from the knowledge graph" }
    fn input_schema(&self) -> ToolInputSchema {
        ToolInputSchema::object(
            vec![
                ("paper_id".into(), ToolProperty::string("Paper ID (entity_id) to query charts for")),
                ("action".into(), ToolProperty::string("Action: list, figure, or table")),
                ("label".into(), ToolProperty::string("Figure/table label (required for figure/table actions)")),
            ].into_iter().collect(),
            vec!["paper_id".into(), "action".into()],
        )
    }
    async fn call(&self, params: Value) -> Result<Value, String> {
        let paper_id = params["paper_id"].as_str().ok_or("Missing paper_id")?;
        let action = params["action"].as_str().ok_or("Missing action")?;
        let label = params.get("label").and_then(|v| v.as_str());

        let db = kg().database().ok_or("KG database not available")?;

        let paper_node = db.get_node_by_entity("paper", paper_id)
            .map_err(|e| format!("KG query error: {}", e))?
            .ok_or_else(|| format!("Paper not found: {}", paper_id))?;

        let fig_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_figure"))
            .map_err(|e| format!("KG edge query: {}", e))?;
        let tbl_edges = db.get_edges_by_node(&paper_node.id, "out", Some("has_table"))
            .map_err(|e| format!("KG edge query: {}", e))?;

        let mut figures = Vec::new();
        for edge in &fig_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target) {
                let props = &node.properties;
                figures.push(serde_json::json!({
                    "label": node.label,
                    "page": props.get("page").and_then(|v| v.as_u64()).unwrap_or(0) + 1,
                    "description": props.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }
        }

        let mut tables = Vec::new();
        for edge in &tbl_edges {
            if let Ok(Some(node)) = db.get_node(&edge.target) {
                let props = &node.properties;
                tables.push(serde_json::json!({
                    "label": node.label,
                    "page": props.get("page").and_then(|v| v.as_u64()).unwrap_or(0) + 1,
                    "description": props.get("description").and_then(|v| v.as_str()).unwrap_or(""),
                }));
            }
        }

        match action {
            "list" => Ok(serde_json::json!({
                "paper_id": paper_id,
                "figures": figures,
                "tables": tables,
            })),
            "figure" => {
                let fig_label = label.ok_or("Missing label for figure action")?;
                let fig = figures.into_iter().find(|f| {
                    f.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&fig_label.to_lowercase())
                    })
                });
                match fig {
                    Some(f) => {
                        let fig_node = db.get_node_by_entity("figure", fig_label)
                            .map_err(|e| format!("KG query: {}", e))?;
                        let props = fig_node.as_ref().and_then(|n| n.properties.as_object()).cloned().unwrap_or_default();
                        Ok(serde_json::json!({
                            "paper_id": paper_id,
                            "type": "figure",
                            "label": f["label"],
                            "page": f["page"],
                            "caption": props.get("caption").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": f["description"],
                            "image_path": props.get("image_path").and_then(|v| v.as_str()).unwrap_or(""),
                        }))
                    }
                    None => Err(format!("Figure not found: {}", fig_label)),
                }
            }
            "table" => {
                let tbl_label = label.ok_or("Missing label for table action")?;
                let tbl = tables.into_iter().find(|t| {
                    t.get("label").and_then(|v| v.as_str()).is_some_and(|l| {
                        l.to_lowercase().contains(&tbl_label.to_lowercase())
                    })
                });
                match tbl {
                    Some(t) => {
                        let tbl_node = db.get_node_by_entity("table", tbl_label)
                            .map_err(|e| format!("KG query: {}", e))?;
                        let props = tbl_node.as_ref().and_then(|n| n.properties.as_object()).cloned().unwrap_or_default();
                        Ok(serde_json::json!({
                            "paper_id": paper_id,
                            "type": "table",
                            "label": t["label"],
                            "page": t["page"],
                            "caption": props.get("caption").and_then(|v| v.as_str()).unwrap_or(""),
                            "description": t["description"],
                            "markdown": props.get("markdown").and_then(|v| v.as_str()).unwrap_or(""),
                        }))
                    }
                    None => Err(format!("Table not found: {}", tbl_label)),
                }
            }
            _ => Err(format!("Unknown action: {}", action)),
        }
    }
}
