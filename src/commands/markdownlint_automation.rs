use std::path::Path;
use std::process::Command;

use crate::commands::{CommandArgs, GitCli, GitHubActions};

pub(crate) struct MarkdownlintAutomation {
    pub(crate) args: CommandArgs,
    git: GitCli,
    github_actions: GitHubActions,
}

impl MarkdownlintAutomation {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            git: GitCli,
            github_actions: GitHubActions,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let base_sha = self.args.extract_flag_value_or("--base-sha", "");
        let worktree = self.args.extract_flag_value_or("--worktree", ".");

        if base_sha.is_empty() {
            return Err("Missing --base-sha".to_string());
        }

        let modified_files = self.list_modified_markdown_files(&worktree, &base_sha)?;

        if modified_files.is_empty() {
            println!("No Markdown files modified. Skipping linting.");
            self.github_actions.write_output("has_issues", "false")?;
            self.github_actions.write_output("has_changes", "false")?;
            return Ok(());
        }

        if self.run_markdownlint(&worktree, &modified_files, false)? {
            println!("Markdown files already clean.");
            self.github_actions.write_output("has_issues", "false")?;
            self.github_actions.write_output("has_changes", "false")?;
            return Ok(());
        }

        println!("Markdown issues detected. Applying automatic fixes.");
        let _ = self.run_markdownlint(&worktree, &modified_files, true)?;
        let has_changes = self.has_diff(&worktree)?;

        self.github_actions.write_output("has_issues", "true")?;
        self.github_actions
            .write_output("has_changes", if has_changes { "true" } else { "false" })?;
        Ok(())
    }

    fn list_modified_markdown_files(
        &self,
        worktree: &str,
        base_sha: &str,
    ) -> Result<Vec<String>, String> {
        let range = format!("{base_sha}..HEAD");
        Ok(self
            .git
            .diff_name_only_with_filter(worktree, &range, "ACMRT")?
            .into_iter()
            .filter(|file| file.ends_with(".md"))
            .filter(|file| Path::new(worktree).join(file).is_file())
            .collect())
    }

    fn run_markdownlint(
        &self,
        worktree: &str,
        files: &[String],
        fix: bool,
    ) -> Result<bool, String> {
        let mut args = vec!["dlx", "markdownlint-cli2"];
        if fix {
            args.push("--fix");
        }
        for file in files {
            args.push(file);
        }

        let output = Command::new("pnpm")
            .args(args)
            .current_dir(worktree)
            .output()
            .map_err(|error| format!("Failed to execute markdownlint-cli2 via pnpm: {error}"))?;

        Ok(output.status.success())
    }

    fn has_diff(&self, worktree: &str) -> Result<bool, String> {
        self.git.has_diff(worktree)
    }
}
