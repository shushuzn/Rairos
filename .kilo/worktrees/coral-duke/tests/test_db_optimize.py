"""Tests for database optimization functionality."""
from db.optimize import (
    OPTIMIZATION_INDEXES,
    PRAGMA_SETTINGS,
)


def test_optimization_indexes_defined():
    """Test that optimization indexes are defined."""
    assert len(OPTIMIZATION_INDEXES) > 0
    for idx_name, create_sql in OPTIMIZATION_INDEXES:
        assert idx_name.startswith("idx_")
        assert "CREATE INDEX" in create_sql


def test_pragma_settings_defined():
    """Test that PRAGMA settings are defined."""
    assert len(PRAGMA_SETTINGS) > 0
    for pragma, value in PRAGMA_SETTINGS:
        assert isinstance(pragma, str)
        assert isinstance(value, str)
        assert value != ""


def test_optimization_index_names_unique():
    """Test that all optimization index names are unique."""
    index_names = [idx_name for idx_name, _ in OPTIMIZATION_INDEXES]
    unique_names = set(index_names)
    assert len(index_names) == len(unique_names)


def test_pragma_settings_format():
    """Test PRAGMA settings have correct format."""
    for pragma, value in PRAGMA_SETTINGS:
        assert pragma.islower()
        assert value.strip() != ""
