"""Research suggestion engine + experiment proposal helpers."""

from __future__ import annotations

import json

from pathlib import Path
from typing import Any, Dict, List

# Gap types the user has NOT explored yet but might find valuable
UNDERREPRESENTED_GAPS = [
    (
        "theoretical_gap",
        "Theoretical foundations",
        "Develop formal theory or proofs for observed empirical patterns in your work",
    ),
    (
        "dataset_gap",
        "Dataset gap",
        "Build or curate a benchmark dataset addressing an under-explored problem domain",
    ),
    (
        "generalization_gap",
        "Generalization gap",
        "Test existing methods on out-of-distribution data to expose failure modes",
    ),
    (
        "scalability_issue",
        "Scalability issue",
        "Push current methods to larger scales and characterize runtime/cost tradeoffs",
    ),
    ("contradiction", "Contradiction", "Reproduce or challenge published findings in this area"),
    (
        "evaluation_gap",
        "Evaluation gap",
        "Design proper evaluation protocols and baselines for this problem",
    ),
]

EXPERIMENTS_DIR = Path.home() / ".ai_research_os" / "experiments"
EXPERIMENTS_DIR.mkdir(parents=True, exist_ok=True)


# ── Consumed suggestion tracking ────────────────────────────────────────────


def _get_consumed_suggestions() -> set:
    try:
        path = Path.home() / ".ai_research_os" / "consumed_suggestions.json"
        if path.exists():
            return set(json.loads(path.read_text(encoding="utf-8")))
    except Exception:
        pass
    return set()


def _mark_suggestion_consumed(gap_type: str, topic_hint: str, title: str, s_type: str = "") -> None:
    try:
        consumed = _get_consumed_suggestions()
        if s_type == "archetype_advice":
            key = f"archetype:{title}"
        else:
            key = f"{gap_type}:{topic_hint[:20]}:{title[:30]}"
        consumed.add(key)
        path = Path.home() / ".ai_research_os" / "consumed_suggestions.json"
        path.parent.mkdir(parents=True, exist_ok=True)
        path.write_text(json.dumps(list(consumed), ensure_ascii=False), encoding="utf-8")
    except Exception:
        pass


# ── Capsule consumed marker ─────────────────────────────────────────────────


def mark_capsule_consumed(capsule_id: str, tracker) -> None:
    """Mark a capsule as consumed in both Gene Pool stores."""
    

    try:
        consumed_title = ""
        consumed_gap_type = ""
        capsules = tracker._load_capsules()
        updated = False
        for c in capsules:
            if c.capsule_id == capsule_id:
                c.status = "consumed"
                updated = True
                consumed_title = c.action_gap_title
                consumed_gap_type = c.action_gap_type
                break
        if updated:
            tracker._save_capsules(capsules)

        tracker.record_capsule_lifecycle_event(
            capsule_id=capsule_id,
            action="consumed",
            gap_title=consumed_title,
            gap_type=consumed_gap_type,
        )
    except Exception:
        pass


# ── Suggestion generation ───────────────────────────────────────────────────


def generate_suggestions(capsules, gap_prefs, topic_freq, archetype, tracker) -> list:
    """Analyze Gene Pool patterns and generate actionable project suggestions."""
    suggestions = []

    if not capsules and not gap_prefs:
        return []

    consumed = _get_consumed_suggestions()
    explored_gaps = set(gap_prefs.keys())
    explored_topics = set(topic_freq.keys())

    # 1. High-performing gap types with low exploration
    high_score_gaps = {k: v for k, v in gap_prefs.items() if v > 0.3}
    suggested_gap_types = set()
    for gap_type, score in high_score_gaps.items():
        for candidate_gap, label, description in UNDERREPRESENTED_GAPS:
            if candidate_gap not in explored_gaps and candidate_gap not in suggested_gap_types:
                suggestions.append(
                    {
                        "type": "explore_new_gap",
                        "icon": "🔍",
                        "title": f"Explore {label} in your research",
                        "body": f"You've had success with {gap_type} (score {score:.2f}). "
                        f"Consider investigating {description.lower()}.",
                        "gap_type": candidate_gap,
                        "confidence": min(score, 0.9),
                        "topic_hint": list(explored_topics)[0] if explored_topics else "your field",
                        "consumed": False,
                    }
                )
                suggested_gap_types.add(candidate_gap)
                break

    # 2. Top topics with no evaluation_gap explored
    top_topics = list(topic_freq.items())[:3]
    evaluated = [g for g in explored_gaps if "evaluation" in g or "benchmark" in g]
    if not evaluated and top_topics:
        topic_name = top_topics[0][0][:40]
        suggestions.append(
            {
                "type": "evaluation_gap",
                "icon": "📏",
                "title": f"Evaluate {topic_name} rigorously",
                "body": f"You've explored '{topic_name}' ({topic_freq[topic_name]}×) "
                "but haven't investigated evaluation gaps. "
                "Proper benchmarks could unlock significant improvements.",
                "gap_type": "evaluation_gap",
                "confidence": 0.7,
                "topic_hint": topic_name,
            }
        )

    # 3. High-scoring capsules
    high_perf_capsules = [c for c in capsules if len(c) >= 5 and c[4] >= 0.7]
    if high_perf_capsules:
        best = high_perf_capsules[0]
        cap_id, topic, gap_type, gap_title, score, date, keywords, status = best
        suggestions.append(
            {
                "type": "build_on_success",
                "icon": "🚀",
                "title": f"Build on: {gap_title[:60]}",
                "body": f"This pattern scored {score * 100:.0f}% success. "
                "Try extending it: add more keywords, test in adjacent domains, "
                "or compose with another high-performing capsule.",
                "gap_type": gap_type,
                "confidence": score,
                "topic_hint": topic[:40],
                "keywords": keywords,
                "consumed": False,
                "source_cap_id": cap_id,
            }
        )

    # 4. Archetype-driven suggestion
    arch_dim = archetype.get("dominant", "")
    if arch_dim == "method_focused":
        suggestions.append(
            {
                "type": "archetype_advice",
                "icon": "⚙️",
                "title": "Your archetype: Method Hunter",
                "body": "Focus on novel architectures, training procedures, or inference optimizations. "
                "Look for published methods with surprising results and improve or extend them.",
                "gap_type": "method_limitation",
                "confidence": archetype.get("confidence", 0.5),
                "topic_hint": list(explored_topics)[0][:40] if explored_topics else "ML",
                "consumed": False,
            }
        )
    elif arch_dim == "high_risk":
        suggestions.append(
            {
                "type": "archetype_advice",
                "icon": "🧗",
                "title": "Your archetype: Risk Taker",
                "body": "Pursue high-uncertainty problems with high payoff: "
                "new domains, controversial claims, unproven scalability. "
                "Your profile suggests you can handle the volatility.",
                "gap_type": "unexplored_application",
                "confidence": archetype.get("confidence", 0.5),
                "topic_hint": list(explored_topics)[0][:40] if explored_topics else "research",
                "consumed": False,
            }
        )

    # 5. Cross-domain
    if archetype.get("dimensions", {}).get("cross_domain", (0, 0, "", ""))[1] >= 0.3:
        suggestions.append(
            {
                "type": "cross_domain",
                "icon": "🌉",
                "title": "Bridge domains with your cross-domain profile",
                "body": "Your research spans multiple areas. Try combining RL concepts with "
                "transformer architectures, or apply your NLP insights to graph problems.",
                "gap_type": "generalization_gap",
                "confidence": 0.65,
                "topic_hint": list(explored_topics)[0][:40]
                if explored_topics
                else "interdisciplinary",
                "consumed": False,
            }
        )

    filtered = []
    for s in suggestions:
        if s["type"] == "archetype_advice":
            key = f"archetype:{s['title']}"
        else:
            key = f"{s['gap_type']}:{s.get('topic_hint', '')[:20]}:{s.get('title', '')[:30]}"
        if key not in consumed:
            filtered.append(s)

    return sorted(filtered, key=lambda s: s.get("confidence", 0), reverse=True)[:5]


# ── Experiment proposals ────────────────────────────────────────────────────


def get_experiment_queue() -> List[Dict[str, Any]]:
    try:
        if not EXPERIMENTS_DIR.exists():
            return []
        files = sorted(
            EXPERIMENTS_DIR.glob("experiment_*.json"),
            key=lambda p: p.stat().st_mtime,
            reverse=True,
        )
        return [json.loads(f.read_text(encoding="utf-8")) for f in files[:20]]
    except Exception:
        return []


def save_experiment(exp: Dict[str, Any]) -> None:
    slug = exp.get("id", "unknown").replace(":", "_")
    path = EXPERIMENTS_DIR / f"experiment_{slug}.json"
    path.write_text(json.dumps(exp, indent=2, ensure_ascii=False), encoding="utf-8")


def render_experiments_html(queue: List[Dict[str, Any]]) -> str:
    if not queue:
        return """
        <div style="text-align:center;padding:40px;color:#888;">
          <div style="font-size:40px;margin-bottom:12px;">🔬</div>
          <div style="font-size:15px;font-weight:600;margin-bottom:6px;">No experiment proposals yet</div>
          <div style="font-size:13px;">Accept a suggestion with a concrete gap, then come back here to run the experiment.</div>
        </div>"""
    rows = ""
    for i, exp in enumerate(queue, 1):
        status = exp.get("status", "pending")
        status_color = {
            "pending": "#FF9800",
            "running": "#2196F3",
            "done": "#4CAF50",
            "failed": "#F44336",
        }.get(status, "#888")
        hypothesis = exp.get("hypothesis", "")
        exp_id_js = exp["id"].replace("'", "\\'")
        run_btn = (
            (
                f"<button onclick=\"runExperiment('{exp_id_js}')\" "
                f'style="background:#4CAF50;color:#fff;border:none;border-radius:6px;padding:7px 16px;font-size:12px;cursor:pointer;">'
                f"▶ Run Experiment</button>"
            )
            if status == "pending"
            else ""
        )
        paper_id = exp.get("paper_id", "")
        paper_meta = (
            f'<div style="font-size:12px;color:#555;margin-bottom:6px;"><span style="color:#888;">paper:</span> '
            f'<a href="/paper/{paper_id}" style="color:#6B8FB5;">{paper_id}</a>'
            f' <span style="font-size:10px;color:#4CAF50;">⚡ Paper2Code</span></div>'
            if paper_id
            else ""
        )
        rows += f"""
        <div style="border:1px solid #e0e8f0;border-radius:8px;padding:16px;margin-bottom:12px;background:#fff;box-shadow:0 2px 4px rgba(0,0,0,0.05);">
          <div style="display:flex;justify-content:space-between;align-items:flex-start;margin-bottom:8px;flex-wrap:wrap;gap:8px;">
            <div style="font-size:14px;font-weight:700;color:#1a2a3a;">{i}. {exp.get("title", "Untitled")[:80]}</div>
            <span style="font-size:11px;font-weight:700;color:{status_color};background:{status_color}22;padding:3px 10px;border-radius:12px;">{status.upper()}</span>
          </div>
          <div style="font-size:12px;color:#555;margin-bottom:6px;"><span style="color:#888;">gap_type:</span> {exp.get("gap_type", "")}</div>
          <div style="font-size:12px;color:#555;margin-bottom:6px;"><span style="color:#888;">difficulty:</span> {exp.get("difficulty", "")}</div>
          {paper_meta}
          {('<div style="font-size:12px;color:#666;margin-bottom:6px;font-style:italic;">&#128161; Hypothesis: ' + hypothesis[:150] + "</div>" if hypothesis else "")}
          <div style="margin-top:10px;display:flex;gap:8px;flex-wrap:wrap;">
            {run_btn}
            <button onclick="removeExperiment('{exp_id_js}')" style="background:transparent;color:#888;border:1px solid #ccc;border-radius:6px;padding:7px 14px;font-size:12px;cursor:pointer;">Remove</button>
          </div>
        </div>"""
    return f"""
    <div style="margin-bottom:20px;">
      <div style="font-size:13px;color:#888;margin-bottom:12px;">{len(queue)} experiment proposal(s)</div>
      {rows}
    </div>
    <script>
    function runExperiment(id) {{
      if (!confirm('Run this experiment? It will execute in the background.')) return;
      fetch('/insights/run-experiment', {{method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{id}})}})
        .then(function(r) {{ return r.json(); }})
        .then(function(d) {{ alert('Experiment started: ' + d.message); location.reload(); }})
        .catch(function(e) {{ alert('Error: ' + e.message); }});
    }}
    function removeExperiment(id) {{
      fetch('/insights/experiments/remove', {{method:'POST', headers:{{'Content-Type':'application/json'}}, body: JSON.stringify({{id}})}})
        .then(function(r) {{ location.reload(); }});
    }}
    </script>"""
