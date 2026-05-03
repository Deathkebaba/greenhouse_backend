---
description: "Use when: planning, scheduling, or managing tickets"
applyTo: "management/**"
---
# Ticket System Contract

## Locations (set these once)
- PLAN_INDEX_PATH: management/plans/index.yaml
- PLAN_DIR: management/plans/
- TICKET_INDEX_PATH: management/tickets/index.yaml
- TICKET_DIR: management/tickets/
- ARCHIVE_DIR: management/archive/
- ARCHIVED_PLAN_DIR: management/archive/plans/
- ARCHIVED_TICKET_DIR: management/archive/tickets/
- ARCHIVED_PLAN_INDEX_PATH: management/archive/plans/index.yaml
- ARCHIVED_TICKET_INDEX_PATH: management/archive/tickets/index.yaml
- DIRECTIVES_PATH: management/directives.md
- ARCHITECTURE_PATH: management/architecture.md

## Timestamp format
- All new `at` and `closed_at` values must use UTC RFC3339 timestamps with second resolution: `YYYY-MM-DDTHH:MM:SSZ`.
- Do not write new date-only timestamps.
- When migrating legacy records that only captured a date, preserve that legacy precision instead of inventing a sub-day time.

## Directive responsibilities
- DIRECTIVES_PATH is the repository-wide source of product intent, scope guardrails, and quality expectations.
- Planner, manager, implementer, and reviewer must load DIRECTIVES_PATH before making decisions.
- If directives conflict with a request, agents must ask for explicit user approval and record the exception in ticket history.

## Architecture responsibilities
- ARCHITECTURE_PATH is the canonical reference for tech stack, module layout, data flows, and architectural decisions.
- Planner must load ARCHITECTURE_PATH before decomposing goals into plans and tickets.
- Planner may update ARCHITECTURE_PATH when an approved plan introduces or modifies architectural decisions; the update must be recorded in the plan's history entry.
- Reviewer must load ARCHITECTURE_PATH before evaluating whether an implementation aligns with established patterns and decisions.

## Plan index responsibilities
- The plan index is the source of truth for plan lifecycle and plan-level dependencies.
- The planner appends new plan entries.
- The manager updates plan status and closure metadata.
- Implementers must not edit the plan index.

## Ticket index responsibilities
- The ticket index is the source of truth for ticket scheduling status, dependencies, and claims.
- The manager agent updates status, deps, and claimed_by on existing tickets.
- The planner agent may append new tickets to the index.
- Implementers must not edit the index.

## Archive responsibilities
- ARCHIVED_PLAN_INDEX_PATH is the source of truth for archived plan index entries.
- ARCHIVED_TICKET_INDEX_PATH is the source of truth for archived ticket index entries.
- Cleaner agent moves archived detail files into ARCHIVED_PLAN_DIR and ARCHIVED_TICKET_DIR.
- Cleaner agent removes archived plan entries from PLAN_INDEX_PATH and appends them to ARCHIVED_PLAN_INDEX_PATH.
- Cleaner agent removes archived ticket entries from TICKET_INDEX_PATH and appends them to ARCHIVED_TICKET_INDEX_PATH.
- Archived detail files are immutable except for explicit migration or audit tasks.
- Active plan and ticket indexes must not retain archived entries.

## Plan index schema (YAML)
version: 1
plans:
  - id: P-1
    status: todo        # todo | in_progress | blocked | done
    depends_on_plans: []
    tickets: [T-1-1]
    detail_ref: P-1     # must equal the plan id
    claimed_by: null    # optional lock for planning workflow
    closed_at: null     # YYYY-MM-DDTHH:MM:SSZ when status becomes done

## Ticket index schema (YAML)
version: 1
tickets:
  - id: T-1-1
    plan_id: P-1
    status: todo        # todo | in_progress | review | blocked | done
    deps: []            # list of ids
    detail_ref: T-1-1   # must equal the ticket id
    claimed_by: null    # lock to prevent duplicate work

## Archived plan index schema (YAML)
version: 1
plans:
  - id: P-1
    status: done
    depends_on_plans: []
    tickets: [T-1-1]
    detail_ref: P-1
    claimed_by: null
    closed_at: "YYYY-MM-DDTHH:MM:SSZ"
    archived_at: "YYYY-MM-DDTHH:MM:SSZ"

## Archived ticket index schema (YAML)
version: 1
tickets:
  - id: T-1-1
    plan_id: P-1
    status: done
    deps: []
    detail_ref: T-1-1
    claimed_by: null
    archived_at: "YYYY-MM-DDTHH:MM:SSZ"

## Ticket detail responsibilities
- Ticket detail files are the source of truth for title, scope, acceptance, and history.
- The planner creates new ticket detail files when instantiating tickets.
- Implementers edit the ticket detail file, not the index.
- Reviewers append structured review submissions to the ticket detail history, not the index.
- Manager appends a history entry on each state change and routes work based on the latest review submission in history.
- Review submissions are append-only history entries with `action: review_submitted`.
- The latest history entry with `action: review_submitted` is the current review.
- Legacy top-level `review` or `reviews` blocks must be migrated into `history` before appending a new review submission.

## Plan detail template (Markdown + YAML frontmatter)
---
id: P-1
title: "Short, specific title"
goal: "One-sentence desired outcome"
summary: "Formalized plan intent and execution outline"
depends_on_plans: []
tickets:
  - T-1-1
history:
  - at: "YYYY-MM-DDTHH:MM:SSZ"
    actor: "planner-agent"
    action: "created"
---

## Ticket detail template (Markdown + YAML frontmatter)
---
id: T-1-1
plan_id: P-1
title: "Short, specific title"
goal: "One-sentence desired outcome"
scope:
  - "Key areas to change"
acceptance:
  - "Testable conditions"
constraints:
  - "Must/avoid rules"
history:
  - at: "YYYY-MM-DDTHH:MM:SSZ"
    actor: "manager-agent"
    action: "created"
---

## Review history entry template (YAML inside `history`)
- at: "YYYY-MM-DDTHH:MM:SSZ"
  actor: "reviewer-agent"
  action: "review_submitted"
  summary: "Short one-line judgment"
  verdict: approved    # approved | changes_requested
  next_actor: manager  # manager | implementer | blocked
  directive_compliance:
    status: compliant  # compliant | non_compliant | approved_exception
    notes: []
    violations: []
  findings:
    critical: []
    major: []
    minor: []
  missing_tests: []
  residual_risks: []

## Scheduling rules
- A ticket is ready if status is todo, deps are all done, and claimed_by is null.
- Claim by setting claimed_by and status to in_progress in a single update.
- If claimed_by is not null, skip the ticket.
- When implementation is complete and ready for review, set status to review.
- After review passes, set status to done and clear claimed_by.
- If blocked, set status to blocked and add a history entry with the reason.
- After every review, reviewer must append a `history` entry with `action: review_submitted`.
- The latest `history` entry with `action: review_submitted` is the authoritative current review.
- If reviewer reports no issues, the latest review submission must use `verdict: approved`, `summary: No findings`, and `next_actor: manager` with any residual risks.
- If reviewer requests another implementation round, the latest review submission must use `verdict: changes_requested` and `next_actor: implementer`.
- If reviewer identifies a hard blocker that should not return to implementation, the latest review submission must use `verdict: changes_requested` and `next_actor: blocked`.
- Manager must read the latest `history` entry with `action: review_submitted` after reviewer completion and route the ticket accordingly.
- Planner must not create a plan or tickets until DIRECTIVES_PATH is loaded and reflected in plan intent.
- Manager must not move tickets to done if the latest review submission shows `directive_compliance.status: non_compliant` without approved exception.
- Reviewer must evaluate directives explicitly and set directive compliance status in each review submission.
- If directives are violated without approved exception, reviewer verdict must be `changes_requested`.
- The planner must load PLAN_INDEX_PATH before creating a new plan and consider all unfinished plans and their open tickets.
- Plan IDs are allocated as P-<planID> and ticket IDs as T-<planID>-<ticketID>.
- Ticket IDs start from 1 within each plan.
- Cross-plan dependencies are allowed at both levels:
  - plan level via depends_on_plans in plan index and plan detail
  - ticket level via deps in ticket index
- The manager marks a plan done only when every ticket listed for that plan is status=done.

## Archival rules
- A plan is archivable when its status is done, every ticket listed in that plan is status=done, and claimed_by is null.
- A plan is not archivable if any plan with status != done depends on it via depends_on_plans.
- A plan is not archivable if any ticket outside the plan with status != done depends on one of that plan's tickets.
- Cleaner must archive a plan together with every ticket listed in that plan; do not archive partial plan contents.
- Cleaner must move the plan detail file into ARCHIVED_PLAN_DIR and the related ticket detail files into ARCHIVED_TICKET_DIR.
- Cleaner must remove archived plan entries from PLAN_INDEX_PATH and append equivalent entries to ARCHIVED_PLAN_INDEX_PATH with archived_at set.
- Cleaner must remove archived ticket entries from TICKET_INDEX_PATH and append equivalent entries to ARCHIVED_TICKET_INDEX_PATH with archived_at set.
- Cleaner may create ARCHIVED_PLAN_DIR, ARCHIVED_TICKET_DIR, ARCHIVED_PLAN_INDEX_PATH, or ARCHIVED_TICKET_INDEX_PATH if they do not yet exist.