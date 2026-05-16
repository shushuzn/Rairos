# Deep Research: Long-Context LLM Inference Efficiency

**Generated:** 2026-05-05T16:10:48
**Papers Analyzed:** 20
**Gene Pool Capsules:** 123 (avg 0.643)
**Gaps Detected:** 1

---

## Paper Inventory

| Year | Title | ID |
|------|-------|----|
| 2026 | Focus-dLLM: Accelerating Long-Context Diffusion LLM Inferenc | 2602.02159v1 |
| 2025 | AlayaDB: The Data Foundation for Efficient and Effective Lon | 2504.10326v1 |
| 2025 | Long Context Tuning for Video Generation | 2503.10589v1 |
| 2025 | Thus Spake Long-Context Large Language Model | 2502.17129v2 |
| 2025 | SlimInfer: Accelerating Long-Context LLM Inference via Dynam | 2508.06447v2 |
| 2025 | Beyond In-Context Learning: Aligning Long-form Generation of | 2506.01265v1 |
| 2025 | Learning Long-Context Diffusion Policies via Past-Token Pred | 2505.09561v2 |
| 2025 | FinLFQA: Evaluating Attributed Text Generation of LLMs in Fi | 2510.06426v1 |
| 2024 | MultiHop-RAG: Benchmarking Retrieval-Augmented Generation fo | 2401.15391v1 |
| 2023 | LM-Infinite: Zero-Shot Extreme Length Generalization for Lar | 2308.16137 |
| 2023 | Efficient Streaming Language Models with Attention Sinks | 2309.17453 |
| 2023 | Giraffe: Adventures in Expanding Context Lengths in LLMs | 2308.10882 |
| 2023 | LongNet: Scaling Transformers to 1,000,000,000 Tokens | 2307.02486 |
| 2023 | RECOMP: Improving Retrieval-Augmented LMs with Compression a | 2310.04408 |
| 2023 | Asymptotic theory for Bayesian inference and prediction: fro | 2310.06720 |
| 2017 | Attention Is All You Need | 2305.00001 |
| ? | Dilated Neighborhood Attention Transformer | 2209.15001v3 |
| ? | Transformer-based Personalized Attention Mechanism for Medic | 2206.03003v2 |
| ? | Transformer: Attention is All You Need | 2301.00005 |
| ? | A Randomized Controlled Trial and Pilot of Scout: an LLM-Bas | 2604.26953 |

## Paper Details


### [2026] Focus-dLLM: Accelerating Long-Context Diffusion LLM Inference via Confidence-Guided Context Focusing
- **ID:** `2602.02159v1`
- **Abstract:** ...


### [2025] AlayaDB: The Data Foundation for Efficient and Effective Long-context LLM Inference
- **ID:** `2504.10326v1`
- **Abstract:** ...


### [2025] Long Context Tuning for Video Generation
- **ID:** `2503.10589v1`
- **Abstract:** ...


### [2025] Thus Spake Long-Context Large Language Model
- **ID:** `2502.17129v2`
- **Abstract:** ...


### [2025] SlimInfer: Accelerating Long-Context LLM Inference via Dynamic Token Pruning
- **ID:** `2508.06447v2`
- **Abstract:** ...


### [2025] Beyond In-Context Learning: Aligning Long-form Generation of Large Language Models via Task-Inherent Attribute Guidelines
- **ID:** `2506.01265v1`
- **Abstract:** ...


### [2025] Learning Long-Context Diffusion Policies via Past-Token Prediction
- **ID:** `2505.09561v2`
- **Abstract:** ...


### [2025] FinLFQA: Evaluating Attributed Text Generation of LLMs in Financial Long-Form Question Answering
- **ID:** `2510.06426v1`
- **Abstract:** ...


### [2024] MultiHop-RAG: Benchmarking Retrieval-Augmented Generation for Multi-Hop Queries
- **ID:** `2401.15391v1`
- **Abstract:** ...


### [2023] LM-Infinite: Zero-Shot Extreme Length Generalization for Large Language Models
- **ID:** `2308.16137`
- **Abstract:** ...


### [2023] Efficient Streaming Language Models with Attention Sinks
- **ID:** `2309.17453`
- **Abstract:** ...


### [2023] Giraffe: Adventures in Expanding Context Lengths in LLMs
- **ID:** `2308.10882`
- **Abstract:** ...


### [2023] LongNet: Scaling Transformers to 1,000,000,000 Tokens
- **ID:** `2307.02486`
- **Abstract:** ...


### [2023] RECOMP: Improving Retrieval-Augmented LMs with Compression and Selective Augmentation
- **ID:** `2310.04408`
- **Abstract:** ...


### [2023] Asymptotic theory for Bayesian inference and prediction: from the ordinary to a conditional Peaks-Over-Threshold method
- **ID:** `2310.06720`
- **Abstract:** ...


### [2017] Attention Is All You Need
- **ID:** `2305.00001`
- **Abstract:** ...


### [] Dilated Neighborhood Attention Transformer
- **ID:** `2209.15001v3`
- **Abstract:** ...


### [] Transformer-based Personalized Attention Mechanism for Medical Images with Clinical Records
- **ID:** `2206.03003v2`
- **Abstract:** ...


### [] Transformer: Attention is All You Need
- **ID:** `2301.00005`
- **Abstract:** ...


### [] A Randomized Controlled Trial and Pilot of Scout: an LLM-Based EHR Search and Synthesis Platform
- **ID:** `2604.26953`
- **Abstract:** ...


## Gap Analysis

**1 gaps found across 22 papers.**

- **Severity:** GapSeverity.MEDIUM | **Confidence:** ?
  基于关键词 'however|but|in contrast|on the contrary' 发现的潜在研究空白


## Gene Pool Context

The gene pool contains **123 capsules** (avg score 0.643).
Related capsules to long-context topic: **13**

| Metric | Value |
|--------|-------|
| exploration_gap | 31 |
| method_gap | 17 |
| application_gap | 16 |
| efficiency_gap | 13 |
| unexplored_application | 12 |
| scalability_issue | 8 |
| evaluation_gap | 7 |
| method_limitation | 6 |
| theoretical_gap | 6 |
| theory_gap | 4 |

| Total capsules | 123 |
| Avg quality | 0.643 |
| Generations | [0, 1] |

## Research Directions

Based on the paper corpus and gap analysis, key research directions:

### 1. Ultra-long context (>1M tokens) without quadratic attention cost
Approaches like LongNet (dilated attention) and SlimInfer push boundaries, but no unified theory of context-length scaling exists.

### 2. Hardware-aware inference optimization
Focus-dLLM and SlimInfer target specific bottlenecks; cross-hardware abstraction is still open.

### 3. Long-context evaluation beyond perplexity
Few standardized benchmarks exist for generation quality at 100K+ context lengths.

### 4. Adaptive context management
Streaming LLM and LM-Infinite show windowing works, but dynamic allocation based on content importance is unexplored.

### 5. Long-context for real-time applications
Current latency profiles (seconds for 100K tokens) prevent interactive use — sub-second is the gap.
