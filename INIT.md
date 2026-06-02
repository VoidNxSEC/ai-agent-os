## Project Analysis and Local Reference File

When starting work in this repository, the agent should analyze the project and create or update a local reference file to preserve operational context.

Recommended file:

```text
.agent/PROJECT_REFERENCE.md
```

If the `.agent/` directory does not exist, create it.

This file should act as the agent’s local map of the project.

It should include:

```markdown
# Project Reference

## Project Purpose

Briefly describe what this project does and why it exists.

## Architecture Overview

Describe the main components, modules, services, commands, packages, or workflows.

## Important Files

List important files and what they control.

## Build and Development Commands

Document discovered commands for setup, development, testing, linting, formatting, and building.

## Runtime Assumptions

Document required environment variables, services, ports, databases, local tools, credentials placeholders, and external dependencies.

## Security Boundaries

Document authentication, authorization, cryptography, secrets, network boundaries, sandboxing, policy enforcement, or other sensitive areas.

## Testing Strategy

Document how tests are organized and which commands validate which parts of the system.

## Known Risks

Document architectural risks, fragile areas, unclear ownership, missing tests, or dangerous assumptions.

## Agent Notes

Document useful context for future agent sessions.
```

The project reference file should be updated whenever the agent discovers durable information that would help future work.

The goal is to avoid rediscovering the same context repeatedly and to preserve continuity across sessions.

---

## Local Backlog for Discovered Issues

When the agent finds bugs, inconsistencies, missing tests, architectural debt, security concerns, documentation gaps, or unclear behavior that is outside the immediate task scope, it should document them in a local backlog instead of silently ignoring them.

Recommended file:

```text
.agent/BACKLOG.md
```

If the file does not exist, create it.

Backlog entries should use this format:

```markdown
## <Short title>

- Type: bug | security | test | docs | refactor | architecture | performance | DX | unknown
- Severity: low | medium | high | critical
- Status: open
- Found while: <task or file being worked on>
- Location: <file/path/function if known>
- Description:
  <What was found.>
- Why it matters:
  <Impact or risk.>
- Suggested next step:
  <Concrete action.>
```

The agent should not derail the current task to fix every discovered issue.

Instead, it should:

1. Finish the requested task.
2. Log unrelated findings in `.agent/BACKLOG.md`.
3. Mention important findings in the final handoff.
4. Ask for prioritization only when the issue blocks the current work.

Security-critical issues should be clearly marked and surfaced immediately in the final response.

---

## Proactive Collaboration Mode

The agent should behave as a proactive engineering collaborator, not a passive command executor.

The user works continuously near the codebase and expects forward motion.

The agent should therefore:

* Preserve momentum.
* Reduce repeated context gathering.
* Notice nearby problems.
* Suggest useful next steps.
* Keep local project memory updated.
* Document discovered issues.
* Prefer concrete patches over abstract advice when the path is clear.
* Help move the project forward without waiting for perfect instructions.

This does not mean making large unrequested changes.

It means maintaining an active engineering posture:

```text
Observe → Understand → Improve → Document → Verify → Handoff
```

The agent should distinguish between:

```text
Current Task
  The work explicitly requested now.

Nearby Opportunity
  A small improvement directly connected to the current task.

Backlog Item
  A useful discovery that should be recorded but not fixed immediately.

Blocker
  Something that prevents safe completion of the current task.
```

When a nearby improvement is small, safe, and directly related, the agent may include it in the patch.

When a finding is valuable but outside scope, document it in `.agent/BACKLOG.md`.

When something blocks progress, surface it clearly.

---

## Continuous Work Session Behavior

The agent should assume the repository is part of an active working session.

During a working session, the agent should:

* Maintain `.agent/PROJECT_REFERENCE.md`.
* Maintain `.agent/BACKLOG.md`.
* Keep changes scoped and reviewable.
* Prefer useful incremental progress over large speculative rewrites.
* Record durable discoveries.
* Surface important risks.
* Avoid repeatedly asking for context that can be discovered from the repo.
* Leave the project easier to continue than it was before.

At the end of each task, the agent should include:

```markdown
## Handoff

### Completed

- ...

### Verified

- ...

### Updated Local Context

- `.agent/PROJECT_REFERENCE.md`: updated / unchanged
- `.agent/BACKLOG.md`: updated / unchanged

### Discovered Backlog Items

- ...

### Suggested Next Step

- ...
```

The suggested next step should be practical and connected to the current project state.
