"""Unit tests for ai_generate_cnote_draft (mocked LLM calls)."""

class TestAiGenerateCnoteDraft:
    def test_ai_generate_cnote_draft_calls_llm(self, monkeypatch):
        import ai_research_os as airo

        def mock_call(base_url, api_key, model, system_prompt, user_prompt):
            assert "RAG" in user_prompt
            assert "核心定义" in user_prompt
            return "> AI Draft\n\n## 核心定义\n\nRAG 是检索增强生成。"

        monkeypatch.setenv("OPENAI_API_KEY", "test-key")
        monkeypatch.setattr("ai_research_os.call_llm_chat_completions", mock_call)

        pnotes = [
            {
                "title": "RAG Paper",
                "authors": ["Author A"],
                "year": "2024",
                "source": "arxiv",
                "uid": "2301.00001",
                "abstract": "Retrieval Augmented Generation",
                "tags": ["RAG"],
            }
        ]
        result = airo.ai_generate_cnote_draft(
            concept="RAG",
            pnotes=pnotes,
            api_key="test-key",
            base_url="https://example.com",
            model="test-model",
        )
        assert "核心定义" in result
        assert "检索增强" in result

    def test_ai_generate_cnote_draft_includes_required_sections(self, monkeypatch):
        import ai_research_os as airo

        def mock_call(base_url, api_key, model, system_prompt, user_prompt):
            sections = [
                "核心定义",
                "产生背景",
                "技术本质",
                "常见实现路径",
                "优势",
                "局限",
                "与其他思想的关系",
                "代表论文",
                "演化时间线",
                "未来趋势",
            ]
            content = "> AI Draft\n\n## " + "\n\n## ".join(sections)
            return content

        monkeypatch.setenv("OPENAI_API_KEY", "test-key")
        monkeypatch.setattr("ai_research_os.call_llm_chat_completions", mock_call)

        result = airo.ai_generate_cnote_draft(
            concept="Agent",
            pnotes=[
                {
                    "title": "Agent Paper",
                    "authors": [],
                    "year": "2024",
                    "source": "arxiv",
                    "uid": "2401.00001",
                    "abstract": "AI Agent",
                    "tags": ["Agent"],
                }
            ],
            api_key="test-key",
            base_url="https://example.com",
            model="test-model",
        )
        for section in [
            "核心定义",
            "产生背景",
            "技术本质",
            "常见实现路径",
            "优势",
            "局限",
            "与其他思想的关系",
            "代表论文",
            "演化时间线",
            "未来趋势",
        ]:
            assert f"## {section}" in result, f"Missing section: {section}"

    def test_ai_generate_cnote_draft_multiple_pnotes(self, monkeypatch):
        import ai_research_os as airo

        captured_prompts = {}

        def mock_call(base_url, api_key, model, system_prompt, user_prompt):
            captured_prompts["user"] = user_prompt
            return "> AI Draft\n\n## 核心定义\n\nMulti-paper concept."

        monkeypatch.setenv("OPENAI_API_KEY", "test-key")
        monkeypatch.setattr("ai_research_os.call_llm_chat_completions", mock_call)

        pnotes = [
            {
                "title": "Paper A",
                "authors": ["Author A"],
                "year": "2023",
                "source": "arxiv",
                "uid": "2301.00001",
                "abstract": "Abstract A",
                "tags": ["RAG"],
            },
            {
                "title": "Paper B",
                "authors": ["Author B"],
                "year": "2024",
                "source": "arxiv",
                "uid": "2401.00002",
                "abstract": "Abstract B",
                "tags": ["RAG"],
            },
        ]
        airo.ai_generate_cnote_draft(
            concept="RAG",
            pnotes=pnotes,
            api_key="test-key",
            base_url="https://example.com",
            model="test-model",
        )
        assert "Paper A" in captured_prompts["user"]
        assert "Paper B" in captured_prompts["user"]
        assert "2301.00001" in captured_prompts["user"]
        assert "2401.00002" in captured_prompts["user"]
