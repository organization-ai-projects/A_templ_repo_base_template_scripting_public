use std::env;
use std::fs::OpenOptions;
use std::io::Write;

use crate::commands::{CommandArgs, GitHubCli, ReferenceInput, ReferenceKind, ReferenceNumber};

pub(crate) struct AutomationSync {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl AutomationSync {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn create_or_update_sync_branch(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let source_branch = self.args.extract_flag_value_or("--source-branch", "main");
        let sync_branch = self
            .args
            .extract_flag_value_or("--sync-branch", "sync/main-into-dev");
        let source_sha = self.read_branch_sha(&repo, &source_branch)?;

        if self.sync_branch_exists(&repo, &sync_branch)? {
            self.update_sync_branch(&repo, &sync_branch, &source_sha)?;
            println!("Updated sync branch '{sync_branch}' from '{source_branch}' ({source_sha}).");
        } else {
            self.create_sync_branch(&repo, &sync_branch, &source_sha)?;
            println!("Created sync branch '{sync_branch}' from '{source_branch}' ({source_sha}).");
        }

        Ok(())
    }

    pub(crate) fn create_sync_pull_request_if_missing(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let source_branch = self.args.extract_flag_value_or("--source-branch", "main");
        let target_branch = self.args.extract_flag_value_or("--target-branch", "dev");
        let sync_branch = self
            .args
            .extract_flag_value_or("--sync-branch", "sync/main-into-dev");

        if let Some(pr_reference) =
            self.find_existing_sync_pull_request(&repo, &sync_branch, &target_branch)?
        {
            println!(
                "Sync pull request already exists: {}",
                pr_reference.show_number()
            );
            self.write_github_env("PR_NUMBER", &pr_reference.as_string())?;
            return Ok(());
        }

        let pr_url =
            self.create_sync_pull_request(&repo, &sync_branch, &target_branch, &source_branch)?;
        let pr_reference = self.resolve_created_sync_pull_request(&repo, &sync_branch, &pr_url)?;

        println!("Created sync pull request: {pr_url}");
        self.write_github_env("PR_NUMBER", &pr_reference.as_string())?;

        Ok(())
    }

    pub(crate) fn merge_sync_pull_request(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let pr_reference = self.resolve_pr_number(&repo)?;
        let pr_number = pr_reference.as_string();

        self.add_sync_pull_request_labels(&repo, &pr_number)?;
        let (mergeable, merge_state_status) = self.read_merge_status(&repo, &pr_number)?;

        if mergeable == "MERGEABLE" && merge_state_status == "CLEAN" {
            self.merge_pull_request_now(&repo, &pr_number)?;
            println!(
                "Merged sync pull request {} immediately.",
                pr_reference.show_number()
            );
        } else {
            self.enable_pull_request_auto_merge(&repo, &pr_number)?;
            println!(
                "Enabled auto-merge for sync pull request {}.",
                pr_reference.show_number()
            );
        }

        Ok(())
    }

    fn resolve_pr_number(&self, repo: &str) -> Result<ReferenceNumber, String> {
        let reference = self
            .args
            .extract_reference_number("--pr-number", Some(ReferenceKind::PullRequest))
            .or_else(|| {
                env::var("PR_NUMBER")
                    .ok()
                    .filter(|value| !value.is_empty() && value != "null")
                    .and_then(|value| {
                        ReferenceInput::from_text(&value)
                            .with_kind(ReferenceKind::PullRequest)
                            .normalize()
                    })
            })
            .ok_or_else(|| "Missing --pr-number and PR_NUMBER".to_string())?;

        match self.github.qualify_reference(repo, reference)? {
            Some(reference) if reference.is_pull_request() => Ok(reference),
            Some(reference) if reference.is_issue() => Err(format!(
                "Reference {} resolved to an issue, not a pull request.",
                reference.show_number()
            )),
            Some(_) => Err("Reference resolved to an unsupported kind.".to_string()),
            None => Err(format!(
                "Reference {} was not found as a pull request.",
                reference.show_number()
            )),
        }
    }

    fn read_branch_sha(&self, repo: &str, branch: &str) -> Result<String, String> {
        self.github.read_branch_sha(repo, branch)
    }

    fn sync_branch_exists(&self, repo: &str, sync_branch: &str) -> Result<bool, String> {
        self.github.branch_exists(repo, sync_branch)
    }

    fn update_sync_branch(
        &self,
        repo: &str,
        sync_branch: &str,
        source_sha: &str,
    ) -> Result<(), String> {
        self.github.update_branch_ref(repo, sync_branch, source_sha)
    }

    fn create_sync_branch(
        &self,
        repo: &str,
        sync_branch: &str,
        source_sha: &str,
    ) -> Result<(), String> {
        self.github.create_branch_ref(repo, sync_branch, source_sha)
    }

    fn find_existing_sync_pull_request(
        &self,
        repo: &str,
        sync_branch: &str,
        target_branch: &str,
    ) -> Result<Option<ReferenceNumber>, String> {
        self.github
            .find_open_pull_request(repo, sync_branch, target_branch)
    }

    fn create_sync_pull_request(
        &self,
        repo: &str,
        sync_branch: &str,
        target_branch: &str,
        source_branch: &str,
    ) -> Result<String, String> {
        self.github.create_pull_request(
            repo,
            sync_branch,
            target_branch,
            &format!("Sync {source_branch} into {target_branch}"),
            &format!("Automated sync pull request from `{source_branch}` into `{target_branch}`."),
        )
    }

    fn resolve_created_sync_pull_request(
        &self,
        repo: &str,
        sync_branch: &str,
        pr_url: &str,
    ) -> Result<ReferenceNumber, String> {
        if let Some(pr_reference) =
            ReferenceInput::find_in_text(pr_url, Some(ReferenceKind::PullRequest))
                .into_iter()
                .next()
        {
            return Ok(pr_reference);
        }

        self.github
            .read_pull_request_number(repo, sync_branch)?
            .ok_or_else(|| "Failed to resolve created sync pull request number.".to_string())
    }

    fn add_sync_pull_request_labels(&self, repo: &str, pr_number: &str) -> Result<(), String> {
        self.github
            .add_pull_request_labels(repo, pr_number, &["automation", "pull-request"])
    }

    fn read_merge_status(&self, repo: &str, pr_number: &str) -> Result<(String, String), String> {
        let mergeable = self
            .github
            .read_pull_request_field(repo, pr_number, "mergeable")?;
        let merge_state_status =
            self.github
                .read_pull_request_field(repo, pr_number, "mergeStateStatus")?;

        Ok((mergeable, merge_state_status))
    }

    fn merge_pull_request_now(&self, repo: &str, pr_number: &str) -> Result<(), String> {
        self.github.merge_pull_request(repo, pr_number, false)
    }

    fn enable_pull_request_auto_merge(&self, repo: &str, pr_number: &str) -> Result<(), String> {
        self.github.merge_pull_request(repo, pr_number, true)
    }

    fn write_github_env(&self, key: &str, value: &str) -> Result<(), String> {
        match env::var("GITHUB_ENV") {
            Ok(path) => {
                let mut file = OpenOptions::new()
                    .append(true)
                    .open(&path)
                    .map_err(|error| format!("Failed to open GITHUB_ENV at '{path}': {error}"))?;
                writeln!(file, "{key}={value}")
                    .map_err(|error| format!("Failed to write {key} to GITHUB_ENV: {error}"))?;
                Ok(())
            }
            Err(_) => Ok(()),
        }
    }
}
