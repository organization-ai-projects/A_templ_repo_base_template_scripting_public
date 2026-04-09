use std::collections::{HashMap, HashSet};

use crate::commands::breaking_change_analysis::BreakingChangeAnalysis;
use crate::commands::github_cli::PullRequestCommit;
use crate::commands::validation_gate_status::ValidationGateState;
use crate::commands::{CommandArgs, GitCli, GitHubActions, GitHubCli, PrDirective, ReferenceKind};
use crate::regex::ISSUE_DIRECTIVE_EVENT_REGEX;

pub(crate) struct PrBodyContractSync {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
    git: GitCli,
    github_actions: GitHubActions,
}

impl PrBodyContractSync {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
            git: GitCli,
            github_actions: GitHubActions,
        }
    }

    pub(crate) fn resolve_context_to_github_output(&self) -> Result<(), String> {
        let pr = match self
            .args
            .extract_reference_number("--pr", Some(ReferenceKind::PullRequest))
        {
            Some(pr) => pr,
            None => {
                println!("No pull request linked to this event; skipping.");
                self.github_actions.write_output("skip", "true")?;
                return Ok(());
            }
        };

        let repo = self.args.resolve_repo()?;
        let pr_number = pr.as_string();
        let mut base_ref = self.args.extract_flag_value_or("--base", "");
        let mut head_ref = self.args.extract_flag_value_or("--head", "");

        if base_ref.is_empty() {
            base_ref = self
                .github
                .read_pull_request_field(&repo, &pr_number, "baseRefName")?;
        }
        if head_ref.is_empty() {
            head_ref = self
                .github
                .read_pull_request_field(&repo, &pr_number, "headRefName")?;
        }

        if base_ref.is_empty() || head_ref.is_empty() {
            return Err(format!(
                "Unable to resolve base/head refs for PR {}.",
                pr.show_number()
            ));
        }

        self.github_actions.write_output("pr_number", &pr_number)?;
        self.github_actions.write_output("base_ref", &base_ref)?;
        self.github_actions.write_output("head_ref", &head_ref)?;
        self.github_actions.write_output("skip", "false")?;
        Ok(())
    }

    pub(crate) fn generate_and_optionally_write(&self) -> Result<(), String> {
        let context = self.resolve_generation_context()?;
        let generated_body = self.generate_body(&context)?;

        if self.args.has_flag("--write-pr") {
            let current_body =
                self.github
                    .read_pull_request_field(&context.repo, &context.pr_number, "body")?;

            if generated_body != current_body {
                self.github.update_pull_request_body(
                    &context.repo,
                    &context.pr_number,
                    &generated_body,
                )?;
                println!("Updated PR body: #{}", context.pr_number);
            } else {
                println!("PR body already up-to-date: #{}", context.pr_number);
            }
        } else {
            println!("{generated_body}");
        }

        Ok(())
    }

    pub(crate) fn generate_and_write_for(
        &self,
        repo: &str,
        pr_number: &str,
        base_ref: &str,
        head_ref: &str,
        worktree: &str,
    ) -> Result<(), String> {
        let context = GenerationContext {
            repo: repo.to_string(),
            pr_number: pr_number.to_string(),
            base_ref: base_ref.to_string(),
            head_ref: head_ref.to_string(),
            worktree: worktree.to_string(),
        };
        let generated_body = self.generate_body(&context)?;

        self.github
            .update_pull_request_body(repo, pr_number, &generated_body)?;

        Ok(())
    }

    pub(crate) fn guard_contract(&self) -> Result<(), String> {
        let context = self.resolve_generation_context()?;
        let expected = self.generate_body(&context)?;
        let current =
            self.github
                .read_pull_request_field(&context.repo, &context.pr_number, "body")?;

        if expected != current {
            return Err("PR body contract mismatch after auto-sync.".to_string());
        }

        Ok(())
    }

    fn resolve_generation_context(&self) -> Result<GenerationContext, String> {
        let repo = self.args.resolve_repo()?;
        let pr = self
            .args
            .extract_reference_number("--pr", Some(ReferenceKind::PullRequest))
            .ok_or_else(|| "Invalid or missing pr.".to_string())?;
        let pr_number = pr.as_string();
        let mut base_ref = self.args.extract_flag_value_or("--base", "");
        let mut head_ref = self.args.extract_flag_value_or("--head", "");
        let worktree = self.args.extract_flag_value_or("--worktree", ".");

        if base_ref.is_empty() {
            base_ref = self
                .github
                .read_pull_request_field(&repo, &pr_number, "baseRefName")?;
        }
        if head_ref.is_empty() {
            head_ref = self
                .github
                .read_pull_request_field(&repo, &pr_number, "headRefName")?;
        }

        if base_ref.is_empty() || head_ref.is_empty() {
            return Err(format!(
                "Unable to resolve base/head refs for PR #{}.",
                pr_number
            ));
        }

        Ok(GenerationContext {
            repo,
            pr_number,
            base_ref,
            head_ref,
            worktree,
        })
    }

    fn generate_body(&self, context: &GenerationContext) -> Result<String, String> {
        let _ = self
            .git
            .fetch_branches(&context.worktree, &context.base_ref, &context.head_ref);

        let pr_body =
            self.github
                .read_pull_request_field(&context.repo, &context.pr_number, "body")?;
        let pr_title =
            self.github
                .read_pull_request_field(&context.repo, &context.pr_number, "title")?;
        let commit_entries = self
            .github
            .read_pull_request_commits(&context.repo, &context.pr_number)?;
        let commit_messages = commit_entries
            .iter()
            .map(|commit| commit.message.clone())
            .collect::<Vec<_>>();
        let payload = format!("{}\n{}\n{}", pr_title, pr_body, commit_messages.join("\n"));

        let breaking_change = BreakingChangeAnalysis::analyze(
            &self.git,
            &context.worktree,
            &context.base_ref,
            &context.head_ref,
        )?;
        let issue_outcomes = self.build_issue_outcomes_section(&payload)?;
        let key_changes = self.build_key_changes_section(&commit_entries);
        let change_footprint = self.build_change_footprint_section(context)?;
        let validation_gate = ValidationGateState::new(breaking_change).render_section();

        Ok(format!(
            "### Description\n\nThis pull request merges the `{}` branch into `{}` and summarizes merged pull requests and resolved issues.\n\n{}\n\n### Issue Outcomes\n\n{}\n\n### Key Changes\n\n{}\n\n#### Change Footprint\n\n{}",
            context.head_ref,
            context.base_ref,
            validation_gate,
            issue_outcomes,
            key_changes,
            change_footprint
        ))
    }

    fn build_issue_outcomes_section(&self, payload: &str) -> Result<String, String> {
        let parsed = PrDirective::parse(payload)?;
        let explicit = parsed.explicit;
        let closing = parsed.closing;
        let reopening = parsed.reopening;

        let mut conflicts = closing
            .intersection(&reopening)
            .cloned()
            .collect::<Vec<_>>();
        conflicts.sort_by_key(|issue| sort_issue_number(issue));

        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();

        for issue in conflicts {
            if let Some(decision) = explicit.get(&issue) {
                resolved.push((issue, decision.clone()));
            } else {
                unresolved.push((
                    issue,
                    "Closes + Reopen detected without explicit decision.".to_string(),
                ));
            }
        }

        let mut close_only = closing.difference(&reopening).cloned().collect::<Vec<_>>();
        close_only.sort_by_key(|issue| sort_issue_number(issue));
        let mut reopen_only = reopening.difference(&closing).cloned().collect::<Vec<_>>();
        reopen_only.sort_by_key(|issue| sort_issue_number(issue));

        let mut lines = vec![
            "#### Category 1: Issues Without Conflicts".to_string(),
            String::new(),
            "##### Closes/Fixes".to_string(),
            String::new(),
        ];

        if close_only.is_empty() {
            lines.push(
                "- No resolved issues detected via GitHub references or PR body keywords."
                    .to_string(),
            );
        } else {
            for issue in close_only {
                lines.push(format!("- Closes {issue}"));
            }
        }

        lines.push(String::new());
        lines.push("##### Reopened".to_string());
        lines.push(String::new());

        if reopen_only.is_empty() {
            lines.push("- No reopened issues detected.".to_string());
        } else {
            for issue in reopen_only {
                lines.push(format!("- Reopen {issue}"));
            }
        }

        lines.push(String::new());
        lines.push("#### Category 2: Issues With Conflicts".to_string());
        lines.push(String::new());
        lines.push("##### Auto-resolved".to_string());
        lines.push(String::new());

        if resolved.is_empty() {
            lines.push("- No auto-resolved directive conflicts.".to_string());
        } else {
            for (issue, decision) in resolved {
                let action = if decision == "close" {
                    "Closes"
                } else {
                    "Reopen"
                };
                lines.push(format!(
                    "- {action} {issue} - Resolved via directive decision => {decision}."
                ));
            }
        }

        lines.push(String::new());
        lines.push("##### Not resolved".to_string());
        lines.push(String::new());

        if unresolved.is_empty() {
            lines.push("- No unresolved directive conflicts.".to_string());
        } else {
            for (issue, reason) in unresolved {
                lines.push(format!("- {issue}: {reason}"));
            }
        }

        Ok(lines.join("\n"))
    }

    fn build_key_changes_section(&self, commits: &[PullRequestCommit]) -> String {
        let mut groups: HashMap<&str, Vec<String>> = HashMap::from([
            ("Synchronization", Vec::new()),
            ("Features", Vec::new()),
            ("Bug Fixes", Vec::new()),
            ("Refactoring", Vec::new()),
            ("Other", Vec::new()),
        ]);
        let mut seen_shas = HashSet::new();

        for commit in commits {
            if !seen_shas.insert(commit.sha.as_str()) {
                continue;
            }

            let line = commit_subject(&commit.message);
            if line.is_empty() || line.starts_with("Merge pull request #") {
                continue;
            }
            if issue_directive_only_line(line) {
                continue;
            }

            let lower = line.to_ascii_lowercase();
            let bucket = if lower.starts_with("sync")
                || lower.contains("merge main into dev")
                || lower.contains("merge dev into main")
            {
                "Synchronization"
            } else if lower.starts_with("feat") || lower.starts_with("feature") {
                "Features"
            } else if lower.starts_with("fix")
                || lower.starts_with("bugfix")
                || lower.starts_with("hotfix")
            {
                "Bug Fixes"
            } else if lower.starts_with("refactor")
                || lower.starts_with("chore")
                || lower.starts_with("cleanup")
            {
                "Refactoring"
            } else {
                "Other"
            };

            groups.entry(bucket).or_default().push(line.to_string());
        }

        let mut parts = Vec::new();
        for name in [
            "Synchronization",
            "Features",
            "Bug Fixes",
            "Refactoring",
            "Other",
        ] {
            let items = groups.get(name).cloned().unwrap_or_default();
            if items.is_empty() {
                continue;
            }
            parts.push(format!("#### {name}"));
            parts.push(String::new());
            for item in items {
                parts.push(format!("- {item}"));
            }
            parts.push(String::new());
        }

        if parts.is_empty() {
            "- No significant items detected.".to_string()
        } else {
            parts.join("\n").trim_end().to_string()
        }
    }

    fn build_change_footprint_section(
        &self,
        context: &GenerationContext,
    ) -> Result<String, String> {
        let range = format!("origin/{}..origin/{}", context.base_ref, context.head_ref);
        let files = self.git.diff_name_only(&context.worktree, &range)?;

        if files.is_empty() {
            return Ok("- No changed files detected for this branch range.".to_string());
        }

        let mut groups: HashMap<&str, Vec<String>> = HashMap::from([
            ("Documentation", Vec::new()),
            ("Shell", Vec::new()),
            ("Crates", Vec::new()),
            ("Workspace", Vec::new()),
            ("Other", Vec::new()),
        ]);

        for file in files {
            let bucket = if file.ends_with(".md")
                || file.starts_with("documentation/")
                || file.starts_with(".github/documentation/")
            {
                "Documentation"
            } else if file.ends_with(".sh") || file.starts_with("scripts/") {
                "Shell"
            } else if file == "Cargo.toml"
                || file == "Cargo.lock"
                || file.starts_with(".cargo/")
                || file.ends_with("/Cargo.toml")
                || file.ends_with("/Cargo.lock")
                || file.starts_with("rust-toolchain")
            {
                "Workspace"
            } else if file.ends_with(".rs") || file.contains("/src/") {
                "Crates"
            } else {
                "Other"
            };
            groups.entry(bucket).or_default().push(file);
        }

        let mut lines = Vec::new();
        for label in ["Documentation", "Shell", "Crates", "Workspace", "Other"] {
            let items = groups.get(label).cloned().unwrap_or_default();
            if items.is_empty() {
                continue;
            }
            lines.push(format!("- {label} ({})", items.len()));
            for item in items.into_iter().take(12) {
                lines.push(format!("  - {item}"));
            }
        }

        Ok(lines.join("\n"))
    }
}

struct GenerationContext {
    repo: String,
    pr_number: String,
    base_ref: String,
    head_ref: String,
    worktree: String,
}

fn sort_issue_number(issue: &str) -> u64 {
    issue.trim_start_matches('#').parse::<u64>().unwrap_or(0)
}

fn issue_directive_only_line(line: &str) -> bool {
    let Ok(regex) = &*ISSUE_DIRECTIVE_EVENT_REGEX else {
        return false;
    };

    regex
        .find(line)
        .is_some_and(|matched| matched.start() == 0 && matched.end() == line.len())
}

fn commit_subject(message: &str) -> &str {
    message
        .split('\n')
        .next()
        .unwrap_or(message)
        .split("\\n")
        .next()
        .unwrap_or(message)
        .trim()
}
