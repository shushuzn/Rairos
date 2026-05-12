"""Tests for llm/research/gap_configs.py — Q1-Q10 GAP_ANALYZER_CONFIGS."""

from llm.research.gap_configs import GAP_ANALYZER_CONFIGS


class TestGapAnalyzerConfigs:
    """Test GAP_ANALYZER_CONFIGS structure and validity."""

    def test_all_configs_have_required_fields(self):
        """All configs must have gap_type, result_fields, and prompt_template."""
        required = {"gap_type", "result_fields", "prompt_template"}
        for name, config in GAP_ANALYZER_CONFIGS.items():
            missing = required - set(config.keys())
            assert not missing, f"Config '{name}' missing fields: {missing}"

    def test_gap_type_matches_config_key(self):
        """Each config's gap_type should match its dictionary key."""
        for name, config in GAP_ANALYZER_CONFIGS.items():
            assert config["gap_type"] == name, (
                f"Config key '{name}' != gap_type '{config['gap_type']}'"
            )

    def test_result_fields_contains_required(self):
        """All configs must have gap_title and summary in result_fields."""
        for name, config in GAP_ANALYZER_CONFIGS.items():
            fields = config["result_fields"]
            assert "gap_title" in fields, f"Config '{name}' missing 'gap_title'"
            assert "summary" in fields, f"Config '{name}' missing 'summary'"

    def test_prompt_templates_not_empty(self):
        """All prompt templates must be non-empty strings."""
        for name, config in GAP_ANALYZER_CONFIGS.items():
            template = config["prompt_template"]
            assert isinstance(template, str), f"Config '{name}' prompt is not a string"
            assert len(template) > 50, f"Config '{name}' prompt too short (<50 chars)"

    def test_prompt_templates_have_paper_placeholders(self):
        """All prompts must contain Title, Authors, Abstract placeholders."""
        for name, config in GAP_ANALYZER_CONFIGS.items():
            template = config["prompt_template"]
            assert "{title}" in template.lower() or "Title:" in template, (
                f"Config '{name}' missing {{title}} placeholder"
            )
            assert "{authors}" in template.lower() or "Authors:" in template, (
                f"Config '{name}' missing {{authors}} placeholder"
            )
            assert "{abstract}" in template.lower() or "Abstract:" in template, (
                f"Config '{name}' missing {{abstract}} placeholder"
            )

    def test_keywords_are_non_empty_lists(self):
        """All configs must have non-empty keywords list."""
        for name, config in GAP_ANALYZER_CONFIGS.items():
            keywords = config.get("keywords", [])
            assert isinstance(keywords, list), f"Config '{name}' keywords not a list"
            assert len(keywords) > 0, f"Config '{name}' has empty keywords"

    def test_all_10_configs_present(self):
        """All 10 Q1-Q10 configs must be present."""
        expected_keys = {
            "embodied_planning",
            "rl_efficiency",
            "reasoning_scaling",
            "sim_to_real",
            "planning_control",
            "representation_learning",
            "rl_pretraining",
            "benchmark_coverage",
            "architecture_agnostic",
            "human_ai_collaboration",
        }
        actual_keys = set(GAP_ANALYZER_CONFIGS.keys())
        missing = expected_keys - actual_keys
        extra = actual_keys - expected_keys
        assert not missing, f"Missing configs: {missing}"
        assert not extra, f"Extra configs: {extra}"

    def test_no_duplicate_gap_types(self):
        """No two configs should have the same gap_type value."""
        gap_types = [c["gap_type"] for c in GAP_ANALYZER_CONFIGS.values()]
        assert len(gap_types) == len(set(gap_types)), (
            f"Duplicate gap_type values: {set([g for g in gap_types if gap_types.count(g) > 1])}"
        )
