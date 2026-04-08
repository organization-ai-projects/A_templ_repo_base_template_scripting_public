use crate::commands::{CommandArgs, GitCli, GitHubActions};

pub(crate) struct RustfmtAutomation {
    pub(crate) args: CommandArgs,
    git: GitCli,
    github_actions: GitHubActions,
}

impl RustfmtAutomation {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            git: GitCli,
            github_actions: GitHubActions,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let base_ref = self.args.extract_flag_value_or("--base-ref", "dev");
        let worktree = self.args.extract_flag_value_or("--worktree", ".");
        let range = format!("origin/{base_ref}..HEAD");
        let modified_files = self
            .git
            .diff_name_only(&worktree, &range)?
            .into_iter()
            .filter(|file| file.ends_with(".rs"))
            .collect::<Vec<_>>();

        if modified_files.is_empty() {
            println!("No Rust files modified. Skipping formatting.");
            self.github_actions
                .write_output("needs_formatting", "false")?;
            self.github_actions.write_output("has_changes", "false")?;
            return Ok(());
        }

        if self.git.rustfmt_files(&worktree, &modified_files, true)? {
            println!("Rust files already formatted.");
            self.github_actions
                .write_output("needs_formatting", "false")?;
            self.github_actions.write_output("has_changes", "false")?;
            return Ok(());
        }

        self.git.rustfmt_files(&worktree, &modified_files, false)?;
        let has_changes = self.git.has_diff(&worktree)?;
        self.github_actions
            .write_output("needs_formatting", "true")?;
        self.github_actions
            .write_output("has_changes", if has_changes { "true" } else { "false" })?;
        Ok(())
    }
}
