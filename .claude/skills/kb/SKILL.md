---
name: kb
description: Knowledge base lifecycle — setup for new projects, maintain for existing ones
user-invocable: true
allowed-tools: Read, Glob, Grep, Bash, Edit, Write
argument-hint: "setup | maintain"
tags:
  - knowledge
  - shared
---

# KB Skill

Two modes: **setup** (seed a new project KB) and **maintain** (keep an existing KB fresh).

Use `setup` when initializing a new project.
Use `maintain` every few runs or after significant work.

See the mode-specific files for step-by-step instructions:
- Setup: `.claude/skills/kb/setup.md`
- Maintain: `.claude/skills/kb/maintain.md`
