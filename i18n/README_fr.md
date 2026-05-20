# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**Un système d'exploitation de recherche auto-évolutif pour chercheurs en IA**

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

<div align="center">
<a href="https://www.star-history.com/#shushuzn/Rairos&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" />
   <img alt="AI Research OS Star History" src="https://api.star-history.com/svg?repos=shushuzn/Rairos&type=Date" style="width: 80%; height: auto;" />
 </picture>
</a>
</div>

## Ce qu'il fait

AI Research OS est un **système de recherche auto-évolutif** qui apprend de vos habitudes d'utilisation. Ce n'est pas un gestionnaire de PDFs — c'est un partenaire de recherche qui devient plus intelligent avec le temps.

Envoyez-lui un article (URL arXiv, DOI ou PDF). Obtenez un **P-Note**, **C-Note**, entrée **Radar** et entrée **Timeline** — tout structuré, tagué et croisé.

| Entrée | Sortie |
|---|---|
| URL/ID arXiv | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| PDF local | P-Note + C-Note + Radar + Timeline |
| PDF scanné | Identique (via OCR) |

Ceci **n'est pas un gestionnaire de PDF**. C'est un **Système Auto-Évolutif** qui :
- Apprend de vos habitudes de recherche
- Améliore ses réponses avec le temps
- S'adapte à votre domaine spécifique

## Fonctionnalités principales

| Fonctionnalité | Description |
|---------|-------------|
| `airos import` | Importer des articles depuis arXiv, DOI, PDF |
| `airos chat` | Q&R alimentées par RAG sur vos articles |
| `airos slides` | Générer des présentations automatiquement |
| `airos kg` | Visualisation de graphe de connaissances |
| Evolution | Auto-amélioration via motifs Gene/Capsule |

## Démarrage rapide

```bash
make build

# Run Rairos (from repo root)
cd Rairos
./rairos.sh 2601.00155 --tags LLM,Agent
```

C'est tout — un article importé en quelques secondes.

### Une ligne, trois entrées

```bash
./rairos.sh 2601.00155                          # ID arXiv
./rairos.sh 10.48550/arXiv.2601.00155           # DOI
./rairos.sh --pdf paper.pdf --tags RAG            # PDF local
./rairos.sh --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # PDF scanné
```

### Trois commandes principales

```bash
./rairos.sh import 2601.00155 10.1038/nature12373   # Ajouter des articles à la DB
./rairos.sh search "attention mechanism" --tag LLM    # Rechercher des articles
./rairos.sh research "RLHF alignment" --limit 5       # Boucle de recherche autonome
```

### Brouillon IA (optionnel)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
./rairos.sh 2601.00155 --tags LLM --ai
```

Pour la configuration complète, voir [docs/configuration.md](docs/configuration.md).

## Arbre de recherche

Les articles sont organisés en 12 répertoires :

```
00-Radar/            Suivi de la chaleur des sujets
01-Foundations/      Articles fondamentaux
02-Models/           Articles sur les modèles
03-Training/         Méthodes d'entraînement
04-Scaling/         Lois de mise à l'échelle
05-Alignment/        Recherche sur l'alignement
06-Agents/           Systèmes d'agents
07-Infrastructure/    Infrastructure
08-Optimization/     Techniques d'optimisation
09-Evaluation/       Méthodes d'évaluation
10-Applications/     Recherche appliquée
11-Future-Directions/
```

## Installation

```bash
make build
```

## Documentation

Documentation complète sur [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/).

| Doc | Description |
|-----|-------------|
| [Architecture](docs/architecture.md) | Conception du système et aperçu des modules |
| [Configuration](docs/configuration.md) | Configuration LLM, DB, Recherche, Outils |
| [Benchmarks](docs/benchmarks.md) | Métriques de performance et couverture des tests |
| [Contributing](CONTRIBUTING.md) | Comment contribuer à ce projet |
| [Roadmap](ROADMAP.md) | Feuille de route et projets futurs |

## Licence

GPL-3.0-or-later. Voir [LICENSE](LICENSE) pour les détails.
