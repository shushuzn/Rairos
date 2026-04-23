"""
Notification System.

Provides notifications for important events.
"""
from typing import List

from dataclasses import dataclass
from enum import Enum


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
        import time
        notification = Notification(
            level=level,
            title=title,
            message=message,
            timestamp=time.time()
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
_notification_manager = None


def get_notification_manager() -> NotificationManager:
    """Get the global notification manager."""
    global _notification_manager
    if _notification_manager is None:
        _notification_manager = NotificationManager()
    return _notification_manager
