# Recursive printing pattern

Add an option for recursive behavior, e.g. "printer prints more printers". This is combined with heyvm worktree sandboxes such that each tasks gets its own agent and printer instance.
## Tasks
- [ ] option integrates with the heyvm plugin for sandboxes and worktrees
- [ ] spawn a worktree and a heyvm sandbox for each task
- [ ] spawn an agent to create a plan and then execute for each task
- [ ] tasks with dependencies should be linked (requires pre-planning step)
- [ ] a codegraph instance should run in each worktree directory to ensure it picks up changes for each agent
- [ ] use the "stacked pr" pattern to merge commits into the branch 

<!--
Spec format reference (full docs in the printer README):
  * Lines starting with `- [ ]`, `- [x]`, `* [ ]`, `+ [ ]` (etc.) at
    column 0 are tasks. The text after the checkbox is the title.
  * Lines indented by 2 spaces or one tab below a task become its
    description body.
  * Any unindented non-task line ends the current task's description.
  * Re-runs of `printer run <this-file>` are idempotent — items are
    matched to existing tasks by a stable anchor derived from this
    file's path + the task title.
-->
