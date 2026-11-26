#!/usr/bin/env bash
set -euo pipefail

# This publisher intentionally manages exactly one dedicated branch.
branch="${1:-updates}"
if [[ "$branch" != "updates" ]]; then
  echo "Refusing to manage unexpected updater-feed branch: $branch" >&2
  exit 2
fi

git check-ref-format "refs/heads/$branch"

# Distinguish a missing remote ref (ls-remote exit 2) from network/auth failures.
set +e
remote_refs="$(git ls-remote --exit-code --heads origin "refs/heads/$branch" 2>&1)"
status=$?
set -e
if [[ $status -eq 0 && -n "$remote_refs" ]]; then
  git fetch --no-tags origin "refs/heads/$branch:refs/remotes/origin/$branch"
  git switch --force-create "$branch" "refs/remotes/origin/$branch"
elif [[ $status -eq 2 && -z "$remote_refs" ]]; then
  # First publication: start an orphan history so application source and secrets
  # are not copied into the public feed branch.
  git switch --orphan "$branch"
  git rm -rf . >/dev/null 2>&1 || true
else
  echo "Unable to determine updater-feed branch state safely: $remote_refs" >&2
  exit "${status:-1}"
fi
