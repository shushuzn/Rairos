# API Reference

## Core Data Structures

### `Paper`

```python
class Paper:
    title: str
    authors: List[str]
    abstract: str
    published_date: str        # ISO date
    url: str                   # arXiv, DOI, or PDF path
    source: str               # "arxiv", "doi", "pdf"
    arxiv_id: Optional[str]
    doi: Optional[str]
```

Created via:
- `parsers.arxiv.fetch_arxiv_metadata(arxiv_id)` → `Paper`
- `parsers.crossref.fetch_crossref_metadata(doi)` → `Tuple[Paper, Optional[str]]`

## Parsers

### arXiv

```python
from parsers.arxiv import fetch_arxiv_metadata, fetch_arxiv_metadata_batch

paper = fetch_arxiv_metadata("2601.00155", timeout=30)
papers = fetch_arxiv_metadata_batch(["2601.00155", "2601.00156"])
```

### CrossRef (DOI)

```python
from parsers.crossref import fetch_crossref_metadata

paper, arxiv_id = fetch_crossref_metadata("10.1038/nature12373")
# arxiv_id may be str or None
```

### Input Detection

```python
from parsers.input_detection import is_probably_doi, normalize_doi, normalize_arxiv_id

is_probably_doi("10.1038/nature12373")  # True
normalize_doi("doi:10.1038/nature12373")  # "10.1038/nature12373"
normalize_arxiv_id("https://arxiv.org/abs/2601.00155")  # "2601.00155"
```

## PDF

### Text Extraction

```python
from pdf.extract import extract_pdf_text, extract_pdf_text_hybrid, extract_pdf_structured

text = extract_pdf_text(Path("paper.pdf"), max_pages=10)
text = extract_pdf_text_hybrid(Path("scanned.pdf"), zoom=2.0)
content = extract_pdf_structured(Path("paper.pdf"))
```

Returns `StructuredPdfContent` with `.text_blocks`, `.table_blocks`, `.math_blocks`.

### Download

```python
from pdf.extract import download_pdf

download_pdf("https://arxiv.org/pdf/2601.00155.pdf", Path("out.pdf"))
```

## Sections

### Segmentation

```python
from sections.segment import segment_into_sections, segment_structured, format_section_snippets

sections = segment_into_sections(text, max_sections=18)
# Returns List[Tuple[str, str]] of (section_title, section_text)

snippets = format_section_snippets(text, max_chars=3000)
```

### Formatting

```python
from sections.segment import format_tables_markdown, format_math_markdown

tables_md = format_tables_markdown(content, max_chars=3000)
math_md = format_math_markdown(content, max_count=5)
```

## LLM

### Client

```python
from llm.client import call_llm_chat_completions

response = call_llm_chat_completions(
    messages=[{"role": "user", "content": "Hello"}],
    model="gpt-4",
    api_key=os.environ.get("OPENAI_API_KEY"),
)
```

### AI Draft Generation

```python
from llm.generate import ai_generate_pnote_draft, ai_generate_cnote_draft

draft = ai_generate_pnote_draft(title, abstract, sections, tags, api_key)
cnote_draft = ai_generate_cnote_draft(title, abstract, tags, api_key)
```

### Parsing AI Output

```python
from llm.parse import parse_ai_pnote_draft, extract_rubric_scores

rubric, sections, comments = parse_ai_pnote_draft(raw_output)
scores = extract_rubric_scores(rubric)
```

## Renderers

```python
from renderers.pnote import render_pnote
from renderers.cnote import render_cnote
from renderers.mnote import render_mnote

md = render_pnote(paper, sections, tags, radar_yaml, timeline_yaml)
md = render_cnote(concept)
md = render_mnote(title, a, b, c)
```

## Updaters

```python
from updaters.radar import update_radar
from updaters.timeline import update_timeline

update_radar(radar_yaml, title, authors, date, url, tags)
update_timeline(timeline_yaml, title, date, url, tags)
```

## Core Utilities

```python
from core import Paper, today_iso
from core.basics import slugify_title, safe_uid, read_text, write_text, ensure_research_tree
from core.cache import get_cached, set_cached

slugify_title("My Paper Title: A Study")  # "my-paper-title"
safe_uid("10.1038/nature12373")           # unique ID string
ensure_research_tree(Path("./research"))  # creates dir structure
```

## Gene Pool

```python
from llm.gene_pool_io import (
    load_capsules,
    get_capsule_by_paper,
    paper_exists_in_pool,
    export_pool,
    import_pool,
    create_backup,
    restore_backup,
    get_backup_info,
    render_io_html,
)

# Load active capsules
capsules = load_capsules(gap_type="research_gap", status="active")

# Check if paper already in pool
exists = paper_exists_in_pool("2601.00155")

# Export/import Gene Pool
pool = export_pool()
import_pool(new_data, merge=True)

# Backup and restore
stamp = create_backup()
restore_backup(stamp)
```

## Deep Research Agent

```python
from research_loop.deep_research import DeepResearchAgent, AgentThought, DeepResearchResult

agent = DeepResearchAgent(
    model="qwen3.5-plus",
    max_iterations=10,
    verbose=True,
)

result: DeepResearchResult = agent.run(
    topic="RAG with agent tools in long context",
    papers=[paper1, paper2],
)
for thought in result.thoughts:
    print(thought.role, thought.content)
```

## Gap Clustering & Analysis

```python
from llm.research.gap_analyzer import GapClusterer, ConfidenceScorer

clusterer = GapClusterer()
clusters = clusterer.get_clusters(gaps)

scorer = ConfidenceScorer()
for gap in gaps:
    score = scorer.score(gap)
    # score.confidence: 0-1, score.is_high_quality: bool
```

## Topic Discovery

```python
from research_loop.topic_discovery import TopicDiscoverer

td = TopicDiscoverer()
suggestions = td.discover_topics(strategy="gap_cluster", top_k=5)
for s in suggestions:
    print(s.topic, s.reason, s.priority_score)
```

## Contradiction Timeline

```python
from llm.research.contradiction_timeline import ContradictionTimeline

timeline = ContradictionTimeline()
timeline.add_event(paper_id, claim_a, claim_b, polarity="contradiction")
shifts = timeline.detect_paradigm_shifts()
```

## Impact Tracking

```python
from llm.research.impact_tracker import GapImpactTracker

tracker = GapImpactTracker()
tracker.record_resolution(gap_id, resolution_confidence=0.8)
summary = tracker.get_summary(gap_id)
print(summary.impact_score)
```

## Observability

```python
from core.observability import get_logger, emit_research_event, setup_observability

setup_observability(level="INFO", json_logs=True)
log = get_logger("orchestrator")
log.info("gap_discovered", gap_type="method_limitation", novelty=0.85)

emit_research_event("paper_ingested", arxiv_id="2601.00155", gap_count=3)
```

## Webhook Notifications

```python
from notifications.dispatcher import WebhookDispatcher, DiscordEmbed, GapCard

dispatcher = WebhookDispatcher()
dispatcher.send_gap_alert(gap, confidence=0.82, webhook_url=discord_url)
dispatcher.send_paradigm_shift_alert(shifts, webhook_url=feishu_url)
```

## Circuit Breaker

```python
from core.retry import circuit_breaker, CircuitBreaker, CircuitOpen

# Decorator form (shared across calls by function key)
@circuit_breaker(failure_threshold=3, recovery_timeout=60.0)
def call_api():
    return requests.get(url)

# Direct form
cb = CircuitBreaker(failure_threshold=2, recovery_timeout=30.0)
cb.record_failure()
cb.record_failure()
assert cb.state == CircuitBreaker.OPEN

# Thread-safe, each decorated function gets its own breaker
```

## Retry

```python
from core.retry import retry, RetryStats, get_retry_stats

@retry(max_attempts=3, base_delay=1.0, max_delay=30.0)
def flaky_call():
    return api.request()

stats = get_retry_stats()
stats.record_attempt("flaky_call", attempt=2, success=True)
```
