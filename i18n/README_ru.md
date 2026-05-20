# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**Самоэволюционирующая исследовательская операционная система для исследователей ИИ**

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
make build

# Run Rairos (from repo root)
cd Rairos
rairos 2601.00155 --tags LLM,Agent
```

Готово — статья импортирована за секунды.

### Одна строка, три входа

```bash
rairos 2601.00155                          # arXiv ID
rairos 10.48550/arXiv.2601.00155           # DOI
rairos --pdf paper.pdf --tags RAG            # Локальный PDF
rairos --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # Скан PDF
```

### Три основных команды

```bash
rairos import 2601.00155 10.1038/nature12373   # Добавить статьи в БД
rairos search "attention mechanism" --tag LLM    # Поиск статей
rairos research "RLHF alignment" --limit 5       # Автономный исследовательский цикл
```

### AI-черновик (опционально)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
rairos 2601.00155 --tags LLM --ai
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
make build
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
