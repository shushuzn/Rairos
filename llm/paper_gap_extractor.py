"""Extract research gaps from a paper using LLM analysis."""

from __future__ import annotations

import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL


def extract_gap_from_paper(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    base_url: Optional[str] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Extract research gap from a paper.

    Returns a dict with:
        gap_type: one of the GapType enum values
        gap_title: short title for the gap
        keywords: list of extracted keywords
        summary: brief explanation of what gap this paper addresses
    """
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    prompt = f"""You are a research gap detector. Given a paper's title and abstract, extract ONE research gap that this paper addresses.

PAPER:
Title: {title}
Authors: {authors_str}
Abstract: {abstract[:800]}

TASK:
Identify what research gap this paper addresses. Choose ONE gap type from:
- unexplored_application: using known methods in new domains
- method_limitation: current methods have specific drawbacks
- contradiction: paper challenges or refutes existing findings
- evaluation_gap: lack of proper benchmarks or evaluation
- scalability_issue: methods don't scale to real-world settings
- theoretical_gap: lack of theoretical foundations
- dataset_gap: no suitable dataset for the problem
- generalization_gap: methods fail on out-of-distribution data

Respond ONLY with a JSON object (no markdown, no code fences):
{{
  "gap_type": "the_gap_type_here",
  "gap_title": "Short descriptive title (max 80 chars)",
  "keywords": ["keyword1", "keyword2", "keyword3"],
  "summary": "One sentence explaining what gap this paper addresses",
  "polarity": "positive or negative — does this paper ADVANCE the gap (positive: proposes new method/scale/improve) or FAIL/challenge it (negative: shows limitation, refutes, or fails to scale)"
}}
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": "Gap extraction requires llm.client.",
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model="claude-3-5-sonnet-latest",
        )
        return json.loads(content.strip())
    except Exception as e:
        return {
            "gap_type": "method_limitation",
            "gap_title": title[:80],
            "keywords": _extract_keywords(abstract),
            "summary": f"LLM call failed: {e}",
            "error": str(e),
        }


def analyze_embodied_planning(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze embodied planning paper: discrete vs continuous latent representation.

    Returns:
        representation_type: "discrete" | "continuous" | "hybrid"
        confidence: float (0-1)
        evidence: list of text snippets from abstract supporting the classification
        gap_title: research question derived from the finding
        summary: 1-2 sentence analysis
    """
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    prompt = f"""You are a research analyst specializing in robotics and embodied AI.
Given a paper's title and abstract, determine how the paper represents physical reasoning (latent reasoning over physical dynamics).

PAPER:
Title: {title}
Authors: {authors_str}
Abstract: {abstract[:800]}

TASK:
Answer these questions about the paper's latent representation approach:

1. Does the paper use DISCRETE latent representations?
   (e.g., discrete tokens, symbolic, quantized, language-like discrete states)
   Look for: "discrete", "symbolic", "token", "quantized", "categorical", "language"

2. Does the paper use CONTINUOUS latent representations?
   (e.g., continuous vectors, real-valued embeddings, diffusion, Gaussian, continuous distributions)
   Look for: "continuous", "diffusion", "Gaussian", "real-valued", "embedding", "vector"

3. Is it HYBRID (both)?
   (e.g., discrete reasoning + continuous execution, world model tokens + action distributions)

4. What is the KEY EVIDENCE from the abstract?

5. What OPEN QUESTION remains about this representation choice?

Provide your analysis in this format:
representation_type: [discrete|continuous|hybrid]
confidence: [0.0-1.0]
evidence: [key phrases from abstract]
gap_title: [specific research question about this representation choice]
summary: [2-sentence analysis]"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "representation_type": "unknown",
            "confidence": 0.0,
            "evidence": [],
            "gap_title": title[:80],
            "summary": "LLM client not available for embodied planning analysis.",
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model=model or "claude-3-5-sonnet-latest",
            api_key=api_key,
        )

        lines = content.strip().split("\n")
        result = {
            "representation_type": "unknown",
            "confidence": 0.5,
            "evidence": [],
            "gap_title": "",
            "summary": "",
        }
        for line in lines:
            if line.startswith("representation_type:"):
                val = line.split(":", 1)[1].strip().lower()
                if val in ("discrete", "continuous", "hybrid"):
                    result["representation_type"] = val
            elif line.startswith("confidence:"):
                try:
                    result["confidence"] = float(line.split(":", 1)[1].strip())
                except ValueError:
                    pass
            elif line.startswith("evidence:"):
                result["evidence"] = [line.split(":", 1)[1].strip()]
            elif line.startswith("gap_title:"):
                result["gap_title"] = line.split(":", 1)[1].strip()
            elif line.startswith("summary:"):
                result["summary"] = line.split(":", 1)[1].strip()

        # Save to Gene Pool
        gap_type = "embodied_planning"
        polarity = "open"
        capsule_id = save_gap_to_gene_pool(
            paper_id=paper_id,
            title=title,
            gap_type=gap_type,
            gap_title=result.get("gap_title") or f"Latent Reasoning Representation: {result['representation_type']}",
            keywords=[result["representation_type"], "embodied", "latent", "reasoning", "VLA"],
            summary=result.get("summary") or f"{result['representation_type'].capitalize()} latent reasoning paper. Confidence: {result['confidence']:.0%}.",
            polarity=polarity,
            extra_fields={
                "representation_type": result["representation_type"],
                "confidence": result["confidence"],
                "evidence": result.get("evidence", []),
            },
        )

        # Contradiction detection: check existing capsules of same gap_type
        result["contradiction_with"] = None
        result["contradiction_type"] = None
        if capsule_id:
            from .gene import _read_capsules_json
            all_capsules = _read_capsules_json()
            existing = [
                c for c in all_capsules
                if c.get("action_gap_type") == "embodied_planning"
                and c.get("archetype", {}).get("source_paper_id") != paper_id
                and c.get("status") != "archived"
            ]
            for ex in existing:
                ex_rep = ex.get("archetype", {}).get("representation_type", "unknown")
                if ex_rep != result["representation_type"] and ex_rep != "unknown":
                    result["contradiction_with"] = ex.get("archetype", {}).get("source_paper_id")
                    result["contradiction_type"] = ex_rep
                    break

        return result

    except Exception as e:
        return {
            "representation_type": "unknown",
            "confidence": 0.0,
            "evidence": [],
            "gap_title": title[:80],
            "summary": f"Analysis failed: {e}",
            "error": str(e),
        }


def batch_analyze_embodied_planning(
    paper_ids: List[str],
    db=None,
) -> Dict[str, Any]:
    """Batch analyze multiple papers for embodied planning representation types.

    Returns comparative report with discrete/continuous/hybrid grouping.
    """
    if db is None:
        from db.database import Database
        db = Database()

    results = []
    for pid in paper_ids:
        paper = db.get_paper(pid)
        if not paper:
            continue
        r = analyze_embodied_planning(
            paper_id=pid,
            title=paper.title,
            abstract=paper.abstract or "",
            authors=paper.authors,
        )
        r["paper_id"] = pid
        r["paper_title"] = paper.title[:80]
        results.append(r)

    # Group by representation type
    by_type: Dict[str, List] = {"discrete": [], "continuous": [], "hybrid": [], "unknown": []}
    for r in results:
        rt = r.get("representation_type", "unknown")
        if rt in by_type:
            by_type[rt].append(r)

    # Sort each group by confidence
    for rt in by_type:
        by_type[rt].sort(key=lambda x: x.get("confidence", 0), reverse=True)

    # Summary stats
    total = len(results)
    summary = {
        "total": total,
        "discrete_count": len(by_type["discrete"]),
        "continuous_count": len(by_type["continuous"]),
        "hybrid_count": len(by_type["hybrid"]),
        "unknown_count": len(by_type["unknown"]),
        "dominant_type": max(by_type, key=lambda k: len(by_type[k])) if total > 0 else "unknown",
    }


    # Contradiction pairs: discrete vs continuous for same tasks
    contradictions = []
    discrete_set = {(r.get("paper_id"), r.get("paper_title","").lower()) for r in by_type["discrete"]}
    continuous_set = {(r.get("paper_id"), r.get("paper_title","").lower()) for r in by_type["continuous"]}
    # Find papers that appear to study similar tasks but use different representations
    for dc in by_type["discrete"]:
        for cc in by_type["continuous"]:
            # Simple keyword overlap check on evidence
            dc_ev = " ".join(dc.get("evidence", [])).lower()
            cc_ev = " ".join(cc.get("evidence", [])).lower()
            if any(w in cc_ev for w in dc_ev.split() if len(w) > 4):
                contradictions.append({
                    "discrete_paper": dc.get("paper_id"),
                    "discrete_title": dc.get("paper_title"),
                    "continuous_paper": cc.get("paper_id"),
                    "continuous_title": cc.get("paper_title"),
                    "evidence_overlap": True,
                })

    return {
        "summary": summary,
        "by_type": {k: [{"paper_id": r["paper_id"], "paper_title": r["paper_title"],
                          "confidence": r["confidence"], "gap_title": r.get("gap_title", ""),
                          "evidence": r.get("evidence", [])}
                         for r in v]
                     for k, v in by_type.items()},
        "contradictions": contradictions[:5],
        "total_analyzed": total,
    }


def render_embodied_planning_dashboard() -> str:
    """Render HTML dashboard of all embodied planning analyses from Gene Pool.

    Shows domain-wide representation type distribution, confidence ranking,
    and contradiction pairs.
    """
    import json as _json

    # Read from capsules.json
    capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    if not capsule_path.exists():
        return _empty_dashboard("No Gene Pool data found.")

    data = _json.loads(capsule_path.read_text(encoding="utf-8"))
    capsules = data.get("capsules", [])

    # Filter embodied_planning gap types
    embodied = [
        c for c in capsules
        if c.get("action_gap_type") == "embodied_planning"
        or c.get("trigger_gap_type") == "embodied_planning"
    ]

    if not embodied:
        return _empty_dashboard(
            "No embodied planning analyses yet. "
            "Open a VLA/robotics paper and click 🦾 Embodied Planning."
        )

    # Group by representation type from gap_title keywords
    discrete = []
    continuous = []
    hybrid = []
    unknown = []

    for c in embodied:
        title = c.get("action_gap_title", "").lower()
        summary = c.get("archetype", {}).get("summary", "").lower()
        combined = title + " " + summary
        if "hybrid" in combined or ("discrete" in combined and "continuous" in combined):
            hybrid.append(c)
        elif "discrete" in combined:
            discrete.append(c)
        elif "continuous" in combined:
            continuous.append(c)
        else:
            unknown.append(c)

    # Sort by confidence
    for lst in [discrete, continuous, hybrid, unknown]:
        lst.sort(key=lambda x: x.get("outcome_success_score", 0.5), reverse=True)

    html = """
    <div style="font-family: var(--font-display);">
      <div style="display:grid;grid-template-columns:1fr 1fr 1fr 1fr;gap:12px;margin-bottom:24px;">
        <div style="text-align:center;padding:16px;background:#f8faf8;border-radius:8px;border-left:4px solid #7A9E7A;">
          <div style="font-size:32px;font-weight:700;color:#7A9E7A;">{dc}</div>
          <div style="font-size:12px;color:#888;text-transform:uppercase;letter-spacing:1px;">Discrete</div>
        </div>
        <div style="text-align:center;padding:16px;background:#f8fafc;border-radius:8px;border-left:4px solid #6B8FB5;">
          <div style="font-size:32px;font-weight:700;color:#6B8FB5;">{cc}</div>
          <div style="font-size:12px;color:#888;text-transform:uppercase;letter-spacing:1px;">Continuous</div>
        </div>
        <div style="text-align:center;padding:16px;background:#fafaf4;border-radius:8px;border-left:4px solid #D4A84B;">
          <div style="font-size:32px;font-weight:700;color:#D4A84B;">{hc}</div>
          <div style="font-size:12px;color:#888;text-transform:uppercase;letter-spacing:1px;">Hybrid</div>
        </div>
        <div style="text-align:center;padding:16px;background:#f5f5f5;border-radius:8px;border-left:4px solid #aaa;">
          <div style="font-size:32px;font-weight:700;color:#888;">{uc}</div>
          <div style="font-size:12px;color:#888;text-transform:uppercase;letter-spacing:1px;">Unclear</div>
        </div>
      </div>
    """.format(
        dc=len(discrete), cc=len(continuous),
        hc=len(hybrid), uc=len(unknown)
    )

    # Mermaid graph
    graph = render_embodied_planning_graph()
    if graph:
        html += """
    <div style="margin-bottom:24px;">
      <div style="font-size:14px;font-weight:700;color:#555;margin-bottom:12px;">🕸️ Representation Atlas</div>
      <div style="background:#fafaf7;border:1px solid #e0dbd4;border-radius:8px;padding:16px;overflow-x:auto;">
        """ + graph + """
      </div>
    </div>"""

    # Papers list by type
    def _render_list(capsules, color, type_label):
        if not capsules:
            return ""
        items = ""
        for c in capsules:
            score = c.get("outcome_success_score", 0.5)
            title = c.get("action_gap_title", "Untitled")[:70]
            paper_id = c.get("archetype", {}).get("source_paper_id", "")
            items += """
            <div style="display:flex;justify-content:space-between;align-items:center;padding:8px 0;border-bottom:1px solid #f0ebe5;">
              <div style="flex:1;min-width:0;">
                <div style="font-size:13px;color:#2a2a2a;margin-bottom:2px;">{title}</div>
                <div style="font-size:11px;color:#aaa;">{paper_id}</div>
              </div>
              <div style="font-size:11px;color:{color};font-weight:700;margin-left:8px;">{score:.0%}</div>
            </div>""".format(
                title=title, paper_id=paper_id[:20],
                color=color, score=score
            )
        return """
        <div style="margin-bottom:20px;">
          <div style="font-size:14px;font-weight:700;color:{color};margin-bottom:8px;padding-bottom:6px;border-bottom:2px solid {color};">{label} ({count})</div>
          {items}
        </div>""".format(color=color, label=type_label, count=len(capsules), items=items)

    html += _render_list(discrete, "#7A9E7A", "Discrete Representation")
    html += _render_list(continuous, "#6B8FB5", "Continuous Representation")
    html += _render_list(hybrid, "#D4A84B", "Hybrid Representation")
    html += _render_list(unknown, "#aaa", "Unclear / Unanalyzed")

    html += "</div>"
    return html


def _empty_dashboard(msg: str) -> str:
    return """
    <div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
      <div style="font-size:48px;margin-bottom:12px;">🦾</div>
      <div style="font-size:15px;font-weight:600;margin-bottom:6px;">No Embodied Planning Data</div>
      <div style="font-size:13px;">{msg}</div>
    </div>
    """.format(msg=msg)


def render_embodied_planning_graph() -> str:
    """Render embodied planning analyses as a Mermaid graph.

    Nodes = papers colored by representation type (discrete/continuous/hybrid).
    Edges = contradictory pairs (same gap, different conclusion).
    Node size reflects confidence.
    """
    import json as _json

    capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    if not capsule_path.exists():
        return ""

    data = _json.loads(capsule_path.read_text(encoding="utf-8"))
    capsules = data.get("capsules", [])

    embodied = [
        c for c in capsules
        if c.get("action_gap_type") == "embodied_planning"
    ]

    if len(embodied) < 1:
        return ""

    # Build node id -> type mapping
    type_colors = {"discrete": "#7A9E7A", "continuous": "#6B8FB5", "hybrid": "#D4A84B"}
    nodes_info: Dict[str, tuple] = {}  # paper_id -> (type, title, confidence)

    for c in embodied:
        pid = c.get("archetype", {}).get("source_paper_id", "")
        title = c.get("action_gap_title", "Untitled")[:40]
        conf = c.get("outcome_success_score", 0.5)
        gap_title = c.get("action_gap_title", "").lower()
        combined = gap_title + " " + c.get("archetype", {}).get("summary", "").lower()
        if "hybrid" in combined or ("discrete" in combined and "continuous" in combined):
            rtype = "hybrid"
        elif "discrete" in combined:
            rtype = "discrete"
        elif "continuous" in combined:
            rtype = "continuous"
        else:
            rtype = "unknown"
        nodes_info[pid] = (rtype, title, conf)

    # Find contradiction edges: same paper mentions both types
    edges = set()
    pids = list(nodes_info.keys())
    for i, p1 in enumerate(pids):
        for p2 in pids[i+1:]:
            if p1 == p2:
                continue
            r1, title1, _ = nodes_info[p1]
            r2, title2, _ = nodes_info[p2]
            if r1 != r2 and r1 != "unknown" and r2 != "unknown":
                # Cross-type edge = potential contradiction
                edges.add((p1[:12], p2[:12], r1, r2))

    # Build Mermaid
    lines = ["```mermaid", "flowchart TD"]
    lines.append("    %% Nodes: papers by representation type")

    for pid, (rtype, title, conf) in nodes_info.items():
        short_id = pid[:12]
        color = type_colors.get(rtype, "#aaa")
        conf_pct = int(conf * 100)
        label = f"{title} {conf_pct}%"
        if rtype == "discrete":
            line = f"    {short_id}(({label}))"
        elif rtype == "continuous":
            line = f"    {short_id}(({label}))"
        elif rtype == "hybrid":
            line = f"    {short_id}{{{{{label}}}}}"
        else:
            line = f"    {short_id}(({label}))"
        lines.append(line)

    lines.append("    %% Edges: cross-type contradictions")
    for e1, e2, r1, r2 in sorted(edges):
        color = {"discrete": "stroke:#7A9E7A", "continuous": "stroke:#6B8FB5", "hybrid": "stroke:#D4A84B"}.get(r1, "")
        lines.append(f"    {e1} -.->|{r1}/{r2}| {e2} {color}")

    lines.append("```")
    return "\n".join(lines)


def _extract_keywords(text: str) -> List[str]:
    """Simple keyword extraction from text."""
    words = text.lower().split()
    stop = {"the", "a", "an", "of", "in", "to", "for", "and", "or", "with", "is", "are", "that", "this", "we", "our", "by", "on", "as", "at", "from"}
    keywords = [w.strip(".,;:!?()[]{}") for w in words if len(w) > 4 and w not in stop]
    return list(dict.fromkeys(keywords))[:6]


def _read_capsules_json() -> List[Dict[str, Any]]:
    """Read capsules from the Gene Pool JSON store."""
    capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    if not capsule_path.exists():
        return []
    try:
        return json.loads(capsule_path.read_text(encoding="utf-8")).get("capsules", [])
    except Exception:
        return []


def save_gap_to_gene_pool(
    paper_id: str,
    title: str,
    gap_type: str,
    gap_title: str,
    keywords: List[str],
    summary: str,
    polarity: str = "positive",
    extra_fields: Optional[Dict[str, Any]] = None,
) -> Optional[str]:
    """Append a new gap as a CapsuleGene entry to both Gene Pool stores.

    Writes to:
    - ~/.ai_research_os/gene_pool/capsules.json   (read by _match_gene_pool / briefing_generator)
    - ~/.ai_research_os/evolution/gene_pool.jsonl (read by find_capsule / Curator / EvolutionTracker)

    Returns capsule_id on success, None on failure.
    """
    try:
        capsule_id = f"extracted_{paper_id}_{uuid.uuid4().hex[:8]}"
        now = datetime.now().isoformat()
        archetype = {
            "extracted_from": "paper_gap_extractor",
            "source_paper_id": paper_id,
            "summary": summary,
        }
        if extra_fields:
            archetype.update(extra_fields)
        capsule = {
            "capsule_id": capsule_id,
            "created_at": now,
            "trigger_topic": title[:200],
            "trigger_gap_type": gap_type,
            "trigger_keywords": keywords,
            "action_gap_type": gap_type,
            "action_gap_title": gap_title[:200],
            "outcome_success_score": 0.5,
            "feedback_count": 0,
            "evolved_generation": 0,
            "polarity": polarity,
            "archetype": archetype,
            "status": "active",
        }

        # Write to capsules.json (briefing_generator reads this)
        capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
        capsule_path.parent.mkdir(parents=True, exist_ok=True)
        if capsule_path.exists():
            data = json.loads(capsule_path.read_text(encoding="utf-8"))
        else:
            data = {"version": 1, "capsules": []}
        data["capsules"].append(capsule)
        capsule_path.write_text(json.dumps(data, indent=2, ensure_ascii=False), encoding="utf-8")

        # Write to gene_pool.jsonl (EvolutionTracker.find_capsule reads this)
        try:
            from llm.insight.gene import CapsuleGene
            from llm.insight.tracker import EvolutionTracker
            tracker = EvolutionTracker()
            tracker.encode_capsule(
                topic=title[:200],
                gap_type=gap_type,
                gap_title=gap_title[:200],
                gap_description=summary,
                success_score=0.5,
                status="active",
            )
        except Exception:
            # Non-critical: capsules.json write succeeded, gene_pool.jsonl is optional for CLI use
            pass

        return capsule_id
    except Exception:
        return None


def detect_contradictions(capsules: list) -> list:
    """Find pairs of capsules with same gap_type but opposite polarity.

    Returns list of dicts:
        {gap_type, positive_capsule, negative_capsule, shared_keywords}
    """
    from collections import defaultdict

    by_type = defaultdict(list)
    for c in capsules:
        if c.get("status") == "archived":
            continue
        polarity = c.get("polarity", "positive")
        gap_type = c.get("action_gap_type") or c.get("trigger_gap_type", "")
        if gap_type and polarity:
            by_type[gap_type].append((polarity, c))

    contradictions = []
    for gap_type, items in by_type.items():
        positives = [c for p, c in items if p == "positive"]
        negatives = [c for p, c in items if p == "negative"]
        for pc in positives:
            for nc in negatives:
                pk = set(pc.get("trigger_keywords", []))
                nk = set(nc.get("trigger_keywords", []))
                shared = pk & nk
                if shared:
                    contradictions.append({
                        "gap_type": gap_type,
                        "positive_capsule": pc,
                        "negative_capsule": nc,
                        "shared_keywords": list(shared),
                    })

    return contradictions


def analyze_multi_paper_gaps(
    papers: List[Dict[str, str]],
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze gaps across multiple papers to surface shared and frontier opportunities.

    papers: list of {id, title, abstract}
    Returns: {shared_themes, complementary_gaps, frontier_gaps, contradictions, paper_count}
    """
    if len(papers) < 2:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": "Need at least 2 papers for multi-paper gap analysis",
        }

    papers_str = "\n\n".join(
        f"Paper {i+1} (ID: {p.get('id', '?')}):\nTitle: {p.get('title', '?')}\nAbstract: {p.get('abstract', '?')[:400]}"
        for i, p in enumerate(papers)
    )

    prompt = f"""You are a research gap analyst. Given a set of papers, identify systematic research gaps and opportunities across the entire collection.

PAPERS:
{papers_str}

TASK:
Analyze ALL papers above and produce a structured gap report. Focus on:

1. SHARED THEMES: research areas/keywords that appear across multiple papers — these represent active frontier themes.

2. COMPLEMENTARY GAPS: for each paper, identify what it DOES NOT address that another paper in the set DOES — these are complementary opportunities.

3. FRONTIER GAPS: research gaps that NONE of the papers address but are clearly suggested by the collection's themes. These are the most valuable opportunities.

4. CONTRADICTIONS: any papers that make opposing claims or findings about the same gap_type.

Respond ONLY with a JSON object (no markdown, no code fences):
{{
  "shared_themes": [
    {{"theme": "reasoning", "papers": ["id1", "id2"], "strength": 0.8, "description": "both papers tackle reasoning in long context"}}
  ],
  "complementary_gaps": [
    {{"gap_title": "..." , "gap_type": "method_limitation", "addressed_by": ["id1"], "missing_from": ["id2"], "description": "..."}}
  ],
  "frontier_gaps": [
    {{"gap_title": "...", "gap_type": "evaluation_gap", "keywords": ["benchmark", "reasoning"], "summary": "No paper evaluates on real-world deployment scenarios"}}
  ],
  "contradictions": [
    {{"gap_type": "scalability_issue", "positive_id": "id1", "negative_id": "id2", "description": "Paper A claims scaling works; Paper B shows it fails on OOD data"}}
  ]
}}
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model=model or "claude-3-5-sonnet-latest",
        )
        result = json.loads(content.strip())
        result["paper_count"] = len(papers)
        return result
    except json.JSONDecodeError:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": f"LLM returned invalid JSON: {content[:200] if 'content' in dir() else '?'}",
        }
    except Exception as e:
        return {
            "shared_themes": [],
            "complementary_gaps": [],
            "frontier_gaps": [],
            "contradictions": [],
            "paper_count": len(papers),
            "error": str(e),
        }


def gaps_to_research_questions(
    frontier_gaps: List[Dict[str, Any]],
    paper_titles: Optional[Dict[str, str]] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Convert frontier gaps into actionable research questions.

    Each question is specific enough to drive a paper2code experiment or
    a targeted literature search.

    frontier_gaps: list of {gap_title, gap_type, keywords, summary}
    paper_titles: optional {paper_id: title} map for context

    Returns: {questions: [{question, gap_title, gap_type, keywords, difficulty, hypothesis}]}
    """
    if not frontier_gaps:
        return {"questions": [], "gap_count": 0}

    gaps_str = "\n".join(
        f"- Gap {i+1}: {g.get('gap_title','?')} [{g.get('gap_type','?')}] "
        f"keywords={', '.join(g.get('keywords',[])[:5])} "
        f"summary={g.get('summary','?')}"
        for i, g in enumerate(frontier_gaps)
    )

    prompt = f"""Given the following frontier research gaps (underexplored problem spaces), generate concrete, actionable research questions.

Each question must be:
- Specific: mentions method names, evaluation criteria, or constraints
- Answerable: can be addressed by running experiments or analyzing existing papers
- Novelty-justified: motivated by the gap it addresses

Output JSON (no markdown, no code fences):
{{
  "questions": [
    {{
      "question": "Can chain-of-thought prompting improve factual recall in 70B+ models on medical VQA when evaluated on the MedQA dataset?",
      "gap_title": "Limited medical VQA benchmark coverage",
      "gap_type": "evaluation_gap",
      "keywords": ["medical-vqa", "chain-of-thought", "factual-recall"],
      "difficulty": "medium",
      "hypothesis": "Chain-of-thought reasoning will improve factual recall by enabling multi-step diagnostic inference"
    }},
    ...
  ]
}}

GAP DATA:
{gaps_str}

Respond ONLY with the JSON object.
"""

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        return {
            "questions": [],
            "gap_count": len(frontier_gaps),
            "error": "llm.client not available",
        }

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model=model or "claude-3-5-sonnet-latest",
        )
        result = json.loads(content.strip())
        result["gap_count"] = len(frontier_gaps)
        return result
    except json.JSONDecodeError:
        return {
            "questions": [],
            "gap_count": len(frontier_gaps),
            "error": f"LLM returned invalid JSON: {content[:200] if 'content' in dir() else '?'}",
        }
    except Exception as e:
        return {
            "questions": [],
            "gap_count": len(frontier_gaps),
            "error": str(e),
        }
