# Installation

## Requirements

- Python 3.9+
- pip

## Core Dependencies

```bash
pip install requests feedparser pymupdf
```

## Optional Dependencies

For scanned PDFs (OCR support):

```bash
pip install pymupdf  # already included
```

For AI-assisted drafting:

```bash
pip install openai  # or your preferred OpenAI-compatible client
```

## API Keys

Set your API key as an environment variable:

```bash
export OPENAI_API_KEY="sk-..."
```

Or pass it via command line (not recommended for security):

```bash
python ai_research_os.py ... --api-key "sk-..."
```

## Verification

```bash
python ai_research_os.py --help
```
