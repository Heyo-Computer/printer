# Create heyvm plugin

Enhance the plugin system to support sandbox drivers and a worktree system.
## Tasks
- [ ] extend the plugin system to support a VM driver for sandboxing
- [ ] the initial and default plugin will be the "heyvm" cli https://docs.heyo.computer
- [ ] a global config file should contain the configuration for the sandboxes 
- [ ] the config should include commands about how to create the sandboxes
- [ ] add an initial printer plugin for heyvm that uses its worktree command to start a new sandbox

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
