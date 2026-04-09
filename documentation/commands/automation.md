# `automation` Commands

## Purpose

The `automation` command groups the reusable Rust subcommands used by template workflows.

Typical execution pattern:

```bash
cargo run -- automation <subcommand> [options]
```

In downstream repositories, workflows usually invoke this from the checked-out central automation repository under `.automation`.

## Main Families

### Sync

- `sync-branch`
- `sync-pull-request`
- `merge-sync-pull-request`

Used by the sync workflow to propagate `main` into `dev`.

Primary module:

- [automation_sync.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/automation_sync.rs)

### PR validation

- `resolve-workflow-run-pr`
- `refresh-pr-validation`

Used to resolve workflow-run PR context and refresh the PR `Validation Gate` after later repository changes when that section needs to be recomputed.

Primary modules:

- [workflow_run_pr.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/workflow_run_pr.rs)
- [pr_validation_refresh.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/pr_validation_refresh.rs)

### Issue lifecycle and guards

- `closure-hygiene`
- `parent-guard`
- `done-in-dev-status`
- `reopen-on-dev`
- `issue-parent-autolink`

These commands manage issue state, parent-child relations, and cross-workflow consistency.

Primary modules:

- [closure_hygiene.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/closure_hygiene.rs)
- [parent_guard.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/parent_guard.rs)
- [done_in_dev_status.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/done_in_dev_status.rs)
- [issue_reopen_on_dev.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/issue_reopen_on_dev.rs)
- [issue_parent_autolink.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/issue_parent_autolink.rs)

### PR body and directive management

- `auto-add-closes`
- `directive-conflict-guard`
- `closure-neutralizer`
- `closure-neutralizer-reevaluate`
- `closure-neutralizer-skip-check`
- `resolve-pr-body-context`
- `generate-pr-description`
- `guard-pr-body-contract`

These commands manage PR body content, directive normalization, and contract enforcement.

For the `Validation Gate`, the current contract is:

- render `- No breaking change` when no breaking change is detected
- render `- Breaking change` when breaking change analysis detects one
- render a `Breaking scope` subsection when a breaking change is detected
- include affected `crate(s)` when they can be inferred
- include `source commit(s)` that triggered the breaking-change analysis

Primary support modules for that contract:

- [breaking_change_analysis.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/breaking_change_analysis.rs)
- [validation_gate_status.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/validation_gate_status.rs)

Primary modules:

- [pr_auto_closes_enrichment.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/pr_auto_closes_enrichment.rs)
- [pr_directive_conflict_guard.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/pr_directive_conflict_guard.rs)
- [pr_closure_neutralizer.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/pr_closure_neutralizer.rs)
- [pr_body_contract_sync.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/pr_body_contract_sync.rs)

### Formatting and integrity

- `rustfmt-pr`
- `markdownlint-pr`
- `scripts-integrity`
- `validate-main-pr-source`
- `stable-deps-placeholder`

These commands support formatting automation, workflow integrity, and small policy gates.

Primary modules:

- [rustfmt_automation.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/rustfmt_automation.rs)
- [markdownlint_automation.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/markdownlint_automation.rs)
- [scripts_integrity.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/scripts_integrity.rs)
- [main_pr_gate.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/main_pr_gate.rs)
- [stable_deps_placeholder.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/stable_deps_placeholder.rs)

## Shared Infrastructure

### CLI routing

- [automation_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/cli/automation_cli.rs)

This file maps each subcommand name to its corresponding business module.

### GitHub and Git process helpers

- [github_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/github_cli.rs)
- [git_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/git_cli.rs)

These modules keep raw process execution and command assembly out of the workflow-domain modules.

### Argument and reference helpers

- [command_args.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/command_args.rs)
- [reference_input.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/reference_input.rs)
- [reference_input_value.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/reference_input_value.rs)
- [reference_kind.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/reference_kind.rs)
- [reference_number.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/reference_number.rs)

These modules normalize flags and reference parsing without leaking that parsing logic into every business module.
