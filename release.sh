#!/bin/bash
# release.sh - Simple release script for projects with git dependencies

set -e

# Check if version argument is provided
if [ -z "$1" ]; then
    echo "Usage: $0 <version>"
    echo "Example: $0 0.1.1"
    exit 1
fi

VERSION=$1
TAG="v$VERSION"

# Check if working directory is clean
if [ -n "$(git status --porcelain)" ]; then
    echo "Error: Working directory is not clean. Please commit or stash changes."
    exit 1
fi

# Check if tag already exists
if git tag -l | grep -q "^$TAG$"; then
    echo "Error: Tag $TAG already exists"
    exit 1
fi

echo "Releasing version $VERSION..."

# Update version in Cargo.toml
sed -i "s/^version = \".*\"/version = \"$VERSION\"/" Cargo.toml

# Update Cargo.lock
cargo check

# Commit version bump
git add Cargo.toml
git commit -m "Bump version to $VERSION"

# Create and push tag
git tag "$TAG"
git push origin main
git push origin "$TAG"

echo "Successfully released $VERSION!"
echo "GitHub Actions should now build and create the release automatically."