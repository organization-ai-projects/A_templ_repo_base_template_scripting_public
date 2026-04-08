use crate::commands::{CommandArgs, GitHubCli};

pub(crate) struct PrValidationRefresh {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl PrValidationRefresh {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn refresh(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let pr_number = self.args.resolve_pull_request_number("--pr")?.as_string();
        let pr_body = self
            .github
            .read_pull_request_field(&repo, &pr_number, "body")?;
        let ci_status = self.github.read_pull_request_ci_status(&repo, &pr_number)?;
        let new_body = self.update_validation_gate(&pr_body, ci_status.as_label());

        if new_body == pr_body {
            println!("PR unchanged: #{pr_number}");
            return Ok(());
        }

        self.github
            .update_pull_request_body(&repo, &pr_number, &new_body)?;
        println!("PR updated: #{pr_number}");
        Ok(())
    }
    fn update_validation_gate(&self, pr_body: &str, ci_status: &str) -> String {
        let mut lines: Vec<String> = pr_body.lines().map(ToString::to_string).collect();

        if let Some(index) = lines
            .iter()
            .position(|line| line.trim_start().starts_with("- CI:"))
        {
            lines[index] = format!("- CI: {ci_status}");
            return lines.join("\n");
        }

        let base = pr_body.trim_end_matches('\n');
        let mut new_body = String::new();

        if !base.is_empty() {
            new_body.push_str(base);
            new_body.push_str("\n\n");
        }

        new_body.push_str("### Validation Gate\n\n");
        new_body.push_str(&format!("- CI: {ci_status}\n"));
        new_body.push_str("- No breaking change");
        new_body
    }
}
