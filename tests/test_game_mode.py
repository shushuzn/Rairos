"""Tests for llm/game_mode.py — badges and progression system."""

import json
import tempfile
from pathlib import Path
from unittest.mock import patch

from llm.game_mode import (
    Badge,
    _load_capsules,
    _load_badges,
    _save_badges,
    _check_gap_extractor,
    _check_evolution_master,
    _check_bold_explorer,
    _check_rigor_rater,
    _check_paradigm_sentinel,
    compute_badges,
    render_game_mode_html,
)


class TestBadge:
    """Test Badge dataclass."""

    def test_creates_with_required_fields(self):
        """Should create Badge with required fields."""
        badge = Badge(id="test", name="Test", description="A test badge", icon="🏆")
        assert badge.id == "test"
        assert badge.earned is False
        assert badge.earned_at is None

    def test_earned_defaults_to_false(self):
        """earned should default to False."""
        badge = Badge(id="t", name="T", description="t", icon="t")
        assert badge.earned is False

    def test_earned_at_optional(self):
        """earned_at is optional."""
        badge = Badge(id="t", name="T", description="t", icon="t", earned=True)
        assert badge.earned_at is None


class TestLoadCapsules:
    """Test _load_capsules()."""

    def test_returns_empty_list_when_file_missing(self):
        """Should return empty list if capsules.json doesn't exist."""
        with patch("llm.game_mode.CAPSULES_PATH", Path("/nonexistent/path/capsules.json")):
            result = _load_capsules()
            assert result == []

    def test_parses_capsules_json(self):
        """Should parse capsules from JSON file."""
        with tempfile.NamedTemporaryFile(
            mode="w", suffix=".json", delete=False, encoding="utf-8"
        ) as f:
            json.dump({"capsules": [{"id": "c1", "status": "active"}]}, f)
            f.flush()
            with patch("llm.game_mode.CAPSULES_PATH", Path(f.name)):
                result = _load_capsules()
                assert len(result) == 1
                assert result[0]["id"] == "c1"
        Path(f.name).unlink()


class TestLoadBadges:
    """Test _load_badges()."""

    def test_returns_empty_dict_when_file_missing(self):
        """Should return empty dict if badges.json doesn't exist."""
        with patch("llm.game_mode.BADGES_PATH", Path("/nonexistent/path/badges.json")):
            result = _load_badges()
            assert result == {}


class TestSaveBadges:
    """Test _save_badges()."""

    def test_writes_json_file(self):
        """Should write badges to JSON file."""
        with tempfile.TemporaryDirectory() as tmpdir:
            badges_file = Path(tmpdir) / "badges.json"
            with patch("llm.game_mode.BADGES_PATH", badges_file):
                _save_badges({"contradiction_hunter": {"earned_at": "2024-01-01"}})
                assert badges_file.exists()
                data = json.loads(badges_file.read_text(encoding="utf-8"))
                assert "contradiction_hunter" in data


class TestCheckGapExtractor:
    """Test _check_gap_extractor()."""

    def test_returns_false_when_fewer_than_10_active(self):
        """Should return False when fewer than 10 active capsules."""
        with patch(
            "llm.game_mode._load_capsules",
            return_value=[
                {"status": "active"},
                {"status": "active"},
                {"status": "active"},
            ],
        ):
            assert _check_gap_extractor() is False

    def test_returns_true_when_10_or_more_active(self):
        """Should return True when 10+ active capsules."""
        capsules = [{"status": "active"} for _ in range(10)]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_gap_extractor() is True

    def test_counts_empty_status_as_active(self):
        """Empty status should count as active."""
        capsules = [{"status": ""} for _ in range(10)]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_gap_extractor() is True

    def test_ignores_archived_capsules(self):
        """Archived capsules should not count."""
        capsules = [{"status": "active"} for _ in range(9)] + [{"status": "archived"}]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_gap_extractor() is False


class TestCheckEvolutionMaster:
    """Test _check_evolution_master()."""

    def test_returns_false_when_no_evolved_capsules(self):
        """Should return False when no capsules have evolved_from."""
        with patch(
            "llm.game_mode._load_capsules",
            return_value=[
                {"id": "c1"},
                {"id": "c2"},
            ],
        ):
            assert _check_evolution_master() is False

    def test_returns_true_when_evolved_from_present(self):
        """Should return True when any capsule has evolved_from."""
        with patch(
            "llm.game_mode._load_capsules",
            return_value=[
                {"id": "c1", "evolved_from": "c0"},
            ],
        ):
            assert _check_evolution_master() is True

    def test_returns_true_when_source_cap_id_present(self):
        """Should return True when any capsule has source_cap_id."""
        with patch(
            "llm.game_mode._load_capsules",
            return_value=[
                {"id": "c1", "source_cap_id": "c0"},
            ],
        ):
            assert _check_evolution_master() is True


class TestCheckBoldExplorer:
    """Test _check_bold_explorer()."""

    def test_counts_theoretical_gap_capsules(self):
        """theoretical_gap gap type counts toward bold explorer."""
        capsules = [
            {"action_gap_type": "theoretical_gap", "polarity": "positive"} for _ in range(5)
        ]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_bold_explorer() is True

    def test_counts_negative_polarity_capsules(self):
        """Negative polarity counts toward bold explorer."""
        capsules = [{"action_gap_type": "other", "polarity": "negative"} for _ in range(5)]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_bold_explorer() is True

    def test_requires_5_or_more(self):
        """Need 5+ bold capsules."""
        capsules = [
            {"action_gap_type": "theoretical_gap", "polarity": "positive"} for _ in range(4)
        ]
        with patch("llm.game_mode._load_capsules", return_value=capsules):
            assert _check_bold_explorer() is False


class TestCheckRigorRater:
    """Test _check_rigor_rater()."""

    def test_returns_false_when_flag_missing(self):
        """Should return False when .rigor_rated flag doesn't exist."""
        with patch("llm.game_mode.Path.exists", return_value=False):
            assert _check_rigor_rater() is False

    def test_returns_false_when_count_below_10(self):
        """Should return False when count < 10."""
        with patch("llm.game_mode.Path.exists", return_value=True):
            with patch("llm.game_mode.Path.read_text", return_value="5"):
                assert _check_rigor_rater() is False

    def test_returns_true_when_count_10_or_more(self):
        """Should return True when count >= 10."""
        with patch("llm.game_mode.Path.exists", return_value=True):
            with patch("llm.game_mode.Path.read_text", return_value="15"):
                assert _check_rigor_rater() is True

    def test_returns_false_for_non_digit(self):
        """Should return False for non-numeric content."""
        with patch("llm.game_mode.Path.exists", return_value=True):
            with patch("llm.game_mode.Path.read_text", return_value="abc"):
                assert _check_rigor_rater() is False


class TestCheckParadigmSentinel:
    """Test _check_paradigm_sentinel()."""

    def test_returns_false_when_check_fails(self):
        """Should return False when check_paradigm_concentration raises."""
        with patch(
            "llm.paradigm_monitor.check_paradigm_concentration", side_effect=Exception("error")
        ):
            assert _check_paradigm_sentinel() is False

    def test_returns_true_when_alert_triggered(self):
        """Should return True when alert_triggered is True."""
        with patch(
            "llm.paradigm_monitor.check_paradigm_concentration",
            return_value={"alert_triggered": True},
        ):
            assert _check_paradigm_sentinel() is True

    def test_returns_false_when_alert_not_triggered(self):
        """Should return False when alert_triggered is False."""
        with patch(
            "llm.paradigm_monitor.check_paradigm_concentration",
            return_value={"alert_triggered": False},
        ):
            assert _check_paradigm_sentinel() is False


class TestComputeBadges:
    """Test compute_badges()."""

    def test_returns_list_of_badges(self):
        """Should return a list of Badge objects."""
        with patch("llm.game_mode._load_badges", return_value={}):
            with patch("llm.game_mode._check_contradiction_hunter", return_value=False):
                with patch("llm.game_mode._check_gap_extractor", return_value=False):
                    with patch("llm.game_mode._check_evolution_master", return_value=False):
                        with patch("llm.game_mode._check_bold_explorer", return_value=False):
                            with patch("llm.game_mode._check_rigor_rater", return_value=False):
                                with patch(
                                    "llm.game_mode._check_paradigm_sentinel", return_value=False
                                ):
                                    badges = compute_badges()
                                    assert isinstance(badges, list)
                                    assert all(isinstance(b, Badge) for b in badges)

    def test_contradiction_hunter_earned(self):
        """contradiction_hunter should be earned when check returns True."""
        with patch("llm.game_mode._load_badges", return_value={}):
            with patch("llm.game_mode._check_contradiction_hunter", return_value=True):
                with patch("llm.game_mode._check_gap_extractor", return_value=False):
                    with patch("llm.game_mode._check_evolution_master", return_value=False):
                        with patch("llm.game_mode._check_bold_explorer", return_value=False):
                            with patch("llm.game_mode._check_rigor_rater", return_value=False):
                                with patch(
                                    "llm.game_mode._check_paradigm_sentinel", return_value=False
                                ):
                                    badges = compute_badges()
                                    ch = next(b for b in badges if b.id == "contradiction_hunter")
                                    assert ch.earned is True

    def test_saves_badges_to_file(self):
        """Should call _save_badges after computing."""
        with patch("llm.game_mode._load_badges", return_value={}):
            with patch("llm.game_mode._check_contradiction_hunter", return_value=False):
                with patch("llm.game_mode._check_gap_extractor", return_value=False):
                    with patch("llm.game_mode._check_evolution_master", return_value=False):
                        with patch("llm.game_mode._check_bold_explorer", return_value=False):
                            with patch("llm.game_mode._check_rigor_rater", return_value=False):
                                with patch(
                                    "llm.game_mode._check_paradigm_sentinel", return_value=False
                                ):
                                    with patch("llm.game_mode._save_badges") as mock_save:
                                        compute_badges()
                                        mock_save.assert_called_once()

    def test_preserves_previously_earned_at(self):
        """Should preserve earned_at from previous badges file."""
        with patch(
            "llm.game_mode._load_badges",
            return_value={"contradiction_hunter": {"earned_at": "2024-01-01T00:00:00"}},
        ):
            with patch("llm.game_mode._check_contradiction_hunter", return_value=True):
                with patch("llm.game_mode._check_gap_extractor", return_value=False):
                    with patch("llm.game_mode._check_evolution_master", return_value=False):
                        with patch("llm.game_mode._check_bold_explorer", return_value=False):
                            with patch("llm.game_mode._check_rigor_rater", return_value=False):
                                with patch(
                                    "llm.game_mode._check_paradigm_sentinel", return_value=False
                                ):
                                    badges = compute_badges()
                                    ch = next(b for b in badges if b.id == "contradiction_hunter")
                                    assert "2024-01-01" in (ch.earned_at or "")


class TestRenderGameModeHtml:
    """Test render_game_mode_html()."""

    def test_renders_html_string(self):
        """Should return HTML string."""
        badges = [
            Badge(id="test", name="Test Badge", description="A test", icon="🏆", earned=True),
            Badge(id="locked", name="Locked", description="Not earned", icon="🔒", earned=False),
        ]
        html = render_game_mode_html(badges)
        assert isinstance(html, str)
        assert "game-mode" in html
        assert "Test Badge" in html

    def test_shows_earned_count(self):
        """Should show earned count."""
        badges = [
            Badge(id="b1", name="B1", description="d", icon="🏆", earned=True),
            Badge(id="b2", name="B2", description="d", icon="🔒", earned=False),
        ]
        html = render_game_mode_html(badges)
        assert "1/2" in html

    def test_renders_earned_badge(self):
        """Earned badges should show without opacity."""
        badges = [
            Badge(id="earned", name="Earned Badge", description="Desc", icon="🏆", earned=True),
        ]
        html = render_game_mode_html(badges)
        assert "Earned Badge" in html

    def test_renders_locked_badge(self):
        """Locked badges should show with opacity."""
        badges = [
            Badge(id="locked", name="Locked Badge", description="Desc", icon="🔒", earned=False),
        ]
        html = render_game_mode_html(badges)
        assert "Locked Badge" in html
        assert "opacity:0.3" in html

    def test_accepts_none_and_calls_compute_badges(self):
        """Should call compute_badges() when badges is None."""
        with patch("llm.game_mode.compute_badges", return_value=[]) as mock_compute:
            render_game_mode_html(None)
            mock_compute.assert_called_once()
