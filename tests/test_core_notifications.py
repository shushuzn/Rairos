"""Tests for core/notifications.py."""
import sys
import time
from pathlib import Path
import pytest

sys.path.insert(0, str(Path(__file__).parent.parent))
from core.notifications import (
    NotificationLevel, Notification, NotificationManager, get_notification_manager
)


class TestNotificationLevel:
    def test_enum_values(self):
        assert NotificationLevel.INFO.value == "info"
        assert NotificationLevel.SUCCESS.value == "success"
        assert NotificationLevel.WARNING.value == "warning"
        assert NotificationLevel.ERROR.value == "error"

    def test_enum_count(self):
        assert len(NotificationLevel) == 4


class TestNotification:
    def test_dataclass_fields(self):
        notif = Notification(NotificationLevel.INFO, "Title", "Message", 1234567890.0)
        assert notif.level == NotificationLevel.INFO
        assert notif.title == "Title"
        assert notif.message == "Message"
        assert notif.timestamp == 1234567890.0

    def test_dataclass_equality(self):
        n1 = Notification(NotificationLevel.INFO, "T", "M", 1.0)
        n2 = Notification(NotificationLevel.INFO, "T", "M", 1.0)
        assert n1 == n2


class TestNotificationManager:
    def setup_method(self):
        self.manager = NotificationManager()

    def test_add(self):
        self.manager.add(NotificationLevel.INFO, "title", "message")
        assert len(self.manager.notifications) == 1
        n = self.manager.notifications[0]
        assert n.level == NotificationLevel.INFO
        assert n.title == "title"
        assert n.message == "message"
        assert isinstance(n.timestamp, float)

    def test_info(self):
        self.manager.info("info title", "info msg")
        assert len(self.manager.notifications) == 1
        assert self.manager.notifications[0].level == NotificationLevel.INFO

    def test_success(self):
        self.manager.success("ok title", "ok msg")
        assert self.manager.notifications[0].level == NotificationLevel.SUCCESS

    def test_warning(self):
        self.manager.warning("warn title", "warn msg")
        assert self.manager.notifications[0].level == NotificationLevel.WARNING

    def test_error(self):
        self.manager.error("err title", "err msg")
        assert self.manager.notifications[0].level == NotificationLevel.ERROR

    def test_get_all(self):
        self.manager.info("i1", "m1")
        self.manager.warning("w1", "m1")
        all_n = self.manager.get_all()
        assert len(all_n) == 2

    def test_get_by_level(self):
        self.manager.info("i1", "m1")
        self.manager.warning("w1", "m1")
        self.manager.error("e1", "m1")
        self.manager.info("i2", "m2")
        assert len(self.manager.get_by_level(NotificationLevel.INFO)) == 2
        assert len(self.manager.get_by_level(NotificationLevel.WARNING)) == 1
        assert len(self.manager.get_by_level(NotificationLevel.SUCCESS)) == 0

    def test_clear(self):
        self.manager.info("i1", "m1")
        self.manager.error("e1", "m1")
        assert len(self.manager.notifications) == 2
        self.manager.clear()
        assert len(self.manager.notifications) == 0

    def test_timestamp_is_float(self):
        before = time.time()
        self.manager.info("t", "m")
        after = time.time()
        ts = self.manager.notifications[0].timestamp
        assert before <= ts <= after


class TestGetNotificationManager:
    def setup_method(self):
        import core.notifications as nm
        nm._notification_manager = None

    def teardown_method(self):
        import core.notifications as nm
        nm._notification_manager = None

    def test_singleton(self):
        m1 = get_notification_manager()
        m2 = get_notification_manager()
        assert m1 is m2

    def test_singleton_persists(self):
        m1 = get_notification_manager()
        m1.info("title", "msg")
        m2 = get_notification_manager()
        assert len(m2.get_all()) == 1
