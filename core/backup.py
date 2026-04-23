"""
Data Backup System - Simple version.
"""
import shutil
from pathlib import Path
from datetime import datetime


class BackupManager:
    """Simple backup manager."""
    
    def __init__(self, backup_dir=None):
        self.backup_dir = backup_dir or Path.home() / ".cache" / "ai_research_os" / "backups"
        self.backup_dir.mkdir(parents=True, exist_ok=True)
    
    def create_backup(self, source_dir, description=""):
        """Create a backup."""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        backup_name = f"backup_{timestamp}"
        backup_path = self.backup_dir / backup_name
        shutil.copytree(source_dir, backup_path)
        return timestamp
    
    def list_backups(self):
        """List backups."""
        backups = []
        for path in self.backup_dir.glob("backup_*"):
            if path.is_dir():
                backups.append(path.name)
        return backups


def get_backup_manager():
    """Get backup manager."""
    return BackupManager()
