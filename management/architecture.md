# Repository Architecture

This file is the canonical architectural reference for the repository.
Planner, manager, implementer, and reviewer agents should consult this before designing or reviewing changes.

---

## Product Context

This repository currently implements a greenhouse backend platform built as a Rust Cargo workspace.
It exposes public HTTP APIs, internal service APIs, shared DTO crates, and example smart-device binaries.

The core domain areas in the current workspace are:

- authentication and user preferences
- device registration, configuration, activation, and telemetry scraping
- alert storage and retrieval
- diary entry storage and retrieval
- script-oriented token management
- smart-device integration examples used for local and staged environments

---

## Tech Stack

| Layer | Technology |
|---|---|
| Language | Rust |
| Async runtime | Tokio |
| HTTP framework | Axum |
| HTTP client | Reqwest |
| Persistence | PostgreSQL |
| ORM / migrations | Diesel, diesel-async, diesel_migrations |
| Auth primitives | JWT, bcrypt, cookie-based session transport at API layer |
| Observability | tracing, tower-http trace, Sentry, Prometheus metrics in device service |
| Serialization | serde, serde_json, serde_yaml |
| Integration tests | cargo test workspace + testcontainers-based end-to-end tests |
| Container orchestration | Docker, Docker Compose |

---

## Repository Layout

```
api/
  script/                  Public scripting API gateway
  web/                     Public web API gateway
services/
  auth_service/            Auth and user preference service
  data_storage_service/    Alert and diary persistence service
  device_service/          Device registry and telemetry scraping service
  scripting_service/       Script token service
greenhouse_core/           Shared DTOs, traits, and optional HTTP error mapping
greenhouse_macro/          Procedural macros used across services and APIs
integration-tests/         End-to-end service and API integration tests
examples/                  Example smart-device binaries
docker/                    Runtime support files, including PostgreSQL bootstrap
scripts/                   Helper scripts and service config assets
management/                Plans, tickets, directives, and architecture reference
```

The workspace is defined in [Cargo.toml](../Cargo.toml) and currently includes both API crates, the shared crates, the four services, the example binaries crate, and the integration test crate.

---

## Workspace Structure

### Public API Layer

The repository exposes two API gateway binaries:

| Crate | Role |
|---|---|
| [api/web](../api/web) | Main user-facing API for settings, diary, alert, device, and user operations |
| [api/script](../api/script) | Scripting-focused API gateway for automation-oriented flows |

Both APIs:

- are built with Axum routers
- load configuration from environment-backed config files
- apply CORS and request tracing middleware
- use cookie middleware
- proxy requests to internal services over HTTP rather than sharing service internals directly

### Internal Service Layer

The service layer is split into four independently runnable crates under [services/](../services):

| Service | Responsibility |
|---|---|
| [auth_service](../services/auth_service) | user registration, login, token validation, one-time token generation, user preferences |
| [data_storage_service](../services/data_storage_service) | alert and diary entry persistence |
| [device_service](../services/device_service) | device CRUD, activation, config access, status, time-series access, telemetry scraping, metrics |
| [scripting_service](../services/scripting_service) | token management for script-based integrations |

Each service follows the same high-level startup pattern:

- deserialize a config structure from environment-backed settings
- initialize a bb8 PostgreSQL connection pool using diesel-async
- run embedded Diesel migrations at startup
- build an Axum router via a crate-level app function
- expose a lightweight /health endpoint

### Shared Crates

| Crate | Role |
|---|---|
| [greenhouse_core](../greenhouse_core) | shared DTO definitions, endpoint constants, smart-device DTOs, smart-device traits, optional HTTP error mapping |
| [greenhouse_macro](../greenhouse_macro) | procedural macros including role-based endpoint authentication and response helpers |

The shared crate boundaries are important to the current architecture:

- service APIs communicate through DTO modules in greenhouse_core
- endpoint paths are centralized in DTO modules where available
- smart-device examples depend on shared DTOs and interfaces instead of service-specific internals

---

## Key API Modules

### Web API

[api/web/src/lib.rs](../api/web/src/lib.rs) builds the main gateway router.

Its top-level route groups are:

- /api/settings
- /api/diary
- /api/alert
- /api/device
- /api/user
- auth routes merged separately
- /health

The web API applies token-check middleware to protected routes and forwards requests to the configured internal service addresses for auth, device, and data-storage capabilities.

### Script API

[api/script/src/lib.rs](../api/script/src/lib.rs) builds the scripting gateway.

Its current surface is narrower than the web API and primarily exposes:

- /alert
- /health

It uses the same general middleware pattern as the web API and is intended for non-browser or automation-oriented clients.

---

## Key Service Modules

### Auth Service

[services/auth_service/src/lib.rs](../services/auth_service/src/lib.rs) wires the authentication service.

Core responsibilities:

- account registration
- login
- token validation
- admin and guest registration flows
- get and set user preferences
- one-time token generation

The service exposes routes based on constants from greenhouse_core auth DTO modules and owns its own migration set under [services/auth_service/migrations](../services/auth_service/migrations).

### Device Service

[services/device_service/src/lib.rs](../services/device_service/src/lib.rs) wires the device service and also starts a background scrape loop.

Core responsibilities:

- device registration and lookup
- device updates and operations
- activation and config endpoints
- device status and time-series queries
- Prometheus metrics exposure at /metrics
- background HTTP scraping of configured devices

The background scraper is a defining architectural behavior in this repo: device data is pulled from smart devices over HTTP on an interval instead of being exclusively pushed through the public APIs.

### Data Storage Service

[services/data_storage_service/src/lib.rs](../services/data_storage_service/src/lib.rs) owns persistence for alerts and diary entries.

It nests routers using endpoint constants from greenhouse_core and keeps storage concerns separated from API gateway concerns.

### Scripting Service

[services/scripting_service/src/lib.rs](../services/scripting_service/src/lib.rs) owns scripting token routes.

It is a dedicated backend dependency for scripting and automation scenarios rather than being folded into auth_service.

---

## Shared Contracts And Macros

### Shared DTOs

[greenhouse_core/src/lib.rs](../greenhouse_core/src/lib.rs) conditionally exposes domain-specific DTO modules for:

- auth service
- data storage service
- device service
- scripting service
- smart device communication
- smart device interface traits

This crate is the main contract boundary across crates.
When adding or changing payloads between APIs, services, and devices, start in greenhouse_core before changing handler logic.

### Procedural Macros

[greenhouse_macro/src/lib.rs](../greenhouse_macro/src/lib.rs) provides reusable compile-time helpers.

The most important current macro is an authenticate attribute that checks cookie-based auth claims and enforces an expected role before executing a handler.

---

## Runtime Data Flow

The dominant runtime flow in the current workspace is service-oriented HTTP composition:

1. A client calls either the web API or the scripting API.
2. The API gateway applies tracing, CORS, cookie handling, and token middleware.
3. The API layer forwards the request to the appropriate internal service over HTTP.
4. The target service executes domain logic and persists or fetches data through PostgreSQL using Diesel.
5. The response is mapped back through the API layer to the caller.

The device flow adds one more actor:

1. Smart devices expose HTTP endpoints using the shared smart-device contracts.
2. The device service stores device metadata and scrape configuration.
3. A background scrape loop periodically calls devices and records or exports telemetry.
4. Downstream API consumers retrieve device status or time-series information through the device-related routes.

---

## Persistence Architecture

The current persistence layer is PostgreSQL-backed and service-oriented.

### Database Ownership

The local Docker bootstrap in [docker/postgres/init.sql](../docker/postgres/init.sql) creates these databases:

- auth
- data
- device

Each service owns its schema through embedded Diesel migrations in its own crate.
The repository structure indicates strong schema ownership by service rather than one shared migration set at the workspace root.

### Access Pattern

Across services, the standard access pattern is:

- connection pool via bb8 plus diesel-async
- startup migration execution using embed_migrations
- per-service database modules under each service crate

This architecture keeps persistence logic close to the owning service and avoids direct database access from the public API crates.

---

## Configuration And Operations

### Environment Configuration

Each API and service crate contains a .env.example file under its config directory, indicating environment-driven runtime configuration per component.

Common config fields across the workspace include:

- service or API port
- database URL where relevant
- upstream service addresses for API gateways
- JWT secret where relevant
- Sentry URL
- environment name

### Local Development Tasks

[justfile](../justfile) is the primary developer entry point for local orchestration.

Important commands include:

- running all services together
- running both APIs together
- starting all runtime components
- starting all except selected components
- cargo test workspace execution
- lint and formatting
- local example device startup

### Container Orchestration

[compose.yaml](../compose.yaml) currently provisions:

- a PostgreSQL container with initialization SQL mounted from docker/postgres/init.sql
- three example smart-device containers built from the examples crate

The compose file is currently oriented toward infrastructure and example-device staging rather than full end-to-end deployment of every Rust service crate.

---

## Examples And Smart Devices

The [examples/](../examples) crate contains runnable smart-device examples used to exercise the device integration surface.

Current examples include:

- input_output_int_saver
- input_alert_trigger
- periodic_alert

These binaries are important architectural fixtures because they document the expected smart-device interaction model for the rest of the system.

---

## Testing Strategy

[integration-tests/](../integration-tests) contains workspace-level end-to-end tests.

The current test suite covers:

- auth flows
- alert flows
- diary flows
- device flows

The integration test crate is structurally significant because it validates behavior across crate boundaries rather than limiting verification to unit-level modules inside each service.

---

## Architectural Decisions

### A. API Gateways Are Thin Proxies Over Internal Services

The public API crates mostly compose middleware, route trees, and HTTP forwarding.
Business logic and persistence remain in the service crates.

### B. Service Contracts Live In A Shared Core Crate

DTOs, endpoint constants, and smart-device interfaces are centralized in greenhouse_core so that APIs, services, tests, and examples share one contract source.

### C. Services Own Their Schema And Startup Migration

Each service embeds and runs its own Diesel migrations at startup instead of depending on a monolithic database migration layer.

### D. Device Integration Is Pull-Based At Runtime

The device service actively scrapes configured devices and exposes metrics, making periodic polling part of the runtime architecture rather than just an operational add-on.

### E. Local Orchestration Is Workspace-Centric

The Cargo workspace, justfile, and compose file are all first-class parts of the repository architecture.
Development and testing assume independently runnable crates that can still be exercised together.

---

## Management System

| Path | Purpose |
|---|---|
| [directives.md](directives.md) | Repository-wide guardrails and quality constraints |
| [plans/index.yaml](plans/index.yaml) | Active plan index |
| [plans/](plans/) | Active plan detail files |
| [tickets/index.yaml](tickets/index.yaml) | Active ticket index |
| [tickets/](tickets/) | Active ticket detail files |
| [archive/plans/index.yaml](archive/plans/index.yaml) | Archived plan index |
| [archive/tickets/index.yaml](archive/tickets/index.yaml) | Archived ticket index |
| [archive/](archive/) | Archived management artifacts |

Agents must load [directives.md](directives.md) before planning, implementation, or review work. See [Ticket System.instructions.md](../.github/instructions/Ticket%20System.instructions.md) for the management contract.
