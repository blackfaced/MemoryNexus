# Issue tracker: GitHub

Issues and PRDs for this repo live in GitHub Issues for `blackfaced/MemoryNexus`. Use the `gh` CLI when an engineering skill needs GitHub issue operations.

The canonical GitHub repository is `blackfaced/MemoryNexus`, inferred from `git remote -v` when commands run inside this clone.

## Conventions

- **Create**: `gh issue create --title "..." --body "..."`
- **Read**: `gh issue view <number> --comments`
- **List**: `gh issue list --state open --json number,title,body,labels,comments`
- **Comment**: `gh issue comment <number> --body "..."`
- **Labels**: `gh issue edit <number> --add-label "..."` / `--remove-label "..."`
- **Close**: `gh issue close <number> --comment "..."`

Infer the repository from `git remote -v` when running inside this clone.

## Pull requests as a triage surface

**PRs as a request surface: no.**

External pull requests are not part of the normal triage queue. Skills that triage incoming work should process GitHub Issues unless the user explicitly asks to inspect or review a PR.

GitHub shares one number space across issues and PRs, so a bare `#42` may be either. Resolve ambiguity with `gh pr view 42` and fall back to `gh issue view 42`.

## Skill conventions

When a skill says "publish to the issue tracker", create a GitHub Issue.

When it says "fetch the relevant ticket", run `gh issue view <number> --comments`.

## Wayfinding operations

- A map is one GitHub Issue labelled `wayfinder:map`.
- Child tickets are GitHub sub-issues where available; otherwise use a task list plus `Part of #<map>`.
- Prefer GitHub native issue dependencies; otherwise use a `Blocked by: #<n>` line.
- Claim an unblocked ticket with `gh issue edit <n> --add-assignee @me`.
- Resolve by commenting, closing the issue, and recording the outcome on the map.
