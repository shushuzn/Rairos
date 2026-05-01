# AI Research OS

<div align="center">
  <img src="logo_hero.png" width="800" alt="AI Research OS Demo"/>
</div>

**AI 연구자 위한 자기 진화형 연구 운영 체제**

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

## 개요

AI Research OS는 여러분의 사용 패턴에서 학습하는 **자기 진화형 연구 시스템**입니다. 단순한 논문 관리자가 아니라 시간이 지나면서 점점 더 똑똑해지는 연구 파트너입니다.

논문(arXiv URL, DOI 또는 PDF)을 입력하면 **P-Note**, **C-Note**, **Radar 항목**, **Timeline 항목**을 얻을 수 있습니다——모두 구조화되고, 태그가 지정되며, 상호 연결되어 있습니다.

| 입력 | 출력 |
|---|---|
| arXiv URL/ID | P-Note + C-Note + Radar + Timeline |
| DOI | P-Note + C-Note + Radar + Timeline |
| 로컬 PDF | P-Note + C-Note + Radar + Timeline |
| 스캔 PDF | 동일 (OCR 경유) |

이것은 **PDF 관리자가 아닙니다**. **자기 진화형 시스템**입니다:
- 연구 패턴에서 학습
- 시간이 지남에 따라 답변 개선
- 특정 도메인에 적응

## 핵심 기능

| 기능 | 설명 |
|---------|-------------|
| `airos import` | arXiv, DOI, PDF에서 논문 가져오기 |
| `airos chat` | RAG 기반 Q&A |
| `airos slides` | 프레젠테이션 자동 생성 |
| `airos kg` | 지식 그래프 시각화 |
| Evolution | Gene/Capsule 패턴을 통한 자기 개선 |

## 빠른 시작

```bash
pip install ai-research-os
airos-cli 2601.00155 --tags LLM,Agent
```

 완료——몇 초 만에 논문을 가져옵니다.

### 한 줄, 세 가지 입력

```bash
airos-cli 2601.00155                          # arXiv ID
airos-cli 10.48550/arXiv.2601.00155           # DOI
airos-cli --pdf paper.pdf --tags RAG            # 로컬 PDF
airos-cli --pdf scanned.pdf --ocr --ocr-lang chi_sim+eng   # 스캔 PDF
```

### 세 가지 핵심 명령

```bash
airos-cli import 2601.00155 10.1038/nature12373   # DB에 논문 추가
airos-cli search "attention mechanism" --tag LLM    # 논문 검색
airos-cli research "RLHF alignment" --limit 5       # 자율 연구 루프
```

### AI 초안 (선택)

```bash
export OPENAI_API_KEY="***"
export OPENAI_BASE_URL="https://dashscope.aliyuncs.com/compatible-mode/v1"
airos-cli 2601.00155 --tags LLM --ai
```

전체 설정은 [API_CONFIG.md](API_CONFIG.md)를 참조하세요.

## 연구 트리

논문은 12개 디렉토리로 구성됩니다:

```
00-Radar/            토픽 열도 추적
01-Foundations/      기초 논문
02-Models/           모델 논문
03-Training/         훈련 방법
04-Scaling/          스케일링 법칙
05-Alignment/        정렬 연구
06-Agents/           에이전트 시스템
07-Infrastructure/  인프라
08-Optimization/     최적화 기술
09-Evaluation/       평가 방법
10-Applications/    응용 연구
11-Future-Directions/
```

## 설치

```bash
pip install ai-research-os
```

또는 소스에서 설치:

```bash
git clone https://github.com/shushuzn/ai_research_os.git
cd ai_research_os
pip install -e .
```

## 문서

전체 문서는 [ai-research-os.readthedocs.io](https://ai-research-os.readthedocs.io/)를 참조하세요.

| 문서 | 설명 |
|-----|-------------|
| [Architecture](docs/architecture.md) | 시스템 설계 및 모듈 개요 |
| [Configuration](docs/configuration.md) | LLM, DB, 검색, 도구 구성 |
| [Benchmarks](docs/benchmarks.md) | 성능 지표 및 테스트 커버리지 |
| [Contributing](CONTRIBUTING.md) | 이 프로젝트에 기여하는 방법 |
| [Roadmap](ROADMAP.md) | 프로젝트 로드맵 및 향후 계획 |

## 라이선스

GPL-3.0-or-later. 자세한 내용은 [LICENSE](LICENSE)를 참조하세요.
