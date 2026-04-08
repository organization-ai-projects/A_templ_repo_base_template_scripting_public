# Workflows Overview

## Purpose

This page summarizes how workflow templates in this repository are structured and how they execute automation logic.

Detailed workflow-by-workflow documentation lives under:

- [repository_elements/.github/workflows/documentation/README.md](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/repository_elements/.github/workflows/documentation/README.md)

## High-Level Model

The reusable workflows under [repository_elements/.github/workflows](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/repository_elements/.github/workflows) follow a shared pattern:

1. trigger from a repository event
2. generate a GitHub App token when GitHub write access is needed
3. optionally check out the current target repository
4. check out the central automation repository into `.automation`
5. run one or more Rust subcommands via `cargo run -- automation ...`

## Why This Structure Exists

This split solves two competing needs:

- each generated repository still owns and runs its own workflows
- the implementation logic is not duplicated across every repository

The result is:

- thin workflow templates
- centralized Rust business logic
- easier incremental migration away from shell-heavy automation

## Workflow Categories

### CI workflows

- branch CI for `dev` and `main`
- dependency scanning
- workflow integrity checks

### Repository automation workflows

- branch sync
- formatting automation
- PR body management
- issue and PR guardrails
- closure and reopen orchestration
- validation status refresh

## Related Docs

- [architecture.md](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/documentation/architecture.md)
- [repository_elements/.github/workflows/documentation/TOC.md](/home/bezotremi/Projects/rust/organization-ai-projects/bezotem_platform/templates/repo_base_template_scripting/public/repository_elements/.github/workflows/documentation/TOC.md)
