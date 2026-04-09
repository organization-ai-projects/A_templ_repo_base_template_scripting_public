use crate::commands::{CommandArgs, GitHubCli, ReferenceKind, ReferenceNumber, ReferenceText};

pub(crate) struct IssueReopenOnDev {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl IssueReopenOnDev {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let label = self.args.extract_flag_value_or("--label", "done-in-dev");
        let pr_number = self
            .args
            .extract_reference_number("--pr", Some(ReferenceKind::PullRequest))
            .ok_or_else(|| "Invalid or missing pr.".to_string())?;
        let pr_number_text = pr_number.as_string();

        if self
            .github
            .read_pull_request_field(&repo, &pr_number_text, "baseRefName")?
            != "dev"
        {
            println!(
                "PR {} does not target dev; nothing to do.",
                pr_number.show_number()
            );
            return Ok(());
        }

        let label_exists = self.github.label_exists(&repo, &label)?;

        for issue in self.read_reopen_issue_numbers(&repo, &pr_number_text)? {
            let issue_number = issue.as_string();
            let issue_details = match self.github.read_issue_details(&repo, &issue_number)? {
                Some(details) => details,
                None => continue,
            };

            if issue_details.state == "CLOSED" {
                self.github.reopen_issue(&repo, &issue_number)?;
                println!("Issue {}: reopened.", issue.show_number());
            } else {
                println!("Issue {}: already open.", issue.show_number());
            }

            if label_exists && self.github.issue_has_label(&repo, &issue_number, &label)? {
                self.github
                    .remove_issue_label(&repo, &issue_number, &label)?;
                println!("Issue {}: removed '{}'.", issue.show_number(), label);
            }
        }

        Ok(())
    }

    fn read_reopen_issue_numbers(
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
            |line| line.to_ascii_lowercase().contains("reopen"),
        ))
    }
}
