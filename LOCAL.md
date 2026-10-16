# Local TODO customization

This fork keeps the upstream workmux features and adds a global TODO footer to
the left sidebar. Press `T` to fold the TODOs; upstream `t` still toggles grouping.
The footer reads `~/.workmux/todos.json`. See the sidebar guide for its format.

## Branches

- `main`: the upstream base used by the last successful synchronization.
- `workmux-local`: the default branch, containing the TODO and automation commits.
- `backup/todo-before-sync`: the original TODO implementation before migration.

Keep TODO changes in focused commits on top of `main`. Do not independently use
GitHub's **Sync fork** on `main`: automation updates both branches together.

## Automatic updates

The **Sync upstream and rebase TODO** workflow runs every Monday at 02:17 UTC
(10:17 in China). It can also be started manually from Actions. Scheduled runs
skip checks if upstream is unchanged; manual runs still validate the current
build.

The workflow rebases the local commits, then checks formatting, Clippy, compilation
and Rust unit tests before publishing. Conflicts, failed checks, rewritten
upstream history and concurrent pushes stop publication. Neither remote branch
is partially updated.

Pushes authenticate with the write-enabled `workmux-upstream-sync` deploy key,
which grants access only to this fork. Its private key is stored as the Actions
secret `UPSTREAM_SYNC_SSH_KEY`. Personal GitHub login tokens are not stored in CI.

## Updating the installed binary

After a successful synchronization, start with a clean working directory:

```bash
git fetch origin
git switch --detach origin/workmux-local
cargo install --path . --locked --force
```

For another local change, create a feature branch from `origin/workmux-local`,
commit it and push to `workmux-local`. Fetch again before pushing if automation
has rebased the branch in the meantime. Avoid `workmux update`, which installs
the upstream release without this fork's TODO feature.

## Recovering a failed synchronization

The existing remote build remains available. Inspect the failed run in Actions,
then resolve the TODO integration in a fresh branch based on the new upstream.
Keep the upstream grouping, help and navigation behavior when editing sidebar
files, and test the TODO footer with grouped agents and short terminal heights.
Publish the reviewed customization together with its corresponding upstream
base, then run the workflow manually again.
