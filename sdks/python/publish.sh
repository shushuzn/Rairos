#!/bin/bash
# Publish Python SDK to PyPI
# Usage: ./publish_python.sh <version>

set -e

VERSION="${1}"
SDK_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if [ -z "$VERSION" ]; then
    echo "Error: Version required"
    echo "Usage: $0 <version>"
    exit 1
fi

cd "$SDK_DIR/python"

# Update version
sed -i "s/version = \".*\"/version = \"$VERSION\"/" pyproject.toml

# Build
python -m build

# Publish to PyPI
echo "Publishing to PyPI..."
python -m twine upload dist/*

echo "Python SDK $VERSION published successfully!"
