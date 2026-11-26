#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "$0")/.." && pwd)"
tmp="$(mktemp -d)"
trap 'rm -rf "$tmp"' EXIT

git init --bare "$tmp/remote.git" >/dev/null
git clone "$tmp/remote.git" "$tmp/work" >/dev/null 2>&1
cd "$tmp/work"
git config user.name "UJANI updater branch test"
git config user.email "ujani-updater-test@example.invalid"
printf 'source-only\n' > source.txt
git add source.txt
git commit -m "seed main" >/dev/null
git branch -M main
git push -u origin main >/dev/null 2>&1

# First publication must create an orphan branch and must not copy source files.
bash "$repo_root/scripts/checkout-updater-feed-branch.sh"
test "$(git branch --show-current)" = "updates"
test ! -e source.txt
test -z "$(git ls-tree -r --name-only HEAD 2>/dev/null || true)"
printf '{"version":"0.1.0","platforms":{}}\n' > latest.json
git add latest.json
git commit -m "seed updater feed" >/dev/null
git push origin HEAD:updates >/dev/null 2>&1

# Subsequent publication must fetch and preserve the existing feed branch.
git switch main >/dev/null 2>&1
bash "$repo_root/scripts/checkout-updater-feed-branch.sh"
test "$(git branch --show-current)" = "updates"
test -f latest.json
grep -q '"version":"0.1.0"' latest.json

# The helper must not accept a caller-selected arbitrary branch name.
if bash "$repo_root/scripts/checkout-updater-feed-branch.sh" attacker >/dev/null 2>&1; then
  echo "Unexpectedly accepted a non-updates branch." >&2
  exit 1
fi

echo "Updater feed branch bootstrap tests passed."
