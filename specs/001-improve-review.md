# Improve Review Agent

Improve the handoff between coding and review to improve the quality of the finished work. 

## Tasks
- [ ] implement a review cycle in which the feedback from review is passed to a coding agent to work on
- [ ] the CLI should have an option for max number of review passes to prevent an infinite loop
- [ ] ensure that the review agent has the tools and instructions to actually test the changes
- [ ] have the review agent check for a "AGENTS.md" file for instructions like building and testing
- [ ] if plugins are not installed when "exec" or "run" is invoked, prompt the user to install (or suppress with a flag for CI systems)

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
