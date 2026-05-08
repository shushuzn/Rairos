"""Tests for arxiv_alert_channels, claude_cli, multi_researcher."""
import pytest
from llm.arxiv_alert_channels import (
    ChannelConfig, get_channels, update_channel,
    render_channels_html, match_paper_to_channels,
)
from llm.claude_cli import ClaudeCLIClient, get_claude_cli_client, is_claude_cli_available
from llm.multi_researcher import get_researchers


class TestChannelConfig:
    def test_fields(self):
        c = ChannelConfig(
            id="ch1", name="ML Alerts", categories=["cs.AI"],
            keywords=["transformer"], priority="high", enabled=True,
        )
        assert c.id == "ch1"
        assert c.priority == "high"


class TestArxivAlertChannels:
    def test_get_channels(self):
        result = get_channels()
        assert isinstance(result, list)

    def test_update_channel(self):
        ok = update_channel("fake-id", {"enabled": False})
        assert isinstance(ok, bool)

    def test_render_channels_html(self):
        result = render_channels_html()
        assert isinstance(result, str)
        assert "<" in result

    def test_match_paper_to_channels(self):
        paper = {"title": "Attention is All You Need", "abstract": "We propose transformer"}
        result = match_paper_to_channels(paper)
        assert isinstance(result, list)


class TestClaudeCLI:
    def test_client_init(self):
        client = ClaudeCLIClient(cli_path="/fake/path")
        assert client.cli_path == "/fake/path"

    def test_get_client(self):
        client = get_claude_cli_client()
        assert client is not None

    def test_is_available(self):
        result = is_claude_cli_available()
        assert isinstance(result, bool)


class TestMultiResearcher:
    def test_get_researchers(self):
        result = get_researchers()
        assert isinstance(result, list)
