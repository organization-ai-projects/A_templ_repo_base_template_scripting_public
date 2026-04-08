use crate::commands::breaking_change_analysis::BreakingChangeAnalysis;
use crate::commands::validation_gate_status::ValidationGateState;
use crate::commands::{CommandArgs, GitCli, GitHubCli};

pub(crate) struct PrValidationRefresh {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
    git: GitCli,
}

impl PrValidationRefresh {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
            git: GitCli,
        }
    }

    pub(crate) fn refresh(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let pr_number = self.args.resolve_pull_request_number("--pr")?.as_string();
        let base_ref = self
            .github
            .read_pull_request_field(&repo, &pr_number, "baseRefName")?;
        let head_ref = self
            .github
            .read_pull_request_field(&repo, &pr_number, "headRefName")?;
        let worktree = self.args.extract_flag_value_or("--worktree", ".");

        let _ = self.git.fetch_branches(&worktree, &base_ref, &head_ref);
        let pr_body = self
            .github
            .read_pull_request_field(&repo, &pr_number, "body")?;
        let breaking_change =
            BreakingChangeAnalysis::analyze(&self.git, &worktree, &base_ref, &head_ref)?;
        let new_body = ValidationGateState::new(breaking_change).replace_in_body(&pr_body);

        if new_body == pr_body {
            println!("PR unchanged: #{pr_number}");
            return Ok(());
        }

        self.github
            .update_pull_request_body(&repo, &pr_number, &new_body)?;
        println!("PR updated: #{pr_number}");
        Ok(())
    }
}
