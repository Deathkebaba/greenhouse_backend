# Repository Directives

This file is the canonical, repository-scoped source of product intent and execution guardrails.
All agents and collaborators must follow these directives unless an explicit exception is approved and recorded.

## Repository Configuration
- BASE_BRANCH: main
- Remote access: none — agents have no access to the remote repository; do not attempt push, pull, or fetch.

## Vision
- The target product is a clean, maintainable greenhouse backend platform built around explicit service boundaries.
- The system provides authentication, device management, alert storage, diary storage, and scripting-oriented integration flows.
- The codebase should remain understandable to a new engineer reading one crate at a time.
- Prefer designs that stay coherent under growth instead of clever local shortcuts.

## Primary Engineering Directive
- Favor clean, beautiful code.
- Clean means clear ownership, low surprise, obvious control flow, and small well-named abstractions.
- Beautiful means the code reads as if it was designed on purpose: consistent boundaries, coherent APIs, and minimal accidental complexity.
- Do not preserve awkward code paths, duplicate protocols, or half-migrated designs unless explicitly required.

## Architectural Baseline
- Keep public API gateways thin. Business rules, authorization policy, and persistence invariants belong in the owning service, not only in the outer API layer.
- Preserve strong service ownership. Each service owns its schema, storage logic, and domain behavior.
- Use shared contracts from greenhouse_core for inter-service and device-facing payloads instead of ad hoc duplicate types.
- Keep cross-crate integration explicit. Prefer clear DTOs and well-defined route contracts over hidden coupling.
- Avoid introducing a second source of truth for endpoint semantics, auth semantics, or data contracts.

## Security And Trust Boundaries
- Treat every network boundary as untrusted unless trust is enforced in code or infrastructure and documented explicitly.
- Authorization must be enforced at the boundary that owns the protected resource.
- Do not rely on “internal only” exposure as the sole protection for mutable service endpoints.
- Device registration and device communication must follow explicit trust rules. Do not allow arbitrary network reachability, scraping targets, or activation flows without deliberate validation and ownership checks.
- Do not add insecure validation shortcuts, unsigned claim parsing, or bypass paths as convenience helpers.

## Data And Persistence Policy
- Persistence changes must preserve clear ownership by service.
- Schema changes require migration safety, rollback thinking, and explicit impact on startup and deployment behavior.
- Do not hide schema mutation inside incidental request-serving paths when a controlled operational path is more appropriate.
- Prefer explicit data lifecycle decisions over silent accumulation of legacy tables, fields, or fallback behavior.

## API And Contract Policy
- Keep APIs predictable and honest.
- Never return success for rejected, skipped, or unauthorized mutations.
- Error semantics must reflect real execution outcomes.
- Shared DTOs and endpoint constants should be updated deliberately and consistently across producers and consumers.
- When a route is gateway-only, service-only, or cross-service, that role must be obvious from the code and documentation.

## Device Integration Policy
- Example devices are first-class architectural fixtures and should remain aligned with the real smart-device contract.
- Device-facing protocols must remain stable, explicit, and testable.
- Polling, scraping, activation, and configuration flows must be bounded and observable.
- Any feature that expands device-driven network access requires explicit review for blast radius, ownership, and failure handling.

## Operational Consistency Policy
- Local development, integration tests, and documented runtime topology must describe the same system.
- Avoid drift between compose files, config examples, test harnesses, and actual service expectations.
- If a service is part of the supported architecture, local bootstrap and test infrastructure should both account for it.
- Observability hooks such as tracing, health endpoints, metrics, and Sentry should remain consistent across services.

## Testing Policy
- Preserve and extend end-to-end coverage across crate boundaries.
- Test coverage should be strongest where behavior crosses service, database, network, or device boundaries.
- Add focused tests for trust-boundary rules, auth behavior, device validation, and persistence invariants when changing those areas.
- Prefer tests that validate real integration behavior over narrow mock-only reassurance for cross-service flows.

## Positive Patterns To Preserve
- Keep greenhouse_core as the shared contract layer for DTOs and device-facing interfaces.
- Keep service responsibilities separated rather than collapsing everything into a single crate.
- Preserve integration-tests as the place where the real composed system is validated.
- Preserve runnable examples as living documentation for smart-device behavior.
- Preserve the justfile and compose-based local workflow as first-class operational tooling.

## Scope Policy
- Do not narrow ticket scope, acceptance, or quality bars without explicit approval.
- If a requirement is ambiguous, ask for clarification instead of silently reducing scope.
- If implementation reveals larger necessary scope, surface it as a dependency or follow-up ticket.
- Distinguish between local defects and architectural policy: fix small defects locally, but only promote recurring concerns into directives.

## Legacy And Cleanup Policy
- Do not retain legacy code paths as fallback by default.
- Remove dead abstractions, duplicate validation paths, and outdated repository assumptions when they are no longer serving the current system.
- When fallback behavior is required, document why it exists, who owns it, and what conditions permit removal.

## Reviewer Enforcement
- Reviewer output must include directive compliance status and any violations.
- If violations exist without an approved exception, reviewer verdict must be `changes_requested`.
- Reviewers should explicitly call out both regressions and positive alignment with the repository’s design principles.

## Exception Process
- Exceptions must be explicit and documented in the ticket detail history.
- Every exception entry must include:
  - directive affected
  - rationale
  - approver
  - expiration or revisit condition