#!/usr/bin/env bash
# Prints one version's section of CHANGELOG.md, without its heading:
#
#   .github/changelog-section.sh 0.3.0
#
# Fails when the version has no section, so a tag can't ship without notes.
set -euo pipefail
version="${1#v}"
section=$(awk -v v="$version" '
  /^## \[/ { p = index($0, "## [" v "]") == 1; next }
  /^\[[^]]*\]: / { exit }
  p
' "$(dirname "$0")/../CHANGELOG.md")
if [[ -z "${section//[[:space:]]/}" ]]; then
  echo "CHANGELOG.md has no section for $version" >&2
  exit 1
fi
printf '%s\n' "$section"
