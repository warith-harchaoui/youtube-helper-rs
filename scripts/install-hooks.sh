#!/usr/bin/env bash
#
# Install the repo's pre-push gate into .git/hooks (which git does not version).
# Run once per clone: scripts/install-hooks.sh
#
# The gate mirrors CI, plus the lock-free resolve that CI runs on Linux only —
# so a push that would go red, or a dependency range that would break a fresh
# `cargo add`, is caught here rather than after the fact on crates.io.
set -euo pipefail

repo_root=$(git rev-parse --show-toplevel)
hook="$repo_root/.git/hooks/pre-push"

cat > "$hook" <<'HOOK'
#!/usr/bin/env bash
# Installed by scripts/install-hooks.sh — re-run that script to update.
set -euo pipefail

echo "pre-push: cargo fmt --all --check"
cargo fmt --all --check

echo "pre-push: cargo clippy --all-targets -- -D warnings"
cargo clippy --all-targets -- -D warnings

echo "pre-push: cargo test --all"
cargo test --all

# The three checks above build against the committed Cargo.lock and therefore
# cannot see a dependency range that has gone bad for everyone else.
echo "pre-push: fresh resolve (no Cargo.lock)"
"$(git rev-parse --show-toplevel)/scripts/check-fresh-resolve.sh"

echo "pre-push: OK"
HOOK

chmod +x "$hook"
echo "installed $hook"
