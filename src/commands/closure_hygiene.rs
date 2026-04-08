use crate::commands::{CommandArgs, GitHubCli, ParentGuard, ReferenceKind};

pub(crate) struct ClosureHygiene {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
    parent_guard: ParentGuard,
}

impl ClosureHygiene {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            github: GitHubCli,
            parent_guard: ParentGuard::new(args.clone()),
            args,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;

        for issue in self.github.list_open_issue_numbers(&repo)? {
            self.parent_guard.evaluate_parent_issue(
                &repo,
                &issue.with_kind(ReferenceKind::Issue),
                false,
            )?;
        }

        for milestone in self.github.list_open_milestones(&repo)? {
            if milestone.open_issues != 0 {
                continue;
            }

            self.github.close_milestone(&repo, milestone.number)?;
            println!(
                "Closed milestone #{} ({}).",
                milestone.number, milestone.title
            );
        }

        println!("Closure hygiene completed.");
        Ok(())
    }
}
