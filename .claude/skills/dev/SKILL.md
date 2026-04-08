---
name: dev
description: Code patterns, conventions, and quality standards shared across all projects
user-invocable: true
allowed-tools: Read, Grep, Glob
argument-hint: "[patterns | testing | errors]"
tags:
  - development
  - shared
---

# Dev Skill

General development patterns that apply to ALL projects. Project-specific knowledge lives in `knowledge-base/` — this skill covers universal standards.

## Code Patterns

### Read Before Write
Always read the file and surrounding code before editing. Understand the existing patterns — match them, don't impose new ones.

### Test Alongside, Not After
Write tests as you implement. Each function gets a test before you move to the next function. Don't batch all tests at the end.

### Minimal Changes
Do what was asked. Don't refactor adjacent code, add comments to code you didn't change, or "improve" things beyond scope.

### Error Handling
- Validate at system boundaries (user input, external APIs, file I/O)
- Trust internal code and framework guarantees
- Don't add error handling for scenarios that can't happen

### Dependencies
- Check that imports resolve after adding them
- Don't add dependencies for things the stdlib handles
- When adding a package, verify it's actually used

## Naming Conventions

Follow whatever convention the project already uses. If the project uses camelCase, use camelCase. If it uses snake_case, use snake_case. Never mix conventions within a file.

## File Organization

- Don't create new files for single functions — add to existing files where the function belongs
- Don't create helpers/utilities for one-time operations
- Don't create abstractions until you have three concrete uses

## Testing Patterns

- Test behavior, not implementation details
- One assertion per test when possible
- Use descriptive test names that explain what's being verified
- Check `knowledge-base/testing/` for project-specific test dependencies, fixtures, and coverage requirements
