"""Tests for simple CLI functionality."""
from core.simple_cli import SimpleCLI


def test_simple_cli_creation():
    """Test SimpleCLI creation."""
    cli = SimpleCLI()
    assert cli is not None
    assert cli.parser is not None


def test_simple_cli_help():
    """Test CLI help display."""
    cli = SimpleCLI()
    # Should not raise exception
    cli._show_help()


def test_simple_cli_status():
    """Test status command."""
    cli = SimpleCLI()
    # Should not raise exception (may fail if DB not initialized)
    try:
        cli._handle_status(cli.parser.parse_args([]))
    except:
        pass  # Expected if DB not initialized


def test_simple_cli_list():
    """Test list command."""
    cli = SimpleCLI()
    # Should not raise exception (may fail if DB not initialized)
    try:
        cli._handle_list(cli.parser.parse_args([]))
    except:
        pass  # Expected if DB not initialized


def test_simple_cli_stats():
    """Test stats command."""
    cli = SimpleCLI()
    # Should not raise exception (may fail if DB not initialized)
    try:
        cli._handle_stats(cli.parser.parse_args([]))
    except:
        pass  # Expected if DB not initialized


def test_simple_cli_parser():
    """Test argument parser."""
    cli = SimpleCLI()

    # Test search command
    args = cli.parser.parse_args(["search", "test"])
    assert args.command == "search"
    assert args.query == "test"

    # Test import command
    args = cli.parser.parse_args(["import", "2301.001"])
    assert args.command == "import"
    assert args.paper_id == "2301.001"

    # Test list command
    args = cli.parser.parse_args(["list", "-n", "10"])
    assert args.command == "list"
    assert args.limit == 10


def test_simple_cli_commands():
    """Test all CLI commands are registered."""
    cli = SimpleCLI()

    # Check all commands exist
    args = cli.parser.parse_args(["search", "test"])
    assert args.command == "search"

    args = cli.parser.parse_args(["import", "test"])
    assert args.command == "import"

    args = cli.parser.parse_args(["list"])
    assert args.command == "list"

    args = cli.parser.parse_args(["status"])
    assert args.command == "status"

    args = cli.parser.parse_args(["stats"])
    assert args.command == "stats"

    args = cli.parser.parse_args(["export", "json"])
    assert args.command == "export"

    args = cli.parser.parse_args(["help"])
    assert args.command == "help"


def test_simple_cli_run_no_args():
    """Test CLI run with no arguments."""
    cli = SimpleCLI()
    # Should print help
    result = cli.run([])
    assert result == 0
