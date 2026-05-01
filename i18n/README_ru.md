# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**Самоэволюционирующая исследовательская операционная система для исследователей ИИ**

[![Python](https://img.shields.io/badge/Python-3.9%2B-blue)](https://python.org)
[![PyPI Version](https://img.shields.io/pypi/v/ai-research-os)](https://pypi.org/project/ai-research-os/)
[![Coverage](https://img.shields.io/codecov/c/github/shushuzn/ai_research_os/main?logo=codecov)](https://app.codecov.io/gh/shushuzn/ai_research_os)
[![Tests](https://github.com/shushuzn/ai_research_os/actions/workflows/ci.yml/badge.svg?branch=main)](https://github.com/shushuzn/ai_research_os/actions)
[![License](https://img.shields.io/badge/License-GPL--3.0--or--later-orange)](#license)

<div align="center">
<a href="https://www.star-history.com/#shushuzn/ai_research_os&Date">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=shushuzn/ai_research_os&type=Date&theme=dark" />
   <source media="(prefers-color-scheme: light)" srcset="https://api.star-history.com/svg?repos=shushuzn/ai_research_os&type=Date" />
   <img alt="AI Research OS Star History" src="https://api.star-history.com/svg?repos=shushuzn/ai_research_os&type=Date" style="width: 80%; height: auto;" />
 </picture>
</a>
</div>

## Что это делает

AI Research OS — это **самоэволюционирующая исследовательская система**, которая учится на ваших паттернах использования. Это не просто менеджер статей — это исследовательский партнёр, который со временем становится умнее.

Отправьте статью (arXiv URL, DOI или PDF). Получите **P-Note**, **C-Note**, запись **Radar** и запись **Timeline** — всё структурированное, с тегами и перекрёстными ссылками.

| Вход | Выход |
|---|---|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| Локальный PDF | P-Note + C-Note + Radar + Timeline |
| Скан PDF | То же (через OCR) |

Это **не менеджер PDF**. Это **самоэволюционирующая система**, которая:
- Учится на ваших исследовательских паттернах
- Улучшает ответы со временем
- Адаптируется к вашей конкретной области

## Основные функции

| Функция | Описание |
|---------|-------------|
| `airos import` | Импорт статей из arXiv, DOI, PDF |
| `airos chat` | RAG-powered Q&A по вашим статьям |
| `airos slides` | Автоматическая генерация презентаций |
| `airos kg` | Визуализация графа знаний |
| Evolution | Самоулучшение через паттерны Gene/Capsule |

## Быстрый старт

```bash
pip install ai-research-os
airos-cli 2601.00155 --tags LLM,Agent
```

Готово — статья импортирована за секунды.

### Одна строка, три входа

```bash
airos-cli 2601.00155                          # arXiv ID
airos-cli 10.48550/arXiv.2601.00155           # DOI
airos-cli --pdf paper.pdf --tags RAG            # Локальный PDF
airos-cli --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # Скан PDF
```

### Три основных команды

```bash
airos-cli import 2601.00155 10.1038/nature12373   # Добавить статьи в БД
airos-cli search "attention mechanism" --tag LLM    # Поиск статей
airos-cli research "RLHF alignment" --limit 5       # Автономный исследовательский цикл
```

### AI-черновик (опционально)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
airos-cli 2601.00155 --tags LLM --ai
```

Полная конфигурация — см. [API_CONFIG.md](API_CONFIG.md).

## Исследовательское дерево

Статьи организованы в 12 каталогов:

```
00-Radar/            Отслеживание горячих тем
01-Foundations/      Базовые статьи
02-Models/           Статьи о моделях
03-Training/         Методы обучения
04-Scaling/          Законы масштабирования
05-Alignment/        Исследования по выравниванию
06-Agents/           Агентные системы
07-Infrastructure/   Инфраструктура
08-Optimization/    Методы оптимизации
09-Evaluation/       Методы оценки
10-Applications/    Прикладные исследования
11-Future-Directions/
```

## Установка

```bash
pip install ai-research-os
```

Или установить из исходников:

```bash
git clone https://github.com/shushuzn/ai_research_os.git
cd ai_research_os
pip install -e .
```

## Документация

Полная документация на [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/).

| Документ | Описание |
|-----|-------------|
| [Architecture](docs/architecture.md) | Архитектура системы и обзор модулей |
| [Configuration](docs/configuration.md) | Конфигурация LLM, БД, Поиска, Инструментов |
| [Benchmarks](docs/benchmarks.md) | Метрики производительности и покрытие тестами |
| [Contributing](CONTRIBUTING.md) | Как внести вклад в проект |
| [Roadmap](ROADMAP.md) | Дорожная карта и планы на будущее |

## Лицензия

GPL-3.0-or-later. Подробности см. [LICENSE](LICENSE).
