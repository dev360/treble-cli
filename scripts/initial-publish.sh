#!/bin/bash
# One-time script to create placeholder packages on npm.
# After this, CI handles all future publishes via semantic-release.
#
# Prerequisites:
#   npm login  (or set NPM_TOKEN in ~/.npmrc)
#
# Usage:
#   bash scripts/initial-publish.sh

set -e

echo "Publishing initial placeholder packages to @treble-app..."
echo ""

# Platform packages first (main package depends on them as optionalDependencies)
for platform in darwin-arm64 darwin-x64 linux-x64 linux-arm64; do
  echo "→ @treble-app/cli-${platform}"
  # Create a dummy binary so npm doesn't complain about missing files
  mkdir -p "npm/${platform}/bin"
  echo "placeholder" > "npm/${platform}/bin/treble"
  chmod +x "npm/${platform}/bin/treble"
  (cd "npm/${platform}" && npm publish --access public)
  # Clean up dummy binary
  rm "npm/${platform}/bin/treble"
  echo ""
done

# Main package last
echo "→ @treble-app/cli"
npm publish --access public

echo ""
echo "Done! All 5 packages published. CI will handle future releases."
echo "Clean up dummy binaries: git checkout npm/"
