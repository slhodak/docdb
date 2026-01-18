# Workflow Rules

## File Protection
- **NEVER delete the PROMPT.md file** - This file contains the core project goals and should be preserved at all times
- The PROMPT.md file is essential for understanding the project's purpose and should not be removed, even during refactoring or cleanup operations

## Three Phases, Two Prompts, One Loop

This workflow uses a funnel with 3 Phases, 2 Prompts, and 1 Loop.

### Phase 1: Define Requirements (LLM conversation)
1. Discuss project ideas → identify Jobs to Be Done (JTBD)
2. Break individual JTBD into topic(s) of concern
3. Use subagents to load info from URLs into context
4. LLM understands JTBD topic of concern: subagent writes specs/FILENAME.md for each topic

### Phase 2 / 3: Run Ralph Loop (two modes, swap PROMPT.md as needed)

Same loop mechanism, different prompts for different objectives:

| Mode | When to use | Prompt focus |
|------|-------------|--------------|
| **PLANNING** | No plan exists, or plan is stale/wrong | Generate/update IMPLEMENTATION_PLAN.md only |
| **BUILDING** | Plan exists | Implement from plan, commit, update plan as side effect |

#### Prompt differences per mode:

- **PLANNING prompt**: Does gap analysis (specs vs code) and outputs a prioritized TODO list—no implementation, no commits.
- **BUILDING prompt**: Assumes plan exists, picks tasks from it, implements, runs tests (backpressure), commits.

#### Why use the loop for both modes?

- **BUILDING requires it**: inherently iterative (many tasks × fresh context = isolation)
- **PLANNING uses it for consistency**: same execution model, though often completes in 1-2 iterations
- **Flexibility**: if plan needs refinement, loop allows multiple passes reading its own output
- **Simplicity**: one mechanism for everything; clean file I/O; easy stop/restart
- **Context loaded each iteration**: PROMPT.md + AGENTS.md

### PLANNING Mode Loop Lifecycle

1. Subagents study specs/* and existing /src
2. Compare specs against code (gap analysis)
3. Create/update IMPLEMENTATION_PLAN.md with prioritized tasks
4. **No implementation**

### BUILDING Mode Loop Lifecycle

1. **Orient** – subagents study specs/* (requirements)
2. **Read plan** – study IMPLEMENTATION_PLAN.md
3. **Select** – pick the most important task
4. **Investigate** – subagents study relevant /src ("don't assume not implemented")
5. **Implement** – N subagents for file operations
6. **Validate** – 1 subagent for build/tests (backpressure)
7. **Update IMPLEMENTATION_PLAN.md** – mark task done, note discoveries/bugs
8. **Update AGENTS.md** – if operational learnings
9. **Commit**
10. Loop ends → context cleared → next iteration starts fresh

## Concepts

| Term | Definition |
|------|------------|
| **Job to be Done (JTBD)** | High-level user need or outcome |
| **Topic of Concern** | A distinct aspect/component within a JTBD |
| **Spec** | Requirements doc for one topic of concern (specs/FILENAME.md) |
| **Task** | Unit of work derived from comparing specs to code |

### Relationships

- 1 JTBD → multiple topics of concern
- 1 topic of concern → 1 spec
- 1 spec → multiple tasks (specs are larger than tasks)

### Example

- **JTBD**: "Help designers create mood boards"
- **Topics**: image collection, color extraction, layout, sharing
- Each topic → one spec file
- Each spec → many tasks in implementation plan

### Topic Scope Test: "One Sentence Without 'And'"

Can you describe the topic of concern in one sentence without conjoining unrelated capabilities?

- ✓ "The color extraction system analyzes images to identify dominant colors"
- ✗ "The user system handles authentication, profiles, and billing" → 3 topics

**If you need "and" to describe what it does, it's probably multiple topics**
