# AI Research OS

<div align="center">
  <img src="logo_hero.svg" width="900" alt="Rairos Demo"/>
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
| `./rairos.sh import` | Importar artículos desde arXiv, DOI, PDF |
| `./rairos.sh chat` | Q&A con RAG sobre tus artículos |
| `./rairos.sh slides` | Generar presentaciones automáticamente |
| `./rairos.sh kg` | Visualización de grafos de conocimiento |
| Evolution | Auto-mejora vía patrones Gene/Capsule |

## Inicio rápido

```bash
make build

# Run Rairos (from repo root)
cd Rairos
./rairos.sh 2601.00155 --tags LLM,Agent
```

Listo — un artículo importado en segundos.

### Una línea, tres entradas

```bash
./rairos.sh 2601.00155                          # ID de arXiv
./rairos.sh 10.48550/arXiv.2601.00155           # DOI
./rairos.sh --pdf paper.pdf --tags RAG            # PDF local
./rairos.sh --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # PDF escaneado
```

### Tres comandos principales

```bash
./rairos.sh import 2601.00155 10.1038/nature12373   # Añadir artículos a la DB
./rairos.sh search "attention mechanism" --tag LLM    # Buscar artículos
./rairos.sh research "RLHF alignment" --limit 5       # Bucle de investigación autónomo
```

### Borrador con IA (opcional)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
./rairos.sh 2601.00155 --tags LLM --ai
```

Para configuración completa, ver [docs/configuration.md](docs/configuration.md).

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
make build
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
