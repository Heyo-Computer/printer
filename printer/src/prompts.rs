/// Sentinel emitted by the agent when every task is done.
pub const SENTINEL_DONE: &str = "<<ALL_DONE>>";
/// Sentinel prefix emitted when the agent is blocked.
pub const SENTINEL_BLOCKED: &str = "<<BLOCKED:";
/// Sentinel emitted after the bootstrap turn writes a checklist back to the
/// spec.
pub const SENTINEL_PLAN_READY: &str = "<<PLAN_READY>>";

/// Prompt for the empty-spec bootstrap turn: ask the agent to write a clean
/// checklist into the spec file. The driver will then parse it and create
/// tasks. The agent does NOT execute work here.
pub fn bootstrap_prompt(spec_path: &str) -> String {
    format!(
        "The file `{spec}` does not currently contain a markdown checklist that the driver can \
parse into tasks. Read it, infer the work it implies, and rewrite it as a flat checklist using \
GitHub-flavored markdown task list lines at column 0:\n\
\n\
  - [ ] Short imperative title for one unit of work\n\
    (optional indented description, 2-space indented, multiple lines OK)\n\
  - [ ] Next item\n\
\n\
Rules:\n\
- Preserve any items that are already `- [x]` (treat them as already done).\n\
- Each top-level `- [ ]` line becomes one task.\n\
- Indented lines beneath an item become its description.\n\
- Do NOT execute any work in this turn — only write the checklist back to `{spec}`.\n\
- Output the literal sentinel {plan_ready} on its own line when finished.\n",
        spec = spec_path,
        plan_ready = SENTINEL_PLAN_READY,
    )
}

/// Per-turn nudge: the agent uses `printer task` to manage its work queue.
pub fn nudge_prompt(printer_bin: &str) -> String {
    format!(
        "You are working through a queue of tasks tracked by `printer task`. The task store is the \
source of truth for status — not the original spec file. Use these commands:\n\
\n\
  {printer} task ready              — list ready tasks (highest priority first)\n\
  {printer} task list               — full table; add `--status in_progress` etc. for filtering\n\
  {printer} task show <ID>          — full description and notes for one task\n\
  {printer} task start <ID>         — claim a task before working on it\n\
  {printer} task comment <ID> \"...\"  — log progress / findings\n\
  {printer} task done <ID>          — mark fully complete\n\
  {printer} task block <ID> --reason \"...\"  — if you are stuck on something external\n\
\n\
This turn:\n\
1. Run `{printer} task ready`. If it is empty AND `{printer} task list --status in_progress` is \
also empty, every task is finished — output the literal sentinel {done} on its own line and stop.\n\
2. Otherwise pick the top ready task. Claim it with `{printer} task start <ID>`. Read its \
description with `{printer} task show <ID>`.\n\
3. Actually do the work — edit code, run commands, etc. Do not just describe what you would do.\n\
4. When the task is fully complete, `{printer} task done <ID>`. If you finish quickly you may \
claim the next ready task and continue; otherwise stop and the driver will call you again.\n\
5. If you cannot proceed, `{printer} task block <ID> --reason \"…\"` and emit \
{blocked} <one-line reason>>> to stop.\n",
        printer = printer_bin,
        done = SENTINEL_DONE,
        blocked = SENTINEL_BLOCKED,
    )
}

/// Stronger nudge after a stalled turn (no task transitions observed).
pub fn unstall_prompt(printer_bin: &str) -> String {
    format!(
        "Last turn made no observable progress in the task store. Re-read `{printer} task ready`, \
claim the top item with `{printer} task start <ID>`, and either finish it now and run `{printer} \
task done <ID>`, or run `{printer} task block <ID> --reason \"…\"` and emit \
{blocked} <reason>>>. Do not stall again.\n",
        printer = printer_bin,
        blocked = SENTINEL_BLOCKED,
    )
}

/// Prompt at the start of a freshly-rotated session (compaction).
pub fn rotation_prompt(printer_bin: &str, spec_path: &str) -> String {
    format!(
        "This is a fresh session — the previous session was rotated for context-window reasons. \
The state of the world is captured in:\n\
\n\
  - the original specification at `{spec}` (read-only reference)\n\
  - the task store, which is authoritative for status\n\
\n\
Run `{printer} task ready` to find the next item, claim it with `{printer} task start <ID>`, \
work it, and finish with `{printer} task done <ID>`. When everything is done emit {done}; if you \
get stuck emit {blocked} <reason>>>.\n",
        printer = printer_bin,
        spec = spec_path,
        done = SENTINEL_DONE,
        blocked = SENTINEL_BLOCKED,
    )
}

pub fn review_prompt(spec_path: &str, base_ref: &str) -> String {
    format!(
        "You are reviewing the result of an agent-driven implementation against its original \
specification.\n\
\n\
1. Read `{spec}`. Treat its checklist as the requirements that were supposed to be delivered.\n\
2. Inspect the working tree changes against the git ref `{base}`. Use `git diff {base}...HEAD` and \
`git status` (and `git diff` for unstaged work) to see what actually changed. Read the relevant \
changed files.\n\
3. Produce a concise markdown review report on stdout with these sections:\n\
   - `## Verdict` — one of: PASS, PARTIAL, FAIL.\n\
   - `## Per-item findings` — for each checklist item in `{spec}`, mark MET / PARTIAL / MISSING and \
explain in one line why, citing files/lines.\n\
   - `## Out-of-scope or risky changes` — anything modified that is not justified by the spec, \
including possible regressions.\n\
   - `## Suggested follow-ups` — short bulleted list, or 'none'.\n\
\n\
Be terse and concrete. Cite file paths. Do not modify any files.\n",
        spec = spec_path,
        base = base_ref,
    )
}
