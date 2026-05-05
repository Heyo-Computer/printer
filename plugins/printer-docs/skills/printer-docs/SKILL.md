---
name: printer-docs
description: Overview of the printer project and its CLIs. Use this skill to understand printer's goal of driving agent workflows with specs/tasks, and to discover other available skills.
version: 0.1.0
---

# printer-docs

## Project Goal

`printer` is a tool for driving agent workflows with specs and tasks. It provides:

- Spec-driven development: write specs in markdown, track tasks from checkboxes
- Agent session management: run agents to implement changes from specs
- Review cycles: automated feedback loops between agents to fix issues
- Sandboxing: optional heyvm integration for isolated worktree environments

## CLI Overview

### printer

The main CLI for spec-driven agent workflows:

- `printer run <spec.md>` — Run agent to implement tasks from spec
- `printer exec <spec.md>` — Run with sandboxing and review cycle
- `printer plan <spec.md>` — Generate detailed plan for a spec
- `printer task` — Task management commands
- `printer init <name>` — Create new spec with numbered naming

### computer

Desktop automation CLI for Wayland (Linux) and macOS:

- `computer outputs` — List connected monitors
- `computer windows` — List toplevel windows
- `computer screenshot` — Capture screen to PNG
- `computer mouse` — Move, click, scroll
- `computer key` — Send keystrokes and chords
- `computer type` — Type text with delay
- `computer browse` — Open URL in default browser

### codegraph

Code indexing and search CLI:

- `codegraph watch <dir>` — Watch directory for changes
- `codegraph search <query>` — Search indexed code
- `codegraph def <symbol>` — Go to definition
- `codegraph refs <symbol>` — Find references

## Skill Index

Available skills in this printer installation:

| Skill | Location | Purpose |
|-------|----------|---------|
| computer | plugins/computer/skills/computer | Desktop automation on Wayland/macOS |
| codegraph-edit | plugins/codegraph/skills/codegraph-edit | Modify source files via unified diff |
| codegraph-search | plugins/codegraph/skills/codegraph-search | Navigate code with tree-sitter |
| heyvm-sandbox | plugins/heyvm/skills/heyvm-sandbox | Understanding sandbox state in heyvm |
| poolside | plugins/poolside/skills/poolside | Running as Poolside ACP agent |
| printer-docs | plugins/printer-docs/skills/printer-docs | This file — project overview |

Skills are resolved from `~/.printer/plugins/*/skills/*/SKILL.md`.