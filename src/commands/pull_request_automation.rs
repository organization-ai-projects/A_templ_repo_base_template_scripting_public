use crate::commands::{CommandArgs, GitHubCli, PrBodyContractSync};

pub(crate) struct PullRequestAutomation {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
    pr_body_contract_sync: PrBodyContractSync,
}

impl PullRequestAutomation {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args: args.clone(),
            github: GitHubCli,
            pr_body_contract_sync: PrBodyContractSync::new(args),
        }
    }

    pub(crate) fn create_or_update_pull_request(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let base_ref = self
            .args
            .extract_flag_value("--base")
            .map_err(|_| "Missing required flag --base".to_string())?;
        let head_ref = self
            .args
            .extract_flag_value("--head")
            .map_err(|_| "Missing required flag --head".to_string())?;
        let title = self
            .args
            .extract_flag_value("--title")
            .map_err(|_| "Missing required flag --title".to_string())?;
        let worktree = self.args.extract_flag_value_or("--worktree", ".");

        let pr_number = if let Some(existing_pr) = self
            .github
            .find_open_pull_request(&repo, &head_ref, &base_ref)?
        {
            println!("Pull request already exists: {}", existing_pr.show_number());
            existing_pr.as_string()
        } else {
            self.github.create_pull_request(
                &repo,
                &head_ref,
                &base_ref,
                &title,
                "Preparing pull request body through automation.",
            )?;

            self.github
                .find_open_pull_request(&repo, &head_ref, &base_ref)?
                .ok_or_else(|| "Failed to resolve the created pull request.".to_string())?
                .as_string()
        };

        self.github
            .add_pull_request_labels(&repo, &pr_number, &["pull-request"])?;

        self.pr_body_contract_sync
            .generate_and_write_for(&repo, &pr_number, &base_ref, &head_ref, &worktree)?;

        println!("Pull request body synchronized for #{}.", pr_number);

        Ok(())
    }
}
