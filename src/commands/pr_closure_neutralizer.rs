use std::collections::HashMap;

use crate::{
    commands::{CommandArgs, GitHubActions, GitHubCli, ReferenceKind, ReferenceNumber},
    regex::{CHECKBOX_REGEX, ISSUE_REF_REGEX, REJECTED_REGEX},
};

pub(crate) struct PrClosureNeutralizer {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
    github_actions: GitHubActions,
}

impl PrClosureNeutralizer {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
            github_actions: GitHubActions,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let pr = self.args.resolve_pull_request_number("--pr")?;
        let pr_number = pr.as_string();
        let marker = format!("<!-- closure-neutralizer:{pr_number} -->");
        let original_body = self
            .github
            .read_pull_request_field(&repo, &pr_number, "body")?;

        if original_body.trim().is_empty() {
            return Err(format!("Error: unable to read PR {}.", pr.show_number()));
        }

        let refs = extract_closing_refs(&original_body)?;
        let mut updated_body = original_body.clone();
        let mut neutralized: HashMap<String, (&'static str, String)> = HashMap::new();

        for issue in refs {
            let issue_number = issue.as_string();
            if self
                .github
                .issue_has_label(&repo, &issue_number, "issue-required-missing")?
            {
                updated_body = mark_closure_rejected(&updated_body, &issue.show_number())?;
                neutralized.insert(
                    issue.show_number(),
                    (
                        "Closes",
                        "label issue-required-missing is set on issue".to_string(),
                    ),
                );
            } else {
                updated_body = unmark_closure_rejected(&updated_body, &issue.show_number())?;
            }
        }

        if updated_body != original_body {
            self.github
                .update_pull_request_body(&repo, &pr_number, &updated_body)?;
        }

        let comment_body = if neutralized.is_empty() {
            format!(
                "{marker}\n### Closure Neutralization Status\n\n✅ No non-compliant closure refs detected. No neutralization applied."
            )
        } else {
            let mut body = format!(
                "{marker}\n### Closure Neutralization Status\n\n⚠️ Non-compliant issue references were neutralized to prevent incorrect auto-close.\n\n"
            );

            let mut issues = neutralized.keys().cloned().collect::<Vec<_>>();
            issues.sort_by_key(|issue| issue.trim_start_matches('#').parse::<u64>().unwrap_or(0));

            for issue in issues {
                if let Some((action, reason)) = neutralized.get(&issue) {
                    body.push_str(&format!("- {action} rejected {issue}: {reason}\n"));
                }
            }

            body.push_str(
                "\nHow to restore standard auto-close:\n- Fix issue required fields/title contract (if applicable).\n- Remove or adjust `Reopen #...` for issues that should close now.\n- Remove `rejected` from closure lines in PR body.",
            );
            body
        };

        self.github
            .upsert_issue_comment_by_marker(&repo, &pr_number, &marker, &comment_body)?;
        println!(
            "Closure neutralization evaluated for PR {}.",
            pr.show_number()
        );
        Ok(())
    }

    pub(crate) fn reevaluate(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let issue = self.args.resolve_issue_number("--issue")?;
        let issue_key = issue.show_number();
        let pr_rows = self.github.list_open_pull_request_rows(&repo)?;

        let mut evaluated_count = 0usize;

        for (pr_number, pr_body) in pr_rows {
            if references_issue_in_closure_line(&pr_body, &issue_key)? {
                let reevaluate_args = CommandArgs::new(vec![
                    "automation".to_string(),
                    "closure-neutralizer".to_string(),
                    "--pr".to_string(),
                    pr_number.clone(),
                    "--repo".to_string(),
                    repo.clone(),
                ]);
                Self::new(reevaluate_args).run()?;
                evaluated_count += 1;
            }
        }

        if evaluated_count == 0 {
            println!(
                "No open PRs found referencing issue {}.",
                issue.show_number()
            );
        } else {
            println!("Re-evaluation complete. {evaluated_count} PR(s) evaluated.");
        }

        Ok(())
    }

    pub(crate) fn write_skip_check_output(&self) -> Result<(), String> {
        let previous = self.args.extract_flag_value_or("--previous-body", "");
        let current = self.args.extract_flag_value_or("--current-body", "");
        let skip = normalize_skip_body(&previous)? == normalize_skip_body(&current)?;
        self.github_actions
            .write_output("skip", if skip { "true" } else { "false" })
    }
}

fn extract_closing_refs(body: &str) -> Result<Vec<ReferenceNumber>, String> {
    let regex = ISSUE_REF_REGEX
        .as_ref()
        .map_err(|e| format!("regex compile error: {e}"))?;

    let mut seen = HashMap::new();

    for capture in regex.captures_iter(body) {
        let Some(number) = capture.get(3).map(|m| m.as_str()) else {
            continue;
        };
        seen.entry(number.to_string()).or_insert_with(|| {
            ReferenceNumber::parse(number).map(|r| r.with_kind(ReferenceKind::Issue))
        });
    }

    Ok(seen.into_values().flatten().collect())
}

fn mark_closure_rejected(body: &str, issue: &str) -> Result<String, String> {
    let pattern = regex::Regex::new(&format!(
        r"(?i)\b(?P<kw>(?:closes|fixes))\b(?P<ws>\s+)(?P<rej>rejected\s+)?(?P<ref>[^\s]*{})\b",
        regex::escape(issue)
    ))
    .map_err(|e| format!("regex compile error: {e}"))?;

    Ok(pattern
        .replace_all(body, |captures: &regex::Captures<'_>| {
            let kw = captures.name("kw").map(|m| m.as_str()).unwrap_or("Closes");
            let ws = captures.name("ws").map(|m| m.as_str()).unwrap_or(" ");
            let rej = captures.name("rej").map(|m| m.as_str()).unwrap_or("");
            let ref_value = captures.name("ref").map(|m| m.as_str()).unwrap_or(issue);
            if rej.is_empty() {
                format!("{kw}{ws}rejected {ref_value}")
            } else {
                format!("{kw}{ws}{rej}{ref_value}")
            }
        })
        .to_string())
}

fn unmark_closure_rejected(body: &str, issue: &str) -> Result<String, String> {
    let pattern = regex::Regex::new(&format!(
        r"(?i)\b(?P<kw>(?:closes|fixes))\b(?P<ws>\s+)rejected\s+(?P<ref>[^\s]*{})\b",
        regex::escape(issue)
    ))
    .map_err(|e| format!("regex compile error: {e}"))?;

    Ok(pattern
        .replace_all(body, |captures: &regex::Captures<'_>| {
            let kw = captures.name("kw").map(|m| m.as_str()).unwrap_or("Closes");
            let ws = captures.name("ws").map(|m| m.as_str()).unwrap_or(" ");
            let ref_value = captures.name("ref").map(|m| m.as_str()).unwrap_or(issue);
            format!("{kw}{ws}{ref_value}")
        })
        .to_string())
}

fn references_issue_in_closure_line(body: &str, issue: &str) -> Result<bool, String> {
    let pattern = regex::Regex::new(&format!(
        r"(?i)(closes|fixes)[[:space:]]+(rejected[[:space:]]+)?{}",
        regex::escape(issue)
    ))
    .map_err(|e| format!("regex compile error: {e}"))?;

    Ok(pattern.is_match(body))
}

fn normalize_skip_body(body: &str) -> Result<String, String> {
    let checkbox_regex = CHECKBOX_REGEX
        .as_ref()
        .map_err(|e| format!("regex compile error: {e}"))?;
    let rejected_regex = REJECTED_REGEX
        .as_ref()
        .map_err(|e| format!("regex compile error: {e}"))?;

    let normalized = checkbox_regex.replace_all(body, "- [_]");
    Ok(rejected_regex.replace_all(&normalized, "$1 $2").to_string())
}
