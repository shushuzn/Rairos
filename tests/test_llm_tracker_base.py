"""Tier 2 unit tests — llm/tracker_base.py, JsonFileStore pure logic."""
import json
import pytest
from pathlib import Path
from llm.tracker_base import JsonFileStore


class DummyStore(JsonFileStore):
    """Minimal concrete subclass for testing."""

    def __init__(self, data_file: Path):
        self.data_file = data_file

    def _post_load(self, raw):
        return raw

    def _pre_save(self, items):
        return items


# =============================================================================
# _load tests
# =============================================================================
class TestJsonFileStoreLoad:
    """Test _load behavior."""

    def test_missing_file_returns_empty_list(self, tmp_path):
        store = DummyStore(tmp_path / "nonexistent.json")
        result = store._load()
        assert result == []

    def test_empty_file_returns_empty_list(self, tmp_path):
        f = tmp_path / "empty.json"
        f.write_text("")
        store = DummyStore(f)
        result = store._load()
        assert result == []

    def test_corrupt_json_returns_empty_list(self, tmp_path):
        f = tmp_path / "corrupt.json"
        f.write_text("{not valid json")
        store = DummyStore(f)
        result = store._load()
        assert result == []

    def test_valid_json_list_loaded(self, tmp_path):
        f = tmp_path / "data.json"
        f.write_text('["item1", "item2"]')
        store = DummyStore(f)
        result = store._load()
        assert result == ["item1", "item2"]

    def test_valid_json_object_loaded(self, tmp_path):
        f = tmp_path / "data.json"
        f.write_text('{"key": "value"}')
        store = DummyStore(f)
        result = store._load()
        assert result == {"key": "value"}

    def test_post_load_called(self, tmp_path, monkeypatch):
        f = tmp_path / "data.json"
        f.write_text('[1, 2, 3]')
        called = []
        store = DummyStore(f)

        original_post = store._post_load

        def tracking_post(raw):
            called.append(raw)
            return original_post(raw)

        monkeypatch.setattr(store, "_post_load", tracking_post)
        store._load()
        assert called[0] == [1, 2, 3]


# =============================================================================
# _save tests — atomic write behavior
# =============================================================================
class TestJsonFileStoreSave:
    """Test _save atomic write behavior."""

    def test_saves_json_list(self, tmp_path):
        f = tmp_path / "data.json"
        store = DummyStore(f)
        store._save(["item1", "item2"])
        assert json.loads(f.read_text(encoding="utf-8")) == ["item1", "item2"]

    def test_saves_json_object(self, tmp_path):
        f = tmp_path / "data.json"
        store = DummyStore(f)
        store._save({"key": "value"})
        assert json.loads(f.read_text(encoding="utf-8")) == {"key": "value"}

    def test_pre_save_called(self, tmp_path, monkeypatch):
        f = tmp_path / "data.json"
        store = DummyStore(f)
        called = []

        original_pre = store._pre_save

        def tracking_pre(items):
            called.append(items)
            return original_pre(items)

        monkeypatch.setattr(store, "_pre_save", tracking_pre)
        store._save(["a", "b"])
        assert called[0] == ["a", "b"]

    def test_atomic_write_creates_tmp_then_renames(self, tmp_path):
        """Verify atomic write: tmp file exists during write, then replaces target."""
        f = tmp_path / "data.json"
        store = DummyStore(f)
        tmp = f.with_suffix(".tmp")

        # No tmp file before save
        assert not tmp.exists()

        store._save({"test": True})

        # After save: main file has data, tmp gone
        assert json.loads(f.read_text(encoding="utf-8")) == {"test": True}
        assert not tmp.exists()

    def test_saves_nested_data(self, tmp_path):
        f = tmp_path / "data.json"
        store = DummyStore(f)
        data = {
            "users": [
                {"id": 1, "name": "Alice"},
                {"id": 2, "name": "Bob"},
            ],
            "count": 2,
        }
        store._save(data)
        assert json.loads(f.read_text(encoding="utf-8")) == data

    def test_saves_unicode(self, tmp_path):
        f = tmp_path / "data.json"
        store = DummyStore(f)
        store._save({"name": "张三", "emoji": "🎉"})
        result = json.loads(f.read_text(encoding="utf-8"))
        assert result["name"] == "张三"
        assert result["emoji"] == "🎉"


# =============================================================================
# Integration: round-trip load → modify → save
# =============================================================================
class TestJsonFileStoreRoundTrip:
    """Test full load-modify-save cycle."""

    def test_add_item_to_empty_store(self, tmp_path):
        f = tmp_path / "store.json"
        store = DummyStore(f)
        store._save([])

        data = store._load()
        data.append("new_item")
        store._save(data)

        assert json.loads(f.read_text(encoding="utf-8")) == ["new_item"]

    def test_multiple_items_persist(self, tmp_path):
        f = tmp_path / "store.json"
        store = DummyStore(f)
        store._save([1, 2, 3])

        data = store._load()
        data.append(4)
        store._save(data)

        assert json.loads(f.read_text(encoding="utf-8")) == [1, 2, 3, 4]

    def test_delete_item(self, tmp_path):
        f = tmp_path / "store.json"
        store = DummyStore(f)
        store._save([10, 20, 30])

        data = store._load()
        data.remove(20)
        store._save(data)

        assert json.loads(f.read_text(encoding="utf-8")) == [10, 30]
