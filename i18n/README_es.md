# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**Un sistema operativo de investigación auto-evolutivo para investigadores de IA**

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

## Qué hace

AI Research OS es un **sistema de investigación auto-evolutivo** que aprende de tus patrones de uso. No es solo un gestor de artículos — es un compañero de investigación que se vuelve más inteligente con el tiempo.

Envíale un artículo (URL de arXiv, DOI o PDF). Obtén un **P-Note**, **C-Note**, entrada de **Radar** y entrada de **Timeline** — todo estructurado, etiquetado y entrelazado.

| Entrada | Salida |
|---|---|
| URL/ID de arXiv | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| PDF local | P-Note + C-Note + Radar + Timeline |
| PDF escaneado | Igual (vía OCR) |

Esto **no es un gestor de PDFs**. Es un **Sistema Auto-Evolutivo** que:
- Aprende de tus patrones de investigación
- Mejora las respuestas con el tiempo
- Se adapta a tu dominio específico

## Características principales

| Función | Descripción |
|---------|-------------|
| `airos import` | Importar artículos desde arXiv, DOI, PDF |
| `airos chat` | Q&A con RAG sobre tus artículos |
| `airos slides` | Generar presentaciones automáticamente |
| `airos kg` | Visualización de grafos de conocimiento |
| Evolution | Auto-mejora vía patrones Gene/Capsule |

## Inicio rápido

```bash
pip install ai-research-os
airos-cli 2601.00155 --tags LLM,Agent
```

Listo — un artículo importado en segundos.

### Una línea, tres entradas

```bash
airos-cli 2601.00155                          # ID de arXiv
airos-cli 10.48550/arXiv.2601.00155           # DOI
airos-cli --pdf paper.pdf --tags RAG            # PDF local
airos-cli --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # PDF escaneado
```

### Tres comandos principales

```bash
airos-cli import 2601.00155 10.1038/nature12373   # Añadir artículos a la DB
airos-cli search "attention mechanism" --tag LLM    # Buscar artículos
airos-cli research "RLHF alignment" --limit 5       # Bucle de investigación autónomo
```

### Borrador con IA (opcional)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
airos-cli 2601.00155 --tags LLM --ai
```

Para configuración completa, ver [API_CONFIG.md](API_CONFIG.md).

## Árbol de investigación

Los artículos se organizan en 12 directorios:

```
00-Radar/            Seguimiento de calor de tema
01-Foundations/      Artículos fundamentales
02-Models/           Artículos de modelos
03-Training/         Métodos de entrenamiento
04-Scaling/          Leyes de escala
05-Alignment/        Investigación de alineación
06-Agents/           Sistemas de agentes
07-Infrastructure/   Infraestructura
08-Optimization/     Técnicas de optimización
09-Evaluation/       Métodos de evaluación
10-Applications/     Investigación aplicada
11-Future-Directions/
```

## Instalación

```bash
pip install ai-research-os
```

O instalar desde código fuente:

```bash
git clone https://github.com/shushuzn/Rairos.git
cd ai_research_os
pip install -e .
```

## Documentación

Documentación completa en [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/).

| Doc | Descripción |
|-----|-------------|
| [Architecture](docs/architecture.md) | Diseño del sistema y descripción de módulos |
| [Configuration](docs/configuration.md) | Configuración de LLM, DB, Búsqueda, Herramientas |
| [Benchmarks](docs/benchmarks.md) | Métricas de rendimiento y cobertura de tests |
| [Contributing](CONTRIBUTING.md) | Cómo contribuir a este proyecto |
| [Roadmap](ROADMAP.md) | Hoja de ruta y planes futuros |

## Licencia

GPL-3.0-or-later. Ver [LICENSE](LICENSE) para detalles.
