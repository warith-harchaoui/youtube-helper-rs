#!/usr/bin/env bash
#
# Enable this repo's pre-push gate. Run once per clone: scripts/install-hooks.sh
#
# It points core.hooksPath at the versioned .githooks/ directory rather than
# copying into .git/hooks, because a *global* core.hooksPath — git-lfs installs
# one at ~/.config/git/hooks — silently wins over .git/hooks, so a hook dropped
# there never runs and the gate looks installed while doing nothing. The local
# setting overrides the global one, and .githooks/ carries the git-lfs
# delegations so redirecting hooksPath doesn't cost this repo its LFS support.
#
# To undo: git config --unset core.hooksPath
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
cd "$repo_root"

git config core.hooksPath .githooks
chmod +x .githooks/*

# A hook left here from an earlier install would be dead weight now, and reading
# it later would suggest a gate that no longer runs from this path.
if [ -f .git/hooks/pre-push ]; then
  rm -f .git/hooks/pre-push
  echo "removed the stale .git/hooks/pre-push (shadowed by core.hooksPath)"
fi

echo "core.hooksPath -> .githooks  (pre-push: fmt, clippy, test, fresh resolve)"
