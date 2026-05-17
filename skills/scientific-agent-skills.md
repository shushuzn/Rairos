---
name: scientific-agent-skills
description: Comprehensive collection of 135 ready-to-use scientific and research skills for AI agents. Covers bioinformatics, drug discovery, clinical research, machine learning, and 100+ scientific databases.
version: 1.0.0
source: https://github.com/K-Dense-AI/scientific-agent-skills
license: MIT
tags:
  - research
  - bioinformatics
  - drug-discovery
  - machine-learning
  - clinical
  - scientific
---

# Scientific Agent Skills

A comprehensive collection of **135 scientific and research skills** for AI agents that support the open [Agent Skills](https://agentskills.io/) standard.

## Capabilities

### 100+ Scientific Databases
- PubChem, ChEMBL, UniProt, PDB, AlphaFold
- KEGG, Reactome, STRING, ClinVar, COSMIC
- ClinicalTrials.gov, FDA, PubMed
- And 78+ more public databases

### Bioinformatics & Genomics (21+ skills)
- **Sequence Analysis**: BioPython, pysam, scikit-bio, BioServices
- **Single-Cell Analysis**: Scanpy, AnnData, scvi-tools, scVelo, Arboreto, Cellxgene Census
- **Genomic Tools**: gget, geniml, gtars, deepTools
- **Differential Expression**: PyDESeq2
- **Phylogenetics**: ETE Toolkit, MAFFT, IQ-TREE 2, FastTree

### Drug Discovery & Medicinal Chemistry (10+ skills)
- **Molecular Manipulation**: RDKit, Datamol, Molfeat
- **Deep Learning**: DeepChem, TorchDrug
- **Docking & Screening**: DiffDock
- **Molecular Dynamics**: OpenMM + MDAnalysis
- **Drug-Likeness**: MedChem

### Clinical Research & Precision Medicine (8+ skills)
- **Clinical Databases**: ClinicalTrials.gov, ClinVar, ClinPGx, COSMIC, FDA, cBioPortal
- **Cancer Genomics**: DepMap
- **Healthcare AI**: PyHealth, NeuroKit2, Clinical Decision Support

### Machine Learning & AI (16+ skills)
- **Deep Learning**: PyTorch Lightning, Transformers, Stable Baselines3
- **Classical ML**: scikit-learn, scikit-survival, SHAP
- **Time Series**: aeon, TimesFM (Google's zero-shot foundation model)
- **Bayesian Methods**: PyMC
- **Graph ML**: Torch Geometric

### Scientific Communication (20+ skills)
- **Literature**: Paper Lookup (PubMed, PMC, bioRxiv, medRxiv, arXiv, OpenAlex, Crossref, Semantic Scholar)
- **Writing**: Scientific Writing, Peer Review
- **Document Processing**: XLSX, PDF, DOCX, PPTX
- **Presentations**: Scientific Slides, LaTeX Posters, PPTX Posters
- **Diagrams**: Scientific Schematics, Markdown & Mermaid

## Installation

```bash
npx skills add K-Dense-AI/scientific-agent-skills
# or
gh skill install K-Dense-AI/scientific-agent-skills
```

## Usage Examples

### Drug Discovery Pipeline
```
Query ChEMBL for EGFR inhibitors (IC50 < 50nM), analyze structure-activity 
relationships with RDKit, generate improved analogs with datamol, perform 
virtual screening with DiffDock against AlphaFold EGFR structure.
```

### Single-Cell RNA-seq Analysis
```
Load 10X dataset with Scanpy, perform QC and doublet removal, integrate 
with Cellxgene Census data, identify cell types using NCBI Gene markers, 
run differential expression with PyDESeq2.
```

### Clinical Variant Interpretation
```
Parse VCF with pysam, annotate variants with Ensembl VEP, query ClinVar 
for pathogenicity, check COSMIC for cancer mutations, retrieve gene info 
from NCBI Gene, analyze protein impact with UniProt.
```

## Prerequisites

- **Python**: 3.11+ (3.12+ recommended)
- **uv**: Python package manager (required for installing skill dependencies)
- **Client**: Any agent that supports the Agent Skills standard (Cursor, Claude Code, Gemini CLI, Codex, etc.)
- **System**: macOS, Linux, or Windows with WSL2

## Security Notice

> **⚠️ Skills can execute code and influence your coding agent's behavior. Review what you install.**

Review the `SKILL.md` before installing. Only install skills you actually need. Run security scans on third-party skills:

```bash
uv pip install cisco-ai-skill-scanner
skill-scanner scan /path/to/skill --use-behavioral
```

## More Information

- **Repository**: https://github.com/K-Dense-AI/scientific-agent-skills
- **Skills Documentation**: https://agentskills.io/
- **License**: MIT

---

*This skill is maintained by K-Dense team. For issues or contributions, see the upstream repository.*
