#!/usr/bin/env bash

# Based on the release script for `git-cliff`, with additional adjustments

REMOTE_NAME="origin"
TAG="$1"

if [ -z "$TAG" ]; then
	>&2 echo 'Please provide the tag as an argument, in the format `vX.Y.Z`.'
	exit 1
fi

if [ -n "$(git status --porcelain)" ]; then
	>&2 echo 'You have uncommitted changes. Please clean up first.'
	exit 1
fi

# Exit early if any command is unsuccessful
set -o errexit

# Update the version
CARGO_VERSION_COMMENT="# Managed by "\`"release.sh"\`""
sed "s/^version = .* $CARGO_VERSION_COMMENT$/version = \"${TAG#v}\" $CARGO_VERSION_COMMENT/" -i Cargo.toml

# Run checks to ensure everything is good
cargo fmt --all --check
cargo clippy # This also updates `Cargo.lock`

# Generate the changelog
git cliff --tag "$TAG" > CHANGELOG.md

# Commit the version update and new changelog
git add --all && git commit -m "misc(release): Prepare for $TAG."
git show

# Create a signed tag for the new version
git tag -s -a "$TAG" -m "Release $TAG."

# Verify and show the new tag
git tag -v "$TAG"

# Done
echo
echo "New version created successfully."
echo
echo "Push the changes and new tag to the remote with (assuming the remote name is "\`"$REMOTE_NAME"\`"):"
echo "	"\`"git push $REMOTE_NAME && git push $REMOTE_NAME $TAG"\`""
echo
echo "Changelog to copy into the release notes:"
echo
git cliff --current --strip all
