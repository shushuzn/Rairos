"""JsonFileStore: reusable JSON file persistence mixin for tracker classes."""
from __future__ import annotations

import json
import logging
from pathlib import Path
from typing import Any, List, TypeVar

T = TypeVar("T")

# Reusable sentinel — subclasses set their data_file attribute
JsonFileStoreSelf = TypeVar("JsonFileStoreSelf", bound="JsonFileStore")


class JsonFileStore:
    """Mixin providing safe JSON file load/save for dataclass-based trackers.

    Subclasses must set:
        data_file: Path   — path to the JSON file

    Subclasses may override:
        _pre_load(self) -> List[dict]:   raw dicts from file (default: json.load)
        _pre_save(self, items) -> List[dict]:  dicts before writing (default: each item to dict)

    Each dataclass tracker also needs from_dict(data) -> T and to_dict(item) -> dict
    class methods, or can override _from_dict / _to_dict with lambdas.
    """

    data_file: Path

    # ─── Load ──────────────────────────────────────────────────────────────────

    def _load(self) -> List[Any]:
        """Load items from JSON file. Returns [] if file is absent or corrupt."""
        if not self.data_file.exists():
            return []

        try:
            with open(self.data_file, "r", encoding="utf-8", errors="replace") as f:
                raw: List[dict] = json.load(f)
        except (json.JSONDecodeError, IOError):
            logging.warning(
                "Failed to load %s from %s. Returning empty list.",
                type(self).__name__,
                self.data_file,
            )
            return []

        return self._post_load(raw)

    def _post_load(self, raw: List[dict]) -> List[Any]:
        """Convert raw dicts to dataclass instances. Override for custom conversion."""
        return raw  # subclasses override when dataclass has from_dict()

    # ─── Save ──────────────────────────────────────────────────────────────────

    def _save(self, items: List[Any]) -> None:
        """Save items to JSON file atomically (write-then-rename)."""
        raw = self._pre_save(items)
        text = json.dumps(raw, ensure_ascii=False, indent=2)

        # Atomic write: write to temp file, then rename over the target
        tmp = self.data_file.with_suffix(".tmp")
        try:
            tmp.write_text(text, encoding="utf-8")
            tmp.replace(self.data_file)
        except OSError as e:
            logging.warning(
                "Failed to save %s to %s: %s",
                type(self).__name__,
                self.data_file,
                e,
            )

    def _pre_save(self, items: List[Any]) -> List[dict]:
        """Convert dataclass instances to dicts before saving. Override for custom conversion."""
        return items  # subclasses override when dataclass has to_dict()
