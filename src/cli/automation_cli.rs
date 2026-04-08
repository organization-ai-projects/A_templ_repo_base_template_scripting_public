use std::process;

use crate::commands::{
    AutomationSync, ClosureHygiene, CommandArgs, DoneInDevStatus, IssueParentAutolink,
    IssueReopenOnDev, MainPrGate, MarkdownlintAutomation, ParentGuard, PrAutoClosesEnrichment,
    PrBodyContractSync, PrClosureNeutralizer, PrDirectiveConflictGuard, PrValidationRefresh,
    RustfmtAutomation, ScriptsIntegrity, StableDepsPlaceholder, WorkflowRunPr,
};

pub(crate) struct AutomationCli {
    automation_sync: AutomationSync,
    closure_hygiene: ClosureHygiene,
    done_in_dev_status: DoneInDevStatus,
    issue_reopen_on_dev: IssueReopenOnDev,
    issue_parent_autolink: IssueParentAutolink,
    main_pr_gate: MainPrGate,
    markdownlint_automation: MarkdownlintAutomation,
    parent_guard: ParentGuard,
    pr_auto_closes_enrichment: PrAutoClosesEnrichment,
    pr_body_contract_sync: PrBodyContractSync,
    pr_closure_neutralizer: PrClosureNeutralizer,
    pr_directive_conflict_guard: PrDirectiveConflictGuard,
    pr_validation_refresh: PrValidationRefresh,
    rustfmt_automation: RustfmtAutomation,
    scripts_integrity: ScriptsIntegrity,
    stable_deps_placeholder: StableDepsPlaceholder,
    workflow_run_pr: WorkflowRunPr,
}

impl AutomationCli {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            automation_sync: AutomationSync::new(args.clone()),
            closure_hygiene: ClosureHygiene::new(args.clone()),
            done_in_dev_status: DoneInDevStatus::new(args.clone()),
            issue_reopen_on_dev: IssueReopenOnDev::new(args.clone()),
            issue_parent_autolink: IssueParentAutolink::new(args.clone()),
            main_pr_gate: MainPrGate::new(args.clone()),
            markdownlint_automation: MarkdownlintAutomation::new(args.clone()),
            parent_guard: ParentGuard::new(args.clone()),
            pr_auto_closes_enrichment: PrAutoClosesEnrichment::new(args.clone()),
            pr_body_contract_sync: PrBodyContractSync::new(args.clone()),
            pr_closure_neutralizer: PrClosureNeutralizer::new(args.clone()),
            pr_directive_conflict_guard: PrDirectiveConflictGuard::new(args.clone()),
            pr_validation_refresh: PrValidationRefresh::new(args.clone()),
            rustfmt_automation: RustfmtAutomation::new(args.clone()),
            scripts_integrity: ScriptsIntegrity::new(args.clone()),
            stable_deps_placeholder: StableDepsPlaceholder::new(),
            workflow_run_pr: WorkflowRunPr::new(args),
        }
    }

    pub(crate) fn handle_subcommand(&self, subcommand: &str) {
        match subcommand {
            "sync-branch" => {
                if self.should_show_help() {
                    Self::print_sync_branch_help();
                    return;
                }

                if let Err(error) = self.automation_sync.create_or_update_sync_branch() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "sync-pull-request" => {
                if self.should_show_help() {
                    Self::print_sync_pull_request_help();
                    return;
                }

                if let Err(error) = self.automation_sync.create_sync_pull_request_if_missing() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "merge-sync-pull-request" => {
                if self.should_show_help() {
                    Self::print_merge_sync_pull_request_help();
                    return;
                }

                if let Err(error) = self.automation_sync.merge_sync_pull_request() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "refresh-pr-validation" => {
                if self.should_show_help() {
                    Self::print_refresh_pr_validation_help();
                    return;
                }

                if let Err(error) = self.pr_validation_refresh.refresh() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "closure-hygiene" => {
                if self.should_show_help() {
                    Self::print_closure_hygiene_help();
                    return;
                }

                if let Err(error) = self.closure_hygiene.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "parent-guard" => {
                if self.should_show_help() {
                    Self::print_parent_guard_help();
                    return;
                }

                if let Err(error) = self.parent_guard.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "done-in-dev-status" => {
                if self.should_show_help() {
                    Self::print_done_in_dev_status_help();
                    return;
                }

                if let Err(error) = self.done_in_dev_status.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "reopen-on-dev" => {
                if self.should_show_help() {
                    Self::print_reopen_on_dev_help();
                    return;
                }

                if let Err(error) = self.issue_reopen_on_dev.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "directive-conflict-guard" => {
                if self.should_show_help() {
                    Self::print_directive_conflict_guard_help();
                    return;
                }

                if let Err(error) = self.pr_directive_conflict_guard.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "auto-add-closes" => {
                if self.should_show_help() {
                    Self::print_auto_add_closes_help();
                    return;
                }

                if let Err(error) = self.pr_auto_closes_enrichment.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "issue-parent-autolink" => {
                if self.should_show_help() {
                    Self::print_issue_parent_autolink_help();
                    return;
                }

                if let Err(error) = self.issue_parent_autolink.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "closure-neutralizer" => {
                if self.should_show_help() {
                    Self::print_closure_neutralizer_help();
                    return;
                }

                if let Err(error) = self.pr_closure_neutralizer.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "closure-neutralizer-reevaluate" => {
                if self.should_show_help() {
                    Self::print_closure_neutralizer_reevaluate_help();
                    return;
                }

                if let Err(error) = self.pr_closure_neutralizer.reevaluate() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "closure-neutralizer-skip-check" => {
                if self.should_show_help() {
                    Self::print_closure_neutralizer_skip_check_help();
                    return;
                }

                if let Err(error) = self.pr_closure_neutralizer.write_skip_check_output() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "resolve-pr-body-context" => {
                if self.should_show_help() {
                    Self::print_resolve_pr_body_context_help();
                    return;
                }

                if let Err(error) = self
                    .pr_body_contract_sync
                    .resolve_context_to_github_output()
                {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "generate-pr-description" => {
                if self.should_show_help() {
                    Self::print_generate_pr_description_help();
                    return;
                }

                if let Err(error) = self.pr_body_contract_sync.generate_and_optionally_write() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "guard-pr-body-contract" => {
                if self.should_show_help() {
                    Self::print_guard_pr_body_contract_help();
                    return;
                }

                if let Err(error) = self.pr_body_contract_sync.guard_contract() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "rustfmt-pr" => {
                if self.should_show_help() {
                    Self::print_rustfmt_pr_help();
                    return;
                }

                if let Err(error) = self.rustfmt_automation.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "markdownlint-pr" => {
                if self.should_show_help() {
                    Self::print_markdownlint_pr_help();
                    return;
                }

                if let Err(error) = self.markdownlint_automation.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "scripts-integrity" => {
                if self.should_show_help() {
                    Self::print_scripts_integrity_help();
                    return;
                }

                if let Err(error) = self.scripts_integrity.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "validate-main-pr-source" => {
                if self.should_show_help() {
                    Self::print_validate_main_pr_source_help();
                    return;
                }

                if let Err(error) = self.main_pr_gate.run() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "stable-deps-placeholder" => {
                if self.should_show_help() {
                    Self::print_stable_deps_placeholder_help();
                    return;
                }

                self.stable_deps_placeholder.run();
            }
            "resolve-workflow-run-pr" => {
                if self.should_show_help() {
                    Self::print_resolve_workflow_run_pr_help();
                    return;
                }

                if let Err(error) = self.workflow_run_pr.resolve_to_github_output() {
                    eprintln!("{error}");
                    process::exit(1);
                }
            }
            "" | "help" | "--help" | "-h" => {
                println!(
                    "Usage: automation <sync-branch|sync-pull-request|merge-sync-pull-request|refresh-pr-validation|closure-hygiene|parent-guard|done-in-dev-status|reopen-on-dev|directive-conflict-guard|auto-add-closes|issue-parent-autolink|closure-neutralizer|closure-neutralizer-reevaluate|closure-neutralizer-skip-check|resolve-pr-body-context|generate-pr-description|guard-pr-body-contract|rustfmt-pr|markdownlint-pr|scripts-integrity|validate-main-pr-source|stable-deps-placeholder|resolve-workflow-run-pr> [options]"
                );
            }
            other => {
                eprintln!("Unknown automation subcommand: {other}");
                process::exit(1);
            }
        }
    }

    fn should_show_help(&self) -> bool {
        self.automation_sync.args.has_flag("--help") || self.automation_sync.args.has_flag("-h")
    }

    fn print_sync_branch_help() {
        println!(
            "Usage: automation sync-branch [--repo <owner/repo>] [--source-branch <branch>] [--sync-branch <branch>]"
        );
    }

    fn print_sync_pull_request_help() {
        println!(
            "Usage: automation sync-pull-request [--repo <owner/repo>] [--source-branch <branch>] [--target-branch <branch>] [--sync-branch <branch>]"
        );
    }

    fn print_merge_sync_pull_request_help() {
        println!(
            "Usage: automation merge-sync-pull-request [--repo <owner/repo>] [--pr-number <number>]"
        );
    }

    fn print_refresh_pr_validation_help() {
        println!("Usage: automation refresh-pr-validation [--pr <number>] [--repo <owner/repo>]");
    }

    fn print_closure_hygiene_help() {
        println!("Usage: automation closure-hygiene [--repo <owner/repo>]");
    }

    fn print_parent_guard_help() {
        println!(
            "Usage: automation parent-guard [--issue <number> | --child <number>] [--strict-guard <true|false>] [--repo <owner/repo>]"
        );
    }

    fn print_done_in_dev_status_help() {
        println!(
            "Usage: automation done-in-dev-status --mode <on-dev-merge|on-issue-closed> [--pr <number>] [--issue <number>] [--repo <owner/repo>] [--label <label>]"
        );
    }

    fn print_reopen_on_dev_help() {
        println!(
            "Usage: automation reopen-on-dev [--pr <number>] [--repo <owner/repo>] [--label <label>]"
        );
    }

    fn print_directive_conflict_guard_help() {
        println!(
            "Usage: automation directive-conflict-guard [--pr <number>] [--repo <owner/repo>]"
        );
    }

    fn print_auto_add_closes_help() {
        println!("Usage: automation auto-add-closes [--pr <number>] [--repo <owner/repo>]");
    }

    fn print_issue_parent_autolink_help() {
        println!(
            "Usage: automation issue-parent-autolink [--issue <number>] [--repo <owner/repo>]"
        );
    }

    fn print_closure_neutralizer_help() {
        println!("Usage: automation closure-neutralizer [--pr <number>] [--repo <owner/repo>]");
    }

    fn print_closure_neutralizer_reevaluate_help() {
        println!(
            "Usage: automation closure-neutralizer-reevaluate [--issue <number>] [--repo <owner/repo>]"
        );
    }

    fn print_closure_neutralizer_skip_check_help() {
        println!(
            "Usage: automation closure-neutralizer-skip-check [--previous-body <text>] [--current-body <text>]"
        );
    }

    fn print_resolve_pr_body_context_help() {
        println!(
            "Usage: automation resolve-pr-body-context [--pr <number>] [--base <branch>] [--head <branch>] [--repo <owner/repo>]"
        );
    }

    fn print_generate_pr_description_help() {
        println!(
            "Usage: automation generate-pr-description --pr <number> [--repo <owner/repo>] [--base <branch>] [--head <branch>] [--worktree <path>] [--write-pr]"
        );
    }

    fn print_guard_pr_body_contract_help() {
        println!(
            "Usage: automation guard-pr-body-contract --pr <number> [--repo <owner/repo>] [--base <branch>] [--head <branch>] [--worktree <path>]"
        );
    }

    fn print_rustfmt_pr_help() {
        println!("Usage: automation rustfmt-pr [--base-ref <branch>] [--worktree <path>]");
    }

    fn print_markdownlint_pr_help() {
        println!("Usage: automation markdownlint-pr [--base-sha <sha>] [--worktree <path>]");
    }

    fn print_scripts_integrity_help() {
        println!("Usage: automation scripts-integrity [--worktree <path>]");
    }

    fn print_validate_main_pr_source_help() {
        println!("Usage: automation validate-main-pr-source --head-ref <branch>");
    }

    fn print_stable_deps_placeholder_help() {
        println!("Usage: automation stable-deps-placeholder");
    }

    fn print_resolve_workflow_run_pr_help() {
        println!("Usage: automation resolve-workflow-run-pr [--event-path <path>]");
    }
}
