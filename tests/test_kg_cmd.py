"""Tests for cli/cmd/kg.py."""

import pytest
from cli.cmd.kg import _build_kg_parser


class TestKgParser:
    def test_build_parser(self):
        import argparse

        sp = argparse.ArgumentParser().add_subparsers()
        parser = _build_kg_parser(sp)
        assert isinstance(parser, argparse.ArgumentParser)
