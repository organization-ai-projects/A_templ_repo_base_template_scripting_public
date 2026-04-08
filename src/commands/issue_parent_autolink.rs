use serde_json::Value;

use crate::commands::{CommandArgs, GitHubCli, ReferenceKind, ReferenceNumber};

const REQUIRED_MISSING_LABEL: &str = "issue-required-missing";
const AUTOMATION_FAILED_LABEL: &str = "automation-failed";

pub(crate) struct IssueParentAutolink {
    pub(crate) args: CommandArgs,
    github: GitHubCli,
}

impl IssueParentAutolink {
    pub(crate) fn new(args: CommandArgs) -> Self {
        Self {
            args,
            github: GitHubCli,
        }
    }

    pub(crate) fn run(&self) -> Result<(), String> {
        let repo = self.args.resolve_repo()?;
        let issue = self.resolve_issue_number()?;
        let issue_number = issue.as_string();
        let marker = format!("<!-- parent-field-autolink:{issue_number} -->");

        let issue_details = self
            .github
            .read_issue_details(&repo, &issue_number)?
            .ok_or_else(|| format!("Unable to read issue {}.", issue.show_number()))?;

        let parent_raw = extract_parent_raw(&issue_details.body);

        if parent_raw.is_empty() {
            self.set_validation_error(
                &repo,
                &issue_number,
                &marker,
                "Missing required field `Parent:` in issue body.",
                "Expected format:\n\n- `Parent: #<issue_number>` for child issues\n- `Parent: none` for independent issues\n- `Parent: base` for cascade root issues\n- `Parent: epic` for epic umbrella issues",
            )?;
            return Ok(());
        }

        let parent_raw_lc = parent_raw.to_ascii_lowercase();

        if matches!(parent_raw_lc.as_str(), "none" | "base" | "epic") {
            return self.handle_no_parent_requested(&repo, &issue_number, &marker, &parent_raw_lc);
        }

        if !parent_raw.starts_with('#') || parent_raw.len() <= 1 {
            self.set_validation_error(
                &repo,
                &issue_number,
                &marker,
                &format!("Invalid `Parent:` value: `{parent_raw}`."),
                "Expected `Parent: #<issue_number>` or one of `Parent: none|base|epic`.",
            )?;
            return Ok(());
        }

        let Some(parent) =
            ReferenceNumber::parse(&parent_raw).map(|r| r.with_kind(ReferenceKind::Issue))
        else {
            self.set_validation_error(
                &repo,
                &issue_number,
                &marker,
                &format!("Invalid `Parent:` value: `{parent_raw}`."),
                "Expected `Parent: #<issue_number>` or one of `Parent: none|base|epic`.",
            )?;
            return Ok(());
        };

        if parent.as_string() == issue_number {
            self.set_validation_error(
                &repo,
                &issue_number,
                &marker,
                &format!(
                    "Issue cannot reference itself as parent (`Parent: {}`).",
                    parent.show_number()
                ),
                "Use another parent issue number or `Parent: none`.",
            )?;
            return Ok(());
        }

        let parent_details = match self.github.read_issue_details(&repo, &parent.as_string())? {
            Some(details) => details,
            None => {
                self.set_validation_error(
                    &repo,
                    &issue_number,
                    &marker,
                    &format!("Parent issue `{}` was not found.", parent.show_number()),
                    "Use an existing issue number in `Parent:`.",
                )?;
                return Ok(());
            }
        };

        if parent_details.state != "OPEN" {
            self.set_validation_error(
                &repo,
                &issue_number,
                &marker,
                &format!(
                    "Parent issue `{}` is not open (state: {}).",
                    parent.show_number(),
                    parent_details.state
                ),
                "Reopen the parent or choose another open parent issue.",
            )?;
            return Ok(());
        }

        let relation = self.github.query_issue_parent_child_relation(
            &repo,
            &issue_number,
            &parent.as_string(),
        )?;

        if let Some(errors) = graphql_error_messages(&relation) {
            self.set_runtime_error(
                &repo,
                &issue_number,
                &marker,
                "GitHub GraphQL query returned errors while reading relation state.",
                &format!("API errors: {errors}"),
            )?;
            return Ok(());
        }

        let current_parent_number = graphql_string(
            &relation,
            &["data", "repository", "child", "parent", "number"],
        );
        let current_parent_node_id =
            graphql_string(&relation, &["data", "repository", "child", "parent", "id"]);
        let child_node_id = graphql_string(&relation, &["data", "repository", "child", "id"]);
        let parent_node_id = graphql_string(&relation, &["data", "repository", "parent", "id"]);

        if current_parent_number == parent.as_string() {
            self.set_success(
                &repo,
                &issue_number,
                &marker,
                &format!("Issue already linked to parent {}.", parent.show_number()),
            )?;
            return Ok(());
        }

        if !current_parent_number.is_empty() && current_parent_number != parent.as_string() {
            if current_parent_node_id.is_empty() || child_node_id.is_empty() {
                self.set_runtime_error(
                    &repo,
                    &issue_number,
                    &marker,
                    &format!(
                        "Missing node IDs required to re-parent issue from #{} to {}.",
                        current_parent_number,
                        parent.show_number()
                    ),
                    "Retry later. If this persists, update parent linkage manually in GitHub UI.",
                )?;
                return Ok(());
            }

            let unlink_result = self
                .github
                .remove_sub_issue_relation(&current_parent_node_id, &child_node_id)?;
            if let Some(errors) = graphql_error_messages(&unlink_result) {
                self.set_runtime_error(
                    &repo,
                    &issue_number,
                    &marker,
                    &format!(
                        "GitHub GraphQL mutation returned errors while unlinking previous parent #{}.",
                        current_parent_number
                    ),
                    &format!("API errors: {errors}"),
                )?;
                return Ok(());
            }
        }

        if child_node_id.is_empty() || parent_node_id.is_empty() {
            self.set_runtime_error(
                &repo,
                &issue_number,
                &marker,
                "Missing GitHub node IDs required for sub-issue linking.",
                "Retry later. If this persists, link parent/child manually in GitHub UI.",
            )?;
            return Ok(());
        }

        let link_result = self
            .github
            .add_sub_issue_relation(&parent_node_id, &child_node_id)?;
        if let Some(errors) = graphql_error_messages(&link_result) {
            self.set_runtime_error(
                &repo,
                &issue_number,
                &marker,
                "GitHub GraphQL mutation returned errors while linking child to parent.",
                &format!("API errors: {errors}"),
            )?;
            return Ok(());
        }

        let success =
            if !current_parent_number.is_empty() && current_parent_number != parent.as_string() {
                format!(
                    "Re-parented this issue from #{} to {}.",
                    current_parent_number,
                    parent.show_number()
                )
            } else {
                format!("Linked this issue as child of {}.", parent.show_number())
            };

        self.set_success(&repo, &issue_number, &marker, &success)?;
        Ok(())
    }

    fn resolve_issue_number(&self) -> Result<ReferenceNumber, String> {
        self.args
            .extract_reference_number("--issue", Some(ReferenceKind::Issue))
            .ok_or_else(|| "Invalid or missing issue.".to_string())
    }

    fn handle_no_parent_requested(
        &self,
        repo: &str,
        issue_number: &str,
        marker: &str,
        parent_raw_lc: &str,
    ) -> Result<(), String> {
        let relation = self
            .github
            .query_issue_child_parent_relation(repo, issue_number)?;

        if let Some(errors) = graphql_error_messages(&relation) {
            self.set_runtime_error(
                repo,
                issue_number,
                marker,
                "GitHub GraphQL query returned errors while reading current parent relation.",
                &format!("API errors: {errors}"),
            )?;
            return Ok(());
        }

        let current_parent_number = graphql_string(
            &relation,
            &["data", "repository", "child", "parent", "number"],
        );
        let current_parent_node_id =
            graphql_string(&relation, &["data", "repository", "child", "parent", "id"]);
        let child_node_id = graphql_string(&relation, &["data", "repository", "child", "id"]);

        if !current_parent_number.is_empty() {
            if current_parent_node_id.is_empty() || child_node_id.is_empty() {
                self.set_runtime_error(
                    repo,
                    issue_number,
                    marker,
                    &format!(
                        "Missing node IDs required to unlink current parent #{}.",
                        current_parent_number
                    ),
                    "Retry later. If this persists, unlink parent manually in GitHub UI.",
                )?;
                return Ok(());
            }

            let unlink_result = self
                .github
                .remove_sub_issue_relation(&current_parent_node_id, &child_node_id)?;
            if let Some(errors) = graphql_error_messages(&unlink_result) {
                self.set_runtime_error(
                    repo,
                    issue_number,
                    marker,
                    &format!(
                        "GitHub GraphQL mutation returned errors while unlinking parent #{}.",
                        current_parent_number
                    ),
                    &format!("API errors: {errors}"),
                )?;
                return Ok(());
            }

            self.set_success(
                repo,
                issue_number,
                marker,
                &format!(
                    "Removed existing parent link #{} (`Parent: {}`).",
                    current_parent_number, parent_raw_lc
                ),
            )?;
        } else {
            self.set_success(
                repo,
                issue_number,
                marker,
                &format!("No parent linking requested (`Parent: {}`).", parent_raw_lc),
            )?;
        }

        Ok(())
    }

    fn set_validation_error(
        &self,
        repo: &str,
        issue_number: &str,
        marker: &str,
        message: &str,
        help_text: &str,
    ) -> Result<(), String> {
        self.github
            .add_label_if_exists(repo, issue_number, REQUIRED_MISSING_LABEL)?;
        self.github
            .remove_label_if_exists(repo, issue_number, AUTOMATION_FAILED_LABEL)?;
        self.github.upsert_issue_comment_by_marker(
            repo,
            issue_number,
            marker,
            &format!("{marker}\n### Parent Field Autolink Status\n\n❌ {message}\n\n{help_text}\n"),
        )
    }

    fn set_runtime_error(
        &self,
        repo: &str,
        issue_number: &str,
        marker: &str,
        message: &str,
        help_text: &str,
    ) -> Result<(), String> {
        self.github
            .add_label_if_exists(repo, issue_number, AUTOMATION_FAILED_LABEL)?;
        self.github.upsert_issue_comment_by_marker(
            repo,
            issue_number,
            marker,
            &format!("{marker}\n### Parent Field Autolink Status\n\n⚠️ {message}\n\n{help_text}\n"),
        )
    }

    fn set_success(
        &self,
        repo: &str,
        issue_number: &str,
        marker: &str,
        message: &str,
    ) -> Result<(), String> {
        self.github
            .remove_label_if_exists(repo, issue_number, REQUIRED_MISSING_LABEL)?;
        self.github
            .remove_label_if_exists(repo, issue_number, AUTOMATION_FAILED_LABEL)?;
        self.github.upsert_issue_comment_by_marker(
            repo,
            issue_number,
            marker,
            &format!("{marker}\n### Parent Field Autolink Status\n\n✅ {message}\n"),
        )
    }
}

fn extract_parent_raw(body: &str) -> String {
    for line in body.lines() {
        if line
            .trim_start()
            .to_ascii_lowercase()
            .starts_with("parent:")
        {
            return line
                .split_once(':')
                .map(|(_, value)| value.split_whitespace().collect::<String>())
                .unwrap_or_default();
        }
    }

    String::new()
}

fn graphql_error_messages(payload: &Value) -> Option<String> {
    let errors = payload.get("errors")?.as_array()?;
    if errors.is_empty() {
        return None;
    }

    let messages = errors
        .iter()
        .filter_map(|error| error.get("message").and_then(Value::as_str))
        .collect::<Vec<_>>();

    if messages.is_empty() {
        Some("Unknown GraphQL error.".to_string())
    } else {
        Some(messages.join(" | "))
    }
}

fn graphql_string(payload: &Value, path: &[&str]) -> String {
    let mut current = payload;

    for key in path {
        let Some(next) = current.get(*key) else {
            return String::new();
        };
        current = next;
    }

    match current {
        Value::String(value) => value.clone(),
        Value::Number(value) => value.to_string(),
        _ => String::new(),
    }
}
