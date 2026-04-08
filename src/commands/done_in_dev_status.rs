use crate::commands::{CommandArgs, GitHubCli, ReferenceNumber, ReferenceText};

pub(crate) struct DoneInDevStatus {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl DoneInDevStatus {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let label = self.args.extract_flag_value_or("--label", "done-in-dev");

        if !self.github.label_exists(&repo, &label)? {
            println!("Label '{label}' does not exist in {repo}; skipping.");
            return Ok(());
        }

        match self.resolve_mode()?.as_str() {
            "on-dev-merge" => self.on_dev_merge(&repo, &label),
            "on-issue-closed" => self.on_issue_closed(&repo, &label),
            other => Err(format!(
                "Unsupported --mode '{other}'. Expected on-dev-merge or on-issue-closed."
            )),
        }
    }

    fn on_dev_merge(&self, repo: &str, label: &str) -> Result<(), String> {
        let pr_number = self.args.resolve_pull_request_number("--pr")?;
        let pr_number_text = pr_number.as_string();

        if self
            .github
            .read_pull_request_field(repo, &pr_number_text, "state")?
            != "MERGED"
        {
            println!(
                "PR {} is not merged; nothing to do.",
                pr_number.show_number()
            );
            return Ok(());
        }

        for issue in self.read_closes_issue_numbers(repo, &pr_number_text)? {
            let issue_number = issue.as_string();
            let issue_details = match self.github.read_issue_details(repo, &issue_number)? {
                Some(details) => details,
                None => continue,
            };

            if issue_details.state != "OPEN" {
                println!("Issue {} is not open; skipping.", issue.show_number());
                continue;
            }

            if self.github.issue_has_label(repo, &issue_number, label)? {
                println!("Issue {} already has '{}'.", issue.show_number(), label);
                continue;
            }

            self.github.add_issue_label(repo, &issue_number, label)?;
            println!("Issue {}: added '{}'.", issue.show_number(), label);
        }

        Ok(())
    }

    fn on_issue_closed(&self, repo: &str, label: &str) -> Result<(), String> {
        let issue = self.args.resolve_issue_number("--issue")?;
        let issue_number = issue.as_string();

        let issue_details = self
            .github
            .read_issue_details(repo, &issue_number)?
            .ok_or_else(|| format!("Issue {} was not found.", issue.show_number()))?;

        if issue_details.state != "CLOSED" {
            println!(
                "Issue {} is not closed; nothing to do.",
                issue.show_number()
            );
            return Ok(());
        }

        if self.github.issue_has_label(repo, &issue_number, label)? {
            self.github.remove_issue_label(repo, &issue_number, label)?;
            println!("Issue {}: removed '{}'.", issue.show_number(), label);
        } else {
            println!("Issue {} has no '{}' label.", issue.show_number(), label);
        }

        Ok(())
    }

    fn resolve_mode(&self) -> Result<String, String> {
        self.args
            .extract_flag_value("--mode")
            .map_err(|error| error.to_string())
    }

    fn read_closes_issue_numbers(
        &self,
        repo: &str,
        pr_number: &str,
    ) -> Result<Vec<ReferenceNumber>, String> {
        let title = self
            .github
            .read_pull_request_field(repo, pr_number, "title")
            .unwrap_or_default();
        let body = self
            .github
            .read_pull_request_field(repo, pr_number, "body")
            .unwrap_or_default();
        let commit_messages = self
            .github
            .read_pull_request_commit_messages(repo, pr_number)?;

        Ok(ReferenceText::collect_issue_references(
            title
                .lines()
                .chain(body.lines())
                .chain(commit_messages.iter().map(String::as_str)),
            |line| {
                let lowered = line.to_ascii_lowercase();
                lowered.contains("closes") || lowered.contains("fixes")
            },
        ))
    }
}
