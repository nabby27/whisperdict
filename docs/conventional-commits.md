# Conventional Commits and Commit Policy

Use Conventional Commits only for changes that belong in the changelog. Routine refactors, tests, or cleanup should use a plain imperative subject instead of a typed prefix.

## When to use Conventional Commits

- `feat`: new user-facing functionality.
- `fix`: bug fixes affecting users.
- `perf`: performance improvements noticeable to users.

## When **not** to use Conventional Commits

- Internal refactors that do not alter behavior.
- Tests-only changes.
- Lint/format/cleanup-only edits.
- Repo housekeeping that does not affect users.

For these, use a short imperative subject without a type, e.g. `Simplify helper` or `Improve test setup`.

## Format (when using Conventional Commits)

```
<type>(<scope>): <subject>
```

- `scope` is optional.
- Use present tense, imperative verbs in the subject line.
- Keep the subject short and descriptive.

### Scopes (Optional)

Use a scope when it helps clarify the area of the change. Examples: `transcription`, `ui`, `language`.

## Examples

Conventional (for changelog-worthy changes):

```
feat: add contribution file import
fix(import): handle missing python path on startup
perf(chart): reduce chart redraws
```

Non-conventional (maintenance-only changes):

```
Tighten tab close hover state
Adjust tab helper for tabs tests
```

## Breaking Changes

Breaking change messages are allowed but not required for this desktop app. If a change is disruptive, call it out in the issue and PR description as well.
