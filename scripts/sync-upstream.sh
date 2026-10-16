#!/usr/bin/env bash
# Run in a disposable GitHub Actions checkout, never in a development worktree.
set -euo pipefail

if [ "${GITHUB_ACTIONS:-}" != true ]; then
    echo 'Run this through the Sync upstream and rebase TODO workflow.' >&2
    exit 1
fi

export GIT_TERMINAL_PROMPT=0
git config user.name workmux-sync-bot
git config user.email workmux-sync-bot@users.noreply.github.com

upstream_url=${WORKMUX_UPSTREAM_URL:-https://github.com/raine/workmux.git}
if git remote get-url upstream >/dev/null 2>&1; then
    git remote set-url upstream "$upstream_url"
else
    git remote add upstream "$upstream_url"
fi
git fetch upstream +refs/heads/main:refs/remotes/upstream/main
git fetch origin \
    +refs/heads/main:refs/remotes/origin/main \
    +refs/heads/workmux-local:refs/remotes/origin/workmux-local

old_custom=$(git rev-parse refs/remotes/origin/workmux-local)
old_base=$(git rev-parse refs/remotes/origin/main)
new_base=$(git rev-parse refs/remotes/upstream/main)
git switch -C workmux-local "$old_custom"

# Fork main records the upstream base of the last successful synchronization.
if ! git merge-base --is-ancestor "$old_base" "$old_custom"; then
    echo '::error::Fork main is no longer the base of workmux-local. Review both branches manually.'
    exit 1
fi
if ! git merge-base --is-ancestor "$old_base" "$new_base"; then
    echo '::error::Upstream history was rewritten. Manual review is required.'
    exit 1
fi

if [ "$old_base" = "$new_base" ]; then
    if [ "${WORKMUX_VERIFY_UNCHANGED:-false}" != true ]; then
        echo 'Upstream is unchanged.'
        exit 0
    fi
    echo 'Upstream is unchanged; validating the current TODO build.'
elif ! git rebase --onto "$new_base" "$old_base"; then
    git status --short
    git rebase --abort
    echo '::error::Rebase conflicted. Neither remote branch has been updated.'
    exit 1
fi

cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo build --locked --all
cargo test --locked --bin workmux --quiet

if [ "$old_base" = "$new_base" ]; then
    echo 'Current TODO build passed all checks; no branch updates needed.'
    exit 0
fi

# Publish both refs together. Refuse to overwrite any concurrent branch update.
git push --atomic \
    "--force-with-lease=refs/heads/workmux-local:$old_custom" \
    "--force-with-lease=refs/heads/main:$old_base" \
    origin \
    HEAD:refs/heads/workmux-local \
    refs/remotes/upstream/main:refs/heads/main
echo 'Updated main and rebased workmux-local successfully.'
