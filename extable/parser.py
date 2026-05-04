"""LLM-based experiment table parser."""

import json
import re




class ExperimentTableParser:
    """Parse raw table data into structured JSON using LLM or regex fallback."""

    # Regex patterns for common metric formats
    _METRIC_PAT = re.compile(r"([\w\-\.]+)\s*[:=]\s*([\d\.]+)")
    _TABLE_NUM  = re.compile(r"^\s*[\d\.]+\s*$")

    def __init__(self, llm_client=None):
        """llm_client: optional OpenAI-compatible API client.

        If None, uses regex fallback mode.
        """
        self._llm = llm_client

    def parse_table_to_struct(
        self,
        table_data: list[list[str]],
        context_title: str = "",
        model: str = "qwen3.5-plus",
    ) -> dict:
        """Parse raw table data into structured dict.

        Returns: {
            "tables": [{
                "caption": str,
                "metrics": [{"name": str, "value": float}],
                "datasets": [str],
                "models": [str],
                "baselines": {model: value},
                "ours_best": {"value": float, "dataset": str, "metric": str}
            }]
        }
        """
        if self._llm is None:
            return self._regex_parse(table_data, context_title)

        try:
            return self._llm_parse(table_data, context_title, model)
        except Exception:
            return self._regex_parse(table_data, context_title)

    def _regex_parse(self, table_data: list[list[str]], title: str) -> dict:
        """Fallback: parse table with regex + heuristics."""
        tables = []
        if not table_data or len(table_data) < 2:
            return {"tables": []}

        header = [c.strip().lower() for c in table_data[0]]
        rows = table_data[1:]

        # Identify metric columns (numeric)
        metric_cols: list[tuple[int, str]] = []
        dataset_cols: list[tuple[int, str]] = []
        model_cols: list[tuple[int, str]] = []

        for i, h in enumerate(header):
            h_clean = h.strip()
            if not h_clean:
                continue
            if any(kw in h_clean for kw in ["accuracy", "precision", "recall", "f1", "bleu", "rouge", "ppl", "perplexity", "auc"]):
                metric_cols.append((i, h_clean))
            elif any(kw in h_clean for kw in ["dataset", "bench", "task", "corpus"]):
                dataset_cols.append((i, h_clean))
            elif any(kw in h_clean for kw in ["model", "method", "approach", "system"]):
                model_cols.append((i, h_clean))

        # If we have metrics but can't find columns, treat all numeric cols as metrics
        if not metric_cols:
            for _i, row in enumerate(rows[:3]):
                for j, cell in enumerate(row):
                    if self._TABLE_NUM.match(cell.strip()):
                        metric_cols.append((j, f"metric_{j}"))

        metrics: list[dict] = []
        datasets: set[str] = set()
        models: set[str] = set()
        baselines: dict[str, float] = {}
        ours_best_val: float = 0.0
        ours_best_dataset: str = ""
        ours_best_metric: str = ""

        # Detect "our method" row heuristically
        our_keywords = ["ours", "our", "proposed", "this", "method", "approach", "system"]
        is_our_row = False

        for _row_idx, row in enumerate(rows):
            needed_cols = [i for i, _ in metric_cols + dataset_cols + model_cols]
            if needed_cols and len(row) <= max(needed_cols):
                continue

            # Model name
            model_name = ""
            if model_cols:
                ci = model_cols[0][0]
                if ci < len(row):
                    model_name = row[ci].strip()
            elif row:
                model_name = row[0].strip()

            if model_name:
                models.add(model_name)
                row_text = " ".join(row).lower()
                is_our_row = any(kw in row_text for kw in our_keywords)

            # Dataset
            for di, _ in dataset_cols:
                if di < len(row):
                    datasets.add(row[di].strip())

            # Metrics
            for mi, mname in metric_cols:
                if mi < len(row):
                    raw = row[mi].strip()
                    num_match = re.search(r"[\d\.]+", raw)
                    if num_match:
                        val = float(num_match.group())
                        metrics.append({"name": mname, "value": val, "row": model_name})
                        if val > ours_best_val:
                            ours_best_val = val
                            ours_best_metric = mname
                            if dataset_cols:
                                di = dataset_cols[0][0]
                                if di < len(row):
                                    ours_best_dataset = row[di].strip()
                            ours_best_dataset = ours_best_dataset or title

                        if not is_our_row and model_name:
                            baselines[model_name] = val

        # Deduplicate metrics by name
        unique_metrics: dict[str, float] = {}
        for m in metrics:
            if m["name"] not in unique_metrics or m["value"] > unique_metrics[m["name"]]:
                unique_metrics[m["name"]] = m["value"]

        tables.append({
            "caption": title,
            "metrics": [{"name": k, "value": v} for k, v in unique_metrics.items()],
            "datasets": sorted(datasets),
            "models": sorted(models),
            "baselines": baselines,
            "ours_best": {
                "value": ours_best_val,
                "dataset": ours_best_dataset,
                "metric": ours_best_metric,
            },
        })

        return {"tables": tables}

    def _llm_parse(self, table_data: list[list[str]], title: str, model: str) -> dict:
        """Use LLM to parse table into structured format."""
        import os

        table_str = "\n".join(" | ".join(row) for row in table_data)
        prompt = (
            f"You are given a table from an AI research paper titled '{title}'.\n"
            f"Table:\n{table_str}\n\n"
            "Extract structured experiment results as JSON with fields:\n"
            "- caption: table caption/title\n"
            "- metrics: list of {name, value} for each metric column\n"
            "- datasets: list of dataset names found\n"
            "- models: list of model/method names\n"
            "- baselines: {model_name: best_score} for baseline methods\n"
            "- ours_best: {value, dataset, metric} for the best score of the proposed method\n"
            "Output ONLY valid JSON."
        )

        api_key = os.environ.get("OPENAI_API_KEY", "")
        base_url = os.environ.get("OPENAI_BASE_URL", "https://dashscope.aliyuncs.com/compatible-mode/v1")

        import urllib.request
        req = urllib.request.Request(
            f"{base_url}/chat/completions",
            data=json.dumps({
                "model": model,
                "messages": [{"role": "user", "content": prompt}],
                "temperature": 0.1,
            }).encode(),
            headers={
                "Authorization": f"Bearer {api_key}",
                "Content-Type": "application/json",
            },
            method="POST",
        )

        with urllib.request.urlopen(req, timeout=30) as resp:
            result = json.loads(resp.read().decode())
            content = result["choices"][0]["message"]["content"]
            # Strip markdown code fences
            content = content.strip()
            if content.startswith("```"):
                content = content.split("```")[1]
                if content.startswith("json"):
                    content = content[4:]
            return json.loads(content.strip())  # type: ignore[no-any-return]
