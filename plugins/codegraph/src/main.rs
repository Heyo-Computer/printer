//! Trivial dispatch shim. The real work this plugin does is register hooks
//! into printer's run/review lifecycle (see `printer-plugin.toml` and the
//! `skills/` directory). When invoked directly via `printer codegraph` we
//! just print a one-screen explainer.

fn main() {
    let usage = "\
codegraph plugin — registers printer hooks so the agent prefers
the `codegraph` CLI for navigation and patches.

Once installed (via `printer add-plugin path:./plugins/codegraph` or by
git URL once published), the plugin contributes the following hooks:

  before_run     agent-skill   skills/codegraph-search/SKILL.md
  before_run     agent-skill   skills/codegraph-edit/SKILL.md
  before_run     agent-cmd     prefer codegraph for navigation/patches
  before_run     cli           codegraph index    (refresh on each run)
  before_review  agent-skill   skills/codegraph-search/SKILL.md

Inspect what's wired up after install with:

  printer hooks list
  printer hooks list --event before_run

This shim binary has no other behaviour. The actual `codegraph` CLI
(searched on $PATH) is what the agent uses inside its session.
";
    print!("{usage}");
}
