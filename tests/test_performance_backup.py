"""Tests for core/performance_guarantee.py and core/backup.py."""

from unittest.mock import patch, MagicMock
from core.performance_guarantee import (
    PerformanceGuarantee,
    PerformanceGuaranteeSystem,
    get_performance_guarantee_system,
    should_throttle_operations,
)
from core.backup import BackupManager


class TestPerformanceGuarantee:
    def test_dataclass_fields(self):
        g = PerformanceGuarantee("CPU", "promise", 10.5, "OK")
        assert g.name == "CPU"
        assert g.promise == "promise"
        assert g.measured_impact == 10.5
        assert g.status == "OK"


class TestPerformanceGuaranteeSystem:
    def test_init_creates_four_guarantees(self):
        with patch("core.performance_guarantee.psutil"):
            system = PerformanceGuaranteeSystem()
            assert len(system.guarantees) == 4
            names = [g.name for g in system.guarantees]
            assert "CPU" in names[0]
            assert "memory" in names[1].lower() or "内" in names[1]
            assert "disk" in names[2].lower() or "磁盘" in names[2]
            assert "bandwidth" in names[3].lower() or "带宽" in names[3]

    def test_init_baseline_structure(self):
        with patch("core.performance_guarantee.psutil"):
            system = PerformanceGuaranteeSystem()
            assert set(system.baseline.keys()) == {"cpu_percent", "memory_percent", "disk_io_read"}

    def test_check_guarantees_updates_cpu_status(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 25.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=40.0)
            system = PerformanceGuaranteeSystem()
            guarantees = system.check_guarantees()
            assert guarantees[0].status == "OK"
            assert guarantees[0].measured_impact == 25.0

    def test_check_guarantees_warnings_on_high_cpu(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 55.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=40.0)
            system = PerformanceGuaranteeSystem()
            guarantees = system.check_guarantees()
            assert guarantees[0].status == "WARNING"

    def test_check_guarantees_warnings_on_high_memory(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 10.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=65.0)
            system = PerformanceGuaranteeSystem()
            guarantees = system.check_guarantees()
            assert guarantees[1].status == "WARNING"

    def test_check_guarantees_handles_oserror(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.side_effect = OSError
            mock_ps.virtual_memory.return_value = MagicMock(percent=40.0)
            system = PerformanceGuaranteeSystem()
            guarantees = system.check_guarantees()
            assert len(guarantees) == 4

    def test_should_throttle_true_on_high_cpu(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 85.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=30.0)
            system = PerformanceGuaranteeSystem()
            assert system.should_throttle() is True

    def test_should_throttle_true_on_high_memory(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 20.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=85.0)
            system = PerformanceGuaranteeSystem()
            assert system.should_throttle() is True

    def test_should_throttle_false_under_thresholds(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.return_value = 30.0
            mock_ps.virtual_memory.return_value = MagicMock(percent=30.0)
            system = PerformanceGuaranteeSystem()
            assert system.should_throttle() is False

    def test_should_throttle_handles_oserror(self):
        with patch("core.performance_guarantee.psutil") as mock_ps:
            mock_ps.cpu_percent.side_effect = OSError
            mock_ps.virtual_memory.return_value = MagicMock(percent=20.0)
            system = PerformanceGuaranteeSystem()
            assert system.should_throttle() is False

    def test_get_protection_report_returns_string(self):
        import core.performance_guarantee as pg

        pg._guarantee_system = None
        with (
            patch("core.performance_guarantee.psutil.cpu_percent", return_value=25.0),
            patch(
                "core.performance_guarantee.psutil.virtual_memory",
                return_value=MagicMock(percent=40.0),
            ),
            patch("core.performance_guarantee.psutil.disk_io_counters"),
        ):
            system = PerformanceGuaranteeSystem()
            report = system.get_protection_report()
            assert isinstance(report, str)
            assert "protection" in report.lower() or "保护" in report
            assert "promise" in report.lower() or "承诺" in report


class TestGlobalFunctions:
    def test_should_throttle_operations_returns_bool(self):
        import core.performance_guarantee as pg

        pg._guarantee_system = None
        with (
            patch("core.performance_guarantee.psutil.cpu_percent", return_value=20.0),
            patch(
                "core.performance_guarantee.psutil.virtual_memory",
                return_value=MagicMock(percent=20.0),
            ),
            patch("core.performance_guarantee.psutil.disk_io_counters"),
        ):
            result = should_throttle_operations()
            assert isinstance(result, bool)

    def test_get_performance_guarantee_system_singleton(self):
        import core.performance_guarantee as pg

        pg._guarantee_system = None
        with patch("core.performance_guarantee.psutil"):
            s1 = get_performance_guarantee_system()
            s2 = get_performance_guarantee_system()
            assert s1 is s2


class TestBackupManager:
    def test_init_creates_default_dir(self, tmp_path):
        BackupManager(backup_dir=tmp_path / "bk")
        assert (tmp_path / "bk").exists()

    def test_init_creates_parent_dirs(self, tmp_path):
        BackupManager(backup_dir=tmp_path / "a" / "b" / "c")
        assert (tmp_path / "a" / "b" / "c").exists()

    def test_list_backups_empty(self, tmp_path):
        manager = BackupManager(backup_dir=tmp_path)
        assert manager.list_backups() == []

    def test_list_backups_finds_directories(self, tmp_path):
        manager = BackupManager(backup_dir=tmp_path)
        (tmp_path / "backup_20240101_010000").mkdir()
        (tmp_path / "backup_20240102_020000").mkdir()
        (tmp_path / "not_a_backup").mkdir()
        backups = manager.list_backups()
        assert "backup_20240101_010000" in backups
        assert "backup_20240102_020000" in backups
        assert "not_a_backup" not in backups

    def test_list_backups_ignores_files(self, tmp_path):
        manager = BackupManager(backup_dir=tmp_path)
        (tmp_path / "backup_20240101_010000").mkdir()
        (tmp_path / "backup_20240101_010000.txt").write_text("not a dir")
        backups = manager.list_backups()
        assert len(backups) == 1

    def test_create_backup_returns_timestamp(self, tmp_path):
        manager = BackupManager(backup_dir=tmp_path)
        source = tmp_path / "source"
        source.mkdir()
        (source / "file.txt").write_text("hello")
        ts = manager.create_backup(source)
        assert isinstance(ts, str)
        assert len(ts) > 0

    def test_create_backup_copies_tree(self, tmp_path):
        manager = BackupManager(backup_dir=tmp_path)
        source = tmp_path / "src"
        source.mkdir()
        (source / "a.txt").write_text("A")
        (source / "sub").mkdir()
        (source / "sub" / "b.txt").write_text("B")
        ts = manager.create_backup(source, "test backup")
        backup_path = tmp_path / f"backup_{ts}"
        assert (backup_path / "a.txt").read_text() == "A"
        assert (backup_path / "sub" / "b.txt").read_text() == "B"
