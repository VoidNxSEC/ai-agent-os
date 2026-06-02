# AGENTS.md

## Purpose

This file defines how AI agents should operate inside this repository.

The goal is to produce reliable, auditable, secure, and maintainable changes with minimal noise and maximum respect for the existing architecture.

Agents should act like careful engineering collaborators: understand the system first, make scoped changes, verify their work, and leave a clear handoff.

---

# 1. Core Operating Principles

## 1.1 Respect the Existing System

Before changing code, understand:

* What the current architecture is trying to preserve.
* Which conventions are already present.
* Which modules own which responsibilities.
* Which abstractions are stable and which are still experimental.
* Which files are generated, vendored, or externally managed.

Prefer extending existing patterns over introducing isolated new ones.

## 1.2 Optimize for Coherence

Every change should improve or preserve:

* Architectural coherence.
* Security posture.
* Operational reliability.
* Testability.
* Reproducibility.
* Observability.
* Developer experience.

A technically clever solution that breaks the shape of the system is usually worse than a smaller coherent change.

## 1.3 Work in Small, Auditable Steps

Prefer small, reviewable patches.

Each meaningful change should have:

* A clear reason.
* A clear scope.
* A clear verification path.
* A clear rollback path when possible.

## 1.4 Treat Instructions as a Hierarchy

When instructions conflict, follow this priority order:

1. Explicit user request for the current task.
2. Security, privacy, and safety requirements.
3. This `AGENTS.md`.
4. More specific nested `AGENTS.md` files.
5. Existing project conventions.
6. General best practices.

Nested `AGENTS.md` files apply to their directory subtree.

---

# 2. Agentic Workflow

Use this loop for every non-trivial task:

```text
INTAKE
  ↓
UNDERSTAND CONTEXT
  ↓
MAP CURRENT STATE
  ↓
PLAN
  ↓
IMPLEMENT
  ↓
VERIFY
  ↓
DOCUMENT / HANDOFF
```

## 2.1 Intake

Clarify internally:

* What is the user asking for?
* Is this a bugfix, feature, refactor, audit, test, migration, or documentation task?
* What is the expected output?
* What files or subsystems are likely involved?
* What would make the task complete?
* What could break if done carelessly?

For ambiguous tasks, make the smallest reasonable assumption and document it in the handoff.

## 2.2 Understand Context

Before editing, inspect relevant files.

Recommended context scan:

```bash
find . -maxdepth 3 -type f | sort
git status --short
git branch --show-current
```

Then inspect:

* README files.
* Existing docs.
* Build files.
* Test files.
* Entry points.
* Configuration files.
* CI definitions.
* Security-sensitive modules.
* Similar implementations already present.

## 2.3 Map Current State

Before proposing a change, answer:

* Where does this behavior currently live?
* What owns this responsibility?
* What public interfaces exist?
* What assumptions are encoded in tests?
* What configuration controls this?
* What dependencies are involved?
* What failure modes already exist?

## 2.4 Plan

For medium or large changes, create a brief plan:

```text
Plan:
1. Inspect the current implementation.
2. Update the minimal set of files.
3. Add or update tests.
4. Run targeted verification.
5. Summarize changes and risks.
```

Plans should be short, practical, and adaptable.

## 2.5 Implement

During implementation:

* Keep changes scoped to the task.
* Preserve public APIs unless the task explicitly requires changing them.
* Update tests alongside behavior.
* Update docs when behavior, configuration, or usage changes.
* Keep naming consistent with the repository.
* Prefer explicit code over magical behavior.
* Prefer simple composition over hidden coupling.
* Preserve formatting conventions.

## 2.6 Verify

Use the strongest available verification that fits the change.

Verification ladder:

```text
Static checks
  ↓
Unit tests
  ↓
Integration tests
  ↓
Build
  ↓
Runtime smoke test
  ↓
Manual behavior check
```

Run targeted checks first, then broader checks when appropriate.

Examples:

```bash
# Generic
git diff --check
make test
make lint
make build

# Node / TypeScript
npm test
npm run lint
npm run typecheck
npm run build

# Python
pytest
ruff check .
mypy .

# Go
go test ./...
go vet ./...

# Rust
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features

# Nix
nix flake check
nix build .#
nix develop --check
```

If a command fails, capture:

* The command.
* The failure.
* The likely cause.
* Whether the failure is related to the current change.
* What remains unresolved.

## 2.7 Handoff

Every completed task should end with a clear handoff:

```text
Summary:
- Changed X to do Y.
- Added/updated Z tests.
- Preserved existing behavior for A/B/C.

Verification:
- Ran: <command>
- Result: passed / failed / not available

Notes:
- Assumption made: ...
- Follow-up worth considering: ...
```

---

# 3. Autonomy Levels

Agents should choose the right autonomy level based on task size and risk.

## Level 1 — Read / Analyze

Use when asked to explain, audit, review, or map the system.

Output:

* Findings.
* File references.
* Risks.
* Suggested next steps.

No code changes unless explicitly requested.

## Level 2 — Targeted Patch

Use for small bugfixes, docs updates, test additions, or local refactors.

Behavior:

* Edit only relevant files.
* Run targeted verification.
* Provide concise handoff.

## Level 3 — Feature Work

Use for new behavior, new commands, new modules, or new integrations.

Behavior:

* Map existing architecture.
* Create a short plan.
* Implement incrementally.
* Add tests.
* Update docs.
* Run broader verification.

## Level 4 — Architectural Change

Use for migrations, public API changes, security model changes, storage changes, or infra changes.

Behavior:

* Preserve compatibility where possible.
* Identify migration impact.
* Document tradeoffs.
* Add regression tests.
* Provide rollback notes.
* Highlight unresolved risks.

---

# 4. Context Gathering Checklist

Before editing code, inspect the following when relevant:

## Repository Shape

* `README.md`
* `docs/`
* `examples/`
* `src/`, `app/`, `lib/`, `packages/`
* `tests/`, `spec/`, `__tests__/`
* `.github/workflows/`
* `Makefile`
* `justfile`
* `flake.nix`
* `package.json`
* `pyproject.toml`
* `Cargo.toml`
* `go.mod`
* `docker-compose.yml`
* `Dockerfile`

## Architecture

Look for:

* Entry points.
* Configuration loading.
* Dependency injection.
* Error handling conventions.
* Logging conventions.
* Security boundaries.
* Data persistence layer.
* API boundaries.
* CLI boundaries.
* Background workers.
* Network calls.
* Authentication / authorization paths.

## Existing Patterns

Search for similar code:

```bash
rg "function_name|class_name|command_name|config_key"
rg "TODO|FIXME|SECURITY|HACK|deprecated"
```

Prefer matching the repository’s current style over importing a new style.

---

# 5. Coding Standards

## 5.1 General

Code should be:

* Clear.
* Minimal.
* Typed where possible.
* Tested.
* Observable.
* Secure by default.
* Easy to delete or replace later.

Prefer:

* Small functions.
* Explicit names.
* Narrow interfaces.
* Local reasoning.
* Deterministic behavior.
* Boring correctness.

## 5.2 Error Handling

Errors should:

* Preserve root cause.
* Include useful context.
* Avoid leaking secrets.
* Be actionable for operators.
* Be testable.

Prefer structured errors where the language supports them.

## 5.3 Logging

Logs should help answer:

* What happened?
* Where did it happen?
* Which request/job/entity was involved?
* Was it expected or exceptional?
* What should an operator do next?

Logs should not expose:

* Secrets.
* Tokens.
* Private keys.
* Passwords.
* Raw personal data.
* Sensitive payloads.
* Internal credentials.
* Full authorization headers.

## 5.4 Configuration

Configuration should be:

* Explicit.
* Documented.
* Validated at startup.
* Safe by default.
* Environment-aware.

When adding config, update:

* Example config.
* Documentation.
* Validation.
* Tests.

## 5.5 Dependencies

Before adding a dependency, consider:

* Is it necessary?
* Is it maintained?
* Is it small enough for the use case?
* Does the project already have an equivalent?
* Does it affect build reproducibility?
* Does it increase attack surface?
* Does it introduce licensing concerns?

Prefer standard library or existing dependencies when practical.

---

# 6. Security Requirements

Security-sensitive changes require extra care.

## 6.1 Secrets

Secrets must never be committed.

Sensitive examples:

* API keys.
* OAuth secrets.
* JWT signing keys.
* Private keys.
* Database passwords.
* Session tokens.
* Cloud credentials.
* Production URLs with embedded credentials.

Use placeholders in docs:

```env
API_KEY=replace-me
DATABASE_URL=postgres://user:password@localhost:5432/app
```

## 6.2 Authentication and Authorization

When touching auth logic, verify:

* Authentication and authorization are separate.
* Access checks happen server-side.
* Default behavior is safe.
* Failure modes deny access.
* Tests cover allowed and denied paths.
* Privilege escalation paths are considered.

## 6.3 Cryptography

When touching cryptographic code:

* Use established libraries.
* Preserve key separation.
* Preserve domain separation.
* Preserve nonce/IV uniqueness.
* Avoid inventing custom primitives.
* Add tests for verification and failure cases.
* Document assumptions.

## 6.4 Input Validation

Validate inputs at trust boundaries:

* HTTP handlers.
* CLI arguments.
* Config files.
* Environment variables.
* Webhook payloads.
* Database writes.
* Message queue consumers.
* File parsers.

## 6.5 Supply Chain

When changing build or dependency files:

* Review lockfile changes.
* Avoid unnecessary dependency drift.
* Preserve reproducibility.
* Prefer pinned versions where the project already pins.
* Update checksums intentionally.

---

# 7. Testing Strategy

## 7.1 Test What Matters

Prioritize tests for:

* Security boundaries.
* Data integrity.
* Parsing.
* Serialization.
* Permission checks.
* Error paths.
* Regression bugs.
* Public APIs.
* Critical workflows.

## 7.2 Test Shape

Good tests should be:

* Deterministic.
* Isolated.
* Clear.
* Fast when possible.
* Focused on behavior, not implementation details.

## 7.3 Regression Tests

For bugfixes:

1. Add a failing test that reproduces the bug.
2. Fix the bug.
3. Confirm the test passes.
4. Confirm nearby behavior still works.

## 7.4 Snapshot Tests

Use snapshots carefully.

Snapshots are useful for:

* Stable generated output.
* CLI output.
* Serialization formats.
* UI rendering with stable structure.

Snapshots are weaker for:

* Complex business logic.
* Security behavior.
* Error handling.
* Permission checks.

---

# 8. Documentation Rules

Update documentation when changing:

* Commands.
* Configuration.
* Public APIs.
* Environment variables.
* Setup flow.
* Deployment flow.
* Security behavior.
* Architecture.
* Operational procedures.

Good docs include:

* What changed.
* Why it exists.
* How to use it.
* How to verify it.
* Common failure modes.
* Minimal examples.

---

# 9. Git Workflow

## 9.1 Before Editing

Check current state:

```bash
git status --short
```

Respect existing user changes.

If files are already modified, treat them as user-owned unless clearly created by the current task.

## 9.2 During Work

Use diffs frequently:

```bash
git diff
git diff --stat
```

Keep the patch focused.

## 9.3 Commit Guidance

When creating commits, prefer conventional style:

```text
feat: add policy validation for agent runtime
fix: handle missing config without panic
docs: document local development flow
test: add regression coverage for auth checks
refactor: simplify event normalization
chore: update generated lockfile
```

Commit messages should explain why, not only what.

## 9.4 Pull Request Handoff

A good PR summary:

```markdown
## Summary

- Added ...
- Changed ...
- Fixed ...

## Verification

- [x] Ran unit tests
- [x] Ran lint
- [x] Ran build

## Risk

- Low / Medium / High
- Notes: ...

## Rollback

- Revert this PR.
- No migration required.
```

---

# 10. Agent Communication Style

Agents should communicate with:

* Clarity.
* Brevity.
* Specificity.
* Honesty about uncertainty.
* Concrete next steps.

Prefer:

```text
I found the issue in `src/auth/policy.ts`.
The deny path was only covered for missing roles, not expired sessions.
I added a regression test and updated the guard.
```

Instead of vague status updates.

## 10.1 Progress Updates

For longer tasks, provide occasional updates:

```text
I found the main path. The change is smaller than expected: one parser function and two tests.
```

Useful updates include:

* What was discovered.
* What changed in the plan.
* What is currently blocked.
* What verification remains.

## 10.2 Final Response Format

Use this structure:

```text
Summary:
- ...

Verification:
- ...

Files changed:
- ...

Notes:
- ...
```

---

# 11. Failure Protocol

When something fails, preserve signal.

Report:

```text
Command:
<command>

Result:
<failure summary>

Likely cause:
<best current explanation>

Impact:
<does this block the task?>

Next useful step:
<concrete next action>
```

If verification cannot run because of missing tools, missing services, or environment limitations, say so clearly.

Example:

```text
Verification:
- `pytest` could not run because the environment is missing the `pytest` executable.
- The code change is limited to parser normalization and includes a new test file.
```

---

# 12. Refactoring Rules

Refactors should have a reason.

Good reasons:

* Remove duplication.
* Improve testability.
* Clarify ownership.
* Reduce coupling.
* Make failure modes explicit.
* Prepare for a requested feature.
* Improve performance with evidence.

Refactors should preserve behavior unless behavior change is explicitly part of the task.

When refactoring:

* Keep public interfaces stable.
* Add tests before risky movement.
* Separate mechanical changes from behavior changes.
* Avoid mixing formatting-only changes with logic changes.

---

# 13. Performance Work

Performance changes should be evidence-driven.

Before optimizing, identify:

* The hot path.
* The baseline.
* The target.
* The measurement method.
* The tradeoff.

Possible verification:

```bash
hyperfine '<command>'
cargo bench
go test -bench=.
pytest --benchmark-only
```

Document performance claims in the handoff.

---

# 14. Data and Migration Work

When changing data models, schemas, or storage:

* Preserve backward compatibility where possible.
* Add migration tests.
* Document migration steps.
* Include rollback notes.
* Consider partial migration failure.
* Consider old clients reading new data.
* Consider new clients reading old data.

Migration handoff should include:

```text
Migration:
- Required: yes/no
- Backward compatible: yes/no
- Rollback: ...
- Data risk: ...
```

---

# 15. API Changes

When changing APIs:

* Preserve existing contracts where practical.
* Version breaking changes.
* Update OpenAPI/schema files if present.
* Update client examples.
* Add tests for old and new behavior when compatibility matters.
* Document error responses.

API checklist:

```text
- Request validation
- Response shape
- Error shape
- Auth behavior
- Rate limiting impact
- Backward compatibility
- Tests
- Docs
```

---

# 16. CLI Changes

When changing CLI behavior:

* Preserve existing flags where possible.
* Add help text.
* Validate arguments.
* Return meaningful exit codes.
* Keep output script-friendly when appropriate.
* Add tests for success and failure paths.

CLI checklist:

```text
- `--help`
- Missing args
- Invalid args
- Success path
- Failure path
- Exit code
- Machine-readable output, if supported
```

---

# 17. UI / Frontend Changes

When changing UI:

* Preserve accessibility.
* Preserve loading states.
* Preserve error states.
* Preserve empty states.
* Avoid layout regressions.
* Keep component boundaries clear.
* Add tests where the project already uses them.

Frontend checklist:

```text
- Loading state
- Empty state
- Error state
- Success state
- Keyboard navigation
- Screen reader labels
- Responsive behavior
```

---

# 18. Infrastructure Changes

When changing infrastructure:

* Prefer reproducibility.
* Avoid hidden manual steps.
* Keep secrets outside the repo.
* Update deployment docs.
* Verify local and CI behavior where possible.
* Preserve rollback path.

Infra checklist:

```text
- Build works
- Deployment path documented
- Secrets documented but not stored
- Health check present
- Logs available
- Rollback understood
```

---

# 19. Nix / Reproducibility Rules

When working with Nix:

* Preserve flake structure.
* Keep hashes intentional.
* Prefer reproducible derivations.
* Avoid impure assumptions.
* Update lockfiles intentionally.
* Run `nix flake check` when practical.
* Keep development shells usable.

Useful commands:

```bash
nix flake check
nix develop
nix build .#
nix flake lock --update-input <input>
```

When hashes change, document why.

---

# 20. Agent Subtask Pattern

For complex work, decompose into subtasks:

```text
Subtask:
- Objective:
- Files:
- Risk:
- Verification:
- Status:
```

Example:

```text
Subtask:
- Objective: Add config validation for missing policy file.
- Files: src/config.rs, tests/config_test.rs
- Risk: Low
- Verification: cargo test config
- Status: complete
```

---

# 21. Decision Log Pattern

For architectural choices, capture a mini decision record:

```markdown
## Decision

We chose X.

## Context

The system needs Y, while preserving Z.

## Options Considered

1. A
2. B
3. C

## Rationale

X fits because ...

## Consequences

- Positive: ...
- Negative: ...
- Follow-up: ...
```

Use this in docs, PRs, or ADR files when the decision has long-term impact.

---

# 22. Definition of Done

A task is done when:

* The requested behavior is implemented.
* The change is scoped.
* Relevant tests are added or updated.
* Verification was run or clearly explained.
* Docs are updated when needed.
* Security-sensitive paths were reviewed.
* The final handoff is clear.
* Known limitations are stated.

---

# 23. Repository-Specific Commands

Update this section with actual project commands.

## Setup

```bash
<setup command>
```

## Development

```bash
<dev command>
```

## Test

```bash
<test command>
```

## Lint

```bash
<lint command>
```

## Typecheck

```bash
<typecheck command>
```

## Build

```bash
<build command>
```

## Format

```bash
<format command>
```

---

# 24. Quick Cheatsheet

## New Task

```text
1. Understand request.
2. Inspect repo state.
3. Find relevant files.
4. Read existing patterns.
5. Plan small patch.
6. Implement.
7. Test.
8. Summarize.
```

## Bugfix

```text
1. Reproduce or locate bug.
2. Add regression test when possible.
3. Fix root cause.
4. Run targeted test.
5. Check nearby behavior.
```

## Feature

```text
1. Identify owner module.
2. Match existing architecture.
3. Add minimal implementation.
4. Add tests.
5. Update docs/config examples.
6. Run build/test.
```

## Refactor

```text
1. Define reason.
2. Preserve behavior.
3. Keep diff reviewable.
4. Run tests before and after.
5. Separate from feature work when possible.
```

## Security Change

```text
1. Identify trust boundary.
2. Verify default-safe behavior.
3. Test allowed and denied paths.
4. Avoid leaking secrets.
5. Document assumptions.
```

## Handoff

```text
Summary:
- ...

Verification:
- ...

Files changed:
- ...

Risks / Notes:
- ...
```

---

# 25. Minimal Agent Runtime Contract

Every agent working in this repository should preserve this contract:

```text
Understand before changing.
Change less than necessary.
Verify more than comfortable.
Explain what changed.
Expose uncertainty.
Preserve coherence.
```
