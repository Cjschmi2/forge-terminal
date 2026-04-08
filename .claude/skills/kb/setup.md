# KB Setup Mode

Run this once when initializing the knowledge base for a new project. Follow each step in order. Do not skip steps.

## Prerequisites

- The project codebase exists and is functional
- You can run the test suite and build commands
- `.claude/knowledge-base/` directory structure exists

## Step 1: Scan the Codebase

Read the project's entry points, directory structure, and package config to understand:
- What language(s) and frameworks are used
- How the project is structured (monorepo, single app, microservices)
- What the build, test, and lint commands are
- What external services it talks to

```bash
# Get the lay of the land
ls -la
find . -name "package.json" -o -name "Cargo.toml" -o -name "pyproject.toml" -o -name "go.mod" | head -20
```

## Step 2: Populate project/

Create these files under `knowledge-base/project/`:

### architecture.md
- System overview: what the project does, major components
- Service map: what talks to what
- Data flow: how data moves through the system
- Key entry points: where execution starts

### integrations.md
- External services: APIs, databases, message queues, caches
- Auth providers, CDNs, storage
- How each integration is configured (env vars, config files)

### data-model.md
- Core entities and their relationships
- Database schemas or key data structures
- Where models/types are defined in the codebase

Keep each file under 3000 tokens. If a section is too large, split into multiple files.

## Step 3: Populate testing/

Create these files under `knowledge-base/testing/`:

### strategy.md
- Test framework(s) in use
- How to run tests (unit, integration, e2e)
- Coverage requirements if any
- What test utilities/helpers exist

### dependencies.md
- Test fixtures and where they live
- Mock/stub patterns used in this project
- External service mocks or test doubles
- Database seeding or test data setup

### critical-paths.md
- What MUST be tested for any change (auth, payments, data integrity)
- Known flaky tests and why
- Areas with low coverage that need extra attention

## Step 4: Populate security/

Create these files under `knowledge-base/security/`:

### auth-model.md
- How authentication works (JWT, sessions, API keys, OAuth)
- Authorization model (RBAC, ABAC, ACLs)
- Where auth logic lives in the codebase
- Token lifecycle, session management

### secrets.md
- What secrets exist (don't include values — just names and purposes)
- Where secrets are configured (env vars, secret manager, config)
- How secrets are accessed in code

## Step 5: Initialize planning/

Create `knowledge-base/planning/current.md` with:
- Current priorities (ask the user or check issue tracker)
- Recent decisions that affect development
- Known blockers or constraints

This section will be maintained by the team lead going forward.

## Step 6: Initialize gotchas/

Create empty gotcha files — these will be populated by agents during work:

```
knowledge-base/gotchas/backend.md    → "# Backend Gotchas\n\nNone yet."
knowledge-base/gotchas/frontend.md   → "# Frontend Gotchas\n\nNone yet."
knowledge-base/gotchas/data-infra.md → "# Data & Infra Gotchas\n\nNone yet."
```

## Step 7: Generate INDEX.md

Create `knowledge-base/INDEX.md` with:
- One-line description of every file in the KB
- Today's date as last-updated for each entry
- Total token estimate per file (rough is fine)

Format:
```markdown
# Knowledge Base Index

Last full setup: YYYY-MM-DD

## project/
- architecture.md — System overview, components, data flow (~2K tokens) [YYYY-MM-DD]
- integrations.md — External services and configuration (~1.5K tokens) [YYYY-MM-DD]
- data-model.md — Core entities and relationships (~1K tokens) [YYYY-MM-DD]

## testing/
- strategy.md — Test framework, commands, coverage (~1K tokens) [YYYY-MM-DD]
- dependencies.md — Fixtures, mocks, test data setup (~1.5K tokens) [YYYY-MM-DD]
- critical-paths.md — Must-test areas, flaky tests (~1K tokens) [YYYY-MM-DD]

## security/
- auth-model.md — Authentication and authorization (~1.5K tokens) [YYYY-MM-DD]
- secrets.md — Secret names and access patterns (~500 tokens) [YYYY-MM-DD]

## planning/
- current.md — Priorities, decisions, blockers (~500 tokens) [YYYY-MM-DD]

## gotchas/
- backend.md — Backend pitfalls [YYYY-MM-DD]
- frontend.md — Frontend pitfalls [YYYY-MM-DD]
- data-infra.md — Data & infra pitfalls [YYYY-MM-DD]
```

## Step 8: Validate

- [ ] Every file is under 3000 tokens
- [ ] INDEX.md lists every file with accurate descriptions
- [ ] No secrets or credentials in any KB file
- [ ] An agent reading INDEX.md can find what it needs in one hop
- [ ] CLAUDE.md build/test commands are accurate
