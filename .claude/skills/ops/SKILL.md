---
name: ops
description: Git workflow, CI/CD, deployment, and environment management
user-invocable: true
allowed-tools: Read, Grep, Glob, Bash
argument-hint: "[git | ci | deploy | env]"
tags:
  - operations
  - shared
---

# Ops Skill

Operational patterns for git, CI/CD, deployment, and environments. These are general patterns — project-specific details live in `knowledge-base/project/`.

## Git Workflow

### Branching
- Feature branches off main: `feature/{short-description}`
- Bug fixes: `fix/{short-description}`
- Keep branches short-lived — one concern per branch

### Commits
- Concise message focused on WHY, not WHAT
- One logical change per commit
- Run tests before committing
- Never commit secrets, .env, credentials, or large binaries

### Pull Requests
- Title under 70 characters
- Body has: summary (what + why), test plan, any KB updates made
- One PR per concern — don't bundle unrelated changes

## CI/CD

- Check CI status after pushing — don't assume green
- If CI fails, read the actual error before attempting a fix
- Don't modify CI config to make a failing test pass — fix the test or the code

## Environments

- Check `knowledge-base/project/` for environment-specific configuration
- Never hardcode environment-specific values — use env vars or config files
- Test locally before pushing to shared environments

## Deployment

- Check `knowledge-base/project/` for deployment procedures
- Verify rollback plan exists before deploying
- Don't deploy on Fridays unless it's a hotfix (check with team lead)
