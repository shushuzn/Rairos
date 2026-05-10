"""
Notification System.

Provides notifications for important events and external webhook delivery.
"""

from __future__ import annotations

import json
import logging
import time
import urllib.parse
import urllib.request
from dataclasses import dataclass
from enum import Enum
from typing import Any, Dict, List, Optional


logger = logging.getLogger(__name__)


class NotificationLevel(Enum):
    """Notification levels."""

    INFO = "info"
    SUCCESS = "success"
    WARNING = "warning"
    ERROR = "error"


@dataclass
class Notification:
    """A notification message."""

    level: NotificationLevel
    title: str
    message: str
    timestamp: float


class NotificationManager:
    """Manage notifications."""

    def __init__(self):
        self.notifications: List[Notification] = []

    def add(self, level: NotificationLevel, title: str, message: str):
        """Add a notification."""
        notification = Notification(
            level=level, title=title, message=message, timestamp=time.time()
        )
        self.notifications.append(notification)

    def info(self, title: str, message: str):
        """Add info notification."""
        self.add(NotificationLevel.INFO, title, message)

    def success(self, title: str, message: str):
        """Add success notification."""
        self.add(NotificationLevel.SUCCESS, title, message)

    def warning(self, title: str, message: str):
        """Add warning notification."""
        self.add(NotificationLevel.WARNING, title, message)

    def error(self, title: str, message: str):
        """Add error notification."""
        self.add(NotificationLevel.ERROR, title, message)

    def get_all(self) -> List[Notification]:
        """Get all notifications."""
        return self.notifications

    def get_by_level(self, level: NotificationLevel) -> List[Notification]:
        """Get notifications by level."""
        return [n for n in self.notifications if n.level == level]

    def clear(self):
        """Clear all notifications."""
        self.notifications.clear()


# Global notification manager
_notification_manager: Optional[NotificationManager] = None


def get_notification_manager() -> NotificationManager:
    """Get the global notification manager."""
    global _notification_manager
    if _notification_manager is None:
        _notification_manager = NotificationManager()
    return _notification_manager


# ─── Webhook Notifier ──────────────────────────────────────────────────────────


class WebhookNotifier:
    """Send notifications to external webhooks (Discord, Feishu, etc.)."""

    def __init__(self, webhook_url: str = ""):
        self.webhook_url = webhook_url

    def _send(self, payload: Dict[str, Any]) -> bool:
        """Send payload to webhook URL. Returns True on success."""
        if not self.webhook_url:
            return False
        parsed = urllib.parse.urlparse(self.webhook_url)
        if parsed.scheme not in ("http", "https"):
            logger.warning(f"Webhook URL scheme {parsed.scheme!r} not allowed (only http/https)")
            return False
        try:
            data = json.dumps(payload).encode("utf-8")
            req = urllib.request.Request(
                self.webhook_url,
                data=data,
                headers={"Content-Type": "application/json"},
                method="POST",
            )
            with urllib.request.urlopen(req, timeout=10) as resp:
                return resp.status in (200, 204)
        except Exception as e:
            logger.warning(f"Webhook send failed: {e}")
            return False

    def send_discord(
        self,
        title: str,
        description: str,
        color: int = 0x5865F2,
        fields: Optional[List[Dict[str, Any]]] = None,
    ) -> bool:
        """Send a Discord embed notification.

        Args:
            title: Embed title
            description: Main text
            color: Decimal color code (e.g. 0x00FF00 = green)
            fields: List of {name, value, inline} dicts
        """
        if not self.webhook_url:
            return False

        embed: Dict[str, Any] = {
            "title": title,
            "description": description,
            "color": color,
        }
        if fields:
            embed["fields"] = [
                {"name": f["name"], "value": f["value"], "inline": f.get("inline", False)}
                for f in fields
            ]

        payload = {"embeds": [embed]}
        return self._send(payload)

    def send_feishu(
        self,
        title: str,
        content: str,
        msg_type: str = "text",
    ) -> bool:
        """Send a Feishu text notification.

        Args:
            title: Card title (for interactive card) or ignored for text
            content: Message text
            msg_type: "text" or "interactive"
        """
        if not self.webhook_url:
            return False

        if msg_type == "interactive":
            payload = {
                "msg_type": "interactive",
                "card": {
                    "header": {"title": {"tag": "plain_text", "content": title}},
                    "elements": [{"tag": "div", "text": {"tag": "lark_md", "content": content}}],
                },
            }
        else:
            payload = {"msg_type": "text", "content": {"text": f"{title}\n{content}"}}

        return self._send(payload)

    def notify_papers_found(
        self,
        subscription_topic: str,
        papers: List[Dict[str, Any]],
        min_score: float = 0.5,
    ) -> int:
        """Send notification for newly found papers.

        Returns number of successfully notified papers.
        """
        if not papers:
            return 0

        count = 0
        top_papers = papers[:5]  # Notify top 5 at most

        for paper in top_papers:
            score = paper.get("score", 0)
            if score < min_score:
                continue

            title = paper.get("title", "Untitled")[:200]
            arxiv_id = paper.get("arxiv_id", "")
            url = f"https://arxiv.org/abs/{arxiv_id}" if arxiv_id else ""

            # Try Discord first
            self.send_discord(
                title=f"📄 New Paper — {subscription_topic}",
                description=f"**{title}**\nScore: {score:.2f}",
                color=0x00FF00,
                fields=[{"name": "arXiv", "value": f"[{arxiv_id}]({url})", "inline": True}]
                if arxiv_id
                else None,
            )

            # Try Feishu
            self.send_feishu(
                title=f"📄 New Paper — {subscription_topic}",
                content=f"**{title}**\nScore: {score:.2f}\n{url}",
            )

            count += 1

        return count


# Global webhook notifier (lazy)
_webhook_notifier: Optional[WebhookNotifier] = None


def get_webhook_notifier() -> WebhookNotifier:
    """Get the global webhook notifier."""
    global _webhook_notifier
    if _webhook_notifier is None:
        _webhook_notifier = WebhookNotifier()
    return _webhook_notifier


def configure_webhook(url: str) -> WebhookNotifier:
    """Configure the global webhook URL and return the notifier."""
    global _webhook_notifier
    _webhook_notifier = WebhookNotifier(webhook_url=url)
    return _webhook_notifier
