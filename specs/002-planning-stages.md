# Planning Improvement

The code agents should begin by creating a plan, on init and after each compaction.
## Tasks

- [ ] Always generate a plan initially; if the spec is valid we should still do a pass at planning first to make it a detailed actionable plan
- [ ] create a new command, "printer plan spec.md" which will generate the plan (should be documented as a checkpoint) and allow for the agent to generate questions for the end user to answer - this is an optional step that allows the agent to gather feedback, this is the only time an agent can stop the flow to prompt the user
- [ ] any time we rotate a session or perform compaction, create an updated plan before writing code

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
