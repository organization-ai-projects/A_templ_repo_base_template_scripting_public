# Architecture

## Purpose

This template repository centralizes automation logic that is meant to be reused by many generated repositories.

The repository has two distinct layers:

1. **Template layer**
   - Stored under [repository_elements](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/repository_elements)
   - Contains the workflow files and repository content that will be copied into downstream repositories

2. **Automation implementation layer**
   - Stored under [src](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src)
   - Contains the Rust binary executed by those workflows

## Execution Model

Downstream repositories do not embed this Rust code directly.

Instead, the copied workflows usually:

1. generate a GitHub App token
2. check out the current target repository when needed
3. check out this central automation repository into `.automation`
4. run `cargo run -- automation ...` from `.automation`

This keeps workflow logic distributed while keeping implementation centralized.

## Main Areas

### Workflow templates

- [repository_elements/.github/workflows](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/repository_elements/.github/workflows)
- Holds the reusable YAML workflows copied into target repositories

### Command-line entrypoint

- [main.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/main.rs)
- [automation_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/cli/automation_cli.rs)

The binary exposes one top-level command:

- `automation`

The `automation` command then routes to focused subcommands such as:

- sync operations
- PR validation refresh
- closure hygiene
- markdown/rustfmt automation
- issue and PR guardrails

### Infrastructure helpers

- [github_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/github_cli.rs)
- [git_cli.rs](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands/git_cli.rs)

These modules keep process execution and GitHub/Git command composition out of the business modules.

### Business modules

Each automation concern is isolated in its own command file under [src/commands](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/src/commands), for example:

- sync orchestration
- PR body contract generation
- parent-child issue guards
- directive conflict handling
- closure neutralization

## Boundary Rules

- Workflow YAML should prefer native workflow configuration and `cargo run -- automation ...`
- Business logic should live in dedicated Rust command modules
- Generic GitHub process handling should stay in `github_cli.rs`
- Generic Git process handling should stay in `git_cli.rs`
- Template docs belong under `repository_elements/.github/workflows/documentation`
- Root architecture docs belong under `documentation/`
