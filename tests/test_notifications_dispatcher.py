"""Tests for notifications/dispatcher.py."""

from __future__ import annotations

from unittest.mock import MagicMock, patch


from notifications.dispatcher import (
    DiscordRenderer,
    FeishuRenderer,
    GapAlertPayload,
    NotificationCenter,
    NotificationType,
    ParadigmShiftPayload,
    Platform,
    WebhookConfig,
    WebhookDispatcher,
)


# ─── Enums ────────────────────────────────────────────────────────────────────


class TestNotificationType:
    def test_values(self):
        assert NotificationType.GAP_ALERT.value == "gap_alert"
        assert NotificationType.PARADIGM_SHIFT.value == "paradigm_shift"
        assert NotificationType.PAPER_INGESTED.value == "paper_ingested"
        assert NotificationType.RESEARCH_COMPLETE.value == "research_complete"
        assert NotificationType.CONTRADICTION_DETECTED.value == "contradiction_detected"
        assert NotificationType.TOPIC_SUGGESTION.value == "topic_suggestion"

    def test_member_count(self):
        assert len(NotificationType) == 6


class TestPlatform:
    def test_values(self):
        assert Platform.DISCORD.value == "discord"
        assert Platform.FEISHU.value == "feishu"
        assert Platform.GENERIC.value == "generic"

    def test_member_count(self):
        assert len(Platform) == 3


# ─── Dataclasses ──────────────────────────────────────────────────────────────


class TestWebhookConfig:
    def test_defaults(self):
        cfg = WebhookConfig(url="https://example.com/webhook")
        assert cfg.url == "https://example.com/webhook"
        assert cfg.platform == Platform.GENERIC
        assert cfg.enabled is True
        assert cfg.label == ""
        assert cfg.secret == ""

    def test_explicit_values(self):
        cfg = WebhookConfig(
            url="https://discord.com/webhook/123",
            platform=Platform.DISCORD,
            enabled=True,
            label="my-discord",
            secret="hmac-secret",
        )
        assert cfg.platform == Platform.DISCORD
        assert cfg.label == "my-discord"
        assert cfg.secret == "hmac-secret"


class TestGapAlertPayload:
    def test_required_fields(self):
        payload = GapAlertPayload(
            gap_type="method_limitation",
            title="Attention scaling is limited",
            novelty=0.85,
            severity="high",
        )
        assert payload.gap_type == "method_limitation"
        assert payload.novelty == 0.85
        assert payload.severity == "high"
        assert payload.supporting_papers == []
        assert payload.source == "deep_research"
        assert payload.confidence == 0.0
        assert payload.impact_score == 0.0

    def test_all_fields(self):
        payload = GapAlertPayload(
            gap_type="evaluation_gap",
            title="No benchmark for X",
            novelty=0.9,
            severity="high",
            supporting_papers=["2201.12345", "2202.67890"],
            source="gap_detector",
            confidence=0.75,
            impact_score=8.5,
        )
        assert payload.supporting_papers == ["2201.12345", "2202.67890"]
        assert payload.source == "gap_detector"
        assert payload.confidence == 0.75
        assert payload.impact_score == 8.5


class TestParadigmShiftPayload:
    def test_required_fields(self):
        payload = ParadigmShiftPayload(
            alert_type="contradiction_cluster",
            gap_type="scaling_laws",
            message="Two papers contradict each other",
            severity="high",
        )
        assert payload.alert_type == "contradiction_cluster"
        assert payload.gap_type == "scaling_laws"
        assert payload.contradictions == []

    def test_with_contradictions(self):
        payload = ParadigmShiftPayload(
            alert_type="polarity_reversal",
            gap_type="warmup_strategy",
            message="Previous consensus reversed",
            severity="medium",
            contradictions=[
                {"paper_a": "2201.00001", "paper_b": "2201.00002"},
            ],
        )
        assert len(payload.contradictions) == 1
        assert payload.contradictions[0]["paper_a"] == "2201.00001"


# ─── DiscordRenderer ────────────────────────────────────────────────────────────


class TestDiscordRendererGapAlert:
    def test_render_gap_alert_basic(self):
        payload = GapAlertPayload(
            gap_type="method_limitation",
            title="Attention scaling is limited",
            novelty=0.85,
            severity="high",
        )
        result = DiscordRenderer.render_gap_alert(payload)
        assert "embeds" in result
        embed = result["embeds"][0]
        assert embed["title"] == "🔬 Attention scaling is limited"
        assert "HIGH" in embed["description"]
        assert embed["color"] == 0xFF4444  # high severity red

    def test_render_gap_alert_medium_severity(self):
        payload = GapAlertPayload(
            gap_type="scalability_issue",
            title="Memory grows O(n^2)",
            novelty=0.6,
            severity="medium",
        )
        result = DiscordRenderer.render_gap_alert(payload)
        assert result["embeds"][0]["color"] == 0xFFAA00  # orange

    def test_render_gap_alert_low_severity(self):
        payload = GapAlertPayload(
            gap_type="dataset_gap",
            title="Small dataset for task X",
            novelty=0.3,
            severity="low",
        )
        result = DiscordRenderer.render_gap_alert(payload)
        assert result["embeds"][0]["color"] == 0x44FF44  # green

    def test_render_gap_alert_with_confidence(self):
        payload = GapAlertPayload(
            gap_type="contradiction",
            title="Contradicting findings",
            novelty=0.7,
            severity="high",
            confidence=0.88,
        )
        result = DiscordRenderer.render_gap_alert(payload)
        embed = result["embeds"][0]
        fields = {f["name"]: f["value"] for f in embed["fields"]}
        assert "Confidence" in fields
        assert "88%" in fields["Confidence"]

    def test_render_gap_alert_with_impact_score(self):
        payload = GapAlertPayload(
            gap_type="unexplored_application",
            title="Could apply to X",
            novelty=0.95,
            severity="high",
            impact_score=9.2,
        )
        result = DiscordRenderer.render_gap_alert(payload)
        embed = result["embeds"][0]
        fields = {f["name"]: f["value"] for f in embed["fields"]}
        assert "Impact Score" in fields
        assert "9.20" in fields["Impact Score"]

    def test_render_gap_alert_with_supporting_papers(self):
        payload = GapAlertPayload(
            gap_type="evaluation_gap",
            title="No benchmark for X",
            novelty=0.8,
            severity="medium",
            supporting_papers=["2201.10001", "2201.10002", "2201.10003", "2201.10004"],
        )
        result = DiscordRenderer.render_gap_alert(payload)
        embed = result["embeds"][0]
        fields = {f["name"]: f["value"] for f in embed["fields"]}
        assert "Supporting Papers" in fields
        assert "2201.10001, 2201.10002, 2201.10003 +1 more" in fields["Supporting Papers"]

    def test_render_gap_alert_title_truncated(self):
        payload = GapAlertPayload(
            gap_type="method_limitation",
            title="A" * 300,
            novelty=0.5,
            severity="medium",
        )
        result = DiscordRenderer.render_gap_alert(payload)
        title = result["embeds"][0]["title"]
        assert len(title) <= 256 + 2  # "� " prefix + 256 max


class TestDiscordRendererParadigmShift:
    def test_render_paradigm_shift_contradiction_cluster(self):
        payload = ParadigmShiftPayload(
            alert_type="contradiction_cluster",
            gap_type="scaling_laws",
            message="Two major papers contradict on compute scaling",
            severity="high",
            contradictions=[{"paper_a": "2201.00001", "paper_b": "2202.99999"}],
        )
        result = DiscordRenderer.render_paradigm_shift(payload)
        assert "embeds" in result
        embed = result["embeds"][0]
        assert "⚠️" in embed["title"]
        assert embed["color"] == 0xFF0000

    def test_render_paradigm_shift_polarity_reversal(self):
        payload = ParadigmShiftPayload(
            alert_type="polarity_reversal",
            gap_type="warmup",
            message="Consensus reversed",
            severity="medium",
        )
        result = DiscordRenderer.render_paradigm_shift(payload)
        embed = result["embeds"][0]
        assert "🔄" in embed["title"]
        assert embed["color"] == 0xFF8800


class TestDiscordRendererPaperIngested:
    def test_render_paper_ingested(self):
        result = DiscordRenderer.render_paper_ingested(
            paper_title="Attention Is All You Need",
            arxiv_id="1706.03762",
            tags=["transformer", "NLP"],
        )
        assert "embeds" in result
        embed = result["embeds"][0]
        assert "Attention Is All You Need" in embed["title"]
        assert "1706.03762" in embed["description"]
        assert embed["color"] == 0x88CCFF

    def test_render_paper_ingested_with_tags(self):
        result = DiscordRenderer.render_paper_ingested(
            paper_title="Test Paper",
            arxiv_id="2201.00001",
            tags=["tag1", "tag2", "tag3"],
        )
        embed = result["embeds"][0]
        fields = {f["name"]: f["value"] for f in embed["fields"]}
        assert "Tags" in fields
        assert "`tag1`" in fields["Tags"]

    def test_render_paper_ingested_no_tags(self):
        result = DiscordRenderer.render_paper_ingested(
            paper_title="No Tags Paper",
            arxiv_id="2201.00002",
            tags=[],
        )
        embed = result["embeds"][0]
        field_names = [f["name"] for f in embed["fields"]]
        assert "Tags" not in field_names


# ─── FeishuRenderer ────────────────────────────────────────────────────────────


class TestFeishuRendererGapAlert:
    def test_render_gap_alert_basic(self):
        payload = GapAlertPayload(
            gap_type="method_limitation",
            title="Memory issue",
            novelty=0.8,
            severity="high",
        )
        result = FeishuRenderer.render_gap_alert(payload)
        assert result["msg_type"] == "interactive"
        assert "card" in result
        header = result["card"]["header"]
        assert "Memory issue" in header["title"]["content"]
        assert header["template"] == "red"

    def test_render_gap_alert_medium_severity(self):
        payload = GapAlertPayload(
            gap_type="scalability_issue",
            title="Scale issue",
            novelty=0.5,
            severity="medium",
        )
        result = FeishuRenderer.render_gap_alert(payload)
        assert result["card"]["header"]["template"] == "yellow"

    def test_render_gap_alert_low_severity(self):
        payload = GapAlertPayload(
            gap_type="dataset_gap",
            title="Small dataset",
            novelty=0.2,
            severity="low",
        )
        result = FeishuRenderer.render_gap_alert(payload)
        assert result["card"]["header"]["template"] == "green"

    def test_render_gap_alert_with_supporting_papers(self):
        payload = GapAlertPayload(
            gap_type="evaluation_gap",
            title="Missing benchmark",
            novelty=0.9,
            severity="high",
            supporting_papers=["2201.10001", "2201.10002"],
        )
        result = FeishuRenderer.render_gap_alert(payload)
        elements = result["card"]["elements"]
        # Supporting papers rendered as a markdown element, not tag elements
        md_elements = [e for e in elements if e.get("tag") == "markdown"]
        content_text = " ".join(e.get("content", "") for e in md_elements)
        assert "2201.10001" in content_text
        assert "2201.10002" in content_text


class TestFeishuRendererParadigmShift:
    def test_render_paradigm_shift_basic(self):
        payload = ParadigmShiftPayload(
            alert_type="contradiction_cluster",
            gap_type="scaling",
            message="Contradiction detected",
            severity="high",
        )
        result = FeishuRenderer.render_paradigm_shift(payload)
        assert result["msg_type"] == "interactive"
        assert result["card"]["header"]["template"] == "red"
        assert "⚠️" in result["card"]["header"]["title"]["content"]


# ─── WebhookDispatcher ────────────────────────────────────────────────────────


class TestWebhookDispatcher:
    """Test WebhookDispatcher using mocked HTTP."""

    def test_send_gap_alert_discord_calls_send_payload(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(
                webhook_url="https://discord.com/api/webhooks/123",
                platform=Platform.DISCORD,
                label="test-discord",
            )
            result = dispatcher.send_gap_alert(
                gap_type="method_limitation",
                title="Test gap",
                novelty=0.8,
                severity="high",
            )

            assert result is True
            mock_instance.post.assert_called_once()
            call_kwargs = mock_instance.post.call_args
            payload = call_kwargs.kwargs["json"]
            assert "embeds" in payload

    def test_send_gap_alert_feishu_calls_send_payload(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(
                webhook_url="https://open.feishu.cn/webhook/123",
                platform=Platform.FEISHU,
                label="test-feishu",
            )
            result = dispatcher.send_gap_alert(
                gap_type="evaluation_gap",
                title="Missing benchmark",
                novelty=0.9,
                severity="medium",
            )

            assert result is True
            mock_instance.post.assert_called_once()
            payload = mock_instance.post.call_args.kwargs["json"]
            assert payload["msg_type"] == "interactive"

    def test_send_gap_alert_generic_calls_send_payload(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(webhook_url="https://example.com/webhook")
            result = dispatcher.send_gap_alert(
                gap_type="dataset_gap",
                title="Small dataset",
                novelty=0.4,
                severity="low",
            )

            assert result is True
            payload = mock_instance.post.call_args.kwargs["json"]
            assert payload["event"] == "gap_alert"
            assert "timestamp" in payload

    def test_send_gap_alert_returns_false_on_nonzero_status(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 400
            mock_instance.post.return_value.text = "Bad Request"
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(webhook_url="https://example.com/webhook")
            result = dispatcher.send_gap_alert(
                gap_type="method_limitation",
                title="Test",
                novelty=0.5,
                severity="medium",
            )

            assert result is False

    def test_send_gap_alert_returns_false_on_timeout(self):
        with patch("requests.Session") as MockSession:
            import requests

            mock_instance = MagicMock()
            mock_instance.post.side_effect = requests.exceptions.Timeout()
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(webhook_url="https://example.com/webhook")
            result = dispatcher.send_gap_alert(
                gap_type="method_limitation",
                title="Test",
                novelty=0.5,
                severity="medium",
            )

            assert result is False

    def test_send_gap_alert_returns_false_on_connection_error(self):
        with patch("requests.Session") as MockSession:
            import requests

            mock_instance = MagicMock()
            mock_instance.post.side_effect = requests.exceptions.ConnectionError()
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(webhook_url="https://example.com/webhook")
            result = dispatcher.send_gap_alert(
                gap_type="method_limitation",
                title="Test",
                novelty=0.5,
                severity="medium",
            )

            assert result is False

    def test_send_paradigm_shift_discord(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(
                webhook_url="https://discord.com/api/webhooks/123",
                platform=Platform.DISCORD,
            )
            result = dispatcher.send_paradigm_shift(
                alert_type="contradiction_cluster",
                gap_type="scaling",
                message="Papers contradict",
                severity="high",
            )

            assert result is True
            payload = mock_instance.post.call_args.kwargs["json"]
            assert "embeds" in payload

    def test_send_paper_ingested(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(
                webhook_url="https://discord.com/api/webhooks/123",
                platform=Platform.DISCORD,
            )
            result = dispatcher.send_paper_ingested(
                paper_title="Test Paper",
                arxiv_id="2201.00001",
                tags=["test", "ml"],
            )

            assert result is True
            payload = mock_instance.post.call_args.kwargs["json"]
            assert "embeds" in payload

    def test_test_notification(self):
        with patch("requests.Session") as MockSession:
            mock_instance = MagicMock()
            mock_instance.post.return_value.status_code = 200
            MockSession.return_value = mock_instance

            dispatcher = WebhookDispatcher(webhook_url="https://example.com/webhook")
            result = dispatcher.test()

            assert result is True


# ─── NotificationCenter ────────────────────────────────────────────────────────


class TestNotificationCenter:
    """Test NotificationCenter with fully mocked dispatchers."""

    def test_add_and_remove(self):
        center = NotificationCenter()

        d1 = MagicMock(spec=WebhookDispatcher)
        d1.label = "discord"
        d1.webhook_url = "https://discord.com/webhook"
        d1.send_gap_alert.return_value = True

        d2 = MagicMock(spec=WebhookDispatcher)
        d2.label = "feishu"
        d2.webhook_url = "https://feishu.cn/webhook"
        d2.send_gap_alert.return_value = True

        center.add(d1)
        center.add(d2)
        assert len(center._dispatchers) == 2

        result = center.remove("discord")
        assert result is True
        assert len(center._dispatchers) == 1

        result = center.remove("nonexistent")
        assert result is False

    def test_send_gap_alert_to_all(self):
        center = NotificationCenter()

        d1 = MagicMock(spec=WebhookDispatcher)
        d1.label = "d1"
        d1.webhook_url = "https://d1.com"
        d1.send_gap_alert.return_value = True

        d2 = MagicMock(spec=WebhookDispatcher)
        d2.label = "d2"
        d2.webhook_url = "https://d2.com"
        d2.send_gap_alert.return_value = False

        center.add(d1)
        center.add(d2)

        results = center.send_gap_alert(
            gap_type="method_limitation",
            title="Test",
            novelty=0.5,
            severity="medium",
        )

        assert results == {"d1": True, "d2": False}
        d1.send_gap_alert.assert_called_once()
        d2.send_gap_alert.assert_called_once()

    def test_send_paradigm_shift_to_all(self):
        center = NotificationCenter()

        d1 = MagicMock(spec=WebhookDispatcher)
        d1.label = "d1"
        d1.webhook_url = "https://d1.com"
        d1.send_paradigm_shift.return_value = True

        center.add(d1)
        results = center.send_paradigm_shift(
            alert_type="contradiction_cluster",
            gap_type="scaling",
            message="Test",
            severity="high",
        )

        assert results == {"d1": True}

    def test_send_paper_ingested_to_all(self):
        center = NotificationCenter()

        d1 = MagicMock(spec=WebhookDispatcher)
        d1.label = "d1"
        d1.webhook_url = "https://d1.com"
        d1.send_paper_ingested.return_value = True

        center.add(d1)
        results = center.send_paper_ingested(
            paper_title="Test Paper",
            arxiv_id="2201.00001",
            tags=["ml"],
        )

        assert results == {"d1": True}

    def test_test_all(self):
        center = NotificationCenter()

        d1 = MagicMock(spec=WebhookDispatcher)
        d1.label = "d1"
        d1.webhook_url = "https://d1.com"
        d1.test.return_value = True

        center.add(d1)
        results = center.test_all()

        assert results == {"d1": True}
        d1.test.assert_called_once()

    def test_empty_center_send_returns_empty_dict(self):
        center = NotificationCenter()
        results = center.send_gap_alert(
            gap_type="test",
            title="Test",
            novelty=0.5,
            severity="low",
        )
        assert results == {}
