---
name: heyvm-workflow
description: Author, validate, and install YAML workflow templates for the Heyo orchestrator. Use when the user wants to write a new workflow (CI build, code-factory matrix, deploy pipeline) or troubleshoot one that isn't loading. Covers the schema, the sandbox.exec adapter, matrix fan-out, and where templates live (~/.heyo/orchestrator/templates/*.yaml or in-repo .heyo/workflows/*.yaml).
argument-hint: "[starter-kind | path-to-existing-yaml]"
allowed-tools: Bash, Read, Write, Edit
---

# Authoring orchestrator workflow YAMLs

You are helping the user author a YAML workflow template for the Heyo orchestrator (`/home/sarocu/Projects/heyo/orchestrator/`). Workflows declare a DAG of steps that run agents, gates, or shell commands inside a heyvm sandbox. The orchestrator parses the YAML, compiles it into the existing `WorkflowTemplateDefinition` struct, and runs it with the same engine as the built-in templates.

## When to use this skill

- "Write me a YAML workflow that builds my repo across rust stable + nightly."
- "How do I declare a workflow that runs `pytest` then deploys?"
- "My YAML isn't showing up in `/orchestration/templates`."
- "What's the schema for the matrix block?"
- "Add a step that runs `cargo audit` to my workflow."

If the user is asking about deploying a single app with the existing built-in `app.deploy_to_heyo` flow, that's not this skill — that's `heyvm-deploy`.

## Argument

`$ARGUMENTS` may be:
- empty — ask what kind of workflow to scaffold (CI build / matrix / deploy / custom).
- one of `ci`, `matrix`, `deploy`, `custom` — scaffold a starter and refine with the user.
- a path to an existing `.yaml` — read it, validate against the schema below, and propose fixes.

## Where workflows live

| Source | Path | When loaded |
|--------|------|-------------|
| Orchestrator-local | `~/.heyo/orchestrator/templates/*.yaml` | At orchestrator boot. Bad files log + skip. |
| In-repo | `<repo>/.heyo/workflows/<name>.yaml` | At compile time when `templateSource: { kind: "repo_file", path: ... }` is passed (Phase 4 — may not be wired yet; check). |

After dropping a YAML in the local templates dir, restart the orchestrator (`pkill -f orchestrator && cargo run -p orchestrator`) and verify it shows in:

```bash
curl -s localhost:4446/orchestration/templates -H "Authorization: Bearer $JWT" | jq '.templates[].id'
```

## Schema (snake_case YAML, compiled to camelCase struct)

```yaml
id: string                     # required, globally unique (e.g. "code_factory.cargo_build")
version: int                   # default 1; bump on breaking changes
name: string                   # required, human-readable
description: string            # optional, one-liner
inputs:                        # optional; produces a JSON Schema validated at compile_parent_job
  required: [list of input keys]
  properties:
    repoRoot: { type: string }
    appName:  { type: string }
    # any JSON Schema fragment is valid here
policy:                        # optional; defaults: all approvals false, allowed_targets: [heyo-sandbox]
  require_plan_approval: bool
  require_patch_approval: bool
  require_deploy_approval: bool
  allow_repo_mutation: bool
  allowed_targets: [heyo-sandbox]
phases: [list of phase names]  # optional; default = phases observed in step order
sandboxes:                     # optional; one entry per declared sandbox
  default:                     # name (referenced by step.sandbox)
    image: ghcr.io/heyo/rust-ci:latest   # required (OCI tag, img-* id, or absolute .ext4 path)
    env: { KEY: value }
    ports: [8080]
    size_class: small | medium | large
steps:                         # required, non-empty
  - key: string                # required, unique within workflow
    type: deterministic | agent | approval   # required
    kind: string               # default "deterministic"; informational (analysis|planning|verification|deployment|approval)
    phase: string              # required; group label
    adapter: string            # required for deterministic/agent (e.g. "sandbox.exec", "ai.discovery")
    depends_on: [other step keys]
    requires_approval_before_start: bool
    artifact_contract: { produces: [kind names] }
    retry_policy: { max_retries: 0 }

    # sandbox.exec specific:
    sandbox: <sandbox name>     # must match a key in sandboxes:
    run: |                      # multi-line shell — preferred
      cargo build --release
    command: string             # alternative to run; one program
    args: [string]              # used with command
    cwd: /path
    env: { KEY: value }
    timeout_seconds: 1800
    allow_failure: bool         # if true, non-zero exit becomes Completed with outputs.failedSoftly: true

    # matrix fan-out (Phase 3 — verify wired):
    strategy:
      matrix:
        os: [linux, macos]
        toolchain: [stable, nightly]
      max_parallel: 2
```

## Available adapters

- `sandbox.exec` — runs `run` (or `command + args`) in a heyvm sandbox via the cloud's exec endpoint. Captures stdout/stderr/exit_code into outputs and an `exec-log` artifact (text, capped at 256 KB).
- `ai.discovery`, `ai.planning` — LLM agent adapters (used by built-in templates; usable from YAML too).
- `repo.patch`, `repo.verify` — repo-mutation / verification adapters used by the integrate template.
- `heyo.deploy_preflight`, `heyo.deploy`, `heyo.healthcheck` — deploy-flow adapters.
- approval-type steps don't need an adapter — they create an approval record and block until `/orchestration/approvals/{id}/decide` is called.

If the user wants something that doesn't fit an existing adapter, do not invent a name — explain that a new adapter has to be added in Rust (`src/orchestration/adapters.rs::execute_adapter`) and offer to draft that.

## How sandbox provisioning works

When the workflow declares `sandboxes:`, the orchestrator at compile time:

1. For each sandbox name not already provisioned in `inputs.sandboxes.<name>.deploymentId`, it calls `cloud_client::create_deployment` with the declared image, ttl=7200s, and writes the resulting `deploymentId` into `inputs.sandboxes.<name>`.
2. The `sandbox.exec` adapter resolves the `deploymentId` from `inputs.sandboxes.<name>.deploymentId` for each step and forwards the command via the internal `/internal/orchestration/deployments/{id}/exec` cloud route.

The user can also pre-provision and pass `inputs.sandboxes.<name>.deploymentId` themselves at compile time — auto-provisioning is skipped for any sandbox name already populated.

The TTL (default 7200s) is the cleanup safety net. There is no explicit teardown step today; the cloud reaps expired sandboxes.

## Authoring procedure

1. **Determine the shape.** Ask the user one or two clarifying questions if the prompt is vague:
   - "Single sandbox or multi-sandbox?"
   - "Matrix over what?"
   - "Does the workflow need to clone the repo first, or is the image pre-baked?"

2. **Pick an `id` and `version`.** Use a namespace (`code_factory.<name>`, `ci.<lang>.<task>`). Always start at version 1.

3. **Declare `inputs`.** At minimum, list what the caller has to provide. Common ones: `repoRoot`, `branch`, `accountId`. Mark required ones in `required:`. Required keys are enforced via JSON Schema validation at `compile_parent_job` time — invalid inputs return 400.

4. **Declare `sandboxes`.** Pick an image. For Rust CI, `ghcr.io/heyo/rust-ci:latest` is a safe default; for generic Linux, `ubuntu:24.04`. If the user has no image in mind, suggest one based on the language and check with them. Don't fabricate registry paths.

5. **Write the steps DAG.**
   - Use `depends_on` to thread phases.
   - Prefer `run: |` blocks (multi-line shell) over `command + args`.
   - Keep one logical action per step — easier to retry, easier to read in artifacts.
   - Default `retry_policy: { max_retries: 0 }`. Bump only for known-flaky steps.

6. **Validate locally before installing.**
   ```bash
   cd /home/sarocu/Projects/heyo/orchestrator
   cargo test --bin orchestrator orchestration::yaml
   ```
   Then drop the YAML in `~/.heyo/orchestrator/templates/` and restart the orchestrator.

7. **Smoke-test via the API.** Use `curl` to compile a parent job against the new template; check `/orchestration/parent-jobs/{id}` for `status: completed` and inspect the `exec-log` artifacts.

## Starter: single-sandbox CI build

```yaml
id: ci.cargo_build
version: 1
name: Cargo CI build
description: Build a Rust crate in a heyvm sandbox and run tests.

inputs:
  required: [repoRoot]
  properties:
    repoRoot: { type: string }

sandboxes:
  default:
    image: ghcr.io/heyo/rust-ci:latest
    env: { CARGO_TERM_COLOR: always }

steps:
  - key: clone
    type: deterministic
    phase: setup
    sandbox: default
    adapter: sandbox.exec
    run: |
      git clone --depth 1 "$REPO_ROOT" /work
    env: { REPO_ROOT: "${{ inputs.repoRoot }}" }
    timeout_seconds: 300

  - key: build
    type: deterministic
    phase: build
    sandbox: default
    adapter: sandbox.exec
    depends_on: [clone]
    run: |
      cd /work
      cargo build --release
    timeout_seconds: 1800

  - key: test
    type: deterministic
    phase: verify
    sandbox: default
    adapter: sandbox.exec
    depends_on: [build]
    run: |
      cd /work
      cargo test --release
    timeout_seconds: 900
```

## Starter: code-factory matrix (Phase 3 — verify wired before promising)

```yaml
id: code_factory.cargo_matrix
version: 1
name: Cargo matrix build

inputs:
  required: [repoRoot]
  properties:
    repoRoot: { type: string }

sandboxes:
  default:
    image: ghcr.io/heyo/rust-ci:latest

steps:
  - key: build
    type: deterministic
    phase: build
    sandbox: default
    adapter: sandbox.exec
    strategy:
      matrix:
        toolchain: [stable, nightly]
      max_parallel: 2
    run: |
      rustup default ${{ matrix.toolchain }}
      cargo build --release
    timeout_seconds: 1800

  - key: bundle
    type: deterministic
    phase: publish
    sandbox: default
    adapter: sandbox.exec
    depends_on: [build]   # waits for both matrix children via the synthesized aggregator
    run: tar czf /out/build.tgz target/release
```

Before showing the matrix starter to the user, **check whether matrix is implemented** by looking for `matrix.aggregate` in `src/orchestration/adapters.rs::execute_adapter` and a substitute fn in `src/orchestration/matrix.rs`. If it isn't wired yet, say so and offer the single-sandbox starter instead.

## Common authoring mistakes

- **Missing `image` on a sandbox** — required; auto-provision will refuse.
- **`depends_on` referencing an unknown step key** — compile fails with a clear error.
- **Using `${{ matrix.x }}` outside a `strategy.matrix` step** — substitution will leave the literal in the command. Compile doesn't (yet) reject it; runtime will exec a malformed shell line.
- **Specifying a `sandbox` name that isn't declared in `sandboxes:`** — compile fails.
- **Forgetting `requires_approval_before_start: true` on dangerous deterministic steps** — if the workflow mutates the repo or shared infra, gate it with an approval step *before* the deterministic mutation, with `depends_on: [<approval key>]`.
- **Camel-cased YAML keys** — YAML is snake_case (`depends_on`, `max_retries`, `timeout_seconds`). The compiled JSON struct is camelCase but that's invisible to the YAML author.

## Verifying changes the orchestrator picked up the file

```bash
# After editing ~/.heyo/orchestrator/templates/<your>.yaml
pkill -f 'target/.*orchestrator' || true
cd /home/sarocu/Projects/heyo/orchestrator && cargo run --release > /tmp/orch.log 2>&1 &
sleep 2
grep -i "Loaded.*YAML workflow template" /tmp/orch.log
```

If the grep is empty: the file failed to parse; tail `/tmp/orch.log` for the warning.

## When to defer to the user

- Choosing the sandbox image — never fabricate a registry path.
- The exact retry budget for known-flaky tasks.
- Whether a deterministic step needs an approval gate.
- Whether to add a new adapter vs. fitting the work into `sandbox.exec`.
