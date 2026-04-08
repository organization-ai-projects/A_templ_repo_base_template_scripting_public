use std::collections::{HashMap, HashSet};

use regex::Regex;

use crate::commands::{CommandArgs, GitHubCli, ManagedBodyBlock, PrDirective, ReferenceText};

const START_MARKER: &str = "<!-- directive-conflicts:start -->";
const END_MARKER: &str = "<!-- directive-conflicts:end -->";

pub(crate) struct PrDirectiveConflictGuard {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

#[derive(Clone)]
struct DirectiveResolution {
    issue: String,
    decision: String,
    origin: String,
}

#[derive(Clone)]
struct DirectiveConflict {
    issue: String,
    reason: String,
}

struct DirectiveReport {
    resolved: Vec<DirectiveResolution>,
    unresolved: Vec<DirectiveConflict>,
}

impl PrDirectiveConflictGuard {
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
        let pr_body = self
            .github
            .read_pull_request_field(&repo, &pr_number, "body")?;
        let commit_messages = self
            .github
            .read_pull_request_commit_messages(&repo, &pr_number)?;
        let payload =
            ReferenceText::build_payload(&[pr_body.as_str(), &commit_messages.join("\n")]);
        let source_branch_count = self.count_source_branches(&commit_messages);
        let report = self.build_report(&payload, source_branch_count);
        let updated_body = self.update_pr_body(&pr_body, &report);

        if updated_body != pr_body {
            self.github
                .update_pull_request_body(&repo, &pr_number, &updated_body)?;
        }

        let marker = format!("<!-- directive-conflict-guard:{pr_number} -->");

        if !report.unresolved.is_empty() {
            self.github.upsert_issue_comment_by_marker(
                &repo,
                &pr_number,
                &marker,
                &format!(
                    "{marker}\n### Directive Conflict Guard\n\n❌ Unresolved Closes/Reopen conflicts detected. Add explicit directive decisions in PR body."
                ),
            )?;
            return Err(format!(
                "Unresolved directive conflicts detected for PR {}.",
                pr.show_number()
            ));
        }

        if !report.resolved.is_empty() {
            self.github.upsert_issue_comment_by_marker(
                &repo,
                &pr_number,
                &marker,
                &format!(
                    "{marker}\n### Directive Conflict Guard\n\n✅ Directive conflicts resolved via explicit decisions."
                ),
            )?;
        }

        println!(
            "Directive conflict guard evaluated for PR {}.",
            pr.show_number()
        );
        Ok(())
    }

    fn count_source_branches(&self, commit_messages: &[String]) -> usize {
        let regex = Regex::new(r"(?m)^Merge pull request #[0-9]+ from [^/]+/(.+)$")
            .expect("source branch regex must compile");
        let text = commit_messages.join("\n");
        let mut branches = HashSet::new();

        for capture in regex.captures_iter(&text) {
            let value = capture.get(1).map(|m| m.as_str().trim()).unwrap_or("");
            if !value.is_empty() {
                branches.insert(value.to_string());
            }
        }

        if branches.is_empty() {
            1
        } else {
            branches.len()
        }
    }

    fn build_report(&self, payload: &str, source_branch_count: usize) -> DirectiveReport {
        let parsed = PrDirective::parse(payload).expect("PR directive regexes must compile");
        let mut closing_requested = parsed.closing;
        let reopen_requested = parsed.reopening;
        let explicit_decision = parsed.explicit;
        let mut inferred_decision = HashMap::new();
        let event_re = crate::regex::ISSUE_DIRECTIVE_EVENT_REGEX
            .as_ref()
            .expect("event regex must compile");

        for capture in event_re.captures_iter(payload) {
            let action = capture
                .get(1)
                .map(|m| m.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let issue = capture.get(3).map(|m| m.as_str()).unwrap_or("").to_string();

            if issue.is_empty() {
                continue;
            }

            match action.as_str() {
                "closes" | "fixes" => {
                    inferred_decision.insert(issue, "close".to_string());
                }
                "reopen" | "reopens" => {
                    inferred_decision.insert(issue, "reopen".to_string());
                }
                "cancel-closes" => {
                    closing_requested.remove(&issue);
                    if inferred_decision.get(&issue).map(String::as_str) == Some("close") {
                        inferred_decision.remove(&issue);
                    }
                }
                _ => {}
            }
        }

        let allow_inferred = source_branch_count <= 1;
        let mut conflicting = closing_requested
            .intersection(&reopen_requested)
            .cloned()
            .collect::<Vec<_>>();
        conflicting.sort_by_key(|issue| issue.trim_start_matches('#').parse::<u64>().unwrap_or(0));

        let mut resolved = Vec::new();
        let mut unresolved = Vec::new();

        for issue in conflicting {
            if let Some(decision) = explicit_decision.get(&issue) {
                resolved.push(DirectiveResolution {
                    issue,
                    decision: decision.clone(),
                    origin: "explicit".to_string(),
                });
            } else if allow_inferred {
                if let Some(decision) = inferred_decision.get(&issue) {
                    resolved.push(DirectiveResolution {
                        issue,
                        decision: decision.clone(),
                        origin: "inferred from latest directive".to_string(),
                    });
                }
            } else {
                unresolved.push(DirectiveConflict {
                    issue,
                    reason: "Closes + Reopen detected across multiple source branches; explicit decision required.".to_string(),
                });
            }
        }

        if allow_inferred {
            for issue in closing_requested.intersection(&reopen_requested) {
                let already_recorded = resolved.iter().any(|entry| &entry.issue == issue)
                    || unresolved.iter().any(|entry| &entry.issue == issue);
                if !already_recorded {
                    unresolved.push(DirectiveConflict {
                        issue: issue.clone(),
                        reason: "Closes + Reopen detected without explicit decision.".to_string(),
                    });
                }
            }
        }

        DirectiveReport {
            resolved,
            unresolved,
        }
    }

    fn update_pr_body(&self, pr_body: &str, report: &DirectiveReport) -> String {
        let mut updated_body = pr_body.to_string();

        for issue in report
            .resolved
            .iter()
            .filter(|entry| entry.decision == "close")
            .map(|entry| entry.issue.as_str())
        {
            updated_body = reject_reopen_issue(&updated_body, issue);
        }

        let clean_body = remove_conflict_block(&updated_body);
        let conflict_block = build_conflict_block(report);

        if conflict_block.is_empty() {
            clean_body
        } else {
            format!("{clean_body}\n\n{conflict_block}\n")
        }
    }
}

fn reject_reopen_issue(body: &str, issue: &str) -> String {
    let regex = Regex::new(&format!(
        r"(?i)\b(reopen|reopens)\b(\s+)(rejected\s+)?([^\s]*{})\b",
        regex::escape(issue)
    ))
    .expect("reopen reject regex must compile");

    regex
        .replace_all(body, |captures: &regex::Captures<'_>| {
            let keyword = captures.get(1).map(|m| m.as_str()).unwrap_or("reopen");
            let whitespace = captures.get(2).map(|m| m.as_str()).unwrap_or(" ");
            let reference = captures.get(4).map(|m| m.as_str()).unwrap_or(issue);
            format!("{keyword}{whitespace}rejected {reference}")
        })
        .to_string()
}

fn remove_conflict_block(body: &str) -> String {
    ManagedBodyBlock::remove(body, START_MARKER, END_MARKER)
}

fn build_conflict_block(report: &DirectiveReport) -> String {
    if report.resolved.is_empty() && report.unresolved.is_empty() {
        return String::new();
    }

    let mut lines = vec![
        START_MARKER.to_string(),
        "### Issue Directive Decisions".to_string(),
    ];

    if !report.resolved.is_empty() {
        lines.push(String::new());
        lines.push("Resolved decisions:".to_string());
        for entry in &report.resolved {
            lines.push(format!(
                "- {} => {} ({})",
                entry.issue, entry.decision, entry.origin
            ));
        }
    }

    if !report.unresolved.is_empty() {
        lines.push(String::new());
        lines.push("❌ Unresolved conflicts (merge blocked):".to_string());
        for entry in &report.unresolved {
            lines.push(format!("- {}: {}", entry.issue, entry.reason));
        }
        lines.push(String::new());
        lines.push("Required decision format:".to_string());
        lines.push("- `Directive Decision: #<issue> => close`".to_string());
        lines.push("- `Directive Decision: #<issue> => reopen`".to_string());
    }

    lines.push(END_MARKER.to_string());
    lines.join("\n")
}
