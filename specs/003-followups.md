# Create follow up for next spec

Make creating specs more ergonomic and automate passing feedback to the next session

## Tasks

- [ ] save the followups from a review agent in the .printer dir for that spec
- [ ] new command to create an agent session to generate a new spec from the follup file
- [ ] when using "init" to create a spec doc, if .printer already exists (e.g. printer has been used in the repo before) then create a specs/ folder and use a numbered system for a naming template, e.g. "001-cool-feature.md" and "002-followup.md" where the user passes in the name to be used, like "printer init feat-deploy-assets" 

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
