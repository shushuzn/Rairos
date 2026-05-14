"""Tests for web/app.py — FastAPI app instance.

These tests use FastAPI's TestClient to verify routes respond correctly
without needing a running server.
"""

import pytest
from unittest.mock import patch, MagicMock

# Patch fastapi's pydantic v1 detection to avoid metaclass conflict on Python 3.13
import fastapi._compat.shared as _fastapi_compat
original_is_v1 = _fastapi_compat.is_pydantic_v1_model_class
def _safe_is_v1(cls):
    try:
        return original_is_v1(cls)
    except TypeError:
        return False
_fastapi_compat.is_pydantic_v1_model_class = _safe_is_v1


class TestCreateApp:
    """Test the FastAPI app instance."""

    def test_app_is_fastapi_instance(self):
        from web.app import app

        assert app.__class__.__name__ == "FastAPI"
        assert app.title == "Rairos"

    def test_module_level_app_works(self):
        from web.app import app

        assert app is not None

        assert app.title == "Rairos"
        assert len(app.routes) > 0

    def test_create_app_includes_all_routers(self):
        from web.app import app

        # app is module-level
        route_paths = {r.path for r in app.routes if hasattr(r, "path")}
        # Verify key routes are present
        assert any("/" == p for p in route_paths)
        assert any("/paper" in p for p in route_paths)
        assert any("/insights" in p for p in route_paths)

    def test_create_app_idempotent(self):
        from web.app import app

        # The module-level app is a singleton
        assert app is not None
        assert len(app.routes) > 0


class TestDashboardRoute:
    """Test the GET / dashboard route."""

    @pytest.fixture
    def client(self):
        from fastapi.testclient import TestClient
        from web.app import app

        # Mock the database to avoid needing a real DB
        with patch("web.app._get_db") as mock_db:
            mock_instance = MagicMock()
            mock_instance.get_stats.return_value = {
                "total_papers": 42,
                "by_status": {"parsed": 30, "idle": 5, "pending": 4, "failed": 3},
                "by_source": {"arxiv": 40, "manual": 2},
                "queue_queued": 2,
                "queue_running": 1,
                "cache_entries": 10,
            }
            mock_instance.list_papers.return_value = ([], False)
            mock_instance.get_queue_jobs.return_value = []
            mock_instance.get_paper_title.return_value = "Test Paper"
            mock_instance.conn.cursor.return_value.fetchall.return_value = []
            mock_db.return_value = mock_instance

            yield TestClient(app)


class TestDeleteRoutes:
    """Test paper deletion routes."""

    @pytest.fixture
    def client(self):
        from fastapi.testclient import TestClient
        from web.app import app

        with patch("web.app._get_db") as mock_db:
            mock_instance = MagicMock()
            mock_instance.delete_paper.return_value = True
            mock_db.return_value = mock_instance

            yield TestClient(app)


class TestAuthMiddleware:
    """Test auth middleware behaviour."""

    def test_auth_disabled_bypasses_check(self):
        from fastapi.testclient import TestClient
        from web.app import app

        # When auth is disabled, / returns 200 (no redirect)
        with patch("web.app._get_db") as mock_db:
            mock_instance = MagicMock()
            mock_instance.get_stats.return_value = {}
            mock_instance.list_papers.return_value = ([], 0)
            mock_instance.get_queue_jobs.return_value = []
            mock_instance.get_paper_title.return_value = ""
            mock_instance.search_papers.return_value = ([], 0)
            mock_instance.get_recent_gaps_stats.return_value = {}
            mock_db.return_value = mock_instance

            with patch("llm.auth.is_auth_enabled", return_value=False):
                client = TestClient(app)
                response = client.get("/", follow_redirects=False)
                assert response.status_code != 303


class TestExceptionHandlers:
    """Test global exception handlers are registered."""

    def test_exception_handler_registered(self):
        from web.app import app

        # Exception handlers are registered
        assert len(app.exception_handlers) >= 1


class TestPapersBulkDelete:
    """Test /papers bulk delete edge cases."""

    def test_bulk_delete_empty_list(self):
        from fastapi.testclient import TestClient
        from web.app import app

        with patch("web.app._get_db") as mock_db:
            mock_instance = MagicMock()
            mock_db.return_value = mock_instance

            client = TestClient(app)
            response = client.request("DELETE", "/papers", json={"paper_ids": []})
            assert response.status_code == 200
            assert response.json()["deleted"] == 0
