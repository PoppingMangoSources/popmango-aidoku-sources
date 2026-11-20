---
title: Instruction and skill layout
type: decision
created: 2026-11-20
updated: 2026-11-20
sources:
  - code: AGENTS.md
  - code: .claude/skills/aidoku-source/SKILL.md
  - code: .claude/skills/aidoku-submit/SKILL.md
  - code: .claude/skills/aidoku-archive/SKILL.md
  - code: scripts/sync-agent-docs.py
  - code: scripts/agent-hooks.py
verified_at: 2534d58
---

# ADR: a short hub, three workflow skills and one shared reference set

- **Status:** Proposed (in effect on main; the owner has not confirmed it)
- **Date:** 2026-11-20
- **Decision makers:** the owner; drafted from the instruction and skill reviews
- **Confidence:** Medium

## Decision

**`AGENTS.md` / `CLAUDE.md` is a hub of hard rules and pointers; the work lives in three skills
split at its hand-offs: `aidoku-source` (recon to verify), `aidoku-submit` (bump, commit, PR,
review rounds) and `aidoku-archive` (moving a merged source off main).** The detail stays in one
shared `aidoku-source/references/` set, so each rule has one home.

## Context

The root file was 1781 lines, loaded whole each session, and the one skill held all nine phases,
so an agent writing a filter also read release, PR and archive steps. The owner's Paperback repos
split their instructions differently: mangago keeps a short root plus reference files; the Mana
repo has one skill per task (`mana-extension`, `mana-images`, `mana-port`, `verify`).

## Candidates

### A. One skill with nine phases (the layout before)
- One trigger for everything; 354 lines with the release and archive steps inline.

### B. One skill per task, as the Mana repo does
- Precise triggers, but recon, scaffold, implement and verify each depend on facts the step
  before established, so splitting them would break one sequence into hand-offs that lose state.

### C. Hub plus three skills split at the hand-offs (chosen)
- Splits only where the work changes hands: finished code to publication, and publication to
  archive. Each later step has its own trigger words (bump, pr-branch, review comment, merged).

## Evaluation

Line counts from `wc -l` on 2026-11-20: root file 1781 to about 450; `aidoku-source` 354 to 230,
with `aidoku-submit` at 96 and `aidoku-archive` at 24. The skill review (meta-dev reviewer,
skill standard) scored the single-skill layout 2/5 on structure and named this split. Trigger
accuracy in practice was not measured.

## Consequences

- An agent building a source no longer reads the PR and archive steps.
- The references stay under `aidoku-source/`; the other two skills link into it.
- `scripts/sync-agent-docs.py` copies every skill tree, so the Codex copies follow.
- `aidoku-archive` stays model-invoked: the owner usually asks for an archive in plain words
  ("move X to merged-sources"), which a user-only skill would not load.

## Re-evaluation triggers

- An agent misses a phase because it loaded the wrong skill.
- A fourth recurring kind of work (icons, ports from another client) appears often enough to
  deserve its own trigger, as `mana-images` and `mana-port` do in the Mana repo.
