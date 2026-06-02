## Living Project Documents

This repository should maintain a small set of living documents used by both humans and agents.

These documents are not static documentation. They are operational memory.

Recommended files:

```text
ROADMAP.md
ARCHITECTURE.md
.agent/PROJECT_REFERENCE.md
.agent/BACKLOG.md
```

Their purpose:

```text
ROADMAP.md
  Tracks direction, milestones, priorities, phases, and next work.

ARCHITECTURE.md
  Explains the current system shape, boundaries, components, and decisions.

.agent/PROJECT_REFERENCE.md
  Local working map for agents: commands, files, risks, conventions, runtime notes.

.agent/BACKLOG.md
  Local backlog for discovered issues, risks, missing tests, refactors, and follow-ups.
```

Agents should update these files whenever they discover durable project information.

---

## Repository State Detection

Before starting a task, the agent should classify the repository state.

### Existing Worked Repository

A repository is considered already worked when it contains signs of active development, such as:

* Existing source code.
* Existing commits and branches.
* Existing tests.
* Existing build files.
* Existing documentation.
* Existing architecture decisions.
* Existing CI/CD.
* Existing Nix flake or development shell.
* Existing issues, TODOs, backlog files, or project notes.

For an existing worked repository, the agent should first perform analysis and inventory.

Recommended first-pass inventory:

```bash
git status --short
git branch --show-current
find . -maxdepth 3 -type f | sort
find . -maxdepth 2 -type d | sort
```

The agent should identify:

* Main language/runtime.
* Project type.
* Entry points.
* Build system.
* Test system.
* Existing commands.
* Architecture boundaries.
* Security-sensitive areas.
* Documentation gaps.
* Missing or stale project files.

Then update or create:

```text
ARCHITECTURE.md
.agent/PROJECT_REFERENCE.md
.agent/BACKLOG.md
```

If `ROADMAP.md` already exists, update it.
If it does not exist, create a first version based on the current project state.

### New Repository

A repository is considered new when it has little or no existing implementation.

Signs of a new repository:

* Empty or near-empty source tree.
* No clear architecture yet.
* No tests yet.
* No build command yet.
* No stable documentation yet.
* No ROADMAP.
* No ARCHITECTURE file.

For a new repository, the agent should create the initial project structure and planning documents before large implementation work.

Recommended initial files:

```text
README.md
ROADMAP.md
ARCHITECTURE.md
.agent/PROJECT_REFERENCE.md
.agent/BACKLOG.md
```

The first roadmap should define:

```markdown
# Roadmap

## Vision

What this project is trying to become.

## Current Phase

The current stage of the project.

## Milestones

### M0 — Foundation

- Project structure
- Reproducible dev environment
- Basic commands
- Initial tests
- Initial documentation

### M1 — Core Functionality

- Main domain model
- Core APIs or CLI
- First useful workflow
- Basic validation

### M2 — Reliability

- Tests
- Error handling
- Observability
- Security checks
- CI

### M3 — Productization

- Packaging
- Release flow
- Examples
- User documentation
- Operational docs

## Active Tasks

- [ ] ...

## Deferred

- [ ] ...

## Open Questions

- ...
```

---

## ROADMAP.md Maintenance

Before each task, the agent should inspect `ROADMAP.md` when present.

The roadmap should be used as a guide for:

* Current priority.
* Project phase.
* Known next steps.
* Deferred work.
* Open architectural questions.
* Milestones already completed.

After completing a task, the agent should update `ROADMAP.md` when the task changes project direction, completes a milestone, creates a new next step, or invalidates an old plan.

The roadmap should not become a dumping ground.

Use `.agent/BACKLOG.md` for discovered issues.
Use `ROADMAP.md` for direction and sequencing.

Recommended structure:

```markdown
# Roadmap

## Vision

## Current Phase

## Milestones

## Active Work

## Next Steps

## Deferred

## Completed

## Open Questions
```

Backlog and roadmap have different responsibilities:

```text
ROADMAP.md
  Where the project is going.

.agent/BACKLOG.md
  What was discovered and may need work.

ARCHITECTURE.md
  How the system is shaped.

.agent/PROJECT_REFERENCE.md
  How to work inside the system.
```

---

## ARCHITECTURE.md Maintenance

Before major implementation work, the agent should inspect `ARCHITECTURE.md`.

If it does not exist, create it.

The architecture file should describe the current system, not an idealized future system.

Recommended structure:

```markdown
# Architecture

## System Purpose

## High-Level Overview

## Components

## Data Flow

## Trust Boundaries

## Runtime Model

## Configuration

## Storage

## External Integrations

## Security Model

## Testing Model

## Operational Notes

## Known Architectural Risks
```

Update `ARCHITECTURE.md` when:

* New modules are added.
* Boundaries change.
* Runtime behavior changes.
* Storage changes.
* Security model changes.
* Configuration changes.
* A major architectural decision is made.

For smaller findings, update `.agent/PROJECT_REFERENCE.md` instead.

---

## Nix and flake.nix Maintenance

When the repository uses Nix, the agent should treat `flake.nix` as part of the developer interface.

If new commands, tools, checks, or workflows are discovered or added, update the development shell accordingly.

The agent should inspect:

```text
flake.nix
flake.lock
.nix files
devShells
packages
checks
apps
formatter
```

When appropriate, expose useful commands through:

* `devShells`
* `packages`
* `apps`
* `checks`
* `formatter`
* `shellHook`

The `shellHook` should help developers understand the available workflow without being noisy.

Example shellHook:

```nix
shellHook = ''
  echo "Dev shell ready."
  echo ""
  echo "Available commands:"
  echo "  just test       Run tests"
  echo "  just lint       Run lint"
  echo "  just fmt        Format code"
  echo "  just build      Build project"
  echo "  nix flake check Run full Nix verification"
  echo ""
'';
```

Prefer putting real commands in `justfile`, `Makefile`, package scripts, or Nix apps, and using `shellHook` as a discovery surface.

Good pattern:

```text
flake.nix
  Provides reproducible tools and checks.

justfile / Makefile
  Provides ergonomic project commands.

shellHook
  Shows the most important commands.

README.md
  Explains user-facing setup.

.agent/PROJECT_REFERENCE.md
  Records discovered local workflow details.
```

---

## Preferred Nix Practices

When working in Nix-based repositories, prefer:

* Reproducible development shells.
* Explicit build inputs.
* Minimal impurity.
* Pinned inputs through `flake.lock`.
* `nix fmt` or a declared formatter.
* `nix flake check` as the main verification gate.
* Project commands exposed through `apps` or `checks` when useful.
* Clear separation between packages, devShells, checks, and overlays.
* Small reusable Nix modules instead of one large unreadable flake.
* Documented shell commands.
* Avoiding hidden system dependencies.
* Avoiding global tools that are not declared in the shell.
* Keeping generated or machine-specific files out of the repo.

When adding a tool required for development, add it to the Nix dev shell.

When adding a verification command, consider adding it to `checks`.

When adding a common project command, consider exposing it through `apps` or documenting it in the shellHook.

When updating inputs, explain why the lockfile changed.

---

## Per-Task Document Update Protocol

Before starting each non-trivial task:

```text
1. Inspect ROADMAP.md if present.
2. Inspect ARCHITECTURE.md if present.
3. Inspect .agent/PROJECT_REFERENCE.md if present.
4. Inspect .agent/BACKLOG.md if relevant.
5. Classify the repository as existing or new.
6. Decide whether the task affects roadmap, architecture, backlog, or local reference.
```

After completing each task:

```text
1. Update ROADMAP.md if direction, phase, milestone, or next steps changed.
2. Update ARCHITECTURE.md if system structure changed.
3. Update .agent/PROJECT_REFERENCE.md if durable workflow knowledge was discovered.
4. Update .agent/BACKLOG.md if issues were discovered outside scope.
5. Update flake.nix / shellHook / justfile / Makefile if project commands changed.
6. Report what was updated in the final handoff.
```

Final handoff should include:

```markdown
## Handoff

### Completed

- ...

### Verified

- ...

### Living Documents Updated

- `ROADMAP.md`: updated / unchanged
- `ARCHITECTURE.md`: updated / unchanged
- `.agent/PROJECT_REFERENCE.md`: updated / unchanged
- `.agent/BACKLOG.md`: updated / unchanged

### Nix / DevShell Updated

- `flake.nix`: updated / unchanged
- `shellHook`: updated / unchanged
- Commands added: ...

### Discovered Follow-ups

- ...

### Suggested Next Step

- ...
```

---

## Agentic Continuity Rule

The agent should leave the repository easier to continue than it found it.

Every task should improve at least one of:

* Code.
* Tests.
* Documentation.
* Roadmap clarity.
* Architecture clarity.
* Developer commands.
* Local project memory.
* Backlog quality.
* Reproducibility.

The agent should preserve momentum with the user by maintaining these living files as part of the normal workflow, not as a separate documentation chore.
