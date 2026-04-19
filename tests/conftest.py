"""Shared fixtures for tests."""
from __future__ import annotations

import pytest

# Frozen date constants used across Tier 4 tests
FROZEN_DATE = "2024-06-15"
FROZEN_DATE_ISO = "2024-06-15"
FROZEN_YEAR = "2024"


@pytest.fixture
def frozen_date_iso() -> str:
    """Return the frozen date ISO string '2024-06-15'."""
    return FROZEN_DATE_ISO


@pytest.fixture
def frozen_year() -> str:
    """Return the frozen year constant '2024'."""
    return FROZEN_YEAR


# ---------------------------------------------------------------------------
# freezegun integration
# ---------------------------------------------------------------------------

def pytest_configure(config: pytest.Config) -> None:
    """Register the freeze_time marker so @pytest.mark.freeze_time works."""
    config.addinivalue_line(
        "markers",
        "freeze_time: freeze datetime to 2024-06-15 (uses freezegun)",
    )


@pytest.fixture(autouse=True)
def freeze_time_fixture():
    """Automatically freeze time to 2024-06-15 for every test in the session.

    This ensures date/time-dependent tests are deterministic regardless of when
    they run. Tests that need to verify date-related logic should hardcode the
    expected frozen value (e.g. assert result == "2024-06-15").
    """
    from freezegun import freeze_time as _freeze_time

    with _freeze_time(FROZEN_DATE):
        yield


def freeze_time(datetime_str: str = FROZEN_DATE):
    """Return a freezegun freeze_time context manager pinned to the given datetime string.

    Usage in tests (alternative to autouse fixture):
        @pytest.mark.freeze_time("2024-06-15")
        def test_something():
            ...
    """
    from freezegun import freeze_time as _freeze_time

    return _freeze_time(datetime_str)
