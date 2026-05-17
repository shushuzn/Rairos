#!/bin/bash
# Publish JavaScript SDK to npm
# Usage: ./publish_js.sh <version>

set -e

VERSION="${1}"
SDK_DIR="$(cd "$(dirname "$0")/.." && pwd)"

if [ -z "$VERSION" ]; then
    echo "Error: Version required"
    echo "Usage: $0 <version>"
    exit 1
fi

cd "$SDK_DIR/js"

# Update version
npm version "$VERSION" --no-git-tag-version

# Build
npm install
npm run build

# Publish to npm
echo "Publishing to npm..."
npm publish --access public

echo "JavaScript SDK $VERSION published successfully!"
