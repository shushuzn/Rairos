"""Tests for auth, briefing_distributor, eval_gap_monitor."""

import json
import pytest
from llm.auth import (
    _hash_password,
    _generate_salt,
    is_auth_enabled,
    setup_admin,
    verify_login,
    create_session,
    validate_session,
    revoke_session,
)
from llm.briefing_distributor import (
    make_short_id,
    _parse_markdown_sections,
    _escape_html,
    render_distributed_briefing,
    render_distributor_panel,
)
from llm.eval_gap_monitor import detect_deployment_claims, check_eval_gaps, render_eval_gap_html


# ── auth ─────────────────────────────────────────────────────────────────────


class TestHashPassword:
    def test_deterministic(self):
        h1 = _hash_password("secret", "salt123")
        h2 = _hash_password("secret", "salt123")
        assert h1 == h2

    def test_different_password(self):
        h1 = _hash_password("a", "s")
        h2 = _hash_password("b", "s")
        assert h1 != h2

    def test_different_salt(self):
        h1 = _hash_password("p", "s1")
        h2 = _hash_password("p", "s2")
        assert h1 != h2


class TestGenerateSalt:
    def test_length(self):
        salt = _generate_salt()
        assert len(salt) == 32  # 16 bytes hex = 32 chars

    def test_random(self):
        s1 = _generate_salt()
        s2 = _generate_salt()
        assert s1 != s2


class TestAuthFlow:
    def test_not_enabled_by_default(self, monkeypatch, tmp_path):
        import llm.auth as mod

        monkeypatch.setattr(mod, "AUTH_FILE", tmp_path / "auth.json")
        assert is_auth_enabled() is False

    def test_setup_and_verify(self, monkeypatch, tmp_path):
        import llm.auth as mod

        auth_p = tmp_path / "auth.json"
        monkeypatch.setattr(mod, "AUTH_FILE", auth_p)
        assert setup_admin("admin", "pass123") is True
        assert is_auth_enabled() is True
        assert verify_login("admin", "pass123") is True
        assert verify_login("admin", "wrong") is False
        assert verify_login("nobody", "pass123") is False

    def test_setup_twice_fails(self, monkeypatch, tmp_path):
        import llm.auth as mod

        monkeypatch.setattr(mod, "AUTH_FILE", tmp_path / "auth.json")
        assert setup_admin("admin", "p") is True
        assert setup_admin("admin2", "p2") is False


class TestSessions:
    def test_create_and_validate(self, monkeypatch, tmp_path):
        import llm.auth as mod

        monkeypatch.setattr(mod, "SESSIONS_FILE", tmp_path / "sessions.json")
        token = create_session("testuser")
        assert len(token) == 64  # 32 bytes hex
        assert validate_session(token) == "testuser"

    def test_validate_invalid(self, monkeypatch, tmp_path):
        import llm.auth as mod

        monkeypatch.setattr(mod, "SESSIONS_FILE", tmp_path / "sessions.json")
        assert validate_session("badtoken") is None

    def test_revoke(self, monkeypatch, tmp_path):
        import llm.auth as mod

        monkeypatch.setattr(mod, "SESSIONS_FILE", tmp_path / "sessions.json")
        token = create_session("user")
        revoke_session(token)
        assert validate_session(token) is None

    def test_expired_session(self, monkeypatch, tmp_path):
        import llm.auth as mod
        import time

        monkeypatch.setattr(mod, "SESSIONS_FILE", tmp_path / "sessions.json")
        token = create_session("user")
        # Fast-forward beyond TTL
        from time import time as real_time

        monkeypatch.setattr(time, "time", lambda: real_time() + mod.SESSION_TTL + 1)
        assert validate_session(token) is None


# ── briefing_distributor ────────────────────────────────────────────────────


class TestMakeShortId:
    def test_deterministic(self):
        id1 = make_short_id("Test Paper", "1234.5678")
        id2 = make_short_id("Test Paper", "1234.5678")
        assert id1 == id2
        assert len(id1) == 6

    def test_different_input(self):
        id1 = make_short_id("A", "1")
        id2 = make_short_id("B", "2")
        assert id1 != id2


class TestParseMarkdownSections:
    def test_empty(self):
        result = _parse_markdown_sections("")
        assert result == {"_body": ""}

    def test_headers(self):
        md = "# Title\n\n## Summary\nThis is a summary.\n\n## Method\nExperiments here."
        result = _parse_markdown_sections(md)
        assert result["_title"] == "Title"
        assert "This is a summary" in result.get("summary", "")
        assert "Experiments" in result.get("method", "")

    def test_no_headers(self):
        result = _parse_markdown_sections("Just some text.")
        assert result == {"_body": "Just some text."}


class TestEscapeHtml:
    def test_special_chars(self):
        assert "&amp;" in _escape_html("a & b")
        assert "&lt;" in _escape_html("a < b")
        assert "&gt;" in _escape_html("a > b")
        assert "&quot;" in _escape_html('a " b')

    def test_safe_text(self):
        assert _escape_html("hello world") == "hello world"


class TestRenderDistributed:
    def test_researcher_audience(self):
        md = "# Paper\n\n## Summary\nTest summary.\n\n## Verdict\nvalidates"
        html = render_distributed_briefing("1234", "Test", md, "researcher")
        assert "briefing-dist" in html
        assert "Test summary" in html

    def test_phd_advisor(self):
        md = "# Paper\n\n## Summary\nTest.\n\n## Methodology\nNovel method."
        html = render_distributed_briefing("1234", "Test", md, "phd_advisor")
        assert "briefing-dist" in html

    def test_industry_engineer(self):
        md = "# Paper\n\n## Summary\nTest.\n\n## Experiments\nResults here."
        html = render_distributed_briefing("1234", "Test", md, "industry_engineer")
        assert "briefing-dist" in html

    def test_policy_maker(self):
        md = "# Paper\n\n## Summary\nTest.\n\n## Limitations\nSome limits."
        html = render_distributed_briefing("1234", "Test", md, "policy_maker")
        assert "briefing-dist" in html

    def test_share_link_created(self, monkeypatch, tmp_path):
        import llm.briefing_distributor as mod

        monkeypatch.setattr(mod, "LINKS_FILE", tmp_path / "links.json")
        render_distributed_briefing("1234", "Test", "# T\n\n## Summary\nS", "researcher")
        links = json.loads((tmp_path / "links.json").read_text(encoding="utf-8"))
        assert len(links) >= 1


class TestRenderDistributorPanel:
    def test_panel(self, monkeypatch, tmp_path):
        import llm.briefing_distributor as mod

        monkeypatch.setattr(mod, "LINKS_FILE", tmp_path / "links.json")
        html = render_distributor_panel("1234", "Test Paper")
        assert "Briefing Distributor" in html
        assert "researcher" in html


# ── eval_gap_monitor ─────────────────────────────────────────────────────────


class TestDetectDeploymentClaims:
    def test_no_keyword(self):
        assert detect_deployment_claims("A study of ML", "abstract text") is None

    def test_with_year(self):
        result = detect_deployment_claims(
            "Deployment of LLMs in production", "Planned deployment in 2025"
        )
        assert result == "2025"

    def test_real_world(self):
        result = detect_deployment_claims("Real-world ML systems", "deployed in 2026")
        assert result == "2026"

    def test_no_year(self):
        result = detect_deployment_claims("Deployment of ML", "no year mentioned")
        assert result is None


class TestCheckEvalGaps:
    def test_no_file(self, monkeypatch, tmp_path):
        import llm.eval_gap_monitor as mod

        monkeypatch.setattr(mod, "PAPERS_DB", tmp_path / "nonexistent.json")
        result = check_eval_gaps()
        assert result["alert_count"] == 0
        assert result["total_domains_checked"] == 0

    def test_empty_papers(self, monkeypatch, tmp_path):
        import llm.eval_gap_monitor as mod

        p = tmp_path / "papers.json"
        p.write_text(json.dumps({"papers": []}), encoding="utf-8")
        monkeypatch.setattr(mod, "PAPERS_DB", p)
        result = check_eval_gaps()
        assert result["alert_count"] == 0


class TestRenderEvalGapHtml:
    def test_no_alerts(self):
        html = render_eval_gap_html({"alerts": [], "alert_count": 0, "total_domains_checked": 5})
        assert "No evaluation gaps detected" in html

    def test_with_alerts(self):
        data = {
            "alerts": [
                {
                    "category": "cs.AI",
                    "paper_count": 2,
                    "nearest_deployment_year": 2027,
                    "headroom_years": 2,
                    "ratio": 0.05,
                    "deploying_papers": [{"title": "Test Deployment", "year": "2027"}],
                    "severity": "medium",
                }
            ],
            "alert_count": 1,
            "total_domains_checked": 5,
        }
        html = render_eval_gap_html(data)
        assert "cs.AI" in html
        assert "medium" not in html  # severity is in alert dict, not rendered as class
