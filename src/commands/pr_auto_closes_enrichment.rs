use std::collections::HashSet;

use crate::commands::{CommandArgs, GitHubCli, ManagedBodyBlock, ReferenceNumber, ReferenceText};

const AUTO_CLOSES_START: &str = "<!-- auto-closes:start -->";
const AUTO_CLOSES_END: &str = "<!-- auto-closes:end -->";

pub(crate) struct PrAutoClosesEnrichment {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl PrAutoClosesEnrichment {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let pr = self.args.resolve_pull_request_number("--pr")?;
        let pr_number = pr.as_string();

        if self
            .github
            .read_pull_request_field(&repo, &pr_number, "state")?
            != "OPEN"
        {
            println!("PR {} is not open; skipping.", pr.show_number());
            return Ok(());
        }

        if self
            .github
            .read_pull_request_field(&repo, &pr_number, "baseRefName")?
            != "dev"
        {
            println!("PR {} does not target dev; skipping.", pr.show_number());
            return Ok(());
        }

        let pr_author = self
            .github
            .read_pull_request_author_login(&repo, &pr_number)?;
        if pr_author.is_empty() {
            println!(
                "PR {}: author login unavailable; skipping.",
                pr.show_number()
            );
            return Ok(());
        }

        let pr_body = self
            .github
            .read_pull_request_field(&repo, &pr_number, "body")?;
        let title = self
            .github
            .read_pull_request_field(&repo, &pr_number, "title")?;
        let commit_messages = self
            .github
            .read_pull_request_commit_messages(&repo, &pr_number)?;
        let payload = ReferenceText::build_payload(&[
            title.as_str(),
            pr_body.as_str(),
            &commit_messages.join("\n"),
        ]);

        let part_of_refs = self.extract_issue_numbers(&payload, ExtractAction::PartOf);
        let closing_refs = self.extract_issue_numbers(&payload, ExtractAction::Closes);

        if part_of_refs.is_empty() {
            println!(
                "PR {}: no Part of refs detected; nothing to enrich.",
                pr.show_number()
            );
            return Ok(());
        }

        let already_closing = closing_refs
            .into_iter()
            .map(|reference| reference.as_string())
            .collect::<HashSet<_>>();

        let mut closes_to_add = Vec::new();

        for issue in part_of_refs {
            let issue_number = issue.as_string();
            if already_closing.contains(&issue_number) {
                continue;
            }

            let assignees = self
                .github
                .read_issue_assignee_logins(&repo, &issue_number)?;
            if assignees.len() == 1 && assignees[0] == pr_author {
                closes_to_add.push(issue);
            }
        }

        if closes_to_add.is_empty() {
            println!(
                "PR {}: no qualifying single-assignee issue found; nothing to enrich.",
                pr.show_number()
            );
            return Ok(());
        }

        let new_body = self.build_updated_body(&pr_body, &closes_to_add);

        if new_body == pr_body {
            println!("PR {}: body already up-to-date.", pr.show_number());
            return Ok(());
        }

        self.github
            .update_pull_request_body(&repo, &pr_number, &new_body)?;
        println!(
            "PR {}: updated body with auto-managed Closes refs.",
            pr.show_number()
        );
        Ok(())
    }

    fn extract_issue_numbers(&self, text: &str, action: ExtractAction) -> Vec<ReferenceNumber> {
        ReferenceText::collect_issue_references(text.lines(), |line| {
            let lowered = line.to_ascii_lowercase();
            match action {
                ExtractAction::Closes => lowered.contains("closes") || lowered.contains("fixes"),
                ExtractAction::PartOf => lowered.contains("part of"),
            }
        })
    }

    fn build_updated_body(&self, pr_body: &str, closes_to_add: &[ReferenceNumber]) -> String {
        let managed_block = format!(
            "{AUTO_CLOSES_START}\n### Auto-managed Issue Closures\n{}{AUTO_CLOSES_END}",
            closes_to_add
                .iter()
                .map(|issue| format!("Closes {}\n", issue.show_number()))
                .collect::<String>()
        );

        ManagedBodyBlock::replace(pr_body, AUTO_CLOSES_START, AUTO_CLOSES_END, &managed_block)
    }
}

#[derive(Clone, Copy)]
enum ExtractAction {
    Closes,
    PartOf,
}
