"""Allow: python -m cli / airos-cli"""
from __future__ import annotations
import sys
import runpy

# When airos-cli.exe (zipapp) runs, it adds only itself to sys.path.
# If Hermes is installed, its _EditableFinder in sys.meta_path intercepts
# 'cli' imports before our package gets found.
# Fix: temporarily remove Hermes editable finders, then import normally.
for hook in list(sys.meta_path):
    hook_name = type(hook).__name__
    if 'hermes' in hook_name.lower() or 'EditableFinder' in hook_name:
        sys.meta_path.remove(hook)

# Also remove Hermes sys.path entries that could shadow us
sys.path = [p for p in sys.path if 'hermes_agent' not in p.lower()]

# Now run our CLI — hermes interceptors are gone
from cli._registry import main
raise SystemExit(main())
