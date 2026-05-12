"""Rich Webhook Notifications — Discord embeds and Feishu markdown cards for gap events.

Supports
────────
- Discord: webhook embeds with color, fields, and author
- Feishu: markdown card with sections, tags, and code blocks
- Generic: JSON POST to any webhook URL

Usage
─────
    from notifications.dispatcher import WebhookDispatcher, NotificationType

    dispatcher = WebhookDispatcher(webhook_url="https://hooks.slack.com/...")
    dispatcher.send_gap_alert(
        gap_type="method_limitation",
        title="Attention scaling is limited",
        novelty=0.85,
        severity="high",
        source="deep_research",
    )
"""

import logging
from dataclasses import dataclass, field
from datetime import datetime as DT
from enum import Enum
from typing import Any, Dict, List, Optional

import requests

logger = logging.getLogger(__name__)


# ─── Enums ────────────────────────────────────────────────────────────────────


class NotificationType(Enum):
    GAP_ALERT = "gap_alert"  # New high-novelty gap discovered
    PARADIGM_SHIFT = "paradigm_shift"  # Paradigm shift alert detected
    PAPER_INGESTED = "paper_ingested"  # New paper added to DB
    RESEARCH_COMPLETE = "research_complete"  # Deep research round finished
    CONTRADICTION_DETECTED = "contradiction_detected"  # New contradiction found
    TOPIC_SUGGESTION = "topic_suggestion"  # New subscription topic suggested


class Platform(Enum):
    DISCORD = "discord"
    FEISHU = "feishu"
    GENERIC = "generic"


# ─── Dataclasses ──────────────────────────────────────────────────────────────


@dataclass
class WebhookConfig:
    """Configuration for a webhook destination."""

    url: str
    platform: Platform = Platform.GENERIC
    enabled: bool = True
    label: str = ""  # Human-readable label for this webhook
    secret: str = ""  # Optional secret for HMAC signing


@dataclass
class GapAlertPayload:
    """Payload for a gap alert notification."""

    gap_type: str
    title: str
    novelty: float
    severity: str  # 'low' | 'medium' | 'high'
    supporting_papers: List[str] = field(default_factory=list)
    source: str = "deep_research"  # Which subsystem found this
    confidence: float = 0.0
    impact_score: float = 0.0


@dataclass
class ParadigmShiftPayload:
    """Payload for a paradigm shift alert."""

    alert_type: str  # 'contradiction_cluster' | 'polarity_reversal'
    gap_type: str
    message: str
    severity: str
    contradictions: List[Dict[str, Any]] = field(default_factory=list)


# ─── Discord Renderer ─────────────────────────────────────────────────────────


class DiscordRenderer:
    """Render notifications as Discord webhook embeds."""

    # Severity → Discord color (integer)
    SEVERITY_COLORS = {
        "high": 0xFF4444,  # Red
        "medium": 0xFFAA00,  # Orange
        "low": 0x44FF44,  # Green
    }

    GAP_TYPE_COLORS = {
        "method_limitation": 0xCC88FF,
        "scalability_issue": 0xFF8800,
        "evaluation_gap": 0x88CCFF,
        "contradiction": 0xFF4444,
        "unexplored_application": 0x44FFAA,
        "dataset_gap": 0xFFFF44,
    }

    @classmethod
    def _color_for(cls, gap_type: str, severity: str) -> int:
        """Get Discord embed color."""
        if severity in cls.SEVERITY_COLORS:
            return cls.SEVERITY_COLORS[severity]
        return cls.GAP_TYPE_COLORS.get(gap_type.lower(), 0x888888)

    @classmethod
    def render_gap_alert(cls, payload: GapAlertPayload) -> Dict[str, Any]:
        """Render a gap alert as a Discord embed."""
        color = cls._color_for(payload.gap_type, payload.severity)
        novelty_pct = int(payload.novelty * 100)

        fields: list[Dict[str, Any]] = [
            {
                "name": "Gap Type",
                "value": payload.gap_type.replace("_", " ").title(),
                "inline": True,
            },
            {
                "name": "Novelty",
                "value": f"{novelty_pct}%",
                "inline": True,
            },
        ]

        if payload.confidence > 0:
            fields.append(
                {
                    "name": "Confidence",
                    "value": f"{int(payload.confidence * 100)}%",
                    "inline": True,
                }
            )

        if payload.impact_score > 0:
            fields.append(
                {
                    "name": "Impact Score",
                    "value": f"{payload.impact_score:.2f}",
                    "inline": True,
                }
            )

        if payload.supporting_papers:
            papers_str = ", ".join(payload.supporting_papers[:3])
            if len(payload.supporting_papers) > 3:
                papers_str += f" +{len(payload.supporting_papers) - 3} more"
            fields.append(
                {
                    "name": "Supporting Papers",
                    "value": papers_str,
                    "inline": False,
                }
            )

        embed: Dict[str, Any] = {
            "title": f"🔬 {payload.title[:256]}",
            "description": f"**{payload.severity.upper()}** novelty gap discovered via **{payload.source}**",
            "color": color,
            "fields": fields,
            "footer": {
                "text": f"Rairos Research Agent • {DT.now().strftime('%Y-%m-%d %H:%M')}",
            },
        }

        return {"embeds": [embed]}

    @classmethod
    def render_paradigm_shift(cls, payload: ParadigmShiftPayload) -> Dict[str, Any]:
        """Render a paradigm shift alert as a Discord embed."""
        icon = "⚠️" if payload.alert_type == "contradiction_cluster" else "🔄"
        color = 0xFF0000 if payload.severity == "high" else 0xFF8800

        fields: list[Dict[str, Any]] = [
            {
                "name": "Alert Type",
                "value": payload.alert_type.replace("_", " ").title(),
                "inline": True,
            },
            {
                "name": "Severity",
                "value": payload.severity.upper(),
                "inline": True,
            },
        ]

        embed: Dict[str, Any] = {
            "title": f"{icon} Paradigm Shift Signal: {payload.gap_type}",
            "description": payload.message[:2048],
            "color": color,
            "fields": fields,
            "footer": {
                "text": f"Rairos Paradigm Watch • {DT.now().strftime('%Y-%m-%d %H:%M')}",
            },
        }

        if payload.contradictions:
            c = payload.contradictions[0]
            embed["fields"].append(
                {
                    "name": "Sample Contradiction",
                    "value": f"Paper A: `{c.get('paper_a', '?')[:32]}`\nPaper B: `{c.get('paper_b', '?')[:32]}`",
                    "inline": False,
                }
            )

        return {"embeds": [embed]}

    @classmethod
    def render_paper_ingested(
        cls, paper_title: str, arxiv_id: str, tags: List[str]
    ) -> Dict[str, Any]:
        """Render a paper ingestion notification."""
        embed: Dict[str, Any] = {
            "title": f"📄 {paper_title[:256]}",
            "description": f"**arXiv:** `{arxiv_id}`",
            "color": 0x88CCFF,
            "fields": [],
            "footer": {
                "text": f"Rairos • {DT.now().strftime('%Y-%m-%d %H:%M')}",
            },
        }

        if tags:
            embed["fields"].append(
                {
                    "name": "Tags",
                    "value": " ".join(f"`{t}`" for t in tags[:8]),
                    "inline": False,
                }
            )

        return {"embeds": [embed]}


# ─── Feishu Renderer ───────────────────────────────────────────────────────────


class FeishuRenderer:
    """Render notifications as Feishu interactive markdown cards."""

    @classmethod
    def _severity_tag(cls, severity: str) -> str:
        emoji = {"high": "🔴", "medium": "🟡", "low": "🟢"}.get(severity, "⚪")
        return f"{emoji} **{severity.upper()}**"

    @classmethod
    def render_gap_alert(cls, payload: GapAlertPayload) -> Dict[str, Any]:
        """Render a gap alert as a Feishu card."""
        novelty_pct = int(payload.novelty * 100)

        elements: list[dict[str, Any]] = [
            {
                "tag": "markdown",
                "content": f"**Gap Type:** {payload.gap_type.replace('_', ' ').title()}",
            },
            {
                "tag": "markdown",
                "content": f"**Novelty:** {novelty_pct}% | **Severity:** {cls._severity_tag(payload.severity)}",
            },
        ]

        if payload.confidence > 0:
            elements.append(
                {
                    "tag": "markdown",
                    "content": f"**Confidence:** {int(payload.confidence * 100)}%",
                }
            )

        if payload.impact_score > 0:
            elements.append(
                {
                    "tag": "markdown",
                    "content": f"**Impact Score:** {payload.impact_score:.2f}",
                }
            )

        if payload.supporting_papers:
            papers_md = "\n".join(f"- `{pid}`" for pid in payload.supporting_papers[:5])
            elements.append(
                {
                    "tag": "markdown",
                    "content": f"**Supporting Papers:**\n{papers_md}",
                }
            )

        elements.append(
            {
                "tag": "note",
                "elements": [
                    {"tag": "plain_text", "content": f"Source: {payload.source}"},
                ],
            }
        )

        return {
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {"tag": "plain_text", "content": f"🔬 {payload.title[:100]}"},
                    "template": cls._feishu_template(payload.severity),
                },
                "elements": elements,
            },
        }

    @classmethod
    def _feishu_template(cls, severity: str) -> str:
        return {
            "high": "red",
            "medium": "yellow",
            "low": "green",
        }.get(severity, "grey")

    @classmethod
    def render_paradigm_shift(cls, payload: ParadigmShiftPayload) -> Dict[str, Any]:
        """Render a paradigm shift alert as a Feishu card."""
        icon = "⚠️" if payload.alert_type == "contradiction_cluster" else "🔄"

        return {
            "msg_type": "interactive",
            "card": {
                "header": {
                    "title": {
                        "tag": "plain_text",
                        "content": f"{icon} Paradigm Shift: {payload.gap_type}",
                    },
                    "template": "red" if payload.severity == "high" else "yellow",
                },
                "elements": [
                    {"tag": "markdown", "content": payload.message[:2000]},
                    {
                        "tag": "markdown",
                        "content": f"**Alert Type:** {payload.alert_type.replace('_', ' ').title()}\n**Severity:** {cls._severity_tag(payload.severity)}",
                    },
                ],
            },
        }


# ─── Webhook Dispatcher ───────────────────────────────────────────────────────


class WebhookDispatcher:
    """Send rich notifications to Discord, Feishu, or generic webhooks."""

    DEFAULT_TIMEOUT = 10  # seconds

    def __init__(self, webhook_url: str, platform: Platform = Platform.GENERIC, label: str = ""):
        self.webhook_url = webhook_url
        self.platform = platform
        self.label = label or platform.value
        self._session = requests.Session()

    def _send_payload(self, payload: Dict[str, Any]) -> bool:
        """Send a payload to the webhook URL. Returns True on success."""
        try:
            headers = {"Content-Type": "application/json"}
            resp = self._session.post(
                self.webhook_url,
                json=payload,
                headers=headers,
                timeout=self.DEFAULT_TIMEOUT,
            )

            if resp.status_code in (200, 204):
                return True

            logger.warning(
                f"Webhook POST to {self.label} returned {resp.status_code}: {resp.text[:200]}"
            )
            return False

        except requests.exceptions.Timeout:
            logger.warning(f"Webhook timeout for {self.label}")
            return False
        except Exception as e:
            logger.error(f"Webhook error for {self.label}: {e}")
            return False

    # ── Gap Alert ────────────────────────────────────────────────────────────

    def send_gap_alert(
        self,
        gap_type: str,
        title: str,
        novelty: float,
        severity: str = "medium",
        supporting_papers: Optional[List[str]] = None,
        source: str = "deep_research",
        confidence: float = 0.0,
        impact_score: float = 0.0,
    ) -> bool:
        """Send a gap alert notification."""
        payload = GapAlertPayload(
            gap_type=gap_type,
            title=title,
            novelty=novelty,
            severity=severity,
            supporting_papers=supporting_papers or [],
            source=source,
            confidence=confidence,
            impact_score=impact_score,
        )

        if self.platform == Platform.DISCORD:
            rendered = DiscordRenderer.render_gap_alert(payload)
        elif self.platform == Platform.FEISHU:
            rendered = FeishuRenderer.render_gap_alert(payload)
        else:
            rendered = self._render_generic("gap_alert", payload)

        return self._send_payload(rendered)

    # ── Paradigm Shift ───────────────────────────────────────────────────────

    def send_paradigm_shift(
        self,
        alert_type: str,
        gap_type: str,
        message: str,
        severity: str = "medium",
        contradictions: Optional[List[Dict[str, Any]]] = None,
    ) -> bool:
        """Send a paradigm shift alert notification."""
        payload = ParadigmShiftPayload(
            alert_type=alert_type,
            gap_type=gap_type,
            message=message,
            severity=severity,
            contradictions=contradictions or [],
        )

        if self.platform == Platform.DISCORD:
            rendered = DiscordRenderer.render_paradigm_shift(payload)
        elif self.platform == Platform.FEISHU:
            rendered = FeishuRenderer.render_paradigm_shift(payload)
        else:
            rendered = self._render_generic("paradigm_shift", payload)

        return self._send_payload(rendered)

    # ── Paper Ingested ───────────────────────────────────────────────────────

    def send_paper_ingested(
        self,
        paper_title: str,
        arxiv_id: str,
        tags: Optional[List[str]] = None,
    ) -> bool:
        """Send a paper ingested notification."""
        if self.platform == Platform.DISCORD:
            rendered = DiscordRenderer.render_paper_ingested(paper_title, arxiv_id, tags or [])
        elif self.platform == Platform.FEISHU:
            rendered = self._render_generic(
                "paper_ingested",
                {
                    "title": paper_title,
                    "arxiv_id": arxiv_id,
                    "tags": tags or [],
                },
            )
        else:
            rendered = self._render_generic(
                "paper_ingested",
                {
                    "title": paper_title,
                    "arxiv_id": arxiv_id,
                    "tags": tags or [],
                },
            )

        return self._send_payload(rendered)

    # ── Generic ──────────────────────────────────────────────────────────────

    def _render_generic(self, event_type: str, data: Any) -> Dict[str, Any]:
        """Render as generic JSON payload."""
        return {
            "event": event_type,
            "timestamp": DT.now().isoformat(),
            "source": "Rairos",
            "data": data if isinstance(data, dict) else {"value": str(data)},
        }

    def test(self) -> bool:
        """Send a test notification to verify the webhook works."""
        return self.send_gap_alert(
            gap_type="test",
            title="Test notification from Rairos",
            novelty=0.5,
            severity="low",
            source="webhook_test",
        )


# ─── Multi-dispatcher ─────────────────────────────────────────────────────────


class NotificationCenter:
    """Manage multiple webhook destinations and send to all enabled ones.

    Usage:
        center = NotificationCenter()
        center.add(WebhookDispatcher("https://discord.com/api/webhooks/...", Platform.DISCORD))
        center.add(WebhookDispatcher("https://open.feishu.cn/...", Platform.FEISHU))
        center.send_gap_alert(...)
    """

    def __init__(self):
        self._dispatchers: List[WebhookDispatcher] = []

    def add(self, dispatcher: WebhookDispatcher) -> None:
        self._dispatchers.append(dispatcher)

    def remove(self, label: str) -> bool:
        """Remove a dispatcher by label. Returns True if found and removed."""
        for i, d in enumerate(self._dispatchers):
            if d.label == label:
                self._dispatchers.pop(i)
                return True
        return False

    def send_gap_alert(self, **kwargs) -> Dict[str, bool]:
        """Send to all enabled dispatchers. Returns {label: success}."""
        return {d.label: d.send_gap_alert(**kwargs) for d in self._dispatchers if d.webhook_url}

    def send_paradigm_shift(self, **kwargs) -> Dict[str, bool]:
        return {
            d.label: d.send_paradigm_shift(**kwargs) for d in self._dispatchers if d.webhook_url
        }

    def send_paper_ingested(self, **kwargs) -> Dict[str, bool]:
        return {
            d.label: d.send_paper_ingested(**kwargs) for d in self._dispatchers if d.webhook_url
        }

    def test_all(self) -> Dict[str, bool]:
        return {d.label: d.test() for d in self._dispatchers if d.webhook_url}
