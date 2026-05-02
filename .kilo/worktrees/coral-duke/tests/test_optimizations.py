"""Tests for new optimization modules."""


def test_backup_manager_creation():
    """Test backup manager creation."""
    from core.backup import BackupManager
    manager = BackupManager()
    assert manager is not None


def test_batch_optimizer_creation():
    """Test batch optimizer creation."""
    from core.batch_optimizer import BatchOptimizer
    optimizer = BatchOptimizer()
    assert optimizer is not None
    assert optimizer.max_workers == 4


def test_notification_manager_creation():
    """Test notification manager creation."""
    from core.notifications import NotificationManager, NotificationLevel
    manager = NotificationManager()
    assert manager is not None
    manager.add(NotificationLevel.INFO, "Test", "Test message")
    assert len(manager.get_all()) == 1


def test_search_optimizer_creation():
    """Test search optimizer creation."""
    from core.search_optimizer import SearchOptimizer
    optimizer = SearchOptimizer()
    assert optimizer is not None
    assert optimizer.search_history == []
