#!/usr/bin/env bash
#
# Build this crate the way a *stranger* gets it: with no Cargo.lock.
#
# Why this exists. A committed Cargo.lock pins the exact dependency versions this
# repo builds against, and `cargo build`, `cargo test` and CI all honour it — so a
# dependency range that has since gone bad stays invisible here while breaking
# every fresh `cargo add` / `cargo install` downstream. `cargo publish --dry-run`
# does not catch it either: it packages and verifies with the same lock.
#
# That is not hypothetical. md2star-rs 0.4.0 shipped `ppt-rs = "0.2"` while its
# lock pinned the working 0.2.22; ppt-rs 0.2.23+ does not compile with
# `default-features = false`, so the published crate was uninstallable while local
# builds and a three-OS CI matrix stayed green.
#
# What it does: exports the committed tree to a scratch directory, deletes the
# lock, resolves from scratch, and builds every target. Exit non-zero on failure.
# Nothing in the real working tree is touched — the repo's own Cargo.lock is
# neither read nor rewritten.
#
# Usage: scripts/check-fresh-resolve.sh
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
crate=$(basename "$repo_root")
work=$(mktemp -d "${TMPDIR:-/tmp}/fresh-resolve-${crate}-XXXXXX")
trap 'rm -rf "$work"' EXIT

echo "==> $crate: exporting the committed tree (HEAD) to a scratch build"
# git archive exports tracked files at HEAD only, so uncommitted noise and the
# local target/ directory can't influence the result. Written to a file rather
# than piped into tar: BSD tar (macOS) stops reading at the archive's end-of-file
# marker and closes the pipe while git is still writing its padding, so git dies
# of SIGPIPE and `set -o pipefail` turns a successful extraction into exit 141.
git -C "$repo_root" archive --format=tar -o "$work/head.tar" HEAD
tar -x -f "$work/head.tar" -C "$work"
rm -f "$work/head.tar"

rm -f "$work/Cargo.lock"

echo "==> resolving dependencies with no lock (newest versions each range allows)"
cargo build --manifest-path "$work/Cargo.toml" --all-targets

echo
echo "==> versions this resolve picked, vs the committed Cargo.lock"
if [ -f "$repo_root/Cargo.lock" ]; then
  # Compare the two locks so a drift that *still compiles* is at least visible in
  # the log — today's silent upgrade is tomorrow's silent breakage.
  # Informational only: a drift that still compiles is not a failure, but today's
  # silent upgrade is tomorrow's silent breakage, so it belongs in the log.
  # `if diff` keeps a difference from tripping `set -e`.
  if diff <(grep -E '^(name|version) = ' "$repo_root/Cargo.lock") \
          <(grep -E '^(name|version) = ' "$work/Cargo.lock") >/dev/null; then
    echo "    identical to the committed lock"
  else
    diff <(grep -E '^(name|version) = ' "$repo_root/Cargo.lock") \
         <(grep -E '^(name|version) = ' "$work/Cargo.lock") || true
  fi
else
  echo "    no committed Cargo.lock to compare against"
fi

echo
echo "==> OK: $crate builds from a clean resolve"
