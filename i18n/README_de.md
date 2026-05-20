# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**Ein selbst-evolvierendes Forschungsbetriebssystem für KI-Forscher**

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

## Was es macht

AI Research OS ist ein **selbst-evolvierendes Forschungssystem**, das aus Ihrem Nutzungsverhalten lernt. Es ist kein einfacher PDF-Manager — es ist ein Forschungspartner, der mit der Zeit intelligenter wird.

Geben Sie ein Paper (arXiv-URL, DOI oder PDF) ein. Sie erhalten einen **P-Note**, **C-Note**, **Radar-Eintrag** und **Timeline-Eintrag** — alles strukturiert, getaggt und vernetzt.

| Eingabe | Ausgabe |
|---|---|
| arXiv-URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Lokale PDF | P-Note + C-Note + Radar + Timeline |
| Gescannte PDF | Gleich (via OCR) |

Dies ist **kein PDF-Manager**. Es ist ein **selbst-evolvierendes System**, das:
- Aus Ihren Forschungsmustern lernt
- Antworten mit der Zeit verbessert
- Sich an Ihre spezifische Domäne anpasst

## Hauptfunktionen

| Funktion | Beschreibung |
|---------|-------------|
| `airos import` | Papers von arXiv, DOI, PDF importieren |
| `airos chat` | RAG-gestützte Q&A mit Ihren Papers |
| `airos slides` | Präsentationen automatisch generieren |
| `airos kg` | Wissensgraph-Visualisierung |
| Evolution | Selbstverbesserung via Gene/Capsule-Muster |

## Schnellstart

```bash
make build

# Run Rairos (from repo root)
cd Rairos
./rairos.sh 2601.00155 --tags LLM,Agent
```

Fertig — ein Paper in Sekunden importiert.

### Eine Zeile, drei Eingaben

```bash
./rairos.sh 2601.00155                          # arXiv-ID
./rairos.sh 10.48550/arXiv.2601.00155           # DOI
./rairos.sh --pdf paper.pdf --tags RAG            # Lokale PDF
./rairos.sh --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # Gescannte PDF
```

### Drei Kernbefehle

```bash
./rairos.sh import 2601.00155 10.1038/nature12373   # Papers zur DB hinzufügen
./rairos.sh search "attention mechanism" --tag LLM    # Papers durchsuchen
./rairos.sh research "RLHF alignment" --limit 5       # Autonome Forschungsschleife
```

### KI-Entwurf (optional)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
./rairos.sh 2601.00155 --tags LLM --ai
```

Für die vollständige Konfiguration, see [docs/configuration.md](docs/configuration.md).

## Forschungbaum

Papers sind in 12 Verzeichnisse organisiert:

```
00-Radar/            Themen-Hitze-Tracking
01-Foundations/      Grundlagenpapiere
02-Models/           Modellpapiere
03-Training/         Trainingsmethoden
04-Scaling/          Skalierungsgesetze
05-Alignment/        Alignementforschung
06-Agents/           Agentensysteme
07-Infrastructure/  Infrastruktur
08-Optimization/     Optimierungstechniken
09-Evaluation/       Evaluationsmethoden
10-Applications/    Angewandte Forschung
11-Future-Directions/
```

## Installation

```bash
make build
```

## Dokumentation

Vollständige Dokumentation unter [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/).

| Dokumentation | Beschreibung |
|-----|-------------|
| [Architecture](docs/architecture.md) | Systemdesign und Modulübersicht |
| [Configuration](docs/configuration.md) | LLM-, DB-, Such- und Tool-Konfiguration |
| [Benchmarks](docs/benchmarks.md) | Leistungskennzahlen und Testabdeckung |
| [Contributing](CONTRIBUTING.md) | Wie Sie zu diesem Projekt beitragen können |
| [Roadmap](ROADMAP.md) | Projekt-Roadmap und Zukunftspläne |

## Lizenz

GPL-3.0-or-later. Siehe [LICENSE](LICENSE) für Details.
