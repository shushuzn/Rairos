# AI Research OS

<div align="center">
  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>
</div>

**Um sistema operacional de pesquisa auto-evolutivo para pesquisadores de IA**

[![Build](https://github.com/shushuzn/Rairos/actions/workflows/rust.yml/badge.svg?branch=main)](https://github.com/shushuzn/Rairos/actions)
![Rust](https://img.shields.io/badge/rust-1.85+-orange.svg?logo=rust)
![Crates](https://img.shields.io/badge/crates-154-blue.svg)
![MCP](https://img.shields.io/badge/mcp_tools-69-blue.svg?logo=robot)
![CLI](https://img.shields.io/badge/cli_commands-105-blue.svg?logo=terminal)
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

## O que faz

AI Research OS é um **sistema de pesquisa auto-evolutivo** que aprende com seus padrões de uso. Não é apenas um gerenciador de artigos — é um parceiro de pesquisa que fica mais inteligente com o tempo.

Forneça um artigo (URL arXiv, DOI ou PDF). Obtenha um **P-Note**, **C-Note**, entrada **Radar** e entrada **Timeline** — tudo estruturado, etiquetado e cruzado.

| Entrada | Saída |
|---|---|
| URL/ID arXiv | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| PDF local | P-Note + C-Note + Radar + Timeline |
| PDF digitalizado | Igual (via OCR) |

Isto **não é um gerenciador de PDFs**. É um **Sistema Auto-Evolutivo** que:
- Aprende com seus padrões de pesquisa
- Melhora respostas com o tempo
- Adapta-se ao seu domínio específico

## Funcionalidades principais

| Funcionalidade | Descrição |
|---------|-------------|
| `./rairos.sh import` | Importar artigos de arXiv, DOI, PDF |
| `./rairos.sh chat` | Q&A com RAG sobre seus artigos |
| `./rairos.sh slides` | Gerar apresentações automaticamente |
| `./rairos.sh kg` | Visualização de grafo de conhecimento |
| Evolution | Auto-melhoria via padrões Gene/Capsule |

## Início rápido

```bash
make build

# Run Rairos (from repo root)
cd Rairos
./rairos.sh 2601.00155 --tags LLM,Agent
```

Pronto — um artigo importado em segundos.

### Uma linha, três entradas

```bash
./rairos.sh 2601.00155                          # ID arXiv
./rairos.sh 10.48550/arXiv.2601.00155           # DOI
./rairos.sh --pdf paper.pdf --tags RAG            # PDF local
./rairos.sh --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # PDF digitalizado
```

### Três comandos principais

```bash
./rairos.sh import 2601.00155 10.1038/nature12373   # Adicionar artigos ao DB
./rairos.sh search "attention mechanism" --tag LLM    # Pesquisar artigos
./rairos.sh research "RLHF alignment" --limit 5       # Loop de pesquisa autônomo
```

### Rascunho com IA (opcional)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
./rairos.sh 2601.00155 --tags LLM --ai
```

Para configuração completa, ver [docs/configuration.md](docs/configuration.md).

## Árvore de pesquisa

Os artigos são organizados em 12 diretórios:

```
00-Radar/            Rastreamento de calor do tema
01-Foundations/      Artigos fundamentais
02-Models/           Artigos de modelos
03-Training/         Métodos de treinamento
04-Scaling/          Leis de escala
05-Alignment/        Pesquisa de alinhamento
06-Agents/           Sistemas de agentes
07-Infrastructure/  Infraestrutura
08-Optimization/     Técnicas de otimização
09-Evaluation/       Métodos de avaliação
10-Applications/     Pesquisa aplicada
11-Future-Directions/
```

## Instalação

```bash
make build
```

## Documentação

Documentação completa em [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/).

| Doc | Descrição |
|-----|-------------|
| [Architecture](docs/architecture.md) | Design do sistema e visão geral dos módulos |
| [Configuration](docs/configuration.md) | Configuração de LLM, DB, Busca, Ferramentas |
| [Benchmarks](docs/benchmarks.md) | Métricas de desempenho e cobertura de testes |
| [Contributing](CONTRIBUTING.md) | Como contribuir para este projeto |
| [Roadmap](ROADMAP.md) | Roadmap do projeto e planos futuros |

## Licença

GPL-3.0-or-later. Ver [LICENSE](LICENSE) para detalhes.
