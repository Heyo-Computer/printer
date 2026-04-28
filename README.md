# Printer
In the "Bobiverse", the printer is the technology that enables the Bobs' self replicating journey across the stars. The printer works at the atomic level and can produce any good, including more printers and the equipment to replicate the Bobs themselves. In this future, humans trade time on the printers as currency. They are the foundation of the universe's economic and scientific ambitions.

LLMs are good at writing code. Actually, that's probably what they are best at. Integrating "tools" and running in a loop creates the powerful "agent" paradigm. One more layer of abstraction is the "code factory" which uses a system of agents to produce software autonomously from a specification. Now, humans can produce a lot of software by managing an agent and poking it when it needs to keep going; this commands a lot of attention from a human and ultimately becomes a bottleneck in agentic development and the code factory pattern tries to solve for that particular bottleneck by allowing the human to draft requirements and then startup the factory before moving on to another task or factory. There are complications of course; a system of agents has a lot of moving parts and can burn tokens at an exorbitant rate. 

`printer` aims to implement the simplest form of the code factory pattern. This isn't Gastown so don't expect a dozen subagents running at once. Progress and memory are file based for durability. The plugin system provides a simple way to extend the printer with existing CLI tools. Multi-agent support lets you configure the models and agents in use. 

## Components
The project is made up of 3 CLIs, agent skills, and a plugin system for extending functionality with other tooling. 

### Printer
The core CLI that manages agent sessions to program against a spec file. 

### Computer
CLI that implements programatic desktop interactions for Wayland. Allows agents to work with the desktop directly (on Linux).

### Codegraph
Uses tree-sitter to parse, graph, and query a codebase. Additionally contains patch commands for applying diffs to files. 

The CLI works with skills to help keep token usage efficient and reduce redundant searching and reading of files. 

### Printer Plugins
The plugin system allows the printer to be extended by running arbitrary commands or skills with lifecycle hooks. See [Hooks](printer/HOOKS.md)

### Agent Plugins
the Printer CLI can install a plugin for Claude and OpenCode agents to utilize `codegraph` for searching and patching files. 

### Skills
Skills are made available to the agents during run and review.
```bash
npx skills add heyo-computer/printer
```

## Getting Started
The printer cli has a helpful command to generate an example spec markdown file, graph the project, and setup the task tracking:
```
printer init

# or give it a filename for the spec file:
printer init feature.md

# you now also have new directories:
ls -la 
...
.printer/
.codegraph/
...
```


## Orchestration
The simplest way to kick off is to use the `exec` command, this is equivalent to running `printer run spec.md && printer review spec.md`.
