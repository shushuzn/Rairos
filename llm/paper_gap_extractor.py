"""Extract research gaps from a paper using LLM analysis."""

from __future__ import annotations

import json
import uuid
from datetime import datetime
from pathlib import Path
from typing import Any, Dict, List, Optional

from llm.constants import LLM_BASE_URL, LLM_MODEL
from llm.gene_pool_io import load_capsules, get_capsule_by_paper


# =============================================================================
# GAP ANALYZER CONFIGS — Q1-Q10 预配置模板
# =============================================================================

GAP_ANALYZER_CONFIGS: Dict[str, dict] = {
    # Q1: embodied_planning — discrete vs continuous latent representation
    "embodied_planning": {
        "gap_type": "embodied_planning",
        "result_fields": ["representation_type", "confidence", "evidence", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in robotics and embodied AI.
Given a paper's title and abstract, determine how the paper represents physical reasoning (latent reasoning over physical dynamics).

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

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
summary: [2-sentence analysis]""",
        "keywords": ["embodied", "latent", "reasoning", "VLA", "robotics"],
    },
    # Q2: rl_efficiency — LAPO vs PPO convergence speed
    "rl_efficiency": {
        "gap_type": "rl_efficiency",
        "result_fields": ["algorithm", "convergence_speed", "sample_efficiency", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in reinforcement learning.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the RL algorithm and its efficiency characteristics:

1. Which RL algorithm does the paper focus on?
   (e.g., PPO, LAPO, SAC, TD3, DDPG, Q-learning variants)

2. How fast does it converge compared to baselines?
   Look for: "converges in X steps", "sample efficiency", "faster than", "reduced samples"

3. What is the sample efficiency ranking?

4. What OPEN QUESTION remains about efficiency tradeoffs?

Provide your analysis in this format:
algorithm: [algorithm name]
convergence_speed: [fast|medium|slow|unknown]
sample_efficiency: [high|medium|low|unknown]
gap_title: [specific research question about RL efficiency]
summary: [2-sentence analysis]""",
        "keywords": ["RL", "reinforcement learning", "efficiency", "convergence", "PPO", "LAPO"],
    },
    # Q3: reasoning_scaling — inference chain length vs task complexity
    "reasoning_scaling": {
        "gap_type": "reasoning_scaling",
        "result_fields": ["chain_length", "task_complexity", "scaling_behavior", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in reasoning systems.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's approach to reasoning chain length and task complexity:

1. What is the inference chain length discussed?
   Look for: "chain-of-thought", "reasoning steps", "N steps", "depth"

2. How does chain length scale with task complexity?

3. What is the relationship between reasoning depth and performance?

4. What OPEN QUESTION remains about scaling reasoning?

Provide your analysis in this format:
chain_length: [short|medium|long|variable|unknown]
task_complexity: [simple|moderate|high|unknown]
scaling_behavior: [linear|sublinear|superlinear|decreasing|unknown]
gap_title: [specific research question about reasoning scaling]
summary: [2-sentence analysis]""",
        "keywords": ["reasoning", "chain-of-thought", "scaling", "inference", "complexity"],
    },
    # Q4: sim_to_real — zero-shot generalization
    "sim_to_real": {
        "gap_type": "sim_to_real",
        "result_fields": ["generalization_level", "domain_gap", "transfer_quality", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in sim-to-real transfer.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's sim-to-real generalization capability:

1. Does the paper achieve zero-shot generalization?
   Look for: "zero-shot", "domain randomization", "unseen", "out-of-distribution"

2. How large is the domain gap between simulation and real?

3. What is the quality of transfer?

4. What OPEN QUESTION remains about generalization bounds?

Provide your analysis in this format:
generalization_level: [zero-shot|few-shot|full-transfer|none|unknown]
domain_gap: [small|medium|large|unknown]
transfer_quality: [high|medium|low|unknown]
gap_title: [specific research question about sim-to-real]
summary: [2-sentence analysis]""",
        "keywords": ["sim-to-real", "zero-shot", "generalization", "domain randomization", "transfer"],
    },
    # Q5: planning_control — reasoning/action alternation frequency
    "planning_control": {
        "gap_type": "planning_control",
        "result_fields": ["alternation_freq", "planning_depth", "control_type", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in planning and control systems.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's planning and control architecture:

1. How often does reasoning alternate with action execution?
   Look for: "replan", "online planning", "action frequency", "control loop"

2. What is the planning depth (how many steps ahead)?

3. Is it hierarchical or flat control?

4. What OPEN QUESTION remains about planning frequency?

Provide your analysis in this format:
alternation_freq: [high|medium|low|adaptive|unknown]
planning_depth: [shallow|medium|deep|variable|unknown]
control_type: [hierarchical|flat|hybrid|unknown]
gap_title: [specific research question about planning/control]
summary: [2-sentence analysis]""",
        "keywords": ["planning", "control", "replanning", "hierarchical", "action"],
    },
    # Q6: representation_learning — visual vs physical latent attention
    "representation_learning": {
        "gap_type": "representation_learning",
        "result_fields": ["attention_type", "modality_focus", "latent_structure", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in representation learning.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's representation learning approach:

1. Where is attention focused — visual features or physical dynamics?
   Look for: "visual", "physical", "latent", "attention", "feature"

2. What modality does the latent representation encode?

3. How is the latent space structured?

4. What OPEN QUESTION remains about representation quality?

Provide your analysis in this format:
attention_type: [visual|physical|both|unknown]
modality_focus: [vision|physics|multimodal|unknown]
latent_structure: [discrete|continuous|structured|unknown]
gap_title: [specific research question about representation learning]
summary: [2-sentence analysis]""",
        "keywords": ["representation", "attention", "visual", "latent", "features"],
    },
    # Q7: rl_pretraining — warm-start strategy quality vs diversity
    "rl_pretraining": {
        "gap_type": "rl_pretraining",
        "result_fields": ["pretrain_strategy", "quality_diversity", "transfer_gain", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in RL pretraining.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's RL pretraining approach:

1. What is the pretraining strategy?
   Look for: "pretrained", "warm-start", "imitation learning", "offline", "online"

2. Does it prioritize quality or diversity of pretraining data?

3. How much does pretraining help downstream tasks?

4. What OPEN QUESTION remains about pretraining tradeoffs?

Provide your analysis in this format:
pretrain_strategy: [imitation|offline|online|multi-task|mixed|unknown]
quality_diversity: [quality-focused|diversity-focused|balanced|unknown]
transfer_gain: [high|medium|low|none|unknown]
gap_title: [specific research question about RL pretraining]
summary: [2-sentence analysis]""",
        "keywords": ["pretraining", "warm-start", "imitation learning", "offline RL", "transfer"],
    },
    # Q8: benchmark_coverage — LIBERO vs real robot evaluation
    "benchmark_coverage": {
        "gap_type": "benchmark_coverage",
        "result_fields": ["benchmark_used", "real_robot_eval", "coverage_gap", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in robot learning benchmarks.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's evaluation approach:

1. Which benchmark does the paper use?
   Look for: "LIBERO", "RLBench", "MetaWorld", "real robot", "simulation benchmark"

2. Does it evaluate on real robots or only simulation?

3. What aspects of real-world deployment are NOT covered?

4. What OPEN QUESTION remains about benchmark validity?

Provide your analysis in this format:
benchmark_used: [LIBERO|RLBench|MetaWorld|other|none|unknown]
real_robot_eval: [yes|no|partial|unknown]
coverage_gap: [small|medium|large|unknown]
gap_title: [specific research question about benchmark coverage]
summary: [2-sentence analysis]""",
        "keywords": ["benchmark", "LIBERO", "RLBench", "evaluation", "real robot"],
    },
    # Q9: architecture_agnostic — non-VLA architecture transfer
    "architecture_agnostic": {
        "gap_type": "architecture_agnostic",
        "result_fields": ["architecture_type", "transfer_scope", "model_agnostic", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in robot learning architectures.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's architecture and transfer properties:

1. What architecture does the paper use?
   Look for: "VLA", "CNN", "Transformer", "diffusion", "GPT", "language model"

2. How well does it transfer to different architectures?

3. Is the approach architecture-agnostic?

4. What OPEN QUESTION remains about architecture transfer?

Provide your analysis in this format:
architecture_type: [VLA|Transformer|CNN|diffusion|hybrid|other|unknown]
transfer_scope: [narrow|medium|broad|architecture-agnostic|unknown]
model_agnostic: [yes|partial|no|unknown]
gap_title: [specific research question about architecture transfer]
summary: [2-sentence analysis]""",
        "keywords": ["VLA", "architecture", "transfer", "Transformer", "CNN", "diffusion"],
    },
    # Q10: human_ai_collaboration — human intervention corrects latent paths
    "human_ai_collaboration": {
        "gap_type": "human_ai_collaboration",
        "result_fields": ["intervention_type", "latent_correction", "collaboration_mode", "gap_title", "summary"],
        "prompt_template": """You are a research analyst specializing in human-AI collaboration.

PAPER:
Title: {title}
Authors: {authors}
Abstract: {abstract}

TASK:
Analyze the paper's human-AI collaboration approach:

1. How does human intervention occur?
   Look for: "human in the loop", "intervention", "correction", "feedback", "teleoperation"

2. Does human correction modify latent representations or only actions?

3. What is the collaboration mode?

4. What OPEN QUESTION remains about human-AI teaming?

Provide your analysis in this format:
intervention_type: [latent|action|both|unknown]
latent_correction: [yes|no|partial|unknown]
collaboration_mode: [teleop|correction|feedback|shared-control|unknown]
gap_title: [specific research question about human-AI collaboration]
summary: [2-sentence analysis]""",
        "keywords": ["human-AI", "collaboration", "intervention", "teleoperation", "correction"],
    },
}


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


def analyze_gap(
    paper_id: str,
    title: str,
    abstract: str,
    gap_type: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    model: Optional[str] = None,
    extra_fields: Optional[Dict[str, Any]] = None,
) -> Dict[str, Any]:
    """Generalized gap analyzer using GAP_ANALYZER_CONFIGS templates.

    Args:
        paper_id: unique paper identifier
        title: paper title
        abstract: paper abstract
        gap_type: key in GAP_ANALYZER_CONFIGS (e.g. "embodied_planning", "rl_efficiency")
        authors: optional list of author names
        api_key: optional LLM API key
        model: optional LLM model override
        extra_fields: additional fields to store in archetype

    Returns:
        dict with result_fields from the config plus contradiction detection
    """
    if gap_type not in GAP_ANALYZER_CONFIGS:
        return {
            "error": f"Unknown gap_type: {gap_type}. Available: {list(GAP_ANALYZER_CONFIGS.keys())}",
        }

    config = GAP_ANALYZER_CONFIGS[gap_type]
    result_fields = config["result_fields"]
    keywords = config.get("keywords", [])
    authors_str = ", ".join(authors[:3]) if authors else "Unknown"

    # Fill prompt template
    prompt = config["prompt_template"].format(
        title=title,
        authors=authors_str,
        abstract=abstract[:800],
    )

    try:
        from llm.client import call_llm_chat_completions
    except ImportError:
        result = {f: "unknown" for f in result_fields}
        result["gap_title"] = title[:80]
        result["summary"] = "LLM client not available."
        result["error"] = "llm.client not available"
        return result

    try:
        content = call_llm_chat_completions(
            messages=[{"role": "user", "content": prompt}],
            model=model or "claude-3-5-sonnet-latest",
            api_key=api_key,
        )

        lines = content.strip().split("\n")
        result = {f: "" for f in result_fields}
        result["confidence"] = 0.5  # default
        for line in lines:
            # Strip markdown bold/heading markers from both ends before parsing
            stripped = line.strip().strip("**").strip()
            for field in result_fields:
                prefix = field + ":"
                if stripped.startswith(prefix):
                    val = stripped.split(":", 1)[1].strip().strip("**`-").strip()
                    if field == "confidence":
                        try:
                            result[field] = float(val)
                        except ValueError:
                            pass
                    elif isinstance(result[field], list):
                        result[field] = [val]
                    else:
                        result[field] = val

        # Build extra_fields for Gene Pool
        gap_extra = extra_fields.copy() if extra_fields else {}
        for field in result_fields:
            if result.get(field) and result[field] != "unknown":
                gap_extra[field] = result[field]

        # Save to Gene Pool
        capsule_id = save_gap_to_gene_pool(
            paper_id=paper_id,
            title=title,
            gap_type=gap_type,
            gap_title=result.get("gap_title") or f"{gap_type}: {title[:60]}",
            keywords=keywords + [str(result.get(result_fields[0], ""))],
            summary=result.get("summary") or f"{gap_type} analysis. {result_fields[0]}: {result.get(result_fields[0], 'unknown')}",
            polarity="open",
            extra_fields=gap_extra,
        )
        result["saved_to_pool"] = capsule_id

        # Contradiction detection via unified detect_field_contradiction
        result["contradiction_with"] = None
        result["contradiction_type"] = None
        if capsule_id:
            primary_field = result_fields[0]
            current_val = result.get(primary_field, "unknown")
            conflict = detect_field_contradiction(gap_type, primary_field, current_val)
            if conflict:
                result["contradiction_with"] = conflict["source_paper_id"]
                result["contradiction_type"] = conflict["conflicting_value"]

        # Track timeline (embodied_planning uses specialized tracker)
        if gap_type == "embodied_planning":
            track_embodied_evolution(
                paper_id=paper_id,
                title=title,
                representation_type=result.get("representation_type", "unknown"),
                confidence=result.get("confidence", 0.5),
                gap_title=result.get("gap_title") or f"Latent Reasoning: {result.get('representation_type', 'unknown')}",
            )

        return result

    except Exception as e:
        result = {f: "unknown" for f in result_fields}
        result["gap_title"] = title[:80]
        result["summary"] = f"Analysis failed: {e}"
        result["error"] = str(e)
        return result


def analyze_embodied_planning(
    paper_id: str,
    title: str,
    abstract: str,
    authors: Optional[List[str]] = None,
    api_key: Optional[str] = None,
    model: Optional[str] = None,
) -> Dict[str, Any]:
    """Analyze embodied planning paper: discrete vs continuous latent representation.

    Delegates to analyze_gap(gap_type="embodied_planning", ...).
    """
    return analyze_gap(
        paper_id=paper_id,
        title=title,
        abstract=abstract,
        gap_type="embodied_planning",
        authors=authors,
        api_key=api_key,
        model=model,
    )


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


    # Contradiction pairs via unified detect_evidence_contradiction
    contradictions = detect_evidence_contradiction(results)

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
    _type_colors = {"discrete": "#7A9E7A", "continuous": "#6B8FB5", "hybrid": "#D4A84B"}
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

    # Find same-type edges (within-cluster) and contradiction edges (cross-type)
    same_type_edges = set()
    contradiction_edges = set()
    pids = list(nodes_info.keys())
    for i, p1 in enumerate(pids):
        for p2 in pids[i+1:]:
            if p1 == p2:
                continue
            r1, title1, _ = nodes_info[p1]
            r2, title2, _ = nodes_info[p2]
            if r1 != r2 and r1 != "unknown" and r2 != "unknown":
                # Cross-type = potential contradiction
                contradiction_edges.add((p1[:12], p2[:12], r1, r2))
            elif r1 == r2 and r1 != "unknown":
                # Same type = within-cluster edge
                same_type_edges.add((p1[:12], p2[:12], r1))

    # Build Mermaid
    lines = ["```mermaid", "flowchart TD"]
    lines.append("    %% Nodes: papers by representation type")

    for pid, (rtype, title, conf) in nodes_info.items():
        short_id = pid[:12]
        conf_pct = int(conf * 100)
        label = f"**{title}**\n({conf_pct}%)"
        if rtype == "discrete":
            line = f"    {short_id}(({label}))"
        elif rtype == "continuous":
            line = f"    {short_id}(({label}))"
        elif rtype == "hybrid":
            line = f"    {short_id}{{{{{label}}}}}"
        else:
            line = f"    {short_id}(({label}))"
        lines.append(line)

    lines.append("    %% Same-type edges: gray solid (within-cluster)")
    for e1, e2, _rtype in sorted(same_type_edges):
        lines.append(f"    {e1} -->|same| {e2}")

    lines.append("    %% Contradiction edges: red dashed (cross-type)")
    for e1, e2, r1, r2 in sorted(contradiction_edges):
        lines.append(f"    {e1} -.->|{r1}/{r2}| {e2}")

    lines.append("")
    lines.append("    subgraph legend[\"\"]")
    lines.append("        direction LR")
    lines.append("        L1[\"\"] --- L2[\"\"]")
    lines.append("        L1[\"\"] -..- L3[\"\"]")
    lines.append("        LS1[\"same type (same color)\"]")
    lines.append("        LS2[\"contradiction (cross-type)\"]")
    lines.append("    end")
    lines.append("```")
    return "\n".join(lines)


def render_compare_view(paper_ids: List[str], db=None) -> str:
    """Render side-by-side HTML comparison of up to 2 papers.

    Each paper shows: representation_type badge, confidence bar, evidence list, gap_title.
    Middle column shows comparison verdict: same type = green "一致", diff type = red "矛盾".
    """
    if not paper_ids:
        return "<div style='font-family:var(--font-display);color:#888;'>No papers selected.</div>"
    if len(paper_ids) > 2:
        paper_ids = paper_ids[:2]
    if db is None:
        from db.database import Database
        db = Database()

    import json as _json
    capsule_path = Path.home() / ".ai_research_os" / "gene_pool" / "capsules.json"
    capsules = []
    if capsule_path.exists():
        capsules = _json.loads(capsule_path.read_text(encoding="utf-8")).get("capsules", [])

    color_map = {"discrete": "#7A9E7A", "continuous": "#6B8FB5", "hybrid": "#D4A84B", "unknown": "#aaa"}
    type_labels = {"discrete": "Discrete", "continuous": "Continuous", "hybrid": "Hybrid", "unknown": "Unclear"}
    badge_colors = {"discrete": "#7A9E7A", "continuous": "#6B8FB5", "hybrid": "#D4A84B", "unknown": "#aaa"}

    paper_data = []
    for pid in paper_ids:
        paper = db.get_paper(pid)
        # Find matching capsule
        capsule = None
        for c in capsules:
            src = c.get("archetype", {}).get("source_paper_id", "")
            if src == pid and (c.get("action_gap_type") == "embodied_planning" or c.get("trigger_gap_type") == "embodied_planning"):
                capsule = c
                break
        rep_type = "unknown"
        confidence = 0.0
        evidence = []
        gap_title = ""
        if capsule:
            rep_type = capsule.get("archetype", {}).get("representation_type", "unknown")
            confidence = capsule.get("outcome_success_score", 0.0)
            evidence = capsule.get("archetype", {}).get("evidence", [])
            gap_title = capsule.get("action_gap_title", "")
        if paper:
            title = paper.title
        else:
            title = f"Paper {pid[:12]}"
        paper_data.append({
            "id": pid,
            "title": title,
            "representation_type": rep_type,
            "confidence": confidence,
            "evidence": evidence,
            "gap_title": gap_title,
        })

    if len(paper_data) == 1:
        pd = paper_data[0]
        badge = f"<span style='background:{badge_colors.get(pd['representation_type'],'#aaa')};color:#fff;padding:2px 8px;border-radius:4px;font-size:12px;'>{type_labels.get(pd['representation_type'],'Unknown')}</span>"
        conf_pct = int(pd['confidence'] * 100)
        conf_bar = f"<div style='background:#eee;border-radius:4px;height:8px;width:100%;'><div style='background:{color_map.get(pd['representation_type'],'#aaa')};height:8px;border-radius:4px;width:{conf_pct}%;'></div></div>"
        evidence_html = "".join(f"<li style='font-size:13px;color:#555;margin-bottom:4px;'>{e}</li>" for e in pd['evidence']) if pd['evidence'] else "<li style='font-size:13px;color:#aaa;'>No evidence extracted.</li>"
        gap_html = f"<div style='font-size:13px;color:#333;margin-top:8px;'><strong>Gap:</strong> {pd['gap_title'] or '—'}</div>"
        return f"""
<div style="font-family:var(--font-display);max-width:600px;">
  <div style="font-size:16px;font-weight:700;margin-bottom:16px;">{pd['title'][:80]}</div>
  <div style="display:flex;align-items:center;gap:12px;margin-bottom:12px;">{badge} <span style="font-size:12px;color:#888;">{conf_pct}% confidence</span></div>
  {conf_bar}
  <ul style="margin-top:12px;padding-left:18px;">{evidence_html}</ul>
  {gap_html}
</div>"""

    pd1, pd2 = paper_data[0], paper_data[1]
    same_type = pd1['representation_type'] == pd2['representation_type'] and pd1['representation_type'] != 'unknown'
    verdict_color = "#4caf50" if same_type else "#e53935"
    verdict_text = "一致" if same_type else "矛盾"
    verdict_icon = "&#10004;" if same_type else "&#10006;"

    def paper_card(pd):
        badge = f"<span style='background:{badge_colors.get(pd['representation_type'],'#aaa')};color:#fff;padding:2px 10px;border-radius:4px;font-size:12px;font-weight:700;'>{type_labels.get(pd['representation_type'],'Unknown')}</span>"
        conf_pct = int(pd['confidence'] * 100)
        conf_bar = f"<div style='background:#eee;border-radius:4px;height:8px;width:100%;'><div style='background:{color_map.get(pd['representation_type'],'#aaa')};height:8px;border-radius:4px;width:{conf_pct}%;'></div></div>"
        evidence_html = "".join(f"<li style='font-size:13px;color:#555;margin-bottom:6px;'>{e}</li>" for e in pd['evidence']) if pd['evidence'] else "<li style='font-size:13px;color:#aaa;'>No evidence extracted.</li>"
        gap_html = f"<div style='font-size:13px;color:#333;margin-top:10px;padding-top:10px;border-top:1px solid #eee;'><strong>Gap:</strong> {pd['gap_title'] or '—'}</div>"
        return f"""
  <td style="vertical-align:top;padding:16px;background:#fafafa;border-radius:8px;border:1px solid #eee;width:40%;">
    <div style="font-size:14px;font-weight:700;color:#222;margin-bottom:10px;line-height:1.4;">{pd['title'][:80]}</div>
    <div style="display:flex;align-items:center;gap:10px;margin-bottom:10px;">{badge} <span style="font-size:11px;color:#888;">{conf_pct}%</span></div>
    {conf_bar}
    <ul style="margin-top:10px;padding-left:18px;">{evidence_html}</ul>
    {gap_html}
  </td>"""

    verdict_html = f"""
  <td style="vertical-align:center;text-align:center;padding:16px;width:20%;">
    <div style="font-size:24px;color:{verdict_color};">{verdict_icon}</div>
    <div style="font-size:16px;font-weight:700;color:{verdict_color};margin-top:6px;">{verdict_text}</div>
    <div style="font-size:11px;color:#888;margin-top:4px;">{"Same representation" if same_type else "Different representations"}</div>
  </td>"""

    return f"""
<div style="font-family:var(--font-display);max-width:900px;">
  <table style="width:100%;border-collapse:separate;border-spacing:8px;">
    <tr>
{paper_card(pd1)}
{verdict_html}
{paper_card(pd2)}
    </tr>
  </table>
</div>"""


def _extract_keywords(text: str) -> List[str]:
    """Simple keyword extraction from text."""
    words = text.lower().split()
    stop = {"the", "a", "an", "of", "in", "to", "for", "and", "or", "with", "is", "are", "that", "this", "we", "our", "by", "on", "as", "at", "from"}
    keywords = [w.strip(".,;:!?()[]{}") for w in words if len(w) > 4 and w not in stop]
    return list(dict.fromkeys(keywords))[:6]


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

    Deduplication: if paper_id + gap_type already exists in capsules.json, skip (return existing capsule_id).

    Writes to:
    - ~/.ai_research_os/gene_pool/capsules.json   (read by _match_gene_pool / briefing_generator)
    - ~/.ai_research_os/evolution/gene_pool.jsonl (read by find_capsule / Curator / EvolutionTracker)

    Returns capsule_id on success, None on failure.
    """
    # Deduplication: skip if paper already analyzed for this gap_type
    existing = get_capsule_by_paper(paper_id, gap_type=gap_type)
    if existing:
        return existing.get("capsule_id")

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
        except Exception as e:
            import logging
            logging.getLogger(__name__).warning(f"gene_pool.jsonl write failed (non-critical): {e}")

        return capsule_id
    except Exception:
        return None


def track_embodied_evolution(
    paper_id: str,
    title: str,
    representation_type: str,
    confidence: float,
    gap_title: str,
) -> bool:
    """Record embodied planning belief change for timeline tracking.

    Writes a timeline entry to ~/.ai_research_os/gene_pool/embodied_timeline.jsonl.
    Each entry captures a 'belief snapshot' of what the field believes at a point in time.
    """
    try:
        timeline_path = Path.home() / ".ai_research_os" / "gene_pool" / "embodied_timeline.jsonl"
        timeline_path.parent.mkdir(parents=True, exist_ok=True)
        entry = {
            "timestamp": datetime.now().isoformat(),
            "paper_id": paper_id,
            "paper_title": title[:120],
            "representation_type": representation_type,
            "confidence": confidence,
            "gap_title": gap_title[:120],
        }
        with open(timeline_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return True
    except Exception:
        return False


def render_evolution_timeline() -> str:
    """Render embodied planning belief timeline as a Mermaid Gantt chart.

    Shows how the field's representation-type 'belief' evolved over time.
    """
    import json as _json
    timeline_path = Path.home() / ".ai_research_os" / "gene_pool" / "embodied_timeline.jsonl"
    if not timeline_path.exists():
        return ""

    entries = []
    try:
        with open(timeline_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                entries.append(_json.loads(line))
    except Exception:
        return ""

    if not entries:
        return ""

    # Sort by timestamp
    entries.sort(key=lambda x: x.get("timestamp", ""))

    type_colors = {"discrete": "#7A9E7A", "continuous": "#6B8FB5", "hybrid": "#D4A84B", "unknown": "#aaa"}

    lines = ["```mermaid", "gantt"]
    lines.append("    title Embodied Planning: Field Belief Evolution")
    lines.append("    dateFormat X")
    lines.append("    axisFormat %m/%d")

    prev_type = None
    section_open = False
    for i, e in enumerate(entries):
        rt = e.get("representation_type", "unknown")
        _color = type_colors.get(rt, "#aaa")
        short_id = e["paper_id"][:10]
        title_short = e.get("paper_title", "Untitled")[:35].replace('"', "'")
        _ts = e.get("timestamp", "")[5:10]  # MM-DD
        label = f"{title_short} ({rt})"

        if rt != prev_type:
            if section_open:
                lines.append("    section ::")
            lines.append(f"    section {rt.capitalize()} ::")
            prev_type = rt
            section_open = True

        lines.append(f"    {label} :done, {short_id}, {i}, {i+1}d")

    lines.append("```")
    return "\n".join(lines)


def detect_field_contradiction(
    gap_type: str,
    primary_field: str,
    current_val: str,
    capsules: Optional[List[Dict[str, Any]]] = None,
) -> Optional[Dict[str, Any]]:
    """Find a capsule with same gap_type but different primary field value.

    Returns {source_paper_id, conflicting_value} or None.
    Used after saving a new analysis to flag if an existing capsule disagrees.
    """
    if current_val == "unknown" or current_val is None:
        return None
    if capsules is None:
        capsules = load_capsules(gap_type=gap_type, status="active")
    for c in capsules:
        if c.get("archetype", {}).get("source_paper_id") is None:
            continue
        ex_val = c.get("archetype", {}).get(primary_field, "unknown")
        if ex_val != current_val and ex_val != "unknown":
            return {"source_paper_id": c["archetype"]["source_paper_id"], "conflicting_value": ex_val}
    return None


def detect_polarity_contradiction(
    gap_type: str,
    capsules: Optional[List[Dict[str, Any]]] = None,
) -> List[Dict[str, Any]]:
    """Find capsule pairs with same gap_type but opposite polarity + shared keywords.

    Returns list of {gap_type, positive_capsule, negative_capsule, shared_keywords}.
    """
    from collections import defaultdict
    if capsules is None:
        capsules = load_capsules(gap_type=gap_type, status="active")

    by_polarity = defaultdict(list)
    for c in capsules:
        if c.get("status") == "archived":
            continue
        polarity = c.get("polarity", "positive")
        by_polarity[polarity].append(c)

    contradictions = []
    for pos_c in by_polarity.get("positive", []):
        for neg_c in by_polarity.get("negative", []):
            shared = set(pos_c.get("trigger_keywords", [])) & set(neg_c.get("trigger_keywords", []))
            if shared:
                contradictions.append({
                    "gap_type": gap_type,
                    "positive_capsule": pos_c,
                    "negative_capsule": neg_c,
                    "shared_keywords": list(shared),
                })
    return contradictions


def detect_evidence_contradiction(
    embodied_results: List[Dict[str, Any]],
) -> List[Dict[str, Any]]:
    """Find discrete-vs-continuous contradictions via evidence keyword overlap.

    Returns list of {discrete_paper, continuous_paper, evidence_overlap}.
    """
    contradictions = []
    discrete = [r for r in embodied_results if r.get("representation_type") == "discrete"]
    continuous = [r for r in embodied_results if r.get("representation_type") == "continuous"]
    for dc in discrete:
        for cc in continuous:
            dc_ev = " ".join(dc.get("evidence", [])).lower()
            cc_ev = " ".join(cc.get("evidence", [])).lower()
            if any(w in cc_ev for w in dc_ev.split() if len(w) > 4):
                contradictions.append({
                    "discrete_paper": dc.get("paper_id"),
                    "continuous_paper": cc.get("paper_id"),
                    "evidence_overlap": True,
                })
    return contradictions


# Backwards-compatible alias
def detect_contradictions(capsules: list) -> list:
    """Legacy wrapper — dispatches to polarity-based detection per gap_type."""
    from collections import defaultdict
    by_type = defaultdict(list)
    for c in capsules:
        gt = c.get("action_gap_type") or c.get("trigger_gap_type", "")
        if gt:
            by_type[gt].append(c)
    all_results = []
    for gt, caps in by_type.items():
        all_results.extend(detect_polarity_contradiction(gt, caps))
    return all_results


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


def semantic_search_papers(
    query: str,
    top_k: int = 5,
    db=None,
) -> List[Dict[str, Any]]:
    """Semantic search across analyzed papers using embeddings.

    如果有sentence-transformers或openai embedding可用，用它。
    否则fallback到关键词+BM25相似度。

    Returns: [{"paper_id", "title", "abstract", "score", "matched_terms"}, ...]
    """
    try:
        from sentence_transformers import SentenceTransformer
        import numpy as np

        # Load papers from Gene Pool
        capsules = load_capsules()
        if not capsules:
            return _keyword_search_fallback(query, db=db, top_k=top_k)

        paper_ids = list({c.get("archetype", {}).get("source_paper_id", "") for c in capsules})
        paper_ids = [pid for pid in paper_ids if pid]

        if not paper_ids:
            return _keyword_search_fallback(query, db=db, top_k=top_k)

        # Fetch paper details from db
        if db is None:
            from db.database import Database
            db = Database()

        paper_map = db.get_papers_bulk(paper_ids)

        texts = []
        valid_ids = []
        for pid in paper_ids:
            p = paper_map.get(pid)
            if not p:
                continue
            title = getattr(p, "title", "") or ""
            abstract = getattr(p, "abstract", "") or ""
            texts.append(f"{title} {abstract}")
            valid_ids.append(pid)

        if not texts:
            return _keyword_search_fallback(query, db=db, top_k=top_k)

        # Encode query and corpus
        model_emb = SentenceTransformer("all-MiniLM-L6-v2")
        query_vec = model_emb.encode([query])
        doc_vecs = model_emb.encode(texts)

        # Cosine similarity
        scores = np.dot(doc_vecs, query_vec.T).flatten()
        top_indices = np.argsort(scores)[::-1][:top_k]

        results = []
        for idx in top_indices:
            pid = valid_ids[idx]
            p = paper_map.get(pid)
            if not p:
                continue
            title = getattr(p, "title", "") or ""
            abstract = getattr(p, "abstract", "") or ""
            results.append({
                "paper_id": pid,
                "title": title,
                "abstract": abstract[:300],
                "score": float(scores[idx]),
                "matched_terms": [],
            })
        return results

    except ImportError:
        pass

    # Fallback: keyword + tf-idf style search
    return _keyword_search_fallback(query, db=db, top_k=top_k)


def _keyword_search_fallback(
    query: str,
    db=None,
    top_k: int = 5,
) -> List[Dict[str, Any]]:
    """Simple keyword + tf-idf style search fallback.

    用简单词频匹配：query terms在title/abstract中出现次数/score。
    """
    capsules = load_capsules()
    if not capsules:
        return []

    paper_ids = list({c.get("archetype", {}).get("source_paper_id", "") for c in capsules})
    paper_ids = [pid for pid in paper_ids if pid]

    if not paper_ids:
        return []

    if db is None:
        from db.database import Database
        db = Database()

    paper_map = db.get_papers_bulk(paper_ids)

    query_terms = [w.lower().strip(".,;:!?()[]{}") for w in query.split() if len(w) > 1]
    if not query_terms:
        return []

    scored = []
    for pid in paper_ids:
        p = paper_map.get(pid)
        if not p:
            continue
        title = getattr(p, "title", "") or ""
        abstract = getattr(p, "abstract", "") or ""
        title_lower = title.lower()
        abstract_lower = abstract.lower()

        matched = set()
        title_hits = 0
        abstract_hits = 0
        for term in query_terms:
            t_count = title_lower.count(term)
            a_count = abstract_lower.count(term)
            title_hits += t_count
            abstract_hits += a_count
            if t_count > 0 or a_count > 0:
                matched.add(term)

        if not matched:
            continue

        # Score: title hits weighted 3x, abstract hits 1x
        score = title_hits * 3 + abstract_hits
        scored.append({
            "paper_id": pid,
            "title": title,
            "abstract": abstract[:300],
            "score": score,
            "matched_terms": list(matched),
        })

    scored.sort(key=lambda x: x["score"], reverse=True)
    return scored[:top_k]


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


# =============================================================================
# RESEARCH LOG — per-paper research notes stored in ~/.ai_research_os/gene_pool/research_log.jsonl
# =============================================================================

def add_research_note(
    paper_id: str,
    note: str,
    tags: Optional[List[str]] = None,
) -> bool:
    """Append a research note to the log file.

    Writes to ~/.ai_research_os/gene_pool/research_log.jsonl
    Each entry: {timestamp, paper_id, note, tags}
    """
    try:
        log_path = Path.home() / ".ai_research_os" / "gene_pool" / "research_log.jsonl"
        log_path.parent.mkdir(parents=True, exist_ok=True)
        entry = {
            "timestamp": datetime.now().isoformat(),
            "paper_id": paper_id,
            "note": note,
            "tags": tags or [],
        }
        with open(log_path, "a", encoding="utf-8") as f:
            f.write(json.dumps(entry, ensure_ascii=False) + "\n")
        return True
    except Exception:
        return False


def get_research_notes(
    paper_id: Optional[str] = None,
    limit: int = 20,
) -> List[Dict[str, Any]]:
    """Read research notes, optionally filtered by paper_id."""
    try:
        log_path = Path.home() / ".ai_research_os" / "gene_pool" / "research_log.jsonl"
        if not log_path.exists():
            return []

        notes = []
        with open(log_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                try:
                    entry = json.loads(line)
                except Exception:
                    continue
                if paper_id and entry.get("paper_id") != paper_id:
                    continue
                notes.append(entry)

        # Sort by timestamp descending
        notes.sort(key=lambda x: x.get("timestamp", ""), reverse=True)
        return notes[:limit]
    except Exception:
        return []


def render_research_log(paper_id: Optional[str] = None) -> str:
    """Return HTML timeline of research notes.

    Each note: timestamp (top-right), paper title, note content, tags as small pills.
    Sorted newest first. Uses font-family: var(--font-display).
    """
    notes = get_research_notes(paper_id=paper_id, limit=50)

    if not notes:
        empty_msg = "No research notes yet."
        if paper_id:
            empty_msg = f"No notes for paper {paper_id} yet."
        return f"""
        <div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
          <div style="font-size:48px;margin-bottom:12px;">📝</div>
          <div style="font-size:15px;font-weight:600;margin-bottom:6px;">{empty_msg}</div>
          <div style="font-size:13px;">Add notes from a paper detail page.</div>
        </div>"""

    # Fetch paper titles for display
    paper_titles: Dict[str, str] = {}
    if paper_id is None:
        # Collect all paper_ids to bulk-fetch titles
        seen_ids = list(dict.fromkeys(n.get("paper_id", "") for n in notes if n.get("paper_id")))
        if seen_ids:
            try:
                from db.database import Database
                db = Database()
                db.init()
                for pid in seen_ids:
                    p = db.get_paper(pid)
                    if p:
                        paper_titles[pid] = p.title
            except Exception:
                pass

    cards = ""
    for n in notes:
        ts = n.get("timestamp", "")
        # Format: YYYY-MM-DD HH:MM
        date_str = ts[:16].replace("T", " ") if ts else "—"
        pid = n.get("paper_id", "")
        title = paper_titles.get(pid, pid[:20] if pid else "—")
        note_text = n.get("note", "")
        tags = n.get("tags", [])

        tags_html = ""
        if tags:
            tags_html = "".join(
                f"<span style='display:inline-block;background:#e8f0fe;color:#1a73e8;padding:2px 8px;border-radius:12px;font-size:11px;margin:2px;'>{t}</span>"
                for t in tags
            )

        cards += f"""
        <div style="background:#fff;border:1px solid #e8e8e8;border-radius:12px;padding:16px 20px;margin-bottom:12px;box-shadow:0 2px 6px rgba(0,0,0,0.05);position:relative;font-family:var(--font-display);">
          <div style="position:absolute;top:12px;right:16px;font-size:11px;color:#aaa;">{date_str}</div>
          <div style="font-size:12px;color:#888;margin-bottom:6px;">{title}</div>
          <div style="font-size:14px;color:#222;line-height:1.6;margin-bottom:8px;">{note_text}</div>
          {('<div style="display:flex;flex-wrap:wrap;gap:4px;">' + tags_html + '</div>' if tags_html else '')}
        </div>"""

    filter_label = ""
    if paper_id:
        filter_label = f" for <strong>{paper_titles.get(paper_id, paper_id[:20])}</strong>"

    return f"""
    <style>
    .rl-header {{ font-family: var(--font-display); font-size: 14px; color: #888; margin-bottom: 16px; }}
    </style>
    <div class="rl-header">{len(notes)} note(s){filter_label}</div>
    <div>{cards}</div>"""


# =============================================================================
# CONFIDENCE CALIBRATION TRACKING
# =============================================================================

def render_confidence_calibration() -> str:
    """Return HTML displaying confidence calibration statistics.

    Reads embodied_timeline.jsonl, groups predictions by confidence bucket,
    and shows how many were verified vs contradicted by subsequent analyses.
    Output: HTML table + bar chart.

    Verification: same representation_type appears again (repeated confirmation).
    Contradiction: a different representation_type appears for the same/related context.
    """
    import json as _json

    timeline_path = Path.home() / ".ai_research_os" / "gene_pool" / "embodied_timeline.jsonl"
    if not timeline_path.exists():
        return """
        <div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
          <div style="font-size:48px;margin-bottom:12px;">📊</div>
          <div style="font-size:15px;font-weight:600;margin-bottom:6px;">No Timeline Data</div>
          <div style="font-size:13px;">Run embodied planning analyses first.</div>
        </div>"""

    entries = []
    try:
        with open(timeline_path, encoding="utf-8") as f:
            for line in f:
                line = line.strip()
                if not line:
                    continue
                entries.append(_json.loads(line))
    except Exception:
        return "<p>Error reading timeline data.</p>"

    if not entries:
        return """
        <div style="text-align:center;padding:60px 20px;color:#888;font-family:var(--font-display);">
          <div style="font-size:48px;margin-bottom:12px;">📊</div>
          <div style="font-size:15px;font-weight:600;margin-bottom:6px;">No Entries Yet</div>
          <div style="font-size:13px;">Embodied planning analyses will appear here.</div>
        </div>"""

    # Sort by timestamp
    entries.sort(key=lambda x: x.get("timestamp", ""))

    # Confidence buckets
    buckets = [
        ("0-20%", 0.0, 0.2),
        ("20-40%", 0.2, 0.4),
        ("40-60%", 0.4, 0.6),
        ("60-80%", 0.6, 0.8),
        ("80-100%", 0.8, 1.01),
    ]

    bucket_stats: Dict[str, Dict[str, int]] = {
        b[0]: {"verified": 0, "contradicted": 0, "unknown": 0}
        for b in buckets
    }

    # For each entry, look at subsequent entries with same/similar title keywords
    for i, entry in enumerate(entries):
        conf = entry.get("confidence", 0.5)
        rt = entry.get("representation_type", "unknown")
        title_words = set((entry.get("paper_title", "") + " " + entry.get("gap_title", "")).lower().split())

        # Find bucket
        bucket_label = "unknown"
        for label, lo, hi in buckets:
            if lo <= conf < hi:
                bucket_label = label
                break

        # Look ahead for subsequent entries (within 30 days)
        verified = False
        contradicted = False
        _entry_ts = entry.get("timestamp", "")

        for j in range(i + 1, len(entries)):
            later = entries[j]
            later_rt = later.get("representation_type", "unknown")
            later_title_words = set((later.get("paper_title", "") + " " + later.get("gap_title", "")).split())
            overlap = title_words & later_title_words

            if overlap and len(overlap) >= 2:  # significant keyword overlap
                if later_rt == rt:
                    verified = True
                elif later_rt != rt and later_rt != "unknown":
                    contradicted = True
                break  # use first meaningful subsequent entry

        if verified:
            bucket_stats[bucket_label]["verified"] += 1
        elif contradicted:
            bucket_stats[bucket_label]["contradicted"] += 1
        else:
            bucket_stats[bucket_label]["unknown"] += 1

    # Build HTML table
    table_rows = ""
    for label, lo, hi in buckets:
        stats = bucket_stats[label]
        total = stats["verified"] + stats["contradicted"] + stats["unknown"]
        v = stats["verified"]
        c = stats["contradicted"]
        u = stats["unknown"]

        # Bar chart
        max_val = max(total, 1)
        bar_w = 300
        v_w = int(v / max_val * bar_w)
        c_w = int(c / max_val * bar_w)
        u_w = int(u / max_val * bar_w)

        table_rows += f"""
        <tr>
          <td style="padding:8px 12px;font-family:var(--font-display);font-size:13px;font-weight:600;color:#555;">{label}</td>
          <td style="padding:8px 12px;">
            <div style="display:flex;height:18px;border-radius:3px;overflow:hidden;background:#f0f0f0;width:{bar_w}px;">
              <div style="width:{v_w}px;background:#4caf50;" title="verified:{v}"></div>
              <div style="width:{c_w}px;background:#e53935;" title="contradicted:{c}"></div>
              <div style="width:{u_w}px;background:#aaa;" title="unknown:{u}"></div>
            </div>
          </td>
          <td style="padding:8px 12px;text-align:center;font-family:var(--font-display);font-size:12px;color:#888;">{total}</td>
          <td style="padding:8px 12px;font-family:var(--font-display);font-size:12px;">
            <span style="color:#4caf50;">&#10003;{v}</span>
            <span style="color:#e53935;margin-left:6px;">&#10007;{c}</span>
            <span style="color:#aaa;margin-left:6px;">?{u}</span>
          </td>
        </tr>"""

    return f"""
    <div style="font-family:var(--font-display);">
      <div style="font-size:14px;font-weight:700;color:#555;margin-bottom:12px;">Confidence Calibration — Embodied Planning</div>
      <table style="width:100%;border-collapse:collapse;margin-bottom:16px;">
        <thead>
          <tr style="background:#f8f8f8;">
            <th style="padding:8px 12px;text-align:left;font-size:12px;color:#888;text-transform:uppercase;letter-spacing:0.5px;">Confidence</th>
            <th style="padding:8px 12px;text-align:left;font-size:12px;color:#888;text-transform:uppercase;letter-spacing:0.5px;">Distribution</th>
            <th style="padding:8px 12px;text-align:center;font-size:12px;color:#888;text-transform:uppercase;letter-spacing:0.5px;">Total</th>
            <th style="padding:8px 12px;text-align:left;font-size:12px;color:#888;text-transform:uppercase;letter-spacing:0.5px;">Breakdown</th>
          </tr>
        </thead>
        <tbody>
          {table_rows}
        </tbody>
      </table>
      <div style="font-size:11px;color:#aaa;margin-top:8px;">
        &#10003; verified = same representation_type confirmed later &nbsp;|&nbsp;
        &#10007; contradicted = different representation_type appeared later &nbsp;|&nbsp;
        ? unknown = no subsequent entry with keyword overlap
      </div>
    </div>"""


# =============================================================================
# HYPOTHESIS GENERATION FROM CONTRADICTIONS
# =============================================================================

def generate_hypothesis_from_contradiction(contradiction_pair: dict) -> str:
    """Generate a new hypothesis suggestion from a contradiction pair.

    Reads both papers' information and uses simple rules to generate a hypothesis:
    - If paper A says discrete/effective and paper B says continuous/ineffective
    - Hypothesis: "A+B hybrid method may combine both advantages"

    Returns a research question string.
    """
    _paper_a_title = contradiction_pair.get("paper_a_title", "")
    _paper_b_title = contradiction_pair.get("paper_b_title", "")
    rep_a = contradiction_pair.get("representation_a", "unknown")
    rep_b = contradiction_pair.get("representation_b", "unknown")
    effect_a = contradiction_pair.get("effectiveness_a", "")
    effect_b = contradiction_pair.get("effectiveness_b", "")
    _paper_a_id = contradiction_pair.get("paper_a_id", "")
    _paper_b_id = contradiction_pair.get("paper_b_id", "")

    # Normalise
    rep_a_lower = rep_a.lower().strip()
    rep_b_lower = rep_b.lower().strip()
    eff_a_lower = effect_a.lower().strip()
    eff_b_lower = effect_b.lower().strip()

    # Rule patterns
    patterns = [
        # (rep_a, rep_b, eff_a, eff_b, hypothesis_template)
        ("discrete", "continuous", "effective", "ineffective",
         f"探索离散的符号化推理与连续的潜空间推理的混合架构，结合两者的精确性与鲁棒性优势"),
        ("continuous", "discrete", "effective", "ineffective",
         f"探索连续的潜空间推理与离散的符号化推理的混合架构，结合两者的表达力与可解释性"),
        ("discrete", "continuous", "ineffective", "effective",
         f"探索混合架构能否结合离散的组合泛化能力与连续的空间推理能力"),
        ("continuous", "discrete", "ineffective", "effective",
         f"探索混合架构能否融合连续表示的平滑性与离散表示的结构化优势"),
        ("discrete", "hybrid", "effective", "ineffective",
         f"离到混合的渐进式过渡：能否在离散推理基础上引入连续层提升鲁棒性？"),
        ("hybrid", "discrete", "ineffective", "effective",
         f"混合到离散的模块化拆解：混合架构中哪些连续组件可以离散化而不损失性能？"),
    ]

    hypothesis = None
    for pat in patterns:
        if (rep_a_lower == pat[0] and rep_b_lower == pat[1] and
                eff_a_lower == pat[2] and eff_b_lower == pat[3]):
            hypothesis = pat[4]
            break

    # Default fallback
    if not hypothesis:
        if rep_a_lower != rep_b_lower:
            hypothesis = (
                f"探索{rep_a}与{rep_b}的协同机制："
                f"能否设计分层架构结合两者的优势，"
                f"在高效性与泛化能力间取得更好平衡？"
            )
        else:
            hypothesis = (
                f"针对同一{rep_a}表示的内部变体，"
                f"探索其在不同任务尺度下的适应性边界。"
            )

    return hypothesis


def append_hypothesis_to_roadmap(
    hypothesis: str,
    paper_id_a: str,
    paper_id_b: str,
) -> bool:
    """Append a hypothesis to ROADMAP.md under ### Pending Hypotheses.

    Creates the section if it doesn't exist.
    """
    try:
        roadmap_path = Path("D:/OpenClaw/workspace/80-PROJECTS/ai_research_os/ROADMAP.md")
        if not roadmap_path.exists():
            return False

        content = roadmap_path.read_text(encoding="utf-8")
        marker = f"- [ ] HYPOTHESIS: {hypothesis} (from: {paper_id_a} vs {paper_id_b})"

        # Check not already present
        if marker in content:
            return True  # Already present, treat as success

        # Find or create ### Pending Hypotheses section
        pending_marker = "### Pending Hypotheses"
        if pending_marker in content:
            # Append after the section heading
            lines = content.split("\n")
            new_lines = []
            for i, line in enumerate(lines):
                new_lines.append(line)
                if line.strip() == pending_marker:
                    # Find the end of this section (next ### or end of file)
                    j = i + 1
                    while j < len(lines) and not lines[j].startswith("### "):
                        j += 1
                    # Insert before the next section (or end)
                    insert_pos = j if j < len(lines) else len(lines)
                    new_lines.insert(insert_pos, f"- [ ] HYPOTHESIS: {hypothesis} (from: {paper_id_a} vs {paper_id_b})")
                    content = "\n".join(new_lines)
                    break
        else:
            # Append at end with new section
            content += f"\n\n{pending_marker}\n- [ ] HYPOTHESIS: {hypothesis} (from: {paper_id_a} vs {paper_id_b})\n"

        roadmap_path.write_text(content, encoding="utf-8")
        return True
    except Exception:
        return False


# =============================================================================
# Shared embodied planning analysis pipeline
# =============================================================================

def run_embodied_analysis(
    paper_ids: List[str],
    db=None,
    save_to_pool: bool = True,
    gap_type: str = "embodied_planning",
) -> Dict[str, Any]:
    """Run embodied planning analysis on a list of paper IDs.

    Shared by auto-scan (save=True) and batch (save=False).
    Returns {analyzed, contradictions, type_counts, total_analyzed}.
    """
    if db is None:
        from db.database import Database
        db = Database()
        db.init()

    analyzed: List[Dict[str, Any]] = []
    contradictions: List[Dict[str, Any]] = []
    type_counts: Dict[str, int] = {"discrete": 0, "continuous": 0, "hybrid": 0, "unknown": 0}

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
        rep_type = r.get("representation_type", "unknown")
        type_counts[rep_type] = type_counts.get(rep_type, 0) + 1
        entry = {
            "paper_id": pid,
            "title": paper.title[:60],
            "representation_type": rep_type,
            "confidence": r.get("confidence", 0),
            "saved": "error" not in r,
        }
        if r.get("contradiction_with"):
            entry["contradiction_with"] = r["contradiction_with"]
            entry["contradiction_type"] = r.get("contradiction_type")
            contradictions.append(entry)
        analyzed.append(entry)

    total = len(analyzed)
    trend = max(type_counts, key=type_counts.get) if total > 0 else "unknown"
    trend_pct = type_counts[trend] / total if total > 0 else 0

    return {
        "analyzed": analyzed,
        "contradictions": contradictions,
        "type_counts": type_counts,
        "total_analyzed": total,
        "trend": trend,
        "trend_pct": trend_pct,
    }
