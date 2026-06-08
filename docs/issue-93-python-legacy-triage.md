# Issue #93 Python Legacy Triage

> Triage date: 2026-06-08
> Issue: [#93 清理 Python 历史遗留代码和配置](https://github.com/blackfaced/MemoryNexus/issues/93)

## Scope

Issue #93 was opened to check whether the old Python/FastAPI backend and Redis
configuration still remain after the project moved to the Rust-first backend
line.

The issue body mentioned these historical paths:

- `src/core/config.py`
- `src/core/database.py`
- `src/core/`
- `REDIS_URL`

## Findings

No Python backend code remains in the current repository.

The following Python project artifacts were not found:

- `*.py`
- `requirements*.txt`
- `pyproject.toml`
- `Pipfile*`
- `poetry.lock`
- `uv.lock`
- `setup.py`
- `tox.ini`
- `pytest.ini`

The paths named in the issue body do not exist in the current tree, and
`REDIS_URL` is not present.

## Remaining References

The remaining Python/FastAPI references are documentation-only and fall into two
groups.

Keep:

- `AGENTS.md`: states that the historical Python/FastAPI skeleton has been
  removed and must not be reintroduced.
- `README.md`: states that Rust + Axum is the current backend path.
- `docs/faq.md`: answers that the old Python API is no longer supported.
- `docs/agent-self-install.md`: tells agents not to reintroduce the old backend.
- `decisions/ADR-009-rust-first-backend.md`: records the Rust-first decision.
- `decisions/ADR-001-backend-language.md`: preserves historical language-choice
  context.

Cleaned:

- `CONTRIBUTING.md`: previously described Python/pytest and React/TypeScript
  contribution rules. It now describes the Rust-first backend, the Rust-served
  static UI boundary, and the Cargo verification commands.

## Recommendation

Do not split #93. The code cleanup target described in the issue is already
resolved in the current repository state.

Recommended issue disposition:

- Rename to `[docs] Remove stale Python contribution guidance`, or leave the
  title as-is and close after the documentation cleanup merges.
- Close #93 after merging this branch.
- No follow-up deletion issue is needed unless new Python backend files appear.

## Suggested Issue Comment

```md
Triage completed.

- No Python/FastAPI backend code remains in the current repository.
- The issue body references `src/core/config.py`, `src/core/database.py`,
  `src/core/`, and `REDIS_URL`, but none of those paths or symbols exist in the
  current tree.
- No Python project artifacts were found: no `*.py`, `requirements*.txt`,
  `pyproject.toml`, `Pipfile*`, `poetry.lock`, `uv.lock`, `setup.py`, `tox.ini`,
  or `pytest.ini`.
- Remaining Python/FastAPI references are documentation-only and either preserve
  historical ADR context or explicitly say not to reintroduce the old backend.
- The only actionable stale item found was `CONTRIBUTING.md`, which still
  described Python/pytest and React/TypeScript contribution rules. This branch
  updates it to Rust-first backend and Rust-served static UI guidance.

Recommendation: do not split this issue. Treat the code cleanup concern as
already resolved by the current repository state, optionally rename the issue to
`[docs] Remove stale Python contribution guidance`, and close it after this
documentation cleanup merges.

Verification: documentation-only change; ran `git diff --check`.
```
