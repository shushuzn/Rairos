use crate::protocol::{ToolHandler, ToolInputSchema, ToolProperty};
use async_trait::async_trait;
use serde_json::Value;

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

        let client = crate::handlers::helpers::http_client(20)?;

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
                                if let Some(a) = results["databases"].as_array_mut() { a.push(serde_json::json!({ "name": "ChEMBL", "source": "chembl", "results": chems })) }
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
                                if let Some(a) = results["databases"].as_array_mut() { a.push(serde_json::json!({ "name": "UniProt", "source": "uniprot", "results": prots })) }
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
                                r["rows"].as_array().and_then(|rows| rows.first()).map(|row| {
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
                                if let Some(a) = results["databases"].as_array_mut() { a.push(serde_json::json!({ "name": "PDB", "source": "pdb", "results": pdb_ids.into_iter().map(|id| serde_json::json!({ "pdb_id": id })).collect::<Vec<_>>() })) }
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
